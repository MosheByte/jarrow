//! Attributes (JVMS 4.7): typed where remapping needs them, raw otherwise.

use std::borrow::Cow;

use crate::Error;
use crate::annotation::{
    Annotation, ElementValue, TypeAnnotation, parse_annotations, parse_element_value,
    parse_parameter_annotations, parse_type_annotations, write_annotations, write_element_value,
    write_parameter_annotations, write_type_annotations,
};
use crate::bytes::{Reader, begin_len, capacity, count_u8, count_u16, patch_len, put_u16, put_u32};
use crate::constant_pool::ConstantPool;
use crate::insn::Instructions;
use crate::stack_map::{StackMapFrame, parse_frames, write_frames};

/// One attribute: the index of its name and its contents.
///
/// On write, `attribute_length` is computed from the contents, so it always matches.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    /// `Utf8`: the attribute name.
    pub name_index: u16,
    /// The contents.
    pub body: AttributeBody,
}

impl Attribute {
    /// A typed attribute, with its standard name taken from (or appended to) `pool`.
    ///
    /// [`AttributeBody::Unknown`] has no standard name; build it with a struct literal.
    pub fn new(pool: &mut ConstantPool, body: AttributeBody) -> Result<Self, Error> {
        let name = body.name().ok_or(Error::InvalidValue {
            what: "an Unknown attribute needs an explicit name",
        })?;
        Ok(Self {
            name_index: pool.get_or_add_utf8(name)?,
            body,
        })
    }

    /// The attribute's name.
    pub fn name<'a>(&self, pool: &'a ConstantPool) -> Result<Cow<'a, str>, Error> {
        pool.utf8_str(self.name_index)
    }
}

/// The contents of an attribute.
///
/// An attribute is parsed into a typed variant when its name is one of those below and it
/// appears where JVMS 4.7 defines it (`Code` on a method, `StackMapTable` inside `Code`,
/// and so on). Anything else, including `Module`, `SourceDebugExtension` and every
/// vendor attribute, is kept as [`AttributeBody::Unknown`] with its bytes untouched.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeBody {
    /// `ConstantValue` (fields).
    ConstantValue {
        /// The constant: `Integer`, `Long`, `Float`, `Double` or `String`.
        constantvalue_index: u16,
    },
    /// `Code` (methods).
    Code(Code),
    /// `StackMapTable` (inside `Code`).
    StackMapTable(Vec<StackMapFrame>),
    /// `Exceptions` (methods): `Class` entries of the `throws` clause.
    Exceptions(Vec<u16>),
    /// `InnerClasses` (classes).
    InnerClasses(Vec<InnerClass>),
    /// `EnclosingMethod` (classes).
    EnclosingMethod {
        /// `Class`: the innermost enclosing class.
        class_index: u16,
        /// `NameAndType` of the enclosing method, or 0.
        method_index: u16,
    },
    /// `Synthetic` (classes, fields, methods).
    Synthetic,
    /// `Signature` (classes, fields, methods, record components).
    Signature {
        /// `Utf8`: the generic signature.
        signature_index: u16,
    },
    /// `SourceFile` (classes).
    SourceFile {
        /// `Utf8`: the source file name.
        sourcefile_index: u16,
    },
    /// `LineNumberTable` (inside `Code`).
    LineNumberTable(Vec<LineNumber>),
    /// `LocalVariableTable` (inside `Code`).
    LocalVariableTable(Vec<LocalVariable>),
    /// `LocalVariableTypeTable` (inside `Code`).
    LocalVariableTypeTable(Vec<LocalVariableType>),
    /// `Deprecated` (classes, fields, methods).
    Deprecated,
    /// `RuntimeVisibleAnnotations`.
    RuntimeVisibleAnnotations(Vec<Annotation>),
    /// `RuntimeInvisibleAnnotations`.
    RuntimeInvisibleAnnotations(Vec<Annotation>),
    /// `RuntimeVisibleParameterAnnotations` (methods): one list per parameter.
    RuntimeVisibleParameterAnnotations(Vec<Vec<Annotation>>),
    /// `RuntimeInvisibleParameterAnnotations` (methods): one list per parameter.
    RuntimeInvisibleParameterAnnotations(Vec<Vec<Annotation>>),
    /// `RuntimeVisibleTypeAnnotations`.
    RuntimeVisibleTypeAnnotations(Vec<TypeAnnotation>),
    /// `RuntimeInvisibleTypeAnnotations`.
    RuntimeInvisibleTypeAnnotations(Vec<TypeAnnotation>),
    /// `AnnotationDefault` (methods of annotation interfaces).
    AnnotationDefault(ElementValue),
    /// `BootstrapMethods` (classes).
    BootstrapMethods(Vec<BootstrapMethod>),
    /// `MethodParameters` (methods).
    MethodParameters(Vec<MethodParameter>),
    /// `NestHost` (classes).
    NestHost {
        /// `Class`: the nest host.
        host_class_index: u16,
    },
    /// `NestMembers` (classes): `Class` entries.
    NestMembers(Vec<u16>),
    /// `Record` (classes).
    Record(Vec<RecordComponent>),
    /// `PermittedSubclasses` (classes): `Class` entries.
    PermittedSubclasses(Vec<u16>),
    /// Any other attribute, or a known one in a place JVMS does not define it: the raw
    /// `info` bytes.
    Unknown(Vec<u8>),
}

