//! Annotations, element values and type annotations (JVMS 4.7.16 to 4.7.22).

use crate::Error;
use crate::bytes::{Reader, capacity, count_u8, count_u16, put_u8, put_u16};

/// How deep annotations and array values may nest before the parser gives up. Each level
/// costs a few bytes of input, so without a limit a small hostile file could exhaust the
/// stack. Compilers nest a handful of levels at most.
pub const MAX_NESTING: usize = 64;

/// One annotation: its type and its element-value pairs.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Annotation {
    /// `Utf8`: the annotation type as a field descriptor (`Lorg/example/Foo;`).
    pub type_index: u16,
    /// The element-value pairs, in file order.
    pub element_value_pairs: Vec<ElementValuePair>,
}

/// One `name = value` pair of an annotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ElementValuePair {
    /// `Utf8`: the element name.
    pub element_name_index: u16,
    /// The value.
    pub value: ElementValue,
}

/// The primitive or string kind of a [`ElementValue::Const`], from its tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstKind {
    /// `B`: an `Integer` constant holding a byte.
    Byte,
    /// `C`: an `Integer` constant holding a char.
    Char,
    /// `D`: a `Double` constant.
    Double,
    /// `F`: a `Float` constant.
    Float,
    /// `I`: an `Integer` constant.
    Int,
    /// `J`: a `Long` constant.
    Long,
    /// `S`: an `Integer` constant holding a short.
    Short,
    /// `Z`: an `Integer` constant holding a boolean.
    Boolean,
    /// `s`: a `Utf8` constant holding a string.
    String,
}

impl ConstKind {
    /// The tag byte.
    pub fn tag(self) -> u8 {
        match self {
            Self::Byte => b'B',
            Self::Char => b'C',
            Self::Double => b'D',
            Self::Float => b'F',
            Self::Int => b'I',
            Self::Long => b'J',
            Self::Short => b'S',
            Self::Boolean => b'Z',
            Self::String => b's',
        }
    }

    /// The kind for a tag byte, if it is a constant tag.
    pub fn from_tag(tag: u8) -> Option<Self> {
        Some(match tag {
            b'B' => Self::Byte,
            b'C' => Self::Char,
            b'D' => Self::Double,
            b'F' => Self::Float,
            b'I' => Self::Int,
            b'J' => Self::Long,
            b'S' => Self::Short,
            b'Z' => Self::Boolean,
            b's' => Self::String,
            _ => return None,
        })
    }
}

/// An annotation element value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ElementValue {
    /// A primitive or string constant.
    Const {
        /// Which kind, from the tag.
        kind: ConstKind,
        /// The constant: `Integer`, `Long`, `Float`, `Double` or `Utf8`, by kind.
        const_value_index: u16,
    },
    /// `e`: an enum constant.
    Enum {
        /// `Utf8`: the enum type as a field descriptor.
        type_name_index: u16,
        /// `Utf8`: the constant's simple name.
        const_name_index: u16,
    },
    /// `c`: a class literal.
    Class {
        /// `Utf8`: a return descriptor (`V` for `void.class`, else a field descriptor).
        class_info_index: u16,
    },
    /// `@`: a nested annotation.
    Annotation(Annotation),
    /// `[`: an array of values.
    Array(Vec<ElementValue>),
}

/// A type annotation (`RuntimeVisibleTypeAnnotations` and `RuntimeInvisibleTypeAnnotations`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeAnnotation {
    /// The `target_type` byte (JVMS tables 4.7.20-A and 4.7.20-B). It decides which
    /// [`TargetInfo`] shape follows, and is kept because several types share one shape.
    pub target_type: u8,
    /// Which type in the declaration or expression is annotated.
    pub target_info: TargetInfo,
    /// Where inside that type the annotation sits.
    pub target_path: Vec<TypePathEntry>,
    /// The annotation itself.
    pub annotation: Annotation,
}

