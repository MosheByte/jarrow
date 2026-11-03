//! A hand-built class that uses every constant tag and every typed attribute, so the
//! writer and parser are exercised without committing a `.class` file.

// Each test file uses part of this module; helpers unwrap, as tests do.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use jarrow_classfile::{
    Annotation, Attribute, AttributeBody, BootstrapMethod, ClassFile, Code, ConstKind, Constant,
    ConstantPool, ElementValue, ElementValuePair, ExceptionHandler, Field, InnerClass, LineNumber,
    LocalVarTarget, LocalVariable, LocalVariableType, Method, MethodParameter, RecordComponent,
    ReferenceKind, StackMapFrame, TargetInfo, TypeAnnotation, TypePathEntry, VerificationType,
    access,
};

/// Assembles the sample method body. Returns the bytes; the constant indices are
/// arguments so the code refers to real entries.
pub fn sample_code(
    methodref: u16,
    string: u16,
    long: u16,
    indy: u16,
    class: u16,
    imethod: u16,
) -> Vec<u8> {
    let mut code = vec![
        0x2A, // aload_0
        0xB7, // invokespecial methodref
    ];
    code.extend_from_slice(&methodref.to_be_bytes());
    code.push(0x12); // ldc string (index fits in a byte)
    code.push(u8::try_from(string).unwrap());
    code.push(0x57); // pop
    code.push(0x14); // ldc2_w long
    code.extend_from_slice(&long.to_be_bytes());
    code.push(0x58); // pop2
    code.push(0xBA); // invokedynamic indy 0 0
    code.extend_from_slice(&indy.to_be_bytes());
    code.extend_from_slice(&[0, 0]);
    code.push(0xB9); // invokeinterface imethod 1 0
    code.extend_from_slice(&imethod.to_be_bytes());
    code.extend_from_slice(&[1, 0]);
    code.push(0xBB); // new class
    code.extend_from_slice(&class.to_be_bytes());
    code.push(0x57); // pop
    code.extend_from_slice(&[0xC4, 0x15, 0x01, 0x00]); // wide iload 256
    code.extend_from_slice(&[0xC4, 0x84, 0x01, 0x00, 0xFF, 0x9C]); // wide iinc 256 -100
    // tableswitch at this offset, with padding up to the next multiple of 4.
    code.push(0xAA);
    while code.len() % 4 != 0 {
        code.push(0);
    }
    for v in [20i32, 0, 1, 12, 16] {
        code.extend_from_slice(&v.to_be_bytes());
    }
    code.push(0xB1); // return
    code
}

