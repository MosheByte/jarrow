//! Field and method descriptors, generic signatures, and Java source type names.
//!
//! Every function here works on JVM internal names (`net/minecraft/Foo$Bar`). The remapping
//! functions take a class-name callback that returns `Some(new_name)` for a mapped class and
//! `None` to keep the name unchanged, which is how classes outside the mappings (the JDK,
//! libraries, the mod's own classes) pass through.
//!
//! Grammar: JVMS 4.3 (descriptors) and JVMS 4.7.9.1 (signatures).

use crate::error::{Error, Result};

/// The JVM limit on array dimensions (JVMS 4.4.1).
const MAX_ARRAY_DIMENSIONS: usize = 255;

/// Checks that `desc` is one well-formed field descriptor, such as `I` or `[Lcom/a/B;`.
pub fn validate_field_desc(desc: &str) -> Result<()> {
    remap_field_desc(desc, |_| None::<&str>).map(|_| ())
}

/// Checks that `desc` is a well-formed method descriptor, such as `(I[J)Lcom/a/B;`.
pub fn validate_method_desc(desc: &str) -> Result<()> {
    remap_method_desc(desc, |_| None::<&str>).map(|_| ())
}

/// Rewrites the class names in a field descriptor.
///
/// ```
/// let out = jarrow_mappings::desc::remap_field_desc("[La;", |name| (name == "a").then_some("b/C"))?;
/// assert_eq!(out, "[Lb/C;");
/// # Ok::<(), jarrow_mappings::Error>(())
/// ```
pub fn remap_field_desc<F, R>(desc: &str, mut map: F) -> Result<String>
where
    F: FnMut(&str) -> Option<R>,
    R: AsRef<str>,
{
    let mut out = String::with_capacity(desc.len() + 8);
    let mut pos = 0;
    field_type(desc, &mut pos, &mut out, &mut map)?;
    if pos != desc.len() {
        return Err(Error::descriptor(desc, pos, "trailing characters"));
    }
    Ok(out)
}

/// Rewrites the class names in a method descriptor.
pub fn remap_method_desc<F, R>(desc: &str, mut map: F) -> Result<String>
where
    F: FnMut(&str) -> Option<R>,
    R: AsRef<str>,
{
    let bytes = desc.as_bytes();
    let mut out = String::with_capacity(desc.len() + 16);
    if bytes.first() != Some(&b'(') {
        return Err(Error::descriptor(
            desc,
            0,
            "a method descriptor starts with '('",
        ));
    }
    out.push('(');
    let mut pos = 1;
    while bytes.get(pos) != Some(&b')') {
        if pos >= bytes.len() {
            return Err(Error::descriptor(desc, pos, "missing ')'"));
        }
        field_type(desc, &mut pos, &mut out, &mut map)?;
    }
    out.push(')');
    pos += 1;
    if bytes.get(pos) == Some(&b'V') {
        out.push('V');
        pos += 1;
    } else {
        field_type(desc, &mut pos, &mut out, &mut map)?;
    }
    if pos != desc.len() {
        return Err(Error::descriptor(desc, pos, "trailing characters"));
    }
    Ok(out)
}

/// Rewrites a class name that may also be an array descriptor, as found in a
/// `CONSTANT_Class` entry (`com/a/B` or `[Lcom/a/B;`).
pub fn remap_class_or_array<F, R>(name: &str, mut map: F) -> Result<String>
where
    F: FnMut(&str) -> Option<R>,
    R: AsRef<str>,
{
    if name.starts_with('[') {
        return remap_field_desc(name, map);
    }
    check_internal_name(name, name, 0)?;
    Ok(match map(name) {
        Some(mapped) => mapped.as_ref().to_owned(),
        None => name.to_owned(),
    })
}

