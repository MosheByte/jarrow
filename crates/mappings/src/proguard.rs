//! ProGuard mapping files, the format of Mojang's `client.txt` and `server.txt`.
//!
//! ```text
//! # comment
//! net.minecraft.Outer -> a:
//!     int count -> a
//!     12:14:void tick(net.minecraft.Outer$Inner,int[]) -> b
//!     20:20:void other():30:30 -> c
//! net.minecraft.Outer$Inner -> a$a:
//! ```
//!
//! The left side holds the original (deobfuscated) names and the types in Java source form;
//! the right side holds the obfuscated names. The parser turns that into a two-namespace set,
//! `(original, obfuscated)`, with descriptors in the original namespace. For Mojang's files
//! those namespaces are `mojang` and `obf` ([`parse_mojang`]).
//!
//! Line-number ranges (`start:end:`) and original line numbers (`:origStart[:origEnd]`) are
//! kept on the method. A method listed several times (once per line range) becomes one
//! method with several ranges. A method name qualified with another class
//! (`net.minecraft.Other.helper`) is inlined code, kept as an [`InlineFrame`] of the class.
//! Comment lines (`#`, including R8's JSON metadata) are skipped.

use std::borrow::Cow;

use crate::Format;
use crate::desc;
use crate::error::{Error, Result};
use crate::model::{FieldMapping, InlineFrame, LineRange, MappingSet, MethodMapping};
use crate::ns;
use crate::text::{numbered_lines, parse_u32, utf8};

const F: Format = Format::Proguard;

/// Parses a ProGuard file from bytes.
pub fn parse_bytes(bytes: &[u8], original_ns: &str, obfuscated_ns: &str) -> Result<MappingSet> {
    parse(utf8(bytes)?, original_ns, obfuscated_ns)
}

/// Parses one of Mojang's mapping files into the namespaces `(mojang, obf)`.
pub fn parse_mojang(text: &str) -> Result<MappingSet> {
    parse(text, ns::MOJANG, ns::OBF)
}

/// Parses a ProGuard file into the namespaces `(original_ns, obfuscated_ns)`.
pub fn parse(text: &str, original_ns: &str, obfuscated_ns: &str) -> Result<MappingSet> {
    let mut set = MappingSet::new([original_ns, obfuscated_ns])?;
    let mut class: Option<usize> = None;
    for (ln, line) in numbered_lines(text) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let at = |e: Error| Error::syntax(F, ln, e.to_string());
        if !line.starts_with([' ', '\t']) {
            let body = trimmed
                .strip_suffix(':')
                .ok_or_else(|| Error::syntax(F, ln, "a class line ends with ':'"))?;
            let (original, obfuscated) = split_arrow(body, ln)?;
            check_token(original, ln)?;
            let names = vec![Some(internal(original)), Some(internal(obfuscated))];
            class = Some(set.add_class(names).map_err(at)?);
            continue;
        }
        let Some(ci) = class else {
            return Err(Error::syntax(F, ln, "member line before any class"));
        };
        let (left, obfuscated) = split_arrow(trimmed, ln)?;
        if !left.contains('(') {
            let (ty, name) = left
                .split_once(' ')
                .ok_or_else(|| Error::syntax(F, ln, "a field line is 'type name -> obfuscated'"))?;
            check_token(name, ln)?;
            let desc = desc::java_type_to_desc(ty, false).map_err(at)?;
            let names = vec![Some(name.to_owned()), Some(obfuscated.to_owned())];
            set.add_field(ci, FieldMapping::new(names, Some(desc)))
                .map_err(at)?;
            continue;
        }
        let method = parse_method(left, ln)?;
        let desc = desc::java_method_to_desc(method.return_type, method.args).map_err(at)?;
        match method.name.rsplit_once('.') {
            Some((owner, name)) => {
                let Some(lines) = method.lines else {
                    return Err(Error::syntax(F, ln, "an inlined method needs a line range"));
                };
                check_token(name, ln)?;
                set.add_inline_frame(
                    ci,
                    InlineFrame {
                        lines,
                        owner: internal(owner),
                        name: name.to_owned(),
                        desc,
                        obfuscated: obfuscated.to_owned(),
                    },
                )
                .map_err(at)?;
            }
            None => {
                check_token(method.name, ln)?;
                let names = vec![Some(method.name.to_owned()), Some(obfuscated.to_owned())];
                let mut m = MethodMapping::new(names, desc);
                if let Some(lines) = method.lines {
                    m = m.with_lines(lines);
                }
                set.add_method(ci, m).map_err(at)?;
            }
        }
    }
    Ok(set)
}