/// The `target_info` union of a type annotation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TargetInfo {
    /// Target types 0x00 and 0x01: a type parameter of a class or method.
    TypeParameter {
        /// Which type parameter.
        type_parameter_index: u8,
    },
    /// Target type 0x10: `extends` or `implements`; 65535 means the superclass.
    Supertype {
        /// Index into `interfaces`, or 65535.
        supertype_index: u16,
    },
    /// Target types 0x11 and 0x12: a bound of a type parameter.
    TypeParameterBound {
        /// Which type parameter.
        type_parameter_index: u8,
        /// Which bound.
        bound_index: u8,
    },
    /// Target types 0x13, 0x14 and 0x15: a field type, a return type or a receiver type.
    Empty,
    /// Target type 0x16: a formal parameter type.
    FormalParameter {
        /// Which parameter.
        formal_parameter_index: u8,
    },
    /// Target type 0x17: a type in a `throws` clause.
    Throws {
        /// Index into the `Exceptions` attribute's table.
        throws_type_index: u16,
    },
    /// Target types 0x40 and 0x41: a local variable or resource variable type.
    LocalVar(Vec<LocalVarTarget>),
    /// Target type 0x42: an exception parameter type.
    Catch {
        /// Index into the `Code` attribute's exception table.
        exception_table_index: u16,
    },
    /// Target types 0x43 to 0x46: `instanceof`, `new` and method reference expressions.
    Offset {
        /// Bytecode offset of the instruction.
        offset: u16,
    },
    /// Target types 0x47 to 0x4B: a cast, or a type argument of a call or method reference.
    TypeArgument {
        /// Bytecode offset of the instruction.
        offset: u16,
        /// Which type argument.
        type_argument_index: u8,
    },
}

impl TargetInfo {
    /// Whether this shape is the one `target_type` calls for.
    pub fn matches_target_type(&self, target_type: u8) -> bool {
        matches!(
            (target_type, self),
            (0x00 | 0x01, Self::TypeParameter { .. })
                | (0x10, Self::Supertype { .. })
                | (0x11 | 0x12, Self::TypeParameterBound { .. })
                | (0x13..=0x15, Self::Empty)
                | (0x16, Self::FormalParameter { .. })
                | (0x17, Self::Throws { .. })
                | (0x40 | 0x41, Self::LocalVar(_))
                | (0x42, Self::Catch { .. })
                | (0x43..=0x46, Self::Offset { .. })
                | (0x47..=0x4B, Self::TypeArgument { .. })
        )
    }
}

/// One live range of an annotated local variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalVarTarget {
    /// Bytecode offset where the range starts.
    pub start_pc: u16,
    /// Length of the range in bytes.
    pub length: u16,
    /// Local variable slot.
    pub index: u16,
}

/// One step of a type path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypePathEntry {
    /// 0: deeper in an array type; 1: deeper in a nested type; 2: on a wildcard bound;
    /// 3: on a type argument.
    pub type_path_kind: u8,
    /// Which type argument, when `type_path_kind` is 3; otherwise 0.
    pub type_argument_index: u8,
}

pub(crate) fn parse_annotations(r: &mut Reader<'_>) -> Result<Vec<Annotation>, Error> {
    let count = usize::from(r.u16()?);
    let mut out = Vec::with_capacity(capacity(count, r.remaining(), 4));
    for _ in 0..count {
        out.push(parse_annotation(r, 0)?);
    }
    Ok(out)
}

pub(crate) fn parse_parameter_annotations(
    r: &mut Reader<'_>,
) -> Result<Vec<Vec<Annotation>>, Error> {
    let count = usize::from(r.u8()?);
    let mut out = Vec::with_capacity(capacity(count, r.remaining(), 2));
    for _ in 0..count {
        out.push(parse_annotations(r)?);
    }
    Ok(out)
}

pub(crate) fn parse_type_annotations(r: &mut Reader<'_>) -> Result<Vec<TypeAnnotation>, Error> {
    let count = usize::from(r.u16()?);
    let mut out = Vec::with_capacity(capacity(count, r.remaining(), 6));
    for _ in 0..count {
        out.push(parse_type_annotation(r)?);
    }
    Ok(out)
}