impl AttributeBody {
    /// The standard attribute name of a typed body, or `None` for [`AttributeBody::Unknown`].
    pub fn name(&self) -> Option<&'static str> {
        Some(match self {
            Self::ConstantValue { .. } => "ConstantValue",
            Self::Code(_) => "Code",
            Self::StackMapTable(_) => "StackMapTable",
            Self::Exceptions(_) => "Exceptions",
            Self::InnerClasses(_) => "InnerClasses",
            Self::EnclosingMethod { .. } => "EnclosingMethod",
            Self::Synthetic => "Synthetic",
            Self::Signature { .. } => "Signature",
            Self::SourceFile { .. } => "SourceFile",
            Self::LineNumberTable(_) => "LineNumberTable",
            Self::LocalVariableTable(_) => "LocalVariableTable",
            Self::LocalVariableTypeTable(_) => "LocalVariableTypeTable",
            Self::Deprecated => "Deprecated",
            Self::RuntimeVisibleAnnotations(_) => "RuntimeVisibleAnnotations",
            Self::RuntimeInvisibleAnnotations(_) => "RuntimeInvisibleAnnotations",
            Self::RuntimeVisibleParameterAnnotations(_) => "RuntimeVisibleParameterAnnotations",
            Self::RuntimeInvisibleParameterAnnotations(_) => "RuntimeInvisibleParameterAnnotations",
            Self::RuntimeVisibleTypeAnnotations(_) => "RuntimeVisibleTypeAnnotations",
            Self::RuntimeInvisibleTypeAnnotations(_) => "RuntimeInvisibleTypeAnnotations",
            Self::AnnotationDefault(_) => "AnnotationDefault",
            Self::BootstrapMethods(_) => "BootstrapMethods",
            Self::MethodParameters(_) => "MethodParameters",
            Self::NestHost { .. } => "NestHost",
            Self::NestMembers(_) => "NestMembers",
            Self::Record(_) => "Record",
            Self::PermittedSubclasses(_) => "PermittedSubclasses",
            Self::Unknown(_) => return None,
        })
    }
}

/// The `Code` attribute of a method.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Code {
    /// Maximum operand stack depth.
    pub max_stack: u16,
    /// Number of local variable slots.
    pub max_locals: u16,
    /// The bytecode. Decode it with [`Code::instructions`].
    pub code: Vec<u8>,
    /// Exception handlers, in file order (which is the order the JVM searches them).
    pub exception_table: Vec<ExceptionHandler>,
    /// Attributes of the code: `LineNumberTable`, `StackMapTable` and so on.
    pub attributes: Vec<Attribute>,
}

