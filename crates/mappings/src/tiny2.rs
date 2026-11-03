//! Tiny v2: the format of intermediary and Yarn (`tiny\t2\t0\t<namespaces>`).
//!
//! Supported: header properties (with `escaped-names`), classes, fields, methods, parameters
//! (`p`), local variables (`v`) and comments (`c`) at every level, and the escapes `\\`,
//! `\n`, `\r`, `\t` and `\0`. Comments are always escaped; names and descriptors only under
//! `escaped-names`. Unknown sections are skipped together with their children, as the format
//! asks of readers. An empty name means the entry has no name in that namespace.

use crate::Format;
use crate::error::{Error, Result};
use crate::model::{FieldMapping, MappingSet, MethodMapping, Names, ParamMapping, VarMapping};
use crate::text::{numbered_lines, parse_u32, tab_depth, utf8};

const F: Format = Format::TinyV2;
const ESCAPED_NAMES: &str = "escaped-names";

/// Parses tiny v2 from bytes (UTF-8, an optional byte-order mark).
pub fn parse_bytes(bytes: &[u8]) -> Result<MappingSet> {
    parse(utf8(bytes)?)
}

enum Member {
    Field(usize),
    Method(usize),
}

enum Child {
    Param(usize),
    Var(usize),
}

/// Parses tiny v2 text.
pub fn parse(text: &str) -> Result<MappingSet> {
    let mut lines = numbered_lines(text);
    let Some((_, header)) = lines.next() else {
        return Err(Error::syntax(F, 1, "empty input"));
    };
    let cols: Vec<&str> = header.split('\t').collect();
    if cols.len() < 4 || cols[0] != "tiny" {
        return Err(Error::syntax(
            F,
            1,
            "expected the header 'tiny<TAB>2<TAB>0<TAB><namespaces>'",
        ));
    }
    if cols[1] != "2" || cols[2] != "0" {
        return Err(Error::syntax(
            F,
            1,
            format!("unsupported tiny version {}.{}", cols[1], cols[2]),
        ));
    }
    let mut set = MappingSet::new(&cols[3..]).map_err(|e| Error::syntax(F, 1, e.to_string()))?;
    let n = cols.len() - 3;

    let mut escaped = false;
    let mut in_header = true;
    let mut class: Option<usize> = None;
    let mut member: Option<Member> = None;
    let mut child: Option<Child> = None;
    let mut skip_deeper_than: Option<usize> = None;

    for (ln, line) in lines {
        if line.is_empty() {
            continue;
        }
        let depth = tab_depth(line);
        if let Some(d) = skip_deeper_than {
            if depth > d {
                continue;
            }
            skip_deeper_than = None;
        }
        let at = |e: Error| Error::syntax(F, ln, e.to_string());
        let cols: Vec<&str> = line[depth..].split('\t').collect();
        let key = cols[0];
        if depth == 0 {
            in_header = false;
        }
        if in_header && depth == 1 {
            if key.is_empty() || cols.len() > 2 {
                return Err(Error::syntax(
                    F,
                    ln,
                    "a property is a key and an optional value",
                ));
            }
            if key == ESCAPED_NAMES {
                escaped = true;
            } else {
                set.set_property(key, cols.get(1).map(|v| (*v).to_owned()));
            }
            continue;
        }
        match depth {
            0 => {
                member = None;
                child = None;
                if key != "c" {
                    class = None;
                    skip_deeper_than = Some(0);
                    continue;
                }
                expect_cols(&cols, n + 1, ln, "a class line has one name per namespace")?;
                let names = names_from(&cols[1..], escaped, ln)?;
                class = Some(set.add_class(names).map_err(at)?);
            }
            1 => {
                let Some(ci) = class else {
                    return Err(Error::syntax(F, ln, "member line outside a class"));
                };
                child = None;
                match key {
                    "f" | "m" => {
                        expect_cols(
                            &cols,
                            n + 2,
                            ln,
                            "a member line has a descriptor and one name per namespace",
                        )?;
                        let desc = name_text(cols[1], escaped, ln)?;
                        let names = names_from(&cols[2..], escaped, ln)?;
                        member = Some(if key == "f" {
                            let field = FieldMapping::new(names, Some(desc));
                            Member::Field(set.add_field(ci, field).map_err(at)?)
                        } else {
                            let method = MethodMapping::new(names, desc);
                            Member::Method(set.add_method(ci, method).map_err(at)?)
                        });
                    }
                    "c" => {
                        member = None;
                        let comment = comment_from(&cols, ln)?;
                        if let Some(c) = set.class_at_mut(ci) {
                            c.set_comment(Some(comment));
                        }
                    }
                    _ => {
                        member = None;
                        skip_deeper_than = Some(1);
                    }
                }
            }
            2 => {
                let (Some(ci), Some(m)) = (class, &member) else {
                    return Err(Error::syntax(F, ln, "line nested under nothing"));
                };
                match (key, m) {
                    ("p", &Member::Method(mi)) => {
                        expect_cols(
                            &cols,
                            n + 2,
                            ln,
                            "a parameter line has an index and one name per namespace",
                        )?;
                        let lv = parse_u32(cols[1])
                            .ok_or_else(|| Error::syntax(F, ln, "bad parameter index"))?;
                        let names = names_from(&cols[2..], escaped, ln)?;
                        let pi = set
                            .add_param(ci, mi, ParamMapping::new(lv, names))
                            .map_err(at)?;
                        child = Some(Child::Param(pi));
                    }
                    ("v", &Member::Method(mi)) => {
                        expect_cols(
                            &cols,
                            n + 4,
                            ln,
                            "a variable line has an index, a start offset, an LVT index and one name per namespace",
                        )?;
                        let bad = || Error::syntax(F, ln, "bad local variable numbers");
                        let lv = parse_u32(cols[1]).ok_or_else(bad)?;
                        let start = parse_u32(cols[2]).ok_or_else(bad)?;
                        let lvt = match cols[3] {
                            "-1" => None,
                            other => Some(parse_u32(other).ok_or_else(bad)?),
                        };
                        let names = names_from(&cols[4..], escaped, ln)?;
                        let vi = set
                            .add_var(ci, mi, VarMapping::new(lv, start, lvt, names))
                            .map_err(at)?;
                        child = Some(Child::Var(vi));
                    }
                    ("c", m) => {
                        child = None;
                        let comment = Some(comment_from(&cols, ln)?);
                        let Some(c) = set.class_at_mut(ci) else {
                            continue;
                        };
                        match *m {
                            Member::Field(fi) => {
                                if let Some(f) = c.field_at_mut(fi) {
                                    f.set_comment(comment);
                                }
                            }
                            Member::Method(mi) => {
                                if let Some(m) = c.method_at_mut(mi) {
                                    m.set_comment(comment);
                                }
                            }
                        }
                    }
                    ("p" | "v", Member::Field(_)) => {
                        return Err(Error::syntax(
                            F,
                            ln,
                            "a field has no parameters or variables",
                        ));
                    }
                    _ => {
                        child = None;
                        skip_deeper_than = Some(2);
                    }
                }
            }
            3 => {
                let (Some(ci), Some(Member::Method(mi)), Some(ch)) = (class, &member, &child)
                else {
                    return Err(Error::syntax(F, ln, "line nested under nothing"));
                };
                if key != "c" {
                    skip_deeper_than = Some(3);
                    continue;
                }
                let comment = Some(comment_from(&cols, ln)?);
                let method = set.class_at_mut(ci).and_then(|c| c.method_at_mut(*mi));
                if let Some(m) = method {
                    match *ch {
                        Child::Param(pi) => {
                            if let Some(p) = m.param_at_mut(pi) {
                                p.set_comment(comment);
                            }
                        }
                        Child::Var(vi) => {
                            if let Some(v) = m.var_at_mut(vi) {
                                v.set_comment(comment);
                            }
                        }
                    }
                }
            }
            _ => return Err(Error::syntax(F, ln, "indented too deep")),
        }
    }
    Ok(set)
}