fn parse_annotation(r: &mut Reader<'_>, depth: usize) -> Result<Annotation, Error> {
    let type_index = r.u16()?;
    let count = usize::from(r.u16()?);
    let mut element_value_pairs = Vec::with_capacity(capacity(count, r.remaining(), 5));
    for _ in 0..count {
        let element_name_index = r.u16()?;
        let value = parse_element_value(r, depth + 1)?;
        element_value_pairs.push(ElementValuePair {
            element_name_index,
            value,
        });
    }
    Ok(Annotation {
        type_index,
        element_value_pairs,
    })
}

pub(crate) fn parse_element_value(r: &mut Reader<'_>, depth: usize) -> Result<ElementValue, Error> {
    let offset = r.offset();
    if depth > MAX_NESTING {
        return Err(Error::TooDeep {
            offset,
            limit: MAX_NESTING,
        });
    }
    let tag = r.u8()?;
    if let Some(kind) = ConstKind::from_tag(tag) {
        return Ok(ElementValue::Const {
            kind,
            const_value_index: r.u16()?,
        });
    }
    Ok(match tag {
        b'e' => ElementValue::Enum {
            type_name_index: r.u16()?,
            const_name_index: r.u16()?,
        },
        b'c' => ElementValue::Class {
            class_info_index: r.u16()?,
        },
        b'@' => ElementValue::Annotation(parse_annotation(r, depth + 1)?),
        b'[' => {
            let count = usize::from(r.u16()?);
            let mut values = Vec::with_capacity(capacity(count, r.remaining(), 3));
            for _ in 0..count {
                values.push(parse_element_value(r, depth + 1)?);
            }
            ElementValue::Array(values)
        }
        other => return Err(Error::BadElementValueTag { tag: other, offset }),
    })
}

fn parse_type_annotation(r: &mut Reader<'_>) -> Result<TypeAnnotation, Error> {
    let offset = r.offset();
    let target_type = r.u8()?;
    let target_info = match target_type {
        0x00 | 0x01 => TargetInfo::TypeParameter {
            type_parameter_index: r.u8()?,
        },
        0x10 => TargetInfo::Supertype {
            supertype_index: r.u16()?,
        },
        0x11 | 0x12 => TargetInfo::TypeParameterBound {
            type_parameter_index: r.u8()?,
            bound_index: r.u8()?,
        },
        0x13..=0x15 => TargetInfo::Empty,
        0x16 => TargetInfo::FormalParameter {
            formal_parameter_index: r.u8()?,
        },
        0x17 => TargetInfo::Throws {
            throws_type_index: r.u16()?,
        },
        0x40 | 0x41 => {
            let count = usize::from(r.u16()?);
            let mut table = Vec::with_capacity(capacity(count, r.remaining(), 6));
            for _ in 0..count {
                table.push(LocalVarTarget {
                    start_pc: r.u16()?,
                    length: r.u16()?,
                    index: r.u16()?,
                });
            }
            TargetInfo::LocalVar(table)
        }
        0x42 => TargetInfo::Catch {
            exception_table_index: r.u16()?,
        },
        0x43..=0x46 => TargetInfo::Offset { offset: r.u16()? },
        0x47..=0x4B => TargetInfo::TypeArgument {
            offset: r.u16()?,
            type_argument_index: r.u8()?,
        },
        other => {
            return Err(Error::BadTargetType {
                target_type: other,
                offset,
            });
        }
    };
    let path_length = usize::from(r.u8()?);
    let mut target_path = Vec::with_capacity(path_length);
    for _ in 0..path_length {
        let kind_offset = r.offset();
        let type_path_kind = r.u8()?;
        if type_path_kind > 3 {
            return Err(Error::BadTypePathKind {
                kind: type_path_kind,
                offset: kind_offset,
            });
        }
        target_path.push(TypePathEntry {
            type_path_kind,
            type_argument_index: r.u8()?,
        });
    }
    let annotation = parse_annotation(r, 0)?;
    Ok(TypeAnnotation {
        target_type,
        target_info,
        target_path,
        annotation,
    })
}

pub(crate) fn write_annotations(list: &[Annotation], out: &mut Vec<u8>) -> Result<(), Error> {
    put_u16(out, count_u16(list.len(), "annotations")?);
    for annotation in list {
        write_annotation(annotation, out)?;
    }
    Ok(())
}

