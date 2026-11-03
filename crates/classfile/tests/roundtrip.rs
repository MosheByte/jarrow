//! Write -> parse -> write on a hand-built class that uses every typed attribute, and the
//! errors the parser returns for broken input.

// Helpers outside #[test] functions unwrap as well: a failed unwrap is a failed test.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use jarrow_classfile::{AttributeBody, ClassFile, Constant, Error, MAGIC, Operands, mutf8};

#[test]
fn sample_round_trips_exactly() {
    let class = common::sample_class();
    let bytes = class.to_bytes().unwrap();
    let parsed = ClassFile::parse(&bytes).unwrap();
    assert_eq!(parsed, class, "parse(write(class)) differs from class");
    assert_eq!(
        parsed.to_bytes().unwrap(),
        bytes,
        "write(parse(bytes)) differs from bytes"
    );
}

#[test]
fn sample_names_resolve() {
    let class = common::sample_class();
    assert_eq!(class.this_class_name().unwrap(), "org/example/Sample");
    assert_eq!(
        class.super_class_name().unwrap().as_deref(),
        Some("java/lang/Record")
    );
    let pool = &class.constant_pool;
    let strings: Vec<_> = pool
        .iter()
        .filter_map(|(_, c)| match c {
            Constant::String { string_index } => {
                Some(mutf8::decode_lossy(pool.utf8(*string_index).unwrap()).into_owned())
            }
            _ => None,
        })
        .collect();
    assert_eq!(strings, ["hello \0 \u{1F600}", "\u{FFFD}"]);
    // The lone surrogate cannot become a str, and says so.
    let lone = pool
        .iter()
        .find(|(_, c)| matches!(c, Constant::Utf8(b) if b == &[0xED, 0xA0, 0x80]))
        .unwrap()
        .0;
    assert!(matches!(pool.utf8_str(lone), Err(Error::Utf8 { .. })));
}

#[test]
fn sample_code_decodes() {
    let class = common::sample_class();
    let code = class.methods[0].code().unwrap();
    let insns: Vec<_> = code.instructions().collect::<Result<_, _>>().unwrap();
    let mnemonics: Vec<_> = insns.iter().map(|i| i.opcode.mnemonic()).collect();
    assert_eq!(
        mnemonics,
        [
            "aload_0",
            "invokespecial",
            "ldc",
            "pop",
            "ldc2_w",
            "pop2",
            "invokedynamic",
            "invokeinterface",
            "new",
            "pop",
            "iload",
            "iinc",
            "tableswitch",
            "return"
        ]
    );
    assert!(insns[10].wide && insns[11].wide);
    assert_eq!(
        insns[11].operands,
        Operands::Iinc {
            index: 256,
            delta: -100
        }
    );
    let total: u32 = insns.iter().map(|i| i.length).sum();
    assert_eq!(total as usize, code.code.len());
    let mut reencoded = Vec::new();
    for insn in &insns {
        insn.encode(&mut reencoded).unwrap();
    }
    assert_eq!(reencoded, code.code);
    // Every constant the code names resolves to the right kind of entry.
    let pool = &class.constant_pool;
    assert_eq!(
        pool.member_ref(insns[1].constant_index().unwrap())
            .unwrap()
            .name,
        "<init>"
    );
    assert_eq!(
        pool.member_ref(insns[7].constant_index().unwrap())
            .unwrap()
            .class,
        "java/lang/Runnable"
    );
    assert!(matches!(
        pool.get(insns[6].constant_index().unwrap()),
        Some(Constant::InvokeDynamic { .. })
    ));
    assert_eq!(
        pool.class_name(insns[8].constant_index().unwrap()).unwrap(),
        "org/example/Sample"
    );
}

#[test]
fn every_truncation_is_an_error() {
    let bytes = common::sample_bytes();
    for len in 0..bytes.len() {
        assert!(
            ClassFile::parse(&bytes[..len]).is_err(),
            "a {len}-byte prefix parsed"
        );
    }
}

#[test]
fn trailing_bytes_are_an_error() {
    let mut bytes = common::sample_bytes();
    let end = bytes.len();
    bytes.push(0);
    assert_eq!(
        ClassFile::parse(&bytes),
        Err(Error::TrailingBytes {
            offset: end,
            count: 1
        })
    );
}