/// Rewrites the class names in a generic signature (a class, method or field `Signature`
/// attribute), keeping type variables, wildcards and type arguments as they are.
///
/// Inner classes in a signature appear as `Lpkg/Outer<..>.Inner;`. The full name
/// `pkg/Outer$Inner` is looked up; when it is unmapped, the inner simple name is kept under the
/// remapped outer class.
///
/// ```
/// let out = jarrow_mappings::desc::remap_signature(
///     "<T:La;>(Ljava/util/List<+La$b;>;)TT;",
///     |name| match name { "a" => Some("x/A"), "a$b" => Some("x/A$Inner"), _ => None },
/// )?;
/// assert_eq!(out, "<T:Lx/A;>(Ljava/util/List<+Lx/A$Inner;>;)TT;");
/// # Ok::<(), jarrow_mappings::Error>(())
/// ```
pub fn remap_signature<F, R>(sig: &str, mut map: F) -> Result<String>
where
    F: FnMut(&str) -> Option<R>,
    R: AsRef<str>,
{
    let mut p = SigParser {
        input: sig,
        pos: 0,
        out: String::with_capacity(sig.len() + 16),
    };
    if p.peek() == Some(b'<') {
        p.type_parameters(&mut map)?;
    }
    if p.peek() == Some(b'(') {
        p.bump_push();
        while p.peek() != Some(b')') {
            if p.at_end() {
                return Err(p.error("missing ')'"));
            }
            p.java_type_signature(&mut map)?;
        }
        p.bump_push();
        if p.peek() == Some(b'V') {
            p.bump_push();
        } else {
            p.java_type_signature(&mut map)?;
        }
        while p.peek() == Some(b'^') {
            p.bump_push();
            match p.peek() {
                Some(b'L' | b'T') => p.reference_type_signature(&mut map)?,
                _ => return Err(p.error("a throws clause names a class or a type variable")),
            }
        }
    } else {
        // A class signature (superclass and interfaces) or a field signature: one or more
        // reference type signatures.
        if p.at_end() {
            return Err(p.error("empty signature"));
        }
        while !p.at_end() {
            p.reference_type_signature(&mut map)?;
        }
    }
    if !p.at_end() {
        return Err(p.error("trailing characters"));
    }
    Ok(p.out)
}

/// Converts a Java source type, as written in ProGuard files (`int`, `java.lang.String[]`,
/// `net.minecraft.Foo$Bar`), to a field descriptor. `void` is accepted only when
/// `allow_void` is true (method return types).
pub fn java_type_to_desc(ty: &str, allow_void: bool) -> Result<String> {
    let mut base = ty;
    let mut dims = 0;
    while let Some(stripped) = base.strip_suffix("[]") {
        base = stripped;
        dims += 1;
    }
    if dims > MAX_ARRAY_DIMENSIONS {
        return Err(Error::descriptor(ty, 0, "more than 255 array dimensions"));
    }
    let mut out = String::with_capacity(ty.len() + 2);
    out.extend(std::iter::repeat_n('[', dims));
    let prim = match base {
        "byte" => Some('B'),
        "char" => Some('C'),
        "double" => Some('D'),
        "float" => Some('F'),
        "int" => Some('I'),
        "long" => Some('J'),
        "short" => Some('S'),
        "boolean" => Some('Z'),
        "void" if allow_void && dims == 0 => Some('V'),
        "void" => return Err(Error::descriptor(ty, 0, "void is only a return type")),
        _ => None,
    };
    match prim {
        Some(c) => out.push(c),
        None => {
            if base.is_empty()
                || base.starts_with('.')
                || base.ends_with('.')
                || base.contains("..")
                || base.bytes().any(|b| {
                    matches!(b, b'/' | b';' | b'[' | b']' | b'(' | b')' | b'<' | b'>')
                        || b.is_ascii_whitespace()
                })
            {
                return Err(Error::descriptor(ty, 0, "not a Java type name"));
            }
            out.push('L');
            out.extend(base.chars().map(|c| if c == '.' { '/' } else { c }));
            out.push(';');
        }
    }
    Ok(out)
}

