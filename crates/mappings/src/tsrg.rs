//! TSRG and TSRG2, the formats of Forge's MCPConfig (obfuscated names to SRG names).
//!
//! TSRG (v1) has no header and exactly two namespaces, which the caller names:
//!
//! ```text
//! net/minecraft/ net/minecraft/
//! a net/minecraft/Foo
//! <TAB>b field_1_b
//! <TAB>c (I)La; func_2_c
//! ```
//!
//! (`<TAB>` stands for one tab character.)
//!
//! TSRG2 starts with `tsrg2 <namespaces>`, gives every name in every namespace, may give a
//! field descriptor, and nests parameters (`\t\t<index> <names>`) and a `static` marker under
//! methods. Package lines end with `/` in both formats; the model keeps package names without
//! it. Tokens are separated by spaces, so no name may contain whitespace.

use crate::Format;
use crate::error::{Error, Result};
use crate::model::{FieldMapping, MappingSet, MethodMapping, ParamMapping};
use crate::text::{numbered_lines, parse_u32, tab_depth, utf8};

/// Parses TSRG (v1) from bytes into the two given namespaces.
pub fn parse_v1_bytes(bytes: &[u8], namespaces: [&str; 2]) -> Result<MappingSet> {
    parse_v1(utf8(bytes)?, namespaces)
}

/// Parses TSRG (v1) text into the two given namespaces (left column first).
pub fn parse_v1(text: &str, namespaces: [&str; 2]) -> Result<MappingSet> {
    const F: Format = Format::Tsrg;
    let mut set = MappingSet::new(namespaces)?;
    let mut class = None;
    for (ln, line) in numbered_lines(text) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let at = |e: Error| Error::syntax(F, ln, e.to_string());
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if !line.starts_with(char::is_whitespace) {
            if tokens.len() != 2 {
                return Err(Error::syntax(
                    F,
                    ln,
                    "a class or package line has two names",
                ));
            }
            if let Some(names) = package_names(&tokens) {
                set.add_package(names).map_err(at)?;
                class = None;
            } else if tokens.iter().any(|t| t.ends_with('/')) {
                return Err(Error::syntax(
                    F,
                    ln,
                    "a package line ends every name with '/'",
                ));
            } else {
                class = Some(set.add_class(crate::names(&tokens)).map_err(at)?);
            }
            continue;
        }
        if tab_depth(line) > 1 {
            return Err(Error::syntax(F, ln, "TSRG v1 has no nested lines"));
        }
        let Some(ci) = class else {
            return Err(Error::syntax(F, ln, "member line outside a class"));
        };
        match tokens.as_slice() {
            [from, to] => {
                set.add_field(ci, FieldMapping::new(crate::names([from, to]), None))
                    .map_err(at)?;
            }
            [from, desc, to] if desc.starts_with('(') => {
                set.add_method(ci, MethodMapping::new(crate::names([from, to]), *desc))
                    .map_err(at)?;
            }
            _ => {
                return Err(Error::syntax(
                    F,
                    ln,
                    "a member line is 'name new' (field) or 'name (desc) new' (method)",
                ));
            }
        }
    }
    Ok(set)
}

/// Package names when every token ends with `/`; the trailing `/` is dropped.
fn package_names(tokens: &[&str]) -> Option<crate::Names> {
    if !tokens.iter().all(|t| t.ends_with('/')) {
        return None;
    }
    Some(crate::names(tokens.iter().map(|t| &t[..t.len() - 1])))
}

/// Parses TSRG2 from bytes.
pub fn parse_v2_bytes(bytes: &[u8]) -> Result<MappingSet> {
    parse_v2(utf8(bytes)?)
}