pub fn sample_class() -> ClassFile {
    let mut cp = ConstantPool::new();
    let this_class = cp.get_or_add_class("org/example/Sample").unwrap();
    let super_class = cp.get_or_add_class("java/lang/Record").unwrap();
    let runnable = cp.get_or_add_class("java/lang/Runnable").unwrap();
    let int = cp.push(Constant::Integer(-7)).unwrap();
    let float = cp.push(Constant::Float(0x7FC0_1234)).unwrap(); // a NaN with a payload
    let long = cp.push(Constant::Long(i64::MIN)).unwrap();
    let double = cp
        .push(Constant::Double(f64::NEG_INFINITY.to_bits()))
        .unwrap();
    let string = cp.get_or_add_string("hello \0 \u{1F600}").unwrap();
    let lone = cp.push(Constant::Utf8(vec![0xED, 0xA0, 0x80])).unwrap();
    let lone_string = cp.push(Constant::String { string_index: lone }).unwrap();
    let fieldref = cp
        .get_or_add_fieldref("org/example/Sample", "count", "I")
        .unwrap();
    let methodref = cp
        .get_or_add_methodref("java/lang/Object", "<init>", "()V")
        .unwrap();
    let imethod = cp
        .get_or_add_interface_methodref("java/lang/Runnable", "run", "()V")
        .unwrap();
    let handle = cp
        .push(Constant::MethodHandle {
            reference_kind: ReferenceKind::InvokeStatic,
            reference_index: methodref,
        })
        .unwrap();
    let field_handle = cp
        .push(Constant::MethodHandle {
            reference_kind: ReferenceKind::GetField,
            reference_index: fieldref,
        })
        .unwrap();
    let void = cp.get_or_add_utf8("()V").unwrap();
    let method_type = cp
        .push(Constant::MethodType {
            descriptor_index: void,
        })
        .unwrap();
    let indy_nat = cp
        .get_or_add_name_and_type("run", "()Ljava/lang/Runnable;")
        .unwrap();
    let indy = cp
        .push(Constant::InvokeDynamic {
            bootstrap_method_attr_index: 0,
            name_and_type_index: indy_nat,
        })
        .unwrap();
    let condy_nat = cp.get_or_add_name_and_type("CONST", "I").unwrap();
    let condy = cp
        .push(Constant::Dynamic {
            bootstrap_method_attr_index: 0,
            name_and_type_index: condy_nat,
        })
        .unwrap();
    let module_name = cp.get_or_add_utf8("java.base").unwrap();
    cp.push(Constant::Module {
        name_index: module_name,
    })
    .unwrap();
    let package_name = cp.get_or_add_utf8("org/example").unwrap();
    cp.push(Constant::Package {
        name_index: package_name,
    })
    .unwrap();

    let u = |cp: &mut ConstantPool, s: &str| cp.get_or_add_utf8(s).unwrap();
    let anno_type = u(&mut cp, "Lorg/example/Marker;");
    let enum_type = u(&mut cp, "Lorg/example/Color;");
    let red = u(&mut cp, "RED");
    let value = u(&mut cp, "value");
    let str_utf8 = u(&mut cp, "text");
    let class_desc = u(&mut cp, "Ljava/lang/String;");
    let signature = u(&mut cp, "<T:Ljava/lang/Object;>Ljava/lang/Record;");
    let source = u(&mut cp, "Sample.java");
    let custom = u(&mut cp, "org.example.Custom");
    let code_name = u(&mut cp, "Code");
    let count = u(&mut cp, "count");
    let int_desc = u(&mut cp, "I");
    let run = u(&mut cp, "run");
    let inner = cp.get_or_add_class("org/example/Sample$Inner").unwrap();
    let inner_name = u(&mut cp, "Inner");
    let x = u(&mut cp, "x");
    let this_desc = u(&mut cp, "Lorg/example/Sample;");

    let marker = |pairs: Vec<ElementValuePair>| Annotation {
        type_index: anno_type,
        element_value_pairs: pairs,
    };
    let every_value = vec![
        ElementValuePair {
            element_name_index: value,
            value: ElementValue::Const {
                kind: ConstKind::Int,
                const_value_index: int,
            },
        },
        ElementValuePair {
            element_name_index: value,
            value: ElementValue::Array(vec![
                ElementValue::Const {
                    kind: ConstKind::Byte,
                    const_value_index: int,
                },
                ElementValue::Const {
                    kind: ConstKind::Char,
                    const_value_index: int,
                },
                ElementValue::Const {
                    kind: ConstKind::Short,
                    const_value_index: int,
                },
                ElementValue::Const {
                    kind: ConstKind::Boolean,
                    const_value_index: int,
                },
                ElementValue::Const {
                    kind: ConstKind::Long,
                    const_value_index: long,
                },
                ElementValue::Const {
                    kind: ConstKind::Float,
                    const_value_index: float,
                },
                ElementValue::Const {
                    kind: ConstKind::Double,
                    const_value_index: double,
                },
                ElementValue::Const {
                    kind: ConstKind::String,
                    const_value_index: str_utf8,
                },
                ElementValue::Enum {
                    type_name_index: enum_type,
                    const_name_index: red,
                },
                ElementValue::Class {
                    class_info_index: class_desc,
                },
                ElementValue::Annotation(marker(vec![])),
                ElementValue::Array(vec![]),
            ]),
        },
    ];
    let type_anno = |target_type: u8, target_info: TargetInfo| TypeAnnotation {
        target_type,
        target_info,
        target_path: vec![
            TypePathEntry {
                type_path_kind: 0,
                type_argument_index: 0,
            },
            TypePathEntry {
                type_path_kind: 3,
                type_argument_index: 1,
            },
        ],
        annotation: marker(vec![]),
    };
    let attr = |cp: &mut ConstantPool, body: AttributeBody| Attribute::new(cp, body).unwrap();

    let code_attributes = vec![
        attr(
            &mut cp,
            AttributeBody::LineNumberTable(vec![LineNumber {
                start_pc: 0,
                line_number: 10,
            }]),
        ),
        attr(
            &mut cp,
            AttributeBody::LocalVariableTable(vec![LocalVariable {
                start_pc: 0,
                length: 5,
                name_index: x,
                descriptor_index: int_desc,
                index: 1,
            }]),
        ),
        attr(
            &mut cp,
            AttributeBody::LocalVariableTypeTable(vec![LocalVariableType {
                start_pc: 0,
                length: 5,
                name_index: x,
                signature_index: signature,
                index: 1,
            }]),
        ),
        attr(
            &mut cp,
            AttributeBody::StackMapTable(vec![
                StackMapFrame::Same { offset_delta: 3 },
                StackMapFrame::SameLocals1StackItem {
                    offset_delta: 63,
                    stack: VerificationType::Integer,
                },
                StackMapFrame::SameLocals1StackItemExtended {
                    offset_delta: 2,
                    stack: VerificationType::Object {
                        cpool_index: this_class,
                    },
                },
                StackMapFrame::Chop {
                    chopped: 2,
                    offset_delta: 1,
                },
                StackMapFrame::SameExtended { offset_delta: 1 },
                StackMapFrame::Append {
                    offset_delta: 0,
                    locals: vec![
                        VerificationType::Long,
                        VerificationType::Uninitialized { offset: 9 },
                    ],
                },
                StackMapFrame::Full {
                    offset_delta: 7,
                    locals: vec![
                        VerificationType::UninitializedThis,
                        VerificationType::Top,
                        VerificationType::Float,
                        VerificationType::Double,
                    ],
                    stack: vec![VerificationType::Null],
                },
            ]),
        ),
        attr(
            &mut cp,
            AttributeBody::RuntimeVisibleTypeAnnotations(vec![
                type_anno(
                    0x40,
                    TargetInfo::LocalVar(vec![LocalVarTarget {
                        start_pc: 0,
                        length: 5,
                        index: 1,
                    }]),
                ),
                type_anno(
                    0x42,
                    TargetInfo::Catch {
                        exception_table_index: 0,
                    },
                ),
                type_anno(0x43, TargetInfo::Offset { offset: 4 }),
                type_anno(
                    0x47,
                    TargetInfo::TypeArgument {
                        offset: 4,
                        type_argument_index: 0,
                    },
                ),
            ]),
        ),
        Attribute {
            name_index: custom,
            body: AttributeBody::Unknown(vec![9, 8, 7]),
        },
    ];
    let code = Code {
        max_stack: 4,
        max_locals: 300,
        code: sample_code(methodref, string, long, indy, this_class, imethod),
        exception_table: vec![ExceptionHandler {
            start_pc: 0,
            end_pc: 4,
            handler_pc: 4,
            catch_type: 0,
        }],
        attributes: code_attributes,
    };

    let method_attributes = vec![
        attr(&mut cp, AttributeBody::Code(code)),
        attr(&mut cp, AttributeBody::Exceptions(vec![runnable])),
        attr(
            &mut cp,
            AttributeBody::RuntimeVisibleParameterAnnotations(vec![
                vec![marker(every_value.clone())],
                vec![],
            ]),
        ),
        attr(
            &mut cp,
            AttributeBody::RuntimeInvisibleParameterAnnotations(vec![vec![]]),
        ),
        attr(
            &mut cp,
            AttributeBody::AnnotationDefault(ElementValue::Const {
                kind: ConstKind::String,
                const_value_index: str_utf8,
            }),
        ),
        attr(
            &mut cp,
            AttributeBody::MethodParameters(vec![
                MethodParameter {
                    name_index: x,
                    access_flags: access::FINAL,
                },
                MethodParameter {
                    name_index: 0,
                    access_flags: access::MANDATED,
                },
            ]),
        ),
        attr(
            &mut cp,
            AttributeBody::Signature {
                signature_index: signature,
            },
        ),
        attr(&mut cp, AttributeBody::Synthetic),
        attr(&mut cp, AttributeBody::Deprecated),
        attr(
            &mut cp,
            AttributeBody::RuntimeInvisibleTypeAnnotations(vec![
                type_anno(
                    0x01,
                    TargetInfo::TypeParameter {
                        type_parameter_index: 0,
                    },
                ),
                type_anno(
                    0x12,
                    TargetInfo::TypeParameterBound {
                        type_parameter_index: 0,
                        bound_index: 1,
                    },
                ),
                type_anno(0x14, TargetInfo::Empty),
                type_anno(0x15, TargetInfo::Empty),
                type_anno(
                    0x16,
                    TargetInfo::FormalParameter {
                        formal_parameter_index: 1,
                    },
                ),
                type_anno(
                    0x17,
                    TargetInfo::Throws {
                        throws_type_index: 0,
                    },
                ),
            ]),
        ),
    ];

    let field_attributes = vec![
        attr(
            &mut cp,
            AttributeBody::ConstantValue {
                constantvalue_index: int,
            },
        ),
        attr(
            &mut cp,
            AttributeBody::Signature {
                signature_index: signature,
            },
        ),
        attr(&mut cp, AttributeBody::Synthetic),
        attr(&mut cp, AttributeBody::Deprecated),
        attr(
            &mut cp,
            AttributeBody::RuntimeVisibleAnnotations(vec![marker(vec![])]),
        ),
        attr(
            &mut cp,
            AttributeBody::RuntimeVisibleTypeAnnotations(vec![type_anno(0x13, TargetInfo::Empty)]),
        ),
        // "Code" on a field is not a JVMS attribute there, so it stays raw.
        Attribute {
            name_index: code_name,
            body: AttributeBody::Unknown(vec![1, 2, 3]),
        },
    ];

    let class_attributes = vec![
        attr(
            &mut cp,
            AttributeBody::SourceFile {
                sourcefile_index: source,
            },
        ),
        attr(
            &mut cp,
            AttributeBody::Signature {
                signature_index: signature,
            },
        ),
        attr(
            &mut cp,
            AttributeBody::InnerClasses(vec![InnerClass {
                inner_class_info_index: inner,
                outer_class_info_index: this_class,
                inner_name_index: inner_name,
                inner_class_access_flags: access::PRIVATE | access::STATIC,
            }]),
        ),
        attr(
            &mut cp,
            AttributeBody::EnclosingMethod {
                class_index: this_class,
                method_index: 0,
            },
        ),
        attr(
            &mut cp,
            AttributeBody::NestHost {
                host_class_index: this_class,
            },
        ),
        attr(&mut cp, AttributeBody::NestMembers(vec![inner])),
        attr(
            &mut cp,
            AttributeBody::PermittedSubclasses(vec![inner, runnable]),
        ),
        attr(
            &mut cp,
            AttributeBody::BootstrapMethods(vec![
                BootstrapMethod {
                    bootstrap_method_ref: handle,
                    bootstrap_arguments: vec![method_type, string, lone_string],
                },
                BootstrapMethod {
                    bootstrap_method_ref: field_handle,
                    bootstrap_arguments: vec![],
                },
            ]),
        ),
        attr(
            &mut cp,
            AttributeBody::RuntimeVisibleAnnotations(vec![marker(every_value)]),
        ),
        attr(&mut cp, AttributeBody::RuntimeInvisibleAnnotations(vec![])),
        attr(
            &mut cp,
            AttributeBody::RuntimeVisibleTypeAnnotations(vec![
                type_anno(
                    0x00,
                    TargetInfo::TypeParameter {
                        type_parameter_index: 0,
                    },
                ),
                type_anno(
                    0x10,
                    TargetInfo::Supertype {
                        supertype_index: 0xFFFF,
                    },
                ),
                type_anno(
                    0x11,
                    TargetInfo::TypeParameterBound {
                        type_parameter_index: 0,
                        bound_index: 0,
                    },
                ),
            ]),
        ),
        attr(&mut cp, AttributeBody::Synthetic),
        attr(&mut cp, AttributeBody::Deprecated),
    ];
    let record_component_attributes = vec![
        attr(
            &mut cp,
            AttributeBody::Signature {
                signature_index: signature,
            },
        ),
        attr(
            &mut cp,
            AttributeBody::RuntimeInvisibleAnnotations(vec![marker(vec![])]),
        ),
        attr(
            &mut cp,
            AttributeBody::RuntimeInvisibleTypeAnnotations(vec![type_anno(
                0x13,
                TargetInfo::Empty,
            )]),
        ),
    ];
    let mut class_attributes = class_attributes;
    class_attributes.push(attr(
        &mut cp,
        AttributeBody::Record(vec![RecordComponent {
            name_index: count,
            descriptor_index: int_desc,
            attributes: record_component_attributes,
        }]),
    ));
    class_attributes.push(Attribute {
        name_index: custom,
        body: AttributeBody::Unknown(vec![]),
    });
    // Keep `condy` and `this_desc` referenced so the pool really holds them.
    let _ = (condy, this_desc);

    ClassFile {
        minor_version: 0,
        major_version: 65,
        constant_pool: cp,
        access_flags: access::PUBLIC | access::FINAL | access::SUPER,
        this_class,
        super_class,
        interfaces: vec![runnable],
        fields: vec![Field {
            access_flags: access::PRIVATE | access::FINAL,
            name_index: count,
            descriptor_index: int_desc,
            attributes: field_attributes,
        }],
        methods: vec![
            Method {
                access_flags: access::PUBLIC,
                name_index: run,
                descriptor_index: void,
                attributes: method_attributes,
            },
            Method {
                access_flags: access::PUBLIC | access::ABSTRACT,
                name_index: run,
                descriptor_index: void,
                attributes: vec![],
            },
        ],
        attributes: class_attributes,
    }
}

/// The sample, written.
pub fn sample_bytes() -> Vec<u8> {
    sample_class().to_bytes().unwrap()
}