impl Code {
    /// An iterator over the decoded instructions of [`Code::code`].
    pub fn instructions(&self) -> Instructions<'_> {
        Instructions::new(&self.code)
    }
}

/// One entry of a `Code` exception table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExceptionHandler {
    /// Start of the protected range (inclusive).
    pub start_pc: u16,
    /// End of the protected range (exclusive).
    pub end_pc: u16,
    /// Start of the handler.
    pub handler_pc: u16,
    /// `Class` of the caught exception, or 0 for any (`finally`).
    pub catch_type: u16,
}

/// One entry of `InnerClasses`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InnerClass {
    /// `Class`: the inner class.
    pub inner_class_info_index: u16,
    /// `Class`: the outer class, or 0 for local and anonymous classes.
    pub outer_class_info_index: u16,
    /// `Utf8`: the simple name, or 0 for anonymous classes.
    pub inner_name_index: u16,
    /// Access flags as declared in source.
    pub inner_class_access_flags: u16,
}

/// One entry of `LineNumberTable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineNumber {
    /// Bytecode offset where the line starts.
    pub start_pc: u16,
    /// Source line number.
    pub line_number: u16,
}

/// One entry of `LocalVariableTable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalVariable {
    /// Bytecode offset where the variable's range starts.
    pub start_pc: u16,
    /// Length of the range.
    pub length: u16,
    /// `Utf8`: the variable name.
    pub name_index: u16,
    /// `Utf8`: the field descriptor of its type.
    pub descriptor_index: u16,
    /// Local variable slot.
    pub index: u16,
}

/// One entry of `LocalVariableTypeTable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalVariableType {
    /// Bytecode offset where the variable's range starts.
    pub start_pc: u16,
    /// Length of the range.
    pub length: u16,
    /// `Utf8`: the variable name.
    pub name_index: u16,
    /// `Utf8`: the generic signature of its type.
    pub signature_index: u16,
    /// Local variable slot.
    pub index: u16,
}

/// One entry of `BootstrapMethods`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BootstrapMethod {
    /// `MethodHandle`: the bootstrap method.
    pub bootstrap_method_ref: u16,
    /// Loadable constants passed as static arguments.
    pub bootstrap_arguments: Vec<u16>,
}

/// One entry of `MethodParameters`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MethodParameter {
    /// `Utf8`: the parameter name, or 0 for none.
    pub name_index: u16,
    /// `ACC_FINAL`, `ACC_SYNTHETIC` and `ACC_MANDATED`.
    pub access_flags: u16,
}

/// One component of a `Record` attribute.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecordComponent {
    /// `Utf8`: the component name.
    pub name_index: u16,
    /// `Utf8`: the field descriptor.
    pub descriptor_index: u16,
    /// Attributes of the component (`Signature`, annotations).
    pub attributes: Vec<Attribute>,
}

/// Where an attribute table sits, which decides the attributes JVMS defines there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Context {
    Class,
    Field,
    Method,
    Code,
    RecordComponent,
}

/// The typed attributes, by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    ConstantValue,
    Code,
    StackMapTable,
    Exceptions,
    InnerClasses,
    EnclosingMethod,
    Synthetic,
    Signature,
    SourceFile,
    LineNumberTable,
    LocalVariableTable,
    LocalVariableTypeTable,
    Deprecated,
    RuntimeVisibleAnnotations,
    RuntimeInvisibleAnnotations,
    RuntimeVisibleParameterAnnotations,
    RuntimeInvisibleParameterAnnotations,
    RuntimeVisibleTypeAnnotations,
    RuntimeInvisibleTypeAnnotations,
    AnnotationDefault,
    BootstrapMethods,
    MethodParameters,
    NestHost,
    NestMembers,
    Record,
    PermittedSubclasses,
}