/// Parses TSRG2 text.
pub fn parse_v2(text: &str) -> Result<MappingSet> {
    const F: Format = Format::Tsrg2;
    let mut lines = numbered_lines(text);
    let Some((_, header)) = lines.next() else {
        return Err(Error::syntax(F, 1, "empty input"));
    };
    let head: Vec<&str> = header.split_whitespace().collect();
    if head.len() < 2 || head[0] != "tsrg2" {
        return Err(Error::syntax(
            F,
            1,
            "expected the header 'tsrg2 <namespaces>'",
        ));
    }
    let mut set = MappingSet::new(&head[1..]).map_err(|e| Error::syntax(F, 1, e.to_string()))?;
    let n = head.len() - 1;
    let mut class = None;
    let mut method = None;
    for (ln, line) in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let at = |e: Error| Error::syntax(F, ln, e.to_string());
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        match tab_depth(line) {
            0 => {
                method = None;
                if tokens.len() != n {
                    return Err(Error::syntax(
                        F,
                        ln,
                        "a class or package line has one name per namespace",
                    ));
                }
                if tokens.iter().any(|t| t.ends_with('/')) {
                    let names = package_names(&tokens).ok_or_else(|| {
                        Error::syntax(F, ln, "a package line ends every name with '/'")
                    })?;
                    set.add_package(names).map_err(at)?;
                    class = None;
                } else {
                    class = Some(set.add_class(crate::names(&tokens)).map_err(at)?);
                }
            }
            1 => {
                let Some(ci) = class else {
                    return Err(Error::syntax(F, ln, "member line outside a class"));
                };
                method = None;
                if tokens.len() == n {
                    set.add_field(ci, FieldMapping::new(crate::names(&tokens), None))
                        .map_err(at)?;
                } else if tokens.len() == n + 1 {
                    let names = crate::names(tokens.iter().take(1).chain(&tokens[2..]));
                    if tokens[1].starts_with('(') {
                        method = Some(
                            set.add_method(ci, MethodMapping::new(names, tokens[1]))
                                .map_err(at)?,
                        );
                    } else {
                        set.add_field(ci, FieldMapping::new(names, Some(tokens[1].to_owned())))
                            .map_err(at)?;
                    }
                } else {
                    return Err(Error::syntax(
                        F,
                        ln,
                        "a member line has one name per namespace and an optional descriptor",
                    ));
                }
            }
            2 => {
                let (Some(ci), Some(mi)) = (class, method) else {
                    return Err(Error::syntax(F, ln, "parameter line outside a method"));
                };
                if tokens == ["static"] {
                    set.set_static(ci, mi);
                } else if tokens.len() == n + 1 {
                    let index = parse_u32(tokens[0])
                        .ok_or_else(|| Error::syntax(F, ln, "bad parameter index"))?;
                    set.add_param(ci, mi, ParamMapping::new(index, crate::names(&tokens[1..])))
                        .map_err(at)?;
                } else {
                    return Err(Error::syntax(
                        F,
                        ln,
                        "a parameter line is an index and one name per namespace",
                    ));
                }
            }
            _ => return Err(Error::syntax(F, ln, "indented too deep")),
        }
    }
    Ok(set)
}

fn push_names(
    out: &mut String,
    format: Format,
    names: &[Option<String>],
    what: impl Fn() -> String,
) -> Result<()> {
    for (i, n) in names.iter().enumerate() {
        let name = match n {
            Some(n) if !n.contains(char::is_whitespace) => n,
            Some(_) => {
                return Err(Error::unrepresentable(
                    format,
                    format!("{} has whitespace in a name", what()),
                ));
            }
            None => {
                return Err(Error::unrepresentable(
                    format,
                    format!("{} is missing a name", what()),
                ));
            }
        };
        if i > 0 {
            out.push(' ');
        }
        out.push_str(name);
    }
    Ok(())
}

/// Writes TSRG (v1). The set must have exactly two namespaces and every entry a name in
/// both. Field descriptors, parameters, comments and line ranges have no place in TSRG and
/// are left out.
pub fn write_v1(set: &MappingSet) -> Result<String> {
    const F: Format = Format::Tsrg;
    if set.namespaces().len() != 2 {
        return Err(Error::unrepresentable(F, "TSRG has exactly two namespaces"));
    }
    let mut out = String::new();
    for p in set.packages() {
        let with_slash: Vec<Option<String>> = p
            .names()
            .iter()
            .map(|n| n.as_ref().map(|s| format!("{s}/")))
            .collect();
        push_names(&mut out, F, &with_slash, || {
            format!("package {}", p.primary())
        })?;
        out.push('\n');
    }
    for c in set.classes() {
        push_names(&mut out, F, c.names(), || format!("class {}", c.primary()))?;
        out.push('\n');
        for f in c.fields() {
            out.push('\t');
            push_names(&mut out, F, f.names(), || {
                format!("field {}.{}", c.primary(), f.primary())
            })?;
            out.push('\n');
        }
        for m in c.methods() {
            let what = || format!("method {}.{}{}", c.primary(), m.primary(), m.desc());
            let (Some(from), Some(to)) = (m.name(0), m.name(1)) else {
                return Err(Error::unrepresentable(
                    F,
                    format!("{} is missing a name", what()),
                ));
            };
            out.push('\t');
            push_names(
                &mut out,
                F,
                &[
                    Some(from.to_owned()),
                    Some(m.desc().to_owned()),
                    Some(to.to_owned()),
                ],
                what,
            )?;
            out.push('\n');
        }
    }
    Ok(out)
}