struct MethodLine<'l> {
    lines: Option<LineRange>,
    return_type: &'l str,
    name: &'l str,
    args: &'l str,
}

fn parse_method(left: &str, ln: usize) -> Result<MethodLine<'_>> {
    let bad = |what: &str| Error::syntax(F, ln, format!("bad method line: {what}"));
    let mut rest = left;
    let mut range = None;
    if rest.starts_with(|c: char| c.is_ascii_digit()) {
        let (start, after) = rest.split_once(':').ok_or_else(|| bad("line range"))?;
        let (end, after) = after.split_once(':').ok_or_else(|| bad("line range"))?;
        let start = parse_u32(start).ok_or_else(|| bad("line range"))?;
        let end = parse_u32(end).ok_or_else(|| bad("line range"))?;
        range = Some((start, end));
        rest = after;
    }
    let (return_type, sig) = rest
        .split_once(' ')
        .ok_or_else(|| bad("missing return type"))?;
    let open = sig.find('(').ok_or_else(|| bad("missing '('"))?;
    let close = sig[open..]
        .find(')')
        .map(|i| open + i)
        .ok_or_else(|| bad("missing ')'"))?;
    let name = &sig[..open];
    let args = &sig[open + 1..close];
    let suffix = &sig[close + 1..];
    let mut original = (None, None);
    if !suffix.is_empty() {
        let numbers = suffix
            .strip_prefix(':')
            .ok_or_else(|| bad("text after ')'"))?;
        let mut parts = numbers.split(':');
        let first = parts
            .next()
            .and_then(parse_u32)
            .ok_or_else(|| bad("original line"))?;
        let second = match parts.next() {
            Some(p) => Some(parse_u32(p).ok_or_else(|| bad("original line"))?),
            None => None,
        };
        if parts.next().is_some() {
            return Err(bad("too many line numbers"));
        }
        original = (Some(first), second);
    }
    let lines = match (range, original) {
        (Some((start, end)), (original_start, original_end)) => Some(LineRange {
            start,
            end,
            original_start,
            original_end,
        }),
        (None, (None, _)) => None,
        (None, (Some(_), _)) => return Err(bad("original lines without a line range")),
    };
    Ok(MethodLine {
        lines,
        return_type,
        name,
        args,
    })
}

fn split_arrow(text: &str, ln: usize) -> Result<(&str, &str)> {
    let (left, right) = text
        .split_once(" -> ")
        .ok_or_else(|| Error::syntax(F, ln, "expected 'original -> obfuscated'"))?;
    let (left, right) = (left.trim(), right.trim());
    if left.is_empty() {
        return Err(Error::syntax(F, ln, "empty original name"));
    }
    check_token(right, ln)?;
    Ok((left, right))
}

fn check_token(s: &str, ln: usize) -> Result<()> {
    if s.is_empty() || s.contains(char::is_whitespace) {
        Err(Error::syntax(F, ln, format!("bad name {s:?}")))
    } else {
        Ok(())
    }
}

fn internal(dotted: &str) -> String {
    dotted.replace('.', "/")
}

fn dotted(internal: &str, what: &str) -> Result<String> {
    if internal.is_empty()
        || internal.contains(['.', '(', ')'])
        || internal.contains(char::is_whitespace)
    {
        return Err(Error::unrepresentable(
            F,
            format!("{what} {internal:?} cannot be written in Java source form"),
        ));
    }
    Ok(internal.replace('/', "."))
}

/// A member name or obfuscated name as it can appear in a ProGuard line: non-empty, no
/// whitespace, none of `forbidden`.
fn token<'s>(
    s: Option<&'s str>,
    forbidden: &[char],
    what: impl FnOnce() -> String,
) -> Result<&'s str> {
    match s {
        Some(s) if !s.is_empty() && !s.contains(char::is_whitespace) && !s.contains(forbidden) => {
            Ok(s)
        }
        _ => Err(Error::unrepresentable(F, what())),
    }
}

/// Writes one of Mojang's mapping files from a set with the namespaces `mojang` and `obf`.
pub fn write_mojang(set: &MappingSet) -> Result<String> {
    write(set, ns::MOJANG, ns::OBF)
}