const KINDS: [(&str, Kind); 26] = [
    ("ConstantValue", Kind::ConstantValue),
    ("Code", Kind::Code),
    ("StackMapTable", Kind::StackMapTable),
    ("Exceptions", Kind::Exceptions),
    ("InnerClasses", Kind::InnerClasses),
    ("EnclosingMethod", Kind::EnclosingMethod),
    ("Synthetic", Kind::Synthetic),
    ("Signature", Kind::Signature),
    ("SourceFile", Kind::SourceFile),
    ("LineNumberTable", Kind::LineNumberTable),
    ("LocalVariableTable", Kind::LocalVariableTable),
    ("LocalVariableTypeTable", Kind::LocalVariableTypeTable),
    ("Deprecated", Kind::Deprecated),
    ("RuntimeVisibleAnnotations", Kind::RuntimeVisibleAnnotations),
    (
        "RuntimeInvisibleAnnotations",
        Kind::RuntimeInvisibleAnnotations,
    ),
    (
        "RuntimeVisibleParameterAnnotations",
        Kind::RuntimeVisibleParameterAnnotations,
    ),
    (
        "RuntimeInvisibleParameterAnnotations",
        Kind::RuntimeInvisibleParameterAnnotations,
    ),
    (
        "RuntimeVisibleTypeAnnotations",
        Kind::RuntimeVisibleTypeAnnotations,
    ),
    (
        "RuntimeInvisibleTypeAnnotations",
        Kind::RuntimeInvisibleTypeAnnotations,
    ),
    ("AnnotationDefault", Kind::AnnotationDefault),
    ("BootstrapMethods", Kind::BootstrapMethods),
    ("MethodParameters", Kind::MethodParameters),
    ("NestHost", Kind::NestHost),
    ("NestMembers", Kind::NestMembers),
    ("Record", Kind::Record),
    ("PermittedSubclasses", Kind::PermittedSubclasses),
];

impl Kind {
    fn from_name(name: &[u8]) -> Option<(&'static str, Self)> {
        KINDS.iter().find(|(n, _)| n.as_bytes() == name).copied()
    }

    /// Whether JVMS 4.7 (table 4.7-C) defines this attribute in `ctx`.
    fn allowed_in(self, ctx: Context) -> bool {
        use Context as C;
        match self {
            Self::ConstantValue => ctx == C::Field,
            Self::Code
            | Self::Exceptions
            | Self::RuntimeVisibleParameterAnnotations
            | Self::RuntimeInvisibleParameterAnnotations
            | Self::AnnotationDefault
            | Self::MethodParameters => ctx == C::Method,
            Self::StackMapTable
            | Self::LineNumberTable
            | Self::LocalVariableTable
            | Self::LocalVariableTypeTable => ctx == C::Code,
            Self::InnerClasses
            | Self::EnclosingMethod
            | Self::SourceFile
            | Self::BootstrapMethods
            | Self::NestHost
            | Self::NestMembers
            | Self::Record
            | Self::PermittedSubclasses => ctx == C::Class,
            Self::Synthetic | Self::Deprecated => matches!(ctx, C::Class | C::Field | C::Method),
            Self::Signature
            | Self::RuntimeVisibleAnnotations
            | Self::RuntimeInvisibleAnnotations => {
                matches!(ctx, C::Class | C::Field | C::Method | C::RecordComponent)
            }
            Self::RuntimeVisibleTypeAnnotations | Self::RuntimeInvisibleTypeAnnotations => true,
        }
    }
}

pub(crate) fn parse_attributes(
    r: &mut Reader<'_>,
    pool: &ConstantPool,
    ctx: Context,
) -> Result<Vec<Attribute>, Error> {
    let count = usize::from(r.u16()?);
    let mut out = Vec::with_capacity(capacity(count, r.remaining(), 6));
    for _ in 0..count {
        let name_index = r.u16()?;
        let length =
            usize::try_from(r.u32()?).map_err(|_| Error::UnexpectedEof { offset: r.offset() })?;
        let mut body = r.sub(length)?;
        let name = pool.utf8(name_index).map_err(|_| Error::BadConstantIndex {
            index: name_index,
            expected: "Utf8",
            context: "attribute name",
        })?;
        let body = match Kind::from_name(name) {
            Some((name, kind)) if kind.allowed_in(ctx) => {
                let offset = body.offset();
                let length_error = Error::AttributeLength {
                    name,
                    declared: length,
                    offset,
                };
                let parsed = parse_body(kind, &mut body, pool).map_err(|e| match e {
                    Error::UnexpectedEof { .. } => length_error.clone(),
                    other => other,
                })?;
                if body.remaining() != 0 {
                    return Err(length_error);
                }
                parsed
            }
            _ => AttributeBody::Unknown(body.rest().to_vec()),
        };
        out.push(Attribute { name_index, body });
    }
    Ok(out)
}