/// Converts a field descriptor (or `V`) to the Java source type ProGuard writes.
pub fn desc_to_java_type(desc: &str) -> Result<String> {
    if desc == "V" {
        return Ok("void".to_owned());
    }
    validate_field_desc(desc)?;
    let dims = desc.bytes().take_while(|&b| b == b'[').count();
    let base = &desc[dims..];
    let mut out = match base.as_bytes().first() {
        Some(b'B') => "byte".to_owned(),
        Some(b'C') => "char".to_owned(),
        Some(b'D') => "double".to_owned(),
        Some(b'F') => "float".to_owned(),
        Some(b'I') => "int".to_owned(),
        Some(b'J') => "long".to_owned(),
        Some(b'S') => "short".to_owned(),
        Some(b'Z') => "boolean".to_owned(),
        _ => base[1..base.len() - 1].replace('/', "."),
    };
    for _ in 0..dims {
        out.push_str("[]");
    }
    Ok(out)
}

/// Builds a method descriptor from a ProGuard return type and argument list
/// (`void` and `int,java.lang.String[]`).
pub fn java_method_to_desc(return_type: &str, args: &str) -> Result<String> {
    let mut out = String::from("(");
    if !args.is_empty() {
        for arg in args.split(',') {
            out.push_str(&java_type_to_desc(arg, false)?);
        }
    }
    out.push(')');
    out.push_str(&java_type_to_desc(return_type, true)?);
    Ok(out)
}

/// Splits a method descriptor into the ProGuard return type and argument list.
pub fn method_desc_to_java(desc: &str) -> Result<(String, String)> {
    validate_method_desc(desc)?;
    let mut args = Vec::new();
    let mut pos = 1;
    let mut sink = String::new();
    while desc.as_bytes().get(pos) != Some(&b')') {
        let start = pos;
        sink.clear();
        field_type(desc, &mut pos, &mut sink, &mut |_: &str| None::<&str>)?;
        args.push(desc_to_java_type(&desc[start..pos])?);
    }
    Ok((desc_to_java_type(&desc[pos + 1..])?, args.join(",")))
}

/// Calls `visit` for each class name referenced by a field or method descriptor.
pub fn for_each_class_in_desc(desc: &str, mut visit: impl FnMut(&str)) -> Result<()> {
    let remap = |name: &str| {
        visit(name);
        None::<&str>
    };
    if desc.starts_with('(') {
        remap_method_desc(desc, remap).map(|_| ())
    } else {
        remap_field_desc(desc, remap).map(|_| ())
    }
}

fn field_type<F, R>(input: &str, pos: &mut usize, out: &mut String, map: &mut F) -> Result<()>
where
    F: FnMut(&str) -> Option<R>,
    R: AsRef<str>,
{
    let bytes = input.as_bytes();
    let start = *pos;
    while bytes.get(*pos) == Some(&b'[') {
        *pos += 1;
    }
    if *pos - start > MAX_ARRAY_DIMENSIONS {
        return Err(Error::descriptor(
            input,
            start,
            "more than 255 array dimensions",
        ));
    }
    out.push_str(&input[start..*pos]);
    match bytes.get(*pos) {
        Some(&c @ (b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z')) => {
            out.push(char::from(c));
            *pos += 1;
            Ok(())
        }
        Some(b'L') => {
            let name_start = *pos + 1;
            let semi = input[name_start..]
                .find(';')
                .map(|i| name_start + i)
                .ok_or_else(|| Error::descriptor(input, *pos, "class type without ';'"))?;
            let name = &input[name_start..semi];
            check_internal_name(input, name, name_start)?;
            out.push('L');
            match map(name) {
                Some(mapped) => out.push_str(mapped.as_ref()),
                None => out.push_str(name),
            }
            out.push(';');
            *pos = semi + 1;
            Ok(())
        }
        Some(_) => Err(Error::descriptor(input, *pos, "not a field type")),
        None => Err(Error::descriptor(input, *pos, "unexpected end")),
    }
}

/// An internal class name: non-empty segments separated by '/', none of `.;[<>`.
fn check_internal_name(input: &str, name: &str, offset: usize) -> Result<()> {
    if name.is_empty() {
        return Err(Error::descriptor(input, offset, "empty class name"));
    }
    if name.split('/').any(str::is_empty) {
        return Err(Error::descriptor(input, offset, "empty package segment"));
    }
    if let Some(i) = name
        .bytes()
        .position(|b| matches!(b, b'.' | b';' | b'[' | b'<' | b'>'))
    {
        return Err(Error::descriptor(
            input,
            offset + i,
            "invalid character in a class name",
        ));
    }
    Ok(())
}

struct SigParser<'s> {
    input: &'s str,
    pos: usize,
    out: String,
}

