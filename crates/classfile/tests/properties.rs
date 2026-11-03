//! Property tests: Modified UTF-8, the constant pool, and arbitrary input.

// Helpers outside #[test] functions unwrap as well: a failed unwrap is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use jarrow_classfile::mutf8::{self, Piece};
use jarrow_classfile::{ClassFile, Constant, ConstantPool, Instructions, ReferenceKind};
use proptest::prelude::*;

/// A lone or paired surrogate, or ordinary bytes, as MUTF-8 bytes; mixed into the inputs
/// so the interesting cases are common rather than rare.
fn mutf8_fragment() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        any::<String>().prop_map(|s| mutf8::encode(&s)),
        (0xD800u32..=0xDFFF).prop_map(|u| {
            vec![
                0xE0 | ((u >> 12) & 0x0F) as u8,
                0x80 | ((u >> 6) & 0x3F) as u8,
                0x80 | (u & 0x3F) as u8,
            ]
        }),
        Just(vec![0xC0, 0x80]),
        Just(vec![0x00]),
        Just(vec![0xC1, 0x81]),
        proptest::collection::vec(any::<u8>(), 0..6),
    ]
}

fn mutf8_bytes() -> impl Strategy<Value = Vec<u8>> {
    proptest::collection::vec(mutf8_fragment(), 0..12).prop_map(|parts| parts.concat())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn mutf8_str_round_trips(s in any::<String>()) {
        let bytes = mutf8::encode(&s);
        prop_assert!(!bytes.contains(&0), "MUTF-8 never contains a zero byte");
        prop_assert!(bytes.iter().all(|&b| b < 0xF0), "MUTF-8 never contains four-byte sequences");
        prop_assert_eq!(mutf8::decode(&bytes).unwrap(), s.as_str());
        let pieces = mutf8::decode_lossless(&bytes);
        if s.is_empty() {
            prop_assert!(pieces.is_empty());
        } else {
            prop_assert_eq!(pieces, vec![Piece::Text(s.clone())]);
        }
    }

    #[test]
    fn mutf8_plain_text_is_utf8(s in "[\u{1}-\u{FFFF}]*") {
        // Without U+0000 and supplementary characters, MUTF-8 and UTF-8 agree.
        prop_assert_eq!(mutf8::encode(&s), s.as_bytes().to_vec());
    }

    #[test]
    fn mutf8_lossless_round_trips_any_bytes(bytes in mutf8_bytes()) {
        let pieces = mutf8::decode_lossless(&bytes);
        prop_assert_eq!(mutf8::encode_pieces(&pieces), bytes.clone());
        // Pieces are merged: never two of the same kind in a row, never empty.
        for pair in pieces.windows(2) {
            let same = matches!((&pair[0], &pair[1]), (Piece::Text(_), Piece::Text(_)) | (Piece::Raw(_), Piece::Raw(_)));
            prop_assert!(!same);
        }
        for piece in &pieces {
            match piece {
                Piece::Text(t) => prop_assert!(!t.is_empty()),
                Piece::Raw(r) => prop_assert!(!r.is_empty()),
            }
        }
        // Strict decoding succeeds exactly when there is no raw piece.
        let raw = pieces.iter().any(|p| matches!(p, Piece::Raw(_)));
        prop_assert_eq!(mutf8::decode(&bytes).is_ok(), !raw);
    }

    #[test]
    fn mutf8_arbitrary_bytes_round_trip(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        prop_assert_eq!(mutf8::encode_pieces(&mutf8::decode_lossless(&bytes)), bytes.clone());
        let _ = mutf8::decode_lossy(&bytes);
    }
}

/// One step of building a constant pool through the public API.
#[derive(Debug, Clone)]
enum Op {
    Utf8(String),
    RawUtf8(Vec<u8>),
    Class(String),
    Str(String),
    NameAndType(String, String),
    Fieldref(String, String, String),
    Methodref(String, String, String),
    InterfaceMethodref(String, String, String),
    Integer(i32),
    Float(u32),
    Long(i64),
    Double(u64),
}

fn name() -> impl Strategy<Value = String> {
    // A small alphabet, so get-or-add hits existing entries often.
    prop_oneof!["[a-c]{1,3}", "[a-c/]{1,6}", any::<String>()]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        name().prop_map(Op::Utf8),
        mutf8_bytes().prop_map(Op::RawUtf8),
        name().prop_map(Op::Class),
        name().prop_map(Op::Str),
        (name(), name()).prop_map(|(a, b)| Op::NameAndType(a, b)),
        (name(), name(), name()).prop_map(|(a, b, c)| Op::Fieldref(a, b, c)),
        (name(), name(), name()).prop_map(|(a, b, c)| Op::Methodref(a, b, c)),
        (name(), name(), name()).prop_map(|(a, b, c)| Op::InterfaceMethodref(a, b, c)),
        any::<i32>().prop_map(Op::Integer),
        any::<u32>().prop_map(Op::Float),
        any::<i64>().prop_map(Op::Long),
        any::<u64>().prop_map(Op::Double),
    ]
}