#[test]
fn bad_magic_is_an_error() {
    let mut bytes = common::sample_bytes();
    bytes[0] = 0xCB;
    assert_eq!(
        ClassFile::parse(&bytes),
        Err(Error::BadMagic {
            found: MAGIC + 0x0100_0000
        })
    );
}

/// Flips every byte of the sample in turn. Each result either fails to parse or, when it
/// still parses, writes back to exactly the mutated bytes. Nothing panics.
#[test]
fn single_byte_mutations_never_panic_and_round_trip() {
    let bytes = common::sample_bytes();
    let mut parsed = 0usize;
    for i in 0..bytes.len() {
        for flip in [0xFFu8, 0x01, 0x80] {
            let mut mutated = bytes.clone();
            mutated[i] ^= flip;
            if let Ok(class) = ClassFile::parse(&mutated) {
                parsed += 1;
                assert_eq!(
                    class.to_bytes().unwrap(),
                    mutated,
                    "mutation at byte {i} (xor {flip:#04x})"
                );
                for method in &class.methods {
                    if let Some(code) = method.code() {
                        for insn in code.instructions() {
                            if insn.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
    // Many flips land in indices, flags or raw bytes and still give a valid class.
    assert!(parsed > 0);
}

fn minimal(pool_extra: impl FnOnce(&mut jarrow_classfile::ConstantPool)) -> ClassFile {
    let mut pool = jarrow_classfile::ConstantPool::new();
    let this_class = pool.get_or_add_class("A").unwrap();
    pool_extra(&mut pool);
    ClassFile {
        minor_version: 0,
        major_version: 52,
        constant_pool: pool,
        access_flags: 0,
        this_class,
        super_class: 0,
        interfaces: vec![],
        fields: vec![],
        methods: vec![],
        attributes: vec![],
    }
}

#[test]
fn wrong_reference_kinds_are_errors() {
    // A Class entry pointing at an Integer.
    let class = minimal(|pool| {
        let int = pool.push(Constant::Integer(1)).unwrap();
        pool.push(Constant::Class { name_index: int }).unwrap();
    });
    let err = ClassFile::parse(&class.to_bytes().unwrap()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::BadConstantIndex {
                expected: "Utf8",
                ..
            }
        ),
        "{err}"
    );

    // this_class pointing at a Utf8.
    let mut class = minimal(|_| {});
    class.this_class = 1;
    let err = ClassFile::parse(&class.to_bytes().unwrap()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::BadConstantIndex {
                context: "this_class",
                ..
            }
        ),
        "{err}"
    );

    // A MethodHandle of kind getField pointing at a Methodref.
    let class = minimal(|pool| {
        let m = pool.get_or_add_methodref("A", "m", "()V").unwrap();
        pool.push(Constant::MethodHandle {
            reference_kind: jarrow_classfile::ReferenceKind::GetField,
            reference_index: m,
        })
        .unwrap();
    });
    let err = ClassFile::parse(&class.to_bytes().unwrap()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::BadConstantIndex {
                expected: "Fieldref",
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn constant_pool_edge_cases_are_errors() {
    let bytes = minimal(|_| {}).to_bytes().unwrap();
    // constant_pool_count = 0.
    let mut zero = bytes.clone();
    zero[8] = 0;
    zero[9] = 0;
    assert_eq!(ClassFile::parse(&zero), Err(Error::EmptyConstantPoolCount));
    // An unknown tag in the first entry.
    let mut bad_tag = bytes.clone();
    bad_tag[10] = 2;
    assert!(matches!(
        ClassFile::parse(&bad_tag),
        Err(Error::BadConstantTag {
            index: 1,
            tag: 2,
            ..
        })
    ));
    // A Long in the last slot.
    let mut wide = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 52, 0, 2, 5];
    wide.extend_from_slice(&[0; 8]);
    assert_eq!(
        ClassFile::parse(&wide),
        Err(Error::WideConstantAtEnd { index: 1 })
    );
}

#[test]
fn attribute_length_mismatch_is_an_error() {
    let mut class = minimal(|_| {});
    let name_index = class.constant_pool.get_or_add_utf8("SourceFile").unwrap();
    // A SourceFile attribute holding three bytes instead of two.
    class.attributes.push(jarrow_classfile::Attribute {
        name_index,
        body: AttributeBody::Unknown(vec![0, 1, 2]),
    });
    let err = ClassFile::parse(&class.to_bytes().unwrap()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::AttributeLength {
                name: "SourceFile",
                declared: 3,
                ..
            }
        ),
        "{err}"
    );
    // And one holding one byte.
    class.attributes[0].body = AttributeBody::Unknown(vec![0]);
    let err = ClassFile::parse(&class.to_bytes().unwrap()).unwrap_err();
    assert!(
        matches!(
            err,
            Error::AttributeLength {
                name: "SourceFile",
                declared: 1,
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn deep_annotation_nesting_is_an_error_not_a_crash() {
    let mut class = minimal(|_| {});
    let name_index = class
        .constant_pool
        .get_or_add_utf8("RuntimeVisibleAnnotations")
        .unwrap();
    let type_index = class.constant_pool.get_or_add_utf8("LA;").unwrap();
    // One annotation whose single value is an array nested 10,000 levels deep.
    let mut body = vec![0, 1];
    body.extend_from_slice(&type_index.to_be_bytes());
    body.extend_from_slice(&[0, 1, 0, 1]);
    for _ in 0..10_000 {
        body.extend_from_slice(&[b'[', 0, 1]);
    }
    body.extend_from_slice(&[b'[', 0, 0]);
    class.attributes.push(jarrow_classfile::Attribute {
        name_index,
        body: AttributeBody::Unknown(body),
    });
    let err = ClassFile::parse(&class.to_bytes().unwrap()).unwrap_err();
    assert!(matches!(err, Error::TooDeep { .. }), "{err}");
}

#[test]
fn writer_rejects_values_the_format_cannot_hold() {
    let mut class = minimal(|_| {});
    class.interfaces = vec![1; 70_000];
    assert!(matches!(
        class.to_bytes(),
        Err(Error::TooMany {
            what: "interfaces",
            ..
        })
    ));

    let mut class = minimal(|_| {});
    let name_index = class
        .constant_pool
        .get_or_add_utf8("StackMapTable")
        .unwrap();
    let frame = jarrow_classfile::StackMapFrame::Same { offset_delta: 64 };
    class.attributes.push(jarrow_classfile::Attribute {
        name_index,
        body: AttributeBody::StackMapTable(vec![frame]),
    });
    assert!(matches!(class.to_bytes(), Err(Error::InvalidValue { .. })));
}

#[test]
fn constant_pool_limits() {
    use jarrow_classfile::ConstantPool;
    let mut pool = ConstantPool::new();
    // A Utf8 entry's length is a u16.
    let long = vec![b'a'; 70_000];
    assert!(matches!(
        pool.push(Constant::Utf8(long)),
        Err(Error::TooMany { .. })
    ));
    assert!(matches!(
        pool.get_or_add_utf8(&"a".repeat(70_000)),
        Err(Error::TooMany { .. })
    ));
    // replace keeps widths, so no other index moves.
    let i = pool.push(Constant::Integer(1)).unwrap();
    assert_eq!(
        pool.replace(i, Constant::Long(1)),
        Err(Error::WidthMismatch { index: i })
    );
    assert_eq!(
        pool.replace(i, Constant::Integer(2)),
        Ok(Constant::Integer(1))
    );
    assert!(matches!(
        pool.replace(0, Constant::Integer(3)),
        Err(Error::BadConstantIndex { .. })
    ));
    // 65535 slots at most: index 65534 is the last one.
    while pool.count() < 65_534 {
        pool.push(Constant::Integer(i32::from(pool.count())))
            .unwrap();
    }
    assert_eq!(pool.push(Constant::Long(0)), Err(Error::ConstantPoolFull));
    assert_eq!(pool.push(Constant::Integer(-1)), Ok(65_534));
    assert_eq!(pool.count(), 65_535);
    assert_eq!(
        pool.push(Constant::Integer(-2)),
        Err(Error::ConstantPoolFull)
    );
    assert_eq!(pool.get_or_add(Constant::Integer(-1)), Ok(65_534));
}