impl SigParser<'_> {
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.pos).copied()
    }

    fn at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn error(&self, message: &'static str) -> Error {
        Error::descriptor(self.input, self.pos, message)
    }

    /// Copies the current (ASCII) byte to the output and advances.
    fn bump_push(&mut self) {
        if let Some(b) = self.peek() {
            self.out.push(char::from(b));
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8, message: &'static str) -> Result<()> {
        if self.peek() == Some(byte) {
            self.bump_push();
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    /// An identifier up to (not including) one of `stops`. Must be non-empty and free of
    /// the characters JVMS 4.2.2 forbids.
    fn identifier(&mut self, stops: &[u8]) -> Result<&str> {
        let start = self.pos;
        let bytes = self.input.as_bytes();
        while let Some(&b) = bytes.get(self.pos) {
            if stops.contains(&b) {
                break;
            }
            if matches!(b, b'.' | b';' | b'[' | b'/' | b'<' | b'>' | b':') {
                return Err(self.error("invalid character in an identifier"));
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.error("empty identifier"));
        }
        Ok(&self.input[start..self.pos])
    }

    fn type_parameters<F, R>(&mut self, map: &mut F) -> Result<()>
    where
        F: FnMut(&str) -> Option<R>,
        R: AsRef<str>,
    {
        self.expect(b'<', "expected '<'")?;
        loop {
            let ident = self.identifier(b":")?.to_owned();
            self.out.push_str(&ident);
            // Class bound: ':' with an optional reference type.
            self.expect(b':', "a type parameter needs a ':' bound")?;
            if matches!(self.peek(), Some(b'L' | b'T' | b'[')) {
                self.reference_type_signature(map)?;
            }
            // Interface bounds.
            while self.peek() == Some(b':') {
                self.bump_push();
                self.reference_type_signature(map)?;
            }
            match self.peek() {
                Some(b'>') => {
                    self.bump_push();
                    return Ok(());
                }
                None => return Err(self.error("missing '>'")),
                Some(_) => {}
            }
        }
    }

    fn java_type_signature<F, R>(&mut self, map: &mut F) -> Result<()>
    where
        F: FnMut(&str) -> Option<R>,
        R: AsRef<str>,
    {
        match self.peek() {
            Some(b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z') => {
                self.bump_push();
                Ok(())
            }
            _ => self.reference_type_signature(map),
        }
    }

    fn reference_type_signature<F, R>(&mut self, map: &mut F) -> Result<()>
    where
        F: FnMut(&str) -> Option<R>,
        R: AsRef<str>,
    {
        match self.peek() {
            Some(b'L') => self.class_type_signature(map),
            Some(b'T') => {
                self.bump_push();
                let ident = self.identifier(b";")?.to_owned();
                self.out.push_str(&ident);
                self.expect(b';', "a type variable ends with ';'")
            }
            Some(b'[') => {
                let start = self.pos;
                while self.peek() == Some(b'[') {
                    self.bump_push();
                }
                if self.pos - start > MAX_ARRAY_DIMENSIONS {
                    return Err(self.error("more than 255 array dimensions"));
                }
                self.java_type_signature(map)
            }
            None => Err(self.error("unexpected end")),
            Some(_) => Err(self.error("not a reference type signature")),
        }
    }

    fn class_type_signature<F, R>(&mut self, map: &mut F) -> Result<()>
    where
        F: FnMut(&str) -> Option<R>,
        R: AsRef<str>,
    {
        self.expect(b'L', "expected 'L'")?;
        let start = self.pos;
        let bytes = self.input.as_bytes();
        while let Some(&b) = bytes.get(self.pos) {
            if matches!(b, b'<' | b'.' | b';') {
                break;
            }
            self.pos += 1;
        }
        let original = &self.input[start..self.pos];
        check_internal_name(self.input, original, start)?;
        let mut original_full = original.to_owned();
        let mut mapped_full = match map(original) {
            Some(m) => m.as_ref().to_owned(),
            None => original.to_owned(),
        };
        self.out.push_str(&mapped_full);
        if self.peek() == Some(b'<') {
            self.type_arguments(map)?;
        }
        while self.peek() == Some(b'.') {
            self.bump_push();
            let inner = self.identifier(b"<.;")?.to_owned();
            original_full.push('$');
            original_full.push_str(&inner);
            let fallback = format!("{mapped_full}${inner}");
            let mapped_inner = match map(&original_full) {
                Some(m) => m.as_ref().to_owned(),
                None => fallback,
            };
            self.out
                .push_str(inner_simple_name(&mapped_full, &mapped_inner));
            mapped_full = mapped_inner;
            if self.peek() == Some(b'<') {
                self.type_arguments(map)?;
            }
        }
        self.expect(b';', "a class type ends with ';'")
    }

    fn type_arguments<F, R>(&mut self, map: &mut F) -> Result<()>
    where
        F: FnMut(&str) -> Option<R>,
        R: AsRef<str>,
    {
        self.expect(b'<', "expected '<'")?;
        loop {
            match self.peek() {
                Some(b'*') => self.bump_push(),
                Some(b'+' | b'-') => {
                    self.bump_push();
                    self.reference_type_signature(map)?;
                }
                Some(_) => self.reference_type_signature(map)?,
                None => return Err(self.error("missing '>'")),
            }
            if self.peek() == Some(b'>') {
                self.bump_push();
                return Ok(());
            }
        }
    }
}

/// The part of a remapped inner class name that a signature's `.Inner` suffix carries.
fn inner_simple_name<'n>(mapped_outer: &str, mapped_inner: &'n str) -> &'n str {
    if let Some(rest) = mapped_inner
        .strip_prefix(mapped_outer)
        .and_then(|r| r.strip_prefix('$'))
        && !rest.is_empty()
    {
        return rest;
    }
    match mapped_inner.rfind(['$', '/']) {
        Some(i) if i + 1 < mapped_inner.len() => &mapped_inner[i + 1..],
        _ => mapped_inner,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ab(name: &str) -> Option<&'static str> {
        match name {
            "a" => Some("pkg/Alpha"),
            "a$b" => Some("pkg/Alpha$Beta"),
            "c" => Some("pkg/Gamma"),
            _ => None,
        }
    }

    #[test]
    fn field_descriptors() {
        assert_eq!(remap_field_desc("I", ab).unwrap(), "I");
        assert_eq!(remap_field_desc("La;", ab).unwrap(), "Lpkg/Alpha;");
        assert_eq!(
            remap_field_desc("[[La$b;", ab).unwrap(),
            "[[Lpkg/Alpha$Beta;"
        );
        assert_eq!(
            remap_field_desc("Ljava/lang/String;", ab).unwrap(),
            "Ljava/lang/String;"
        );
        for bad in [
            "", "V", "L;", "La", "II", "[", "Q", "La.b;", "L/a;", "La//b;", "La/;",
        ] {
            assert!(remap_field_desc(bad, ab).is_err(), "{bad:?} should fail");
        }
        let deep = format!("{}I", "[".repeat(256));
        assert!(validate_field_desc(&deep).is_err());
        assert!(validate_field_desc(&format!("{}I", "[".repeat(255))).is_ok());
    }

    #[test]
    fn method_descriptors() {
        assert_eq!(remap_method_desc("()V", ab).unwrap(), "()V");
        assert_eq!(
            remap_method_desc("(ILa;[J)Lc;", ab).unwrap(),
            "(ILpkg/Alpha;[J)Lpkg/Gamma;"
        );
        for bad in ["", "V", "(", "()", "(V)V", "()VV", "(I", "I)V", "()[V"] {
            assert!(remap_method_desc(bad, ab).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn class_or_array() {
        assert_eq!(remap_class_or_array("a", ab).unwrap(), "pkg/Alpha");
        assert_eq!(remap_class_or_array("[La;", ab).unwrap(), "[Lpkg/Alpha;");
        assert_eq!(remap_class_or_array("x/Y", ab).unwrap(), "x/Y");
        assert!(remap_class_or_array("", ab).is_err());
    }

    #[test]
    fn signatures() {
        let cases = [
            ("La;", "Lpkg/Alpha;"),
            ("TT;", "TT;"),
            ("[TT;", "[TT;"),
            ("Ljava/util/List<La;>;", "Ljava/util/List<Lpkg/Alpha;>;"),
            (
                "Ljava/util/Map<*+La;-[Lc;>;",
                "Ljava/util/Map<*+Lpkg/Alpha;-[Lpkg/Gamma;>;",
            ),
            (
                "<T:Ljava/lang/Object;>Ljava/lang/Object;",
                "<T:Ljava/lang/Object;>Ljava/lang/Object;",
            ),
            (
                "<T::La;U:TT;>Lc;La;",
                "<T::Lpkg/Alpha;U:TT;>Lpkg/Gamma;Lpkg/Alpha;",
            ),
            ("(La;I)Lc;", "(Lpkg/Alpha;I)Lpkg/Gamma;"),
            (
                "<E:La;>(TE;)V^TE;^Lc;",
                "<E:Lpkg/Alpha;>(TE;)V^TE;^Lpkg/Gamma;",
            ),
            ("La<TT;>.b<TU;>;", "Lpkg/Alpha<TT;>.Beta<TU;>;"),
            ("La.zz;", "Lpkg/Alpha.zz;"),
            ("Lx/Outer<La;>.Inner;", "Lx/Outer<Lpkg/Alpha;>.Inner;"),
        ];
        for (input, expected) in cases {
            assert_eq!(remap_signature(input, ab).unwrap(), expected, "{input}");
        }
        for bad in [
            "", "<>V", "<T>La;", "(", "(I", "L", "La", "La<>;", "La<;", "TT", "<T:La;", "()",
            "()V^I", "La.;", "Q", "La;X", "<T:La;>",
        ] {
            assert!(remap_signature(bad, ab).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn inner_names_that_leave_the_outer_class() {
        // A mapping that moves the inner class out of its outer class keeps a valid simple name.
        let map = |name: &str| match name {
            "a" => Some("p/A"),
            "a$b" => Some("q/Elsewhere"),
            _ => None,
        };
        assert_eq!(remap_signature("La.b;", map).unwrap(), "Lp/A.Elsewhere;");
    }

    #[test]
    fn java_types() {
        assert_eq!(java_type_to_desc("int", false).unwrap(), "I");
        assert_eq!(java_type_to_desc("void", true).unwrap(), "V");
        assert!(java_type_to_desc("void", false).is_err());
        assert!(java_type_to_desc("void[]", true).is_err());
        assert_eq!(
            java_type_to_desc("java.lang.String[][]", false).unwrap(),
            "[[Ljava/lang/String;"
        );
        assert_eq!(java_type_to_desc("a.B$C", false).unwrap(), "La/B$C;");
        for bad in ["", "[]", "a b", "a..b", ".a", "a/b", "a;"] {
            assert!(
                java_type_to_desc(bad, false).is_err(),
                "{bad:?} should fail"
            );
        }
        assert_eq!(desc_to_java_type("[[Z").unwrap(), "boolean[][]");
        assert_eq!(desc_to_java_type("La/B$C;").unwrap(), "a.B$C");
        assert_eq!(desc_to_java_type("V").unwrap(), "void");
        assert_eq!(java_method_to_desc("void", "").unwrap(), "()V");
        assert_eq!(
            java_method_to_desc("a.B", "int,long[],a.C").unwrap(),
            "(I[JLa/C;)La/B;"
        );
        assert!(java_method_to_desc("void", "int,").is_err());
        assert_eq!(
            method_desc_to_java("(I[JLa/C;)La/B;").unwrap(),
            ("a.B".to_owned(), "int,long[],a.C".to_owned())
        );
        assert_eq!(
            method_desc_to_java("()V").unwrap(),
            ("void".to_owned(), String::new())
        );
    }

    #[test]
    fn visit_classes() {
        let mut seen = Vec::new();
        for_each_class_in_desc("(La;[Lb/C;I)Ld;", |n| seen.push(n.to_owned())).unwrap();
        assert_eq!(seen, ["a", "b/C", "d"]);
    }
}