/// Applies `op` and checks what it promised: the returned index holds a matching entry,
/// a repeat returns the same index, and no existing entry moved.
fn apply(pool: &mut ConstantPool, op: &Op) -> Result<(), TestCaseError> {
    let before: Vec<(u16, Constant)> = pool.iter().map(|(i, c)| (i, c.clone())).collect();
    let index = match op {
        Op::Utf8(s) => pool.get_or_add_utf8(s),
        Op::RawUtf8(b) => pool.get_or_add_utf8_bytes(b),
        Op::Class(s) => pool.get_or_add_class(s),
        Op::Str(s) => pool.get_or_add_string(s),
        Op::NameAndType(n, d) => pool.get_or_add_name_and_type(n, d),
        Op::Fieldref(c, n, d) => pool.get_or_add_fieldref(c, n, d),
        Op::Methodref(c, n, d) => pool.get_or_add_methodref(c, n, d),
        Op::InterfaceMethodref(c, n, d) => pool.get_or_add_interface_methodref(c, n, d),
        Op::Integer(v) => pool.get_or_add(Constant::Integer(*v)),
        Op::Float(v) => pool.get_or_add(Constant::Float(*v)),
        Op::Long(v) => pool.get_or_add(Constant::Long(*v)),
        Op::Double(v) => pool.get_or_add(Constant::Double(*v)),
    }
    .unwrap();
    for (i, c) in &before {
        prop_assert_eq!(pool.get(*i), Some(c), "entry #{} changed", i);
    }
    let entry = pool.get(index).unwrap().clone();
    match op {
        Op::Utf8(s) => prop_assert_eq!(pool.utf8_str(index).unwrap(), s.as_str()),
        Op::RawUtf8(b) => prop_assert_eq!(pool.utf8(index).unwrap(), b.as_slice()),
        Op::Class(s) => prop_assert_eq!(pool.class_name(index).unwrap(), s.as_str()),
        Op::Str(s) => {
            let Constant::String { string_index } = entry else {
                return Err(TestCaseError::fail("not a String"));
            };
            prop_assert_eq!(pool.utf8_str(string_index).unwrap(), s.as_str());
        }
        Op::NameAndType(n, d) => {
            let (name, descriptor) = pool.name_and_type(index).unwrap();
            prop_assert_eq!(
                (name.as_ref(), descriptor.as_ref()),
                (n.as_str(), d.as_str())
            );
        }
        Op::Fieldref(c, n, d) | Op::Methodref(c, n, d) | Op::InterfaceMethodref(c, n, d) => {
            let r = pool.member_ref(index).unwrap();
            prop_assert_eq!(
                (r.class.as_ref(), r.name.as_ref(), r.descriptor.as_ref()),
                (c.as_str(), n.as_str(), d.as_str())
            );
        }
        Op::Integer(v) => prop_assert_eq!(&entry, &Constant::Integer(*v)),
        Op::Float(v) => prop_assert_eq!(&entry, &Constant::Float(*v)),
        Op::Long(v) => prop_assert_eq!(&entry, &Constant::Long(*v)),
        Op::Double(v) => prop_assert_eq!(&entry, &Constant::Double(*v)),
    }
    let count = pool.count();
    let again = match op {
        Op::Utf8(s) => pool.get_or_add_utf8(s),
        Op::RawUtf8(b) => pool.get_or_add_utf8_bytes(b),
        Op::Class(s) => pool.get_or_add_class(s),
        Op::Str(s) => pool.get_or_add_string(s),
        Op::NameAndType(n, d) => pool.get_or_add_name_and_type(n, d),
        Op::Fieldref(c, n, d) => pool.get_or_add_fieldref(c, n, d),
        Op::Methodref(c, n, d) => pool.get_or_add_methodref(c, n, d),
        Op::InterfaceMethodref(c, n, d) => pool.get_or_add_interface_methodref(c, n, d),
        Op::Integer(_) | Op::Float(_) | Op::Long(_) | Op::Double(_) => {
            pool.get_or_add(entry.clone())
        }
    }
    .unwrap();
    prop_assert_eq!(
        again,
        index,
        "a repeated get-or-add appended a second entry"
    );
    prop_assert_eq!(pool.count(), count);
    Ok(())
}

