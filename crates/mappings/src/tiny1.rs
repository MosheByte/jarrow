//! Tiny v1: the older Fabric format (`v1\t<namespaces>`, then `CLASS`, `FIELD` and `METHOD`
//! lines). Members name their owner in the primary namespace and may come before the
//! owner's `CLASS` line. Lines starting with `#` (such as intermediary's counters) are
//! comments. An empty name means the entry has no name in that namespace.

use crate::Format;
use crate::error::{Error, Result};
use crate::model::{FieldMapping, MappingSet, MethodMapping};
use crate::text::{numbered_lines, utf8};

const F: Format = Format::TinyV1;

/// Parses tiny v1 from bytes.
pub fn parse_bytes(bytes: &[u8]) -> Result<MappingSet> {
    parse(utf8(bytes)?)
}

/// Parses tiny v1 text.
pub fn parse(text: &str) -> Result<MappingSet> {
    let mut lines = numbered_lines(text);
    let Some((_, header)) = lines.next() else {
        return Err(Error::syntax(F, 1, "empty input"));
    };
    let cols: Vec<&str> = header.split('\t').collect();
    if cols.len() < 2 || cols[0] != "v1" {
        return Err(Error::syntax(
            F,
            1,
            "expected the header 'v1<TAB><namespaces>'",
        ));
    }
    let mut set = MappingSet::new(&cols[1..]).map_err(|e| Error::syntax(F, 1, e.to_string()))?;
    let n = cols.len() - 1;
    for (ln, line) in lines {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = |e: Error| Error::syntax(F, ln, e.to_string());
        let cols: Vec<&str> = line.split('\t').collect();
        let names = |from: usize| crate::model::names(cols[from..].iter());
        match cols[0] {
            "CLASS" => {
                if cols.len() != n + 1 {
                    return Err(Error::syntax(
                        F,
                        ln,
                        "a CLASS line has one name per namespace",
                    ));
                }
                set.add_class(names(1)).map_err(at)?;
            }
            kind @ ("FIELD" | "METHOD") => {
                if cols.len() != n + 3 {
                    return Err(Error::syntax(
                        F,
                        ln,
                        "a member line has an owner, a descriptor and one name per namespace",
                    ));
                }
                let mut owner = vec![None; n];
                owner[0] = Some(cols[1].to_owned());
                let ci = set.add_class(owner).map_err(at)?;
                if kind == "FIELD" {
                    set.add_field(ci, FieldMapping::new(names(3), Some(cols[2].to_owned())))
                        .map_err(at)?;
                } else {
                    set.add_method(ci, MethodMapping::new(names(3), cols[2]))
                        .map_err(at)?;
                }
            }
            other => {
                return Err(Error::syntax(F, ln, format!("unknown line type {other:?}")));
            }
        }
    }
    Ok(set)
}

fn check_name(s: &str) -> Result<()> {
    if s.contains(['\t', '\n', '\r']) {
        Err(Error::unrepresentable(
            F,
            format!("{s:?} contains a tab or a line break"),
        ))
    } else {
        Ok(())
    }
}

fn push_names(out: &mut String, names: &[Option<String>]) -> Result<()> {
    for n in names {
        out.push('\t');
        if let Some(n) = n {
            check_name(n)?;
            out.push_str(n);
        }
    }
    out.push('\n');
    Ok(())
}

/// Writes tiny v1. Fails for package mappings, fields without a descriptor and names with
/// tabs or line breaks. Comments, parameters and local variables have no place in tiny v1
/// and are left out.
pub fn write(set: &MappingSet) -> Result<String> {
    if !set.packages().is_empty() {
        return Err(Error::unrepresentable(F, "tiny v1 has no package mappings"));
    }
    let mut out = String::from("v1");
    for ns in set.namespaces() {
        out.push('\t');
        out.push_str(ns);
    }
    out.push('\n');
    for c in set.classes() {
        out.push_str("CLASS");
        push_names(&mut out, c.names())?;
        for f in c.fields() {
            let Some(desc) = f.desc() else {
                return Err(Error::unrepresentable(
                    F,
                    format!("field {}.{} has no descriptor", c.primary(), f.primary()),
                ));
            };
            out.push_str("FIELD\t");
            out.push_str(c.primary());
            out.push('\t');
            out.push_str(desc);
            push_names(&mut out, f.names())?;
        }
        for m in c.methods() {
            out.push_str("METHOD\t");
            out.push_str(c.primary());
            out.push('\t');
            out.push_str(m.desc());
            push_names(&mut out, m.names())?;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_before_their_class() {
        let text = "v1\tobf\tintermediary\n# INTERMEDIARY-COUNTER class 3\nFIELD\ta\tI\tb\tfield_1\nCLASS\ta\tnet/minecraft/class_1\n";
        let set = parse(text).unwrap();
        assert_eq!(
            set.class("a").unwrap().name(1),
            Some("net/minecraft/class_1")
        );
        assert_eq!(
            set.field("a", "b", Some("I")).unwrap().name(1),
            Some("field_1")
        );
    }

    #[test]
    fn errors() {
        for bad in [
            "",
            "v2\ta\tb",
            "v1",
            "v1\ta\tb\nCLASS\tx",
            "v1\ta\tb\nFIELD\tx\tI\ty",
            "v1\ta\tb\nMETHOD\tx\tI\ty\tz",
            "v1\ta\tb\nPACKAGE\tx\ty",
            "v1\ta\tb\nCLASS\t\ty",
        ] {
            assert!(parse(bad).is_err(), "{bad:?} should fail");
        }
    }
}