fn u16_list(r: &mut Reader<'_>) -> Result<Vec<u16>, Error> {
    let count = usize::from(r.u16()?);
    let mut out = Vec::with_capacity(capacity(count, r.remaining(), 2));
    for _ in 0..count {
        out.push(r.u16()?);
    }
    Ok(out)
}

fn parse_body(kind: Kind, r: &mut Reader<'_>, pool: &ConstantPool) -> Result<AttributeBody, Error> {
    Ok(match kind {
        Kind::ConstantValue => AttributeBody::ConstantValue {
            constantvalue_index: r.u16()?,
        },
        Kind::Code => {
            let max_stack = r.u16()?;
            let max_locals = r.u16()?;
            let code_length = usize::try_from(r.u32()?)
                .map_err(|_| Error::UnexpectedEof { offset: r.offset() })?;
            let code = r.bytes(code_length)?.to_vec();
            let n = usize::from(r.u16()?);
            let mut exception_table = Vec::with_capacity(capacity(n, r.remaining(), 8));
            for _ in 0..n {
                exception_table.push(ExceptionHandler {
                    start_pc: r.u16()?,
                    end_pc: r.u16()?,
                    handler_pc: r.u16()?,
                    catch_type: r.u16()?,
                });
            }
            let attributes = parse_attributes(r, pool, Context::Code)?;
            AttributeBody::Code(Code {
                max_stack,
                max_locals,
                code,
                exception_table,
                attributes,
            })
        }
        Kind::StackMapTable => AttributeBody::StackMapTable(parse_frames(r)?),
        Kind::Exceptions => AttributeBody::Exceptions(u16_list(r)?),
        Kind::InnerClasses => {
            let n = usize::from(r.u16()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 8));
            for _ in 0..n {
                list.push(InnerClass {
                    inner_class_info_index: r.u16()?,
                    outer_class_info_index: r.u16()?,
                    inner_name_index: r.u16()?,
                    inner_class_access_flags: r.u16()?,
                });
            }
            AttributeBody::InnerClasses(list)
        }
        Kind::EnclosingMethod => AttributeBody::EnclosingMethod {
            class_index: r.u16()?,
            method_index: r.u16()?,
        },
        Kind::Synthetic => AttributeBody::Synthetic,
        Kind::Signature => AttributeBody::Signature {
            signature_index: r.u16()?,
        },
        Kind::SourceFile => AttributeBody::SourceFile {
            sourcefile_index: r.u16()?,
        },
        Kind::LineNumberTable => {
            let n = usize::from(r.u16()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 4));
            for _ in 0..n {
                list.push(LineNumber {
                    start_pc: r.u16()?,
                    line_number: r.u16()?,
                });
            }
            AttributeBody::LineNumberTable(list)
        }
        Kind::LocalVariableTable => {
            let n = usize::from(r.u16()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 10));
            for _ in 0..n {
                list.push(LocalVariable {
                    start_pc: r.u16()?,
                    length: r.u16()?,
                    name_index: r.u16()?,
                    descriptor_index: r.u16()?,
                    index: r.u16()?,
                });
            }
            AttributeBody::LocalVariableTable(list)
        }
        Kind::LocalVariableTypeTable => {
            let n = usize::from(r.u16()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 10));
            for _ in 0..n {
                list.push(LocalVariableType {
                    start_pc: r.u16()?,
                    length: r.u16()?,
                    name_index: r.u16()?,
                    signature_index: r.u16()?,
                    index: r.u16()?,
                });
            }
            AttributeBody::LocalVariableTypeTable(list)
        }
        Kind::Deprecated => AttributeBody::Deprecated,
        Kind::RuntimeVisibleAnnotations => {
            AttributeBody::RuntimeVisibleAnnotations(parse_annotations(r)?)
        }
        Kind::RuntimeInvisibleAnnotations => {
            AttributeBody::RuntimeInvisibleAnnotations(parse_annotations(r)?)
        }
        Kind::RuntimeVisibleParameterAnnotations => {
            AttributeBody::RuntimeVisibleParameterAnnotations(parse_parameter_annotations(r)?)
        }
        Kind::RuntimeInvisibleParameterAnnotations => {
            AttributeBody::RuntimeInvisibleParameterAnnotations(parse_parameter_annotations(r)?)
        }
        Kind::RuntimeVisibleTypeAnnotations => {
            AttributeBody::RuntimeVisibleTypeAnnotations(parse_type_annotations(r)?)
        }
        Kind::RuntimeInvisibleTypeAnnotations => {
            AttributeBody::RuntimeInvisibleTypeAnnotations(parse_type_annotations(r)?)
        }
        Kind::AnnotationDefault => AttributeBody::AnnotationDefault(parse_element_value(r, 0)?),
        Kind::BootstrapMethods => {
            let n = usize::from(r.u16()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 4));
            for _ in 0..n {
                let bootstrap_method_ref = r.u16()?;
                let bootstrap_arguments = u16_list(r)?;
                list.push(BootstrapMethod {
                    bootstrap_method_ref,
                    bootstrap_arguments,
                });
            }
            AttributeBody::BootstrapMethods(list)
        }
        Kind::MethodParameters => {
            let n = usize::from(r.u8()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 4));
            for _ in 0..n {
                list.push(MethodParameter {
                    name_index: r.u16()?,
                    access_flags: r.u16()?,
                });
            }
            AttributeBody::MethodParameters(list)
        }
        Kind::NestHost => AttributeBody::NestHost {
            host_class_index: r.u16()?,
        },
        Kind::NestMembers => AttributeBody::NestMembers(u16_list(r)?),
        Kind::Record => {
            let n = usize::from(r.u16()?);
            let mut list = Vec::with_capacity(capacity(n, r.remaining(), 6));
            for _ in 0..n {
                let name_index = r.u16()?;
                let descriptor_index = r.u16()?;
                let attributes = parse_attributes(r, pool, Context::RecordComponent)?;
                list.push(RecordComponent {
                    name_index,
                    descriptor_index,
                    attributes,
                });
            }
            AttributeBody::Record(list)
        }
        Kind::PermittedSubclasses => AttributeBody::PermittedSubclasses(u16_list(r)?),
    })
}