/// Writes TSRG2. Every class, field, method, parameter and package needs a name in every
/// namespace. Comments, local variables and line ranges have no place in TSRG2 and are left
/// out.
pub fn write_v2(set: &MappingSet) -> Result<String> {
    const F: Format = Format::Tsrg2;
    let mut out = String::from("tsrg2");
    for ns in set.namespaces() {
        out.push(' ');
        out.push_str(ns);
    }
    out.push('\n');
    for p in set.packages() {
        let with_slash: Vec<Option<String>> = p
            .names()
            .iter()
            .map(|n| n.as_ref().map(|s| format!("{s}/")))
            .collect();
        push_names(&mut out, F, &with_slash, || {
            format!("package {}", p.primary())
        })?;
        out.push('\n');
    }
    for c in set.classes() {
        push_names(&mut out, F, c.names(), || format!("class {}", c.primary()))?;
        out.push('\n');
        for f in c.fields() {
            let what = || format!("field {}.{}", c.primary(), f.primary());
            out.push('\t');
            match f.desc() {
                Some(desc) => {
                    let mut names = vec![f.names()[0].clone(), Some(desc.to_owned())];
                    names.extend(f.names()[1..].iter().cloned());
                    push_names(&mut out, F, &names, what)?;
                }
                None => push_names(&mut out, F, f.names(), what)?,
            }
            out.push('\n');
        }
        for m in c.methods() {
            let what = || format!("method {}.{}{}", c.primary(), m.primary(), m.desc());
            let mut names = vec![m.names()[0].clone(), Some(m.desc().to_owned())];
            names.extend(m.names()[1..].iter().cloned());
            out.push('\t');
            push_names(&mut out, F, &names, what)?;
            out.push('\n');
            if m.is_static() {
                out.push_str("\t\tstatic\n");
            }
            for p in m.params() {
                out.push_str("\t\t");
                out.push_str(&p.lv_index().to_string());
                out.push(' ');
                push_names(&mut out, F, p.names(), || {
                    format!("parameter {} of {}", p.lv_index(), what())
                })?;
                out.push('\n');
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_errors() {
        for bad in [
            "a",
            "a b c",
            "\tx y",
            "a b\n\t\tx y",
            "a b\n\tx y z",
            "a b\n\tx (I) z w",
            "a/ b",
        ] {
            assert!(
                parse_v1(bad, ["obf", "srg"]).is_err(),
                "{bad:?} should fail"
            );
        }
    }

    #[test]
    fn v2_errors() {
        for bad in [
            "",
            "tsrg2",
            "tsrg a b",
            "tsrg2 a b\na",
            "tsrg2 a b\n\tx y",
            "tsrg2 a b\na b\n\tx",
            "tsrg2 a b\na b\n\tx y z w",
            "tsrg2 a b\na b\n\tx y\n\t\tstatic",
            "tsrg2 a b\na b\n\tx ()V y\n\t\tq x y",
            "tsrg2 a b\na b\n\tx ()V y\n\t\t1 x",
            "tsrg2 a b\na b\n\tx ()V y\n\t\t\t1 x y",
            "tsrg2 a b\na/ b",
            "tsrg2 a b\na b\n\tx Q y",
        ] {
            assert!(parse_v2(bad).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn v2_fields_with_and_without_descriptors() {
        let set = parse_v2("tsrg2 obf srg id\na net/minecraft/A 1\n\tb f_1_ 1\n\tc I f_2_ 2\n\td (I)V m_3_ 3\n\t\tstatic\n\t\t0 o p_3_0 4\n").unwrap();
        assert_eq!(set.field("a", "b", None).unwrap().desc(), None);
        assert_eq!(set.field("a", "c", None).unwrap().desc(), Some("I"));
        let m = set.method("a", "d", "(I)V").unwrap();
        assert!(m.is_static());
        assert_eq!(m.param(0).unwrap().name(1), Some("p_3_0"));
    }
}