fn expect_cols(cols: &[&str], expected: usize, line: usize, message: &str) -> Result<()> {
    if cols.len() == expected {
        Ok(())
    } else {
        Err(Error::syntax(
            F,
            line,
            format!(
                "{message} ({expected} columns expected, {} found)",
                cols.len()
            ),
        ))
    }
}

fn name_text(col: &str, escaped: bool, line: usize) -> Result<String> {
    if escaped {
        unescape(col).ok_or_else(|| Error::syntax(F, line, "bad escape sequence"))
    } else {
        Ok(col.to_owned())
    }
}

fn names_from(cols: &[&str], escaped: bool, line: usize) -> Result<Names> {
    cols.iter()
        .map(|c| {
            if c.is_empty() {
                Ok(None)
            } else {
                name_text(c, escaped, line).map(Some)
            }
        })
        .collect()
}

fn comment_from(cols: &[&str], line: usize) -> Result<String> {
    expect_cols(
        cols,
        2,
        line,
        "a comment line is 'c' and the escaped comment",
    )?;
    unescape(cols[1]).ok_or_else(|| Error::syntax(F, line, "bad escape sequence in a comment"))
}

/// Undoes tiny v2 escaping. `None` on an unknown escape or a trailing backslash.
pub(crate) fn unescape(s: &str) -> Option<String> {
    if !s.contains('\\') {
        return Some(s.to_owned());
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            '\\' => '\\',
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '\0',
            _ => return None,
        });
    }
    Some(out)
}