pub(crate) fn write_attributes(list: &[Attribute], out: &mut Vec<u8>) -> Result<(), Error> {
    put_u16(out, count_u16(list.len(), "attributes")?);
    for attribute in list {
        put_u16(out, attribute.name_index);
        let at = begin_len(out);
        write_body(&attribute.body, out)?;
        patch_len(out, at, "attribute bytes")?;
    }
    Ok(())
}

fn put_u16_list(list: &[u16], what: &'static str, out: &mut Vec<u8>) -> Result<(), Error> {
    put_u16(out, count_u16(list.len(), what)?);
    for value in list {
        put_u16(out, *value);
    }
    Ok(())
}

fn write_body(body: &AttributeBody, out: &mut Vec<u8>) -> Result<(), Error> {
    match body {
        AttributeBody::ConstantValue {
            constantvalue_index: a,
        }
        | AttributeBody::Signature { signature_index: a }
        | AttributeBody::SourceFile {
            sourcefile_index: a,
        }
        | AttributeBody::NestHost {
            host_class_index: a,
        } => put_u16(out, *a),
        AttributeBody::Code(code) => {
            put_u16(out, code.max_stack);
            put_u16(out, code.max_locals);
            let len = u32::try_from(code.code.len()).map_err(|_| Error::TooMany {
                what: "bytes of code",
                count: code.code.len(),
                max: usize::try_from(u32::MAX).unwrap_or(usize::MAX),
            })?;
            put_u32(out, len);
            out.extend_from_slice(&code.code);
            put_u16(
                out,
                count_u16(code.exception_table.len(), "exception handlers")?,
            );
            for h in &code.exception_table {
                put_u16(out, h.start_pc);
                put_u16(out, h.end_pc);
                put_u16(out, h.handler_pc);
                put_u16(out, h.catch_type);
            }
            write_attributes(&code.attributes, out)?;
        }
        AttributeBody::StackMapTable(frames) => write_frames(frames, out)?,
        AttributeBody::Exceptions(list) => put_u16_list(list, "exceptions", out)?,
        AttributeBody::InnerClasses(list) => {
            put_u16(out, count_u16(list.len(), "inner classes")?);
            for c in list {
                put_u16(out, c.inner_class_info_index);
                put_u16(out, c.outer_class_info_index);
                put_u16(out, c.inner_name_index);
                put_u16(out, c.inner_class_access_flags);
            }
        }
        AttributeBody::EnclosingMethod {
            class_index,
            method_index,
        } => {
            put_u16(out, *class_index);
            put_u16(out, *method_index);
        }
        AttributeBody::Synthetic | AttributeBody::Deprecated => {}
        AttributeBody::LineNumberTable(list) => {
            put_u16(out, count_u16(list.len(), "line numbers")?);
            for l in list {
                put_u16(out, l.start_pc);
                put_u16(out, l.line_number);
            }
        }
        AttributeBody::LocalVariableTable(list) => {
            put_u16(out, count_u16(list.len(), "local variables")?);
            for v in list {
                for value in [
                    v.start_pc,
                    v.length,
                    v.name_index,
                    v.descriptor_index,
                    v.index,
                ] {
                    put_u16(out, value);
                }
            }
        }
        AttributeBody::LocalVariableTypeTable(list) => {
            put_u16(out, count_u16(list.len(), "local variable types")?);
            for v in list {
                for value in [
                    v.start_pc,
                    v.length,
                    v.name_index,
                    v.signature_index,
                    v.index,
                ] {
                    put_u16(out, value);
                }
            }
        }
        AttributeBody::RuntimeVisibleAnnotations(list)
        | AttributeBody::RuntimeInvisibleAnnotations(list) => {
            write_annotations(list, out)?;
        }
        AttributeBody::RuntimeVisibleParameterAnnotations(list)
        | AttributeBody::RuntimeInvisibleParameterAnnotations(list) => {
            write_parameter_annotations(list, out)?
        }
        AttributeBody::RuntimeVisibleTypeAnnotations(list)
        | AttributeBody::RuntimeInvisibleTypeAnnotations(list) => {
            write_type_annotations(list, out)?;
        }
        AttributeBody::AnnotationDefault(value) => write_element_value(value, out)?,
        AttributeBody::BootstrapMethods(list) => {
            put_u16(out, count_u16(list.len(), "bootstrap methods")?);
            for m in list {
                put_u16(out, m.bootstrap_method_ref);
                put_u16_list(&m.bootstrap_arguments, "bootstrap arguments", out)?;
            }
        }
        AttributeBody::MethodParameters(list) => {
            out.push(count_u8(list.len(), "method parameters")?);
            for p in list {
                put_u16(out, p.name_index);
                put_u16(out, p.access_flags);
            }
        }
        AttributeBody::NestMembers(list) => put_u16_list(list, "nest members", out)?,
        AttributeBody::PermittedSubclasses(list) => {
            put_u16_list(list, "permitted subclasses", out)?
        }
        AttributeBody::Record(list) => {
            put_u16(out, count_u16(list.len(), "record components")?);
            for c in list {
                put_u16(out, c.name_index);
                put_u16(out, c.descriptor_index);
                write_attributes(&c.attributes, out)?;
            }
        }
        AttributeBody::Unknown(bytes) => out.extend_from_slice(bytes),
    }
    Ok(())
}
