//! The one error type of the crate.

use std::fmt;

use crate::mutf8::Mutf8Error;

/// Everything that can go wrong while reading, writing or decoding a class file.
///
/// Offsets named `offset` are byte offsets into the class file, except for the
/// instruction errors, whose `offset` is the bytecode offset inside the `Code` array.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input ended in the middle of a structure.
    UnexpectedEof {
        /// Where the missing bytes should have started.
        offset: usize,
    },
    /// The file does not start with `0xCAFEBABE`.
    BadMagic {
        /// The first four bytes, big-endian.
        found: u32,
    },
    /// Bytes follow the last attribute of the class.
    TrailingBytes {
        /// Where the extra bytes start.
        offset: usize,
        /// How many extra bytes there are.
        count: usize,
    },
    /// `constant_pool_count` is zero; the smallest valid value is 1 (an empty pool).
    EmptyConstantPoolCount,
    /// A constant-pool entry has a tag the JVM does not define.
    BadConstantTag {
        /// Constant-pool index of the entry.
        index: u16,
        /// The tag byte found.
        tag: u8,
        /// Offset of the tag byte.
        offset: usize,
    },
    /// A `Long` or `Double` entry sits in the last slot, so its second slot does not exist.
    WideConstantAtEnd {
        /// Constant-pool index of the entry.
        index: u16,
    },
    /// An index points at no constant, or at a constant of the wrong kind.
    BadConstantIndex {
        /// The index that was looked up.
        index: u16,
        /// What the index should have pointed at, such as `"Utf8"` or `"Class"`.
        expected: &'static str,
        /// Where the index came from, such as `"this_class"`.
        context: &'static str,
    },
    /// A `MethodHandle` entry has a reference kind outside 1..=9.
    BadReferenceKind {
        /// Constant-pool index of the `MethodHandle` entry.
        index: u16,
        /// The reference kind found.
        kind: u8,
    },
    /// A known attribute's contents do not fill its declared `attribute_length` exactly.
    AttributeLength {
        /// The attribute name.
        name: &'static str,
        /// The declared `attribute_length`.
        declared: usize,
        /// Offset of the attribute contents.
        offset: usize,
    },
    /// A `StackMapTable` frame uses a reserved frame type (128..=246).
    BadFrameType {
        /// The frame type found.
        frame_type: u8,
        /// Offset of the frame type byte.
        offset: usize,
    },
    /// A verification type has an undefined tag.
    BadVerificationType {
        /// The tag found.
        tag: u8,
        /// Offset of the tag byte.
        offset: usize,
    },
    /// An annotation element value has an undefined tag.
    BadElementValueTag {
        /// The tag found.
        tag: u8,
        /// Offset of the tag byte.
        offset: usize,
    },
    /// A type annotation has an undefined `target_type`.
    BadTargetType {
        /// The target type found.
        target_type: u8,
        /// Offset of the target type byte.
        offset: usize,
    },
    /// A type path entry has a `type_path_kind` outside 0..=3.
    BadTypePathKind {
        /// The kind found.
        kind: u8,
        /// Offset of the kind byte.
        offset: usize,
    },
    /// Annotations or element values nest deeper than the parser allows.
    TooDeep {
        /// Offset of the value that went past the limit.
        offset: usize,
        /// The nesting limit.
        limit: usize,
    },
    /// A list is too long for the count field the class-file format gives it.
    TooMany {
        /// What the list holds.
        what: &'static str,
        /// How many entries it has.
        count: usize,
        /// The largest count the format can hold.
        max: usize,
    },
    /// A value cannot be written because it breaks a rule of the format.
    InvalidValue {
        /// What is wrong.
        what: &'static str,
    },
    /// Appending would push `constant_pool_count` past 65535.
    ConstantPoolFull,
    /// A constant-pool replacement would change an entry's width (one slot versus two).
    WidthMismatch {
        /// The index being replaced.
        index: u16,
    },
    /// A `Utf8` constant is not valid Modified UTF-8, or holds a lone surrogate.
    Utf8 {
        /// Constant-pool index of the entry.
        index: u16,
        /// What the decoder found.
        error: Mutf8Error,
    },
    /// The bytecode holds an opcode the JVM does not define, or a reserved one.
    BadOpcode {
        /// Bytecode offset of the instruction.
        offset: u32,
        /// The opcode byte.
        opcode: u8,
    },
    /// A `wide` prefix modifies an opcode that `wide` cannot modify.
    BadWide {
        /// Bytecode offset of the `wide` prefix.
        offset: u32,
        /// The opcode after the prefix.
        opcode: u8,
    },
    /// The code array ends inside an instruction.
    TruncatedInstruction {
        /// Bytecode offset of the instruction.
        offset: u32,
    },
    /// A `tableswitch` has `low > high`, or a `lookupswitch` a negative pair count.
    BadSwitch {
        /// Bytecode offset of the instruction.
        offset: u32,
    },
    /// An operand byte that must be zero (`invokeinterface`, `invokedynamic`) is not.
    NonZeroOperand {
        /// Bytecode offset of the instruction.
        offset: u32,
    },
    /// An instruction's operands do not match its opcode, or do not fit its encoding.
    OperandMismatch {
        /// Bytecode offset of the instruction.
        offset: u32,
        /// The opcode.
        opcode: u8,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { offset } => write!(f, "input ends early at byte {offset}"),
            Self::BadMagic { found } => write!(f, "bad magic 0x{found:08X}, expected 0xCAFEBABE"),
            Self::TrailingBytes { offset, count } => {
                write!(
                    f,
                    "{count} trailing bytes after the class file at byte {offset}"
                )
            }
            Self::EmptyConstantPoolCount => write!(f, "constant_pool_count is 0"),
            Self::BadConstantTag { index, tag, offset } => {
                write!(
                    f,
                    "constant #{index} has unknown tag {tag} at byte {offset}"
                )
            }
            Self::WideConstantAtEnd { index } => {
                write!(
                    f,
                    "Long or Double constant #{index} has no room for its second slot"
                )
            }
            Self::BadConstantIndex {
                index,
                expected,
                context,
            } => {
                write!(f, "{context}: constant #{index} is not a {expected} entry")
            }
            Self::BadReferenceKind { index, kind } => {
                write!(f, "MethodHandle #{index} has invalid reference kind {kind}")
            }
            Self::AttributeLength {
                name,
                declared,
                offset,
            } => write!(
                f,
                "{name} attribute at byte {offset} does not fill its declared length {declared}"
            ),
            Self::BadFrameType { frame_type, offset } => {
                write!(
                    f,
                    "reserved stack map frame type {frame_type} at byte {offset}"
                )
            }
            Self::BadVerificationType { tag, offset } => {
                write!(f, "unknown verification type tag {tag} at byte {offset}")
            }
            Self::BadElementValueTag { tag, offset } => {
                write!(f, "unknown element value tag 0x{tag:02X} at byte {offset}")
            }
            Self::BadTargetType {
                target_type,
                offset,
            } => {
                write!(
                    f,
                    "unknown type annotation target 0x{target_type:02X} at byte {offset}"
                )
            }
            Self::BadTypePathKind { kind, offset } => {
                write!(f, "unknown type path kind {kind} at byte {offset}")
            }
            Self::TooDeep { offset, limit } => {
                write!(
                    f,
                    "annotation values nest deeper than {limit} at byte {offset}"
                )
            }
            Self::TooMany { what, count, max } => {
                write!(f, "{count} {what} do not fit the format's limit of {max}")
            }
            Self::InvalidValue { what } => write!(f, "cannot write: {what}"),
            Self::ConstantPoolFull => write!(f, "constant pool is full (65535 slots)"),
            Self::WidthMismatch { index } => {
                write!(f, "replacing constant #{index} would change its slot width")
            }
            Self::Utf8 { index, error } => write!(f, "Utf8 constant #{index}: {error}"),
            Self::BadOpcode { offset, opcode } => {
                write!(
                    f,
                    "invalid opcode 0x{opcode:02X} at bytecode offset {offset}"
                )
            }
            Self::BadWide { offset, opcode } => {
                write!(
                    f,
                    "wide cannot modify opcode 0x{opcode:02X} at bytecode offset {offset}"
                )
            }
            Self::TruncatedInstruction { offset } => {
                write!(
                    f,
                    "code ends inside the instruction at bytecode offset {offset}"
                )
            }
            Self::BadSwitch { offset } => {
                write!(f, "malformed switch at bytecode offset {offset}")
            }
            Self::NonZeroOperand { offset } => {
                write!(
                    f,
                    "reserved operand byte is not zero at bytecode offset {offset}"
                )
            }
            Self::OperandMismatch { offset, opcode } => write!(
                f,
                "operands do not fit opcode 0x{opcode:02X} at bytecode offset {offset}"
            ),
        }
    }
}

impl std::error::Error for Error {}