fn class_with(pool: ConstantPool, this_class: u16) -> ClassFile {
    ClassFile {
        minor_version: 0,
        major_version: 65,
        constant_pool: pool,
        access_flags: 0x0021,
        this_class,
        super_class: 0,
        interfaces: vec![],
        fields: vec![],
        methods: vec![],
        attributes: vec![],
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 500, ..ProptestConfig::default() })]

    #[test]
    fn constant_pool_get_or_add_and_round_trip(ops in proptest::collection::vec(op(), 0..60)) {
        let mut pool = ConstantPool::new();
        let this_class = pool.get_or_add_class("T").unwrap();
        for op in &ops {
            apply(&mut pool, op)?;
        }
        // Method handles and dynamic constants over what was built.
        let refs: Vec<(u16, Constant)> = pool.iter().map(|(i, c)| (i, c.clone())).collect();
        for (i, c) in refs {
            let kind = match c {
                Constant::Fieldref { .. } => Some(ReferenceKind::PutStatic),
                Constant::Methodref { .. } => Some(ReferenceKind::InvokeVirtual),
                Constant::InterfaceMethodref { .. } => Some(ReferenceKind::InvokeInterface),
                _ => None,
            };
            if let Some(reference_kind) = kind {
                pool.get_or_add(Constant::MethodHandle { reference_kind, reference_index: i }).unwrap();
            }
            if let Constant::NameAndType { .. } = c {
                pool.get_or_add(Constant::InvokeDynamic { bootstrap_method_attr_index: 0, name_and_type_index: i }).unwrap();
                pool.get_or_add(Constant::Dynamic { bootstrap_method_attr_index: 1, name_and_type_index: i }).unwrap();
            }
        }
        let class = class_with(pool, this_class);
        let bytes = class.to_bytes().unwrap();
        let parsed = ClassFile::parse(&bytes).unwrap();
        prop_assert_eq!(&parsed, &class);
        prop_assert_eq!(parsed.to_bytes().unwrap(), bytes);
    }

    #[test]
    fn wide_constants_take_two_slots(values in proptest::collection::vec(any::<i64>(), 1..20)) {
        let mut pool = ConstantPool::new();
        let mut last = None;
        for v in values {
            let index = pool.push(Constant::Long(v)).unwrap();
            if let Some(prev) = last {
                prop_assert_eq!(index, prev + 2);
            }
            prop_assert!(pool.get(index + 1).is_none(), "the slot after a Long is unusable");
            last = Some(index);
        }
        prop_assert_eq!(pool.count(), last.unwrap() + 2);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 3000, ..ProptestConfig::default() })]

    /// Arbitrary bytes never panic the parser; when they do parse, they write back exactly.
    #[test]
    fn random_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        if let Ok(class) = ClassFile::parse(&bytes) {
            prop_assert_eq!(class.to_bytes().unwrap(), bytes);
        }
    }

    /// Random bytes behind a valid header and pool reach the deeper parsers.
    #[test]
    fn random_tails_never_panic(tail in proptest::collection::vec(any::<u8>(), 0..256)) {
        // Keep magic, versions and the constant pool of the sample, then random bytes.
        let mut bytes = common::sample_bytes();
        bytes.truncate(8 + pool_len(&common::sample_class().constant_pool));
        bytes.extend_from_slice(&tail);
        if let Ok(class) = ClassFile::parse(&bytes) {
            prop_assert_eq!(class.to_bytes().unwrap(), bytes);
        }
    }

    /// Random multi-byte mutations of the sample never panic, and round-trip when they parse.
    #[test]
    fn random_mutations_never_panic(edits in proptest::collection::vec((any::<prop::sample::Index>(), any::<u8>()), 1..8)) {
        let mut bytes = common::sample_bytes();
        for (at, value) in edits {
            let i = at.index(bytes.len());
            bytes[i] = value;
        }
        if let Ok(class) = ClassFile::parse(&bytes) {
            prop_assert_eq!(class.to_bytes().unwrap(), bytes);
        }
    }

    /// The instruction decoder never panics on arbitrary bytes, and what it decodes
    /// covers the input exactly up to the first error.
    #[test]
    fn decoder_never_panics(code in proptest::collection::vec(any::<u8>(), 0..256)) {
        let mut covered = 0usize;
        for insn in Instructions::new(&code) {
            match insn {
                Ok(insn) => {
                    prop_assert_eq!(insn.offset as usize, covered);
                    covered += insn.length as usize;
                    let mut out = Vec::new();
                    insn.encode(&mut out).unwrap();
                    prop_assert_eq!(out.len(), insn.length as usize);
                }
                Err(_) => break,
            }
        }
        prop_assert!(covered <= code.len());
    }
}

/// Bytes the pool takes when written: the count plus every entry.
fn pool_len(pool: &ConstantPool) -> usize {
    let class = class_with(pool.clone(), 1);
    // A class with no members and no attributes: magic and versions (8), the pool, then
    // access, this, super and four empty counts (14).
    class.to_bytes().unwrap().len() - 8 - 14
}