/// Writes a ProGuard file with `original_ns` on the left and `obfuscated_ns` on the right.
///
/// Every class, field and method needs a name in both namespaces and every field a
/// descriptor; package mappings cannot be written. Comments, parameters, local variables
/// and TSRG2 static markers have no place in the format and are left out.
pub fn write(set: &MappingSet, original_ns: &str, obfuscated_ns: &str) -> Result<String> {
    let view = if set.namespaces() == [original_ns, obfuscated_ns] {
        Cow::Borrowed(set)
    } else {
        Cow::Owned(set.project(&[original_ns, obfuscated_ns])?)
    };
    if !view.packages().is_empty() {
        return Err(Error::unrepresentable(
            F,
            "ProGuard has no package mappings",
        ));
    }
    let mut out = String::new();
    for c in view.classes() {
        let missing = || format!("class {} has no {obfuscated_ns} name", c.primary());
        let obf = c
            .name(1)
            .ok_or_else(|| Error::unrepresentable(F, missing()))?;
        out.push_str(&dotted(c.primary(), "class")?);
        out.push_str(" -> ");
        out.push_str(&dotted(obf, "class")?);
        out.push_str(":\n");
        for f in c.fields() {
            let what = || format!("field {}.{}", c.primary(), f.primary());
            let desc = f.desc().ok_or_else(|| {
                Error::unrepresentable(F, format!("{} has no descriptor", what()))
            })?;
            let name = token(Some(f.primary()), &['('], || {
                format!("{} has an unwritable name", what())
            })?;
            let obf = token(f.name(1), &[], || {
                format!("{} has no usable {obfuscated_ns} name", what())
            })?;
            out.push_str("    ");
            out.push_str(&desc::desc_to_java_type(desc)?);
            out.push(' ');
            out.push_str(name);
            out.push_str(" -> ");
            out.push_str(obf);
            out.push('\n');
        }
        for m in c.methods() {
            let what = || format!("method {}.{}{}", c.primary(), m.primary(), m.desc());
            let name = token(Some(m.primary()), &['.', '(', ')'], || {
                format!("{} has an unwritable name", what())
            })?;
            let obf = token(m.name(1), &[], || {
                format!("{} has no usable {obfuscated_ns} name", what())
            })?;
            let (ret, args) = desc::method_desc_to_java(m.desc())?;
            if m.lines().is_empty() {
                push_method(&mut out, None, &ret, name, &args, obf);
            }
            for r in m.lines() {
                push_method(&mut out, Some(r), &ret, name, &args, obf);
            }
        }
        for frame in c.inline_frames() {
            let (ret, args) = desc::method_desc_to_java(&frame.desc)?;
            let name = format!("{}.{}", dotted(&frame.owner, "class")?, frame.name);
            push_method(
                &mut out,
                Some(&frame.lines),
                &ret,
                &name,
                &args,
                &frame.obfuscated,
            );
        }
    }
    Ok(out)
}

fn push_method(
    out: &mut String,
    lines: Option<&LineRange>,
    ret: &str,
    name: &str,
    args: &str,
    obf: &str,
) {
    out.push_str("    ");
    if let Some(r) = lines {
        out.push_str(&format!("{}:{}:", r.start, r.end));
    }
    out.push_str(ret);
    out.push(' ');
    out.push_str(name);
    out.push('(');
    out.push_str(args);
    out.push(')');
    if let Some(r) = lines
        && let Some(os) = r.original_start
    {
        out.push_str(&format!(":{os}"));
        if let Some(oe) = r.original_end {
            out.push_str(&format!(":{oe}"));
        }
    }
    out.push_str(" -> ");
    out.push_str(obf);
    out.push('\n');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_lines() {
        let m = parse_method("1:2:void a.B.c(int,a.B$C):10:12", 1).unwrap();
        assert_eq!(m.name, "a.B.c");
        assert_eq!(m.args, "int,a.B$C");
        assert_eq!(
            m.lines,
            Some(LineRange {
                start: 1,
                end: 2,
                original_start: Some(10),
                original_end: Some(12)
            })
        );
        let m = parse_method("int size()", 1).unwrap();
        assert_eq!(
            (m.return_type, m.name, m.args, m.lines),
            ("int", "size", "", None)
        );
        for bad in [
            "1:void a()",
            "void a(",
            "voida()",
            "void a():3",
            "1:1:void a():x",
            "1:1:void a():1:2:3",
            "1:1:void a()x",
        ] {
            assert!(parse_method(bad, 1).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn errors() {
        for bad in [
            "    int a -> b",
            "a -> b",
            "a.B -> c:\n    int -> b",
            "a.B -> c:\n    int a -> ",
            "a.B -> c:\n    int a b -> c",
            "a.B -> c:\n    in t a -> b",
            "a.B -> c:\n    void a(int) -> b c",
            "a.B ->c:",
            "a.B -> c:\n    1:2:void x.Y.z() -> b\n    void x.Y.w() -> c",
        ] {
            assert!(parse_mojang(bad).is_err(), "{bad:?} should fail");
        }
    }
}