pub(crate) fn write_parameter_annotations(
    list: &[Vec<Annotation>],
    out: &mut Vec<u8>,
) -> Result<(), Error> {
    put_u8(out, count_u8(list.len(), "annotated parameters")?);
    for annotations in list {
        write_annotations(annotations, out)?;
    }
    Ok(())
}

pub(crate) fn write_type_annotations(
    list: &[TypeAnnotation],
    out: &mut Vec<u8>,
) -> Result<(), Error> {
    put_u16(out, count_u16(list.len(), "type annotations")?);
    for annotation in list {
        write_type_annotation(annotation, out)?;
    }
    Ok(())
}

fn write_annotation(annotation: &Annotation, out: &mut Vec<u8>) -> Result<(), Error> {
    put_u16(out, annotation.type_index);
    put_u16(
        out,
        count_u16(annotation.element_value_pairs.len(), "element-value pairs")?,
    );
    for pair in &annotation.element_value_pairs {
        put_u16(out, pair.element_name_index);
        write_element_value(&pair.value, out)?;
    }
    Ok(())
}

pub(crate) fn write_element_value(value: &ElementValue, out: &mut Vec<u8>) -> Result<(), Error> {
    match value {
        ElementValue::Const {
            kind,
            const_value_index,
        } => {
            put_u8(out, kind.tag());
            put_u16(out, *const_value_index);
        }
        ElementValue::Enum {
            type_name_index,
            const_name_index,
        } => {
            put_u8(out, b'e');
            put_u16(out, *type_name_index);
            put_u16(out, *const_name_index);
        }
        ElementValue::Class { class_info_index } => {
            put_u8(out, b'c');
            put_u16(out, *class_info_index);
        }
        ElementValue::Annotation(annotation) => {
            put_u8(out, b'@');
            write_annotation(annotation, out)?;
        }
        ElementValue::Array(values) => {
            put_u8(out, b'[');
            put_u16(out, count_u16(values.len(), "array element values")?);
            for value in values {
                write_element_value(value, out)?;
            }
        }
    }
    Ok(())
}

fn write_type_annotation(annotation: &TypeAnnotation, out: &mut Vec<u8>) -> Result<(), Error> {
    if !annotation
        .target_info
        .matches_target_type(annotation.target_type)
    {
        return Err(Error::InvalidValue {
            what: "type annotation target_info does not match its target_type",
        });
    }
    put_u8(out, annotation.target_type);
    match &annotation.target_info {
        TargetInfo::TypeParameter {
            type_parameter_index,
        } => put_u8(out, *type_parameter_index),
        TargetInfo::Supertype { supertype_index } => put_u16(out, *supertype_index),
        TargetInfo::TypeParameterBound {
            type_parameter_index,
            bound_index,
        } => {
            put_u8(out, *type_parameter_index);
            put_u8(out, *bound_index);
        }
        TargetInfo::Empty => {}
        TargetInfo::FormalParameter {
            formal_parameter_index,
        } => put_u8(out, *formal_parameter_index),
        TargetInfo::Throws { throws_type_index } => put_u16(out, *throws_type_index),
        TargetInfo::LocalVar(table) => {
            put_u16(out, count_u16(table.len(), "local variable targets")?);
            for entry in table {
                put_u16(out, entry.start_pc);
                put_u16(out, entry.length);
                put_u16(out, entry.index);
            }
        }
        TargetInfo::Catch {
            exception_table_index,
        } => put_u16(out, *exception_table_index),
        TargetInfo::Offset { offset } => put_u16(out, *offset),
        TargetInfo::TypeArgument {
            offset,
            type_argument_index,
        } => {
            put_u16(out, *offset);
            put_u8(out, *type_argument_index);
        }
    }
    put_u8(
        out,
        count_u8(annotation.target_path.len(), "type path entries")?,
    );
    for entry in &annotation.target_path {
        if entry.type_path_kind > 3 {
            return Err(Error::InvalidValue {
                what: "type_path_kind above 3",
            });
        }
        put_u8(out, entry.type_path_kind);
        put_u8(out, entry.type_argument_index);
    }
    write_annotation(&annotation.annotation, out)
}