fn needs_escape(s: &str) -> bool {
    s.contains(['\\', '\n', '\r', '\t', '\0'])
}

/// Applies tiny v2 escaping.
pub(crate) fn escape(s: &str) -> String {
    if !needs_escape(s) {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' => out.push_str("\\0"),
            c => out.push(c),
        }
    }
    out
}

fn any_name_needs_escape(set: &MappingSet) -> bool {
    let names = |n: &[Option<String>]| n.iter().flatten().any(|s| needs_escape(s));
    set.classes().iter().any(|c| {
        names(c.names())
            || c.fields()
                .iter()
                .any(|f| names(f.names()) || f.desc().is_some_and(needs_escape))
            || c.methods().iter().any(|m| {
                names(m.names())
                    || needs_escape(m.desc())
                    || m.params().iter().any(|p| names(p.names()))
                    || m.vars().iter().any(|v| names(v.names()))
            })
    })
}

/// Writes tiny v2. Fails for package mappings and for fields without a descriptor, which the
/// format cannot hold. ProGuard line ranges, inline frames and TSRG2 static markers have no
/// place in tiny v2 and are left out.
pub fn write(set: &MappingSet) -> Result<String> {
    if !set.packages().is_empty() {
        return Err(Error::unrepresentable(F, "tiny v2 has no package mappings"));
    }
    let escaped = any_name_needs_escape(set);
    let esc = |s: &str| if escaped { escape(s) } else { s.to_owned() };
    let mut out = String::new();
    out.push_str("tiny\t2\t0");
    for ns in set.namespaces() {
        out.push('\t');
        out.push_str(ns);
    }
    out.push('\n');
    if escaped {
        out.push('\t');
        out.push_str(ESCAPED_NAMES);
        out.push('\n');
    }
    for (key, value) in set.properties() {
        if key == ESCAPED_NAMES {
            continue;
        }
        if key.is_empty() || needs_escape(key) || value.as_deref().is_some_and(needs_escape) {
            return Err(Error::unrepresentable(
                F,
                format!("property {key:?} has tabs or line breaks"),
            ));
        }
        out.push('\t');
        out.push_str(key);
        if let Some(v) = value {
            out.push('\t');
            out.push_str(v);
        }
        out.push('\n');
    }
    let push_names = |out: &mut String, names: &[Option<String>]| {
        for n in names {
            out.push('\t');
            if let Some(n) = n {
                out.push_str(&esc(n));
            }
        }
        out.push('\n');
    };
    let push_comment = |out: &mut String, depth: usize, comment: Option<&str>| {
        if let Some(c) = comment {
            out.extend(std::iter::repeat_n('\t', depth));
            out.push_str("c\t");
            out.push_str(&escape(c));
            out.push('\n');
        }
    };
    for c in set.classes() {
        out.push('c');
        push_names(&mut out, c.names());
        push_comment(&mut out, 1, c.comment());
        for f in c.fields() {
            let Some(desc) = f.desc() else {
                return Err(Error::unrepresentable(
                    F,
                    format!("field {}.{} has no descriptor", c.primary(), f.primary()),
                ));
            };
            out.push_str("\tf\t");
            out.push_str(&esc(desc));
            push_names(&mut out, f.names());
            push_comment(&mut out, 2, f.comment());
        }
        for m in c.methods() {
            out.push_str("\tm\t");
            out.push_str(&esc(m.desc()));
            push_names(&mut out, m.names());
            push_comment(&mut out, 2, m.comment());
            for p in m.params() {
                out.push_str("\t\tp\t");
                out.push_str(&p.lv_index().to_string());
                push_names(&mut out, p.names());
                push_comment(&mut out, 3, p.comment());
            }
            for v in m.vars() {
                out.push_str("\t\tv\t");
                out.push_str(&v.lv_index().to_string());
                out.push('\t');
                out.push_str(&v.start_offset().to_string());
                out.push('\t');
                match v.lvt_index() {
                    Some(i) => out.push_str(&i.to_string()),
                    None => out.push_str("-1"),
                }
                push_names(&mut out, v.names());
                push_comment(&mut out, 3, v.comment());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_round_trip() {
        for s in [
            "plain",
            "a\\b",
            "tab\there",
            "line\nbreak",
            "cr\r",
            "nul\0",
            "\\n literal",
        ] {
            assert_eq!(unescape(&escape(s)).unwrap(), s);
        }
        assert_eq!(unescape("bad\\x"), None);
        assert_eq!(unescape("trailing\\"), None);
    }

    #[test]
    fn header_errors() {
        for bad in [
            "",
            "tiny\t2\t0",
            "tiny\t1\t0\ta\tb",
            "tiny\t2\t1\ta\tb",
            "v1\ta\tb",
            "tiny\t2\t0\ta\ta",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn structure_errors() {
        let head = "tiny\t2\t0\ta\tb\n";
        for body in [
            "\tf\tI\tx\ty\n",
            "c\tA\n",
            "c\tA\tB\tC\n",
            "c\t\tB\n",
            "c\tA\tB\n\tf\tQ\tx\ty\n",
            "c\tA\tB\n\tm\t()V\tx\n",
            "c\tA\tB\n\t\tp\t1\tx\ty\n",
            "c\tA\tB\n\tf\tI\tx\ty\n\t\tp\t1\tx\ty\n",
            "c\tA\tB\n\tm\t()V\tx\ty\n\t\tp\tz\tx\ty\n",
            "c\tA\tB\n\tm\t()V\tx\ty\n\t\tv\t1\t2\tx\ty\n",
            "c\tA\tB\n\tm\t()V\tx\ty\n\t\tv\t1\t2\t-2\tx\ty\n",
            "c\tA\tB\n\tc\tbad\\q\n",
            "c\tA\tB\n\t\t\t\tc\tdeep\n",
            "c\tA\tB\nc\tA\tC\n",
        ] {
            let text = format!("{head}{body}");
            assert!(parse(&text).is_err(), "{body:?} should fail");
        }
    }

    #[test]
    fn unknown_sections_are_skipped() {
        let text = "tiny\t2\t0\ta\tb\n\tsome-property\tvalue\nx\tunknown\n\tchild\nc\tA\tB\n\tz\tq\n\t\tdeeper\n\tf\tI\tx\ty\n\t\tw\n\t\t\tc\tx\n";
        let set = parse(text).unwrap();
        assert_eq!(
            set.properties(),
            [("some-property".to_owned(), Some("value".to_owned()))]
        );
        assert_eq!(set.field("A", "x", Some("I")).unwrap().name(1), Some("y"));
    }
}
