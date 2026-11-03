//! `StackMapTable` frames and verification types (JVMS 4.7.4).

use crate::Error;
use crate::bytes::{Reader, capacity, count_u16, put_u8, put_u16};

/// One verification type of a stack map frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VerificationType {
    /// Tag 0.
    Top,
    /// Tag 1.
    Integer,
    /// Tag 2.
    Float,
    /// Tag 3.
    Double,
    /// Tag 4.
    Long,
    /// Tag 5.
    Null,
    /// Tag 6.
    UninitializedThis,
    /// Tag 7: an instance of a class.
    Object {
        /// `Class`: the type.
        cpool_index: u16,
    },
    /// Tag 8: the result of a `new` that has not been initialized yet.
    Uninitialized {
        /// Bytecode offset of the `new` instruction.
        offset: u16,
    },
}

/// One frame of a `StackMapTable`.
///
/// Each frame type the format defines has its own variant, so a table written back is
/// byte-identical: [`StackMapFrame::Same`] with a small delta and
/// [`StackMapFrame::SameExtended`] mean the same thing but are encoded differently.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StackMapFrame {
    /// Frame types 0..=63; the frame type is the offset delta.
    Same {
        /// Offset delta, 0..=63.
        offset_delta: u8,
    },
    /// Frame types 64..=127; the offset delta is the frame type minus 64.
    SameLocals1StackItem {
        /// Offset delta, 0..=63.
        offset_delta: u8,
        /// The one stack item.
        stack: VerificationType,
    },
    /// Frame type 247.
    SameLocals1StackItemExtended {
        /// Offset delta.
        offset_delta: u16,
        /// The one stack item.
        stack: VerificationType,
    },
    /// Frame types 248..=250: the last 1 to 3 locals are gone and the stack is empty.
    Chop {
        /// How many locals were removed, 1..=3 (251 minus the frame type).
        chopped: u8,
        /// Offset delta.
        offset_delta: u16,
    },
    /// Frame type 251.
    SameExtended {
        /// Offset delta.
        offset_delta: u16,
    },
    /// Frame types 252..=254: 1 to 3 more locals and an empty stack.
    Append {
        /// Offset delta.
        offset_delta: u16,
        /// The added locals, 1..=3 of them (the frame type minus 251).
        locals: Vec<VerificationType>,
    },
    /// Frame type 255.
    Full {
        /// Offset delta.
        offset_delta: u16,
        /// All locals.
        locals: Vec<VerificationType>,
        /// All stack items.
        stack: Vec<VerificationType>,
    },
}

fn parse_type(r: &mut Reader<'_>) -> Result<VerificationType, Error> {
    let offset = r.offset();
    Ok(match r.u8()? {
        0 => VerificationType::Top,
        1 => VerificationType::Integer,
        2 => VerificationType::Float,
        3 => VerificationType::Double,
        4 => VerificationType::Long,
        5 => VerificationType::Null,
        6 => VerificationType::UninitializedThis,
        7 => VerificationType::Object {
            cpool_index: r.u16()?,
        },
        8 => VerificationType::Uninitialized { offset: r.u16()? },
        tag => return Err(Error::BadVerificationType { tag, offset }),
    })
}

fn parse_types(r: &mut Reader<'_>, count: usize) -> Result<Vec<VerificationType>, Error> {
    let mut out = Vec::with_capacity(capacity(count, r.remaining(), 1));
    for _ in 0..count {
        out.push(parse_type(r)?);
    }
    Ok(out)
}

pub(crate) fn parse_frames(r: &mut Reader<'_>) -> Result<Vec<StackMapFrame>, Error> {
    let count = usize::from(r.u16()?);
    let mut frames = Vec::with_capacity(capacity(count, r.remaining(), 1));
    for _ in 0..count {
        let offset = r.offset();
        let frame_type = r.u8()?;
        let frame = match frame_type {
            0..=63 => StackMapFrame::Same {
                offset_delta: frame_type,
            },
            64..=127 => StackMapFrame::SameLocals1StackItem {
                offset_delta: frame_type - 64,
                stack: parse_type(r)?,
            },
            247 => StackMapFrame::SameLocals1StackItemExtended {
                offset_delta: r.u16()?,
                stack: parse_type(r)?,
            },
            248..=250 => StackMapFrame::Chop {
                chopped: 251 - frame_type,
                offset_delta: r.u16()?,
            },
            251 => StackMapFrame::SameExtended {
                offset_delta: r.u16()?,
            },
            252..=254 => {
                let offset_delta = r.u16()?;
                let locals = parse_types(r, usize::from(frame_type - 251))?;
                StackMapFrame::Append {
                    offset_delta,
                    locals,
                }
            }
            255 => {
                let offset_delta = r.u16()?;
                let n_locals = usize::from(r.u16()?);
                let locals = parse_types(r, n_locals)?;
                let n_stack = usize::from(r.u16()?);
                let stack = parse_types(r, n_stack)?;
                StackMapFrame::Full {
                    offset_delta,
                    locals,
                    stack,
                }
            }
            _ => return Err(Error::BadFrameType { frame_type, offset }),
        };
        frames.push(frame);
    }
    Ok(frames)
}

fn write_type(t: VerificationType, out: &mut Vec<u8>) {
    match t {
        VerificationType::Top => put_u8(out, 0),
        VerificationType::Integer => put_u8(out, 1),
        VerificationType::Float => put_u8(out, 2),
        VerificationType::Double => put_u8(out, 3),
        VerificationType::Long => put_u8(out, 4),
        VerificationType::Null => put_u8(out, 5),
        VerificationType::UninitializedThis => put_u8(out, 6),
        VerificationType::Object { cpool_index } => {
            put_u8(out, 7);
            put_u16(out, cpool_index);
        }
        VerificationType::Uninitialized { offset } => {
            put_u8(out, 8);
            put_u16(out, offset);
        }
    }
}

pub(crate) fn write_frames(frames: &[StackMapFrame], out: &mut Vec<u8>) -> Result<(), Error> {
    put_u16(out, count_u16(frames.len(), "stack map frames")?);
    for frame in frames {
        match frame {
            StackMapFrame::Same { offset_delta } => {
                if *offset_delta > 63 {
                    return Err(Error::InvalidValue {
                        what: "same frame offset_delta above 63",
                    });
                }
                put_u8(out, *offset_delta);
            }
            StackMapFrame::SameLocals1StackItem {
                offset_delta,
                stack,
            } => {
                if *offset_delta > 63 {
                    return Err(Error::InvalidValue {
                        what: "same_locals_1_stack_item offset_delta above 63",
                    });
                }
                put_u8(out, 64 + offset_delta);
                write_type(*stack, out);
            }
            StackMapFrame::SameLocals1StackItemExtended {
                offset_delta,
                stack,
            } => {
                put_u8(out, 247);
                put_u16(out, *offset_delta);
                write_type(*stack, out);
            }
            StackMapFrame::Chop {
                chopped,
                offset_delta,
            } => {
                if !(1..=3).contains(chopped) {
                    return Err(Error::InvalidValue {
                        what: "chop frame must remove 1 to 3 locals",
                    });
                }
                put_u8(out, 251 - chopped);
                put_u16(out, *offset_delta);
            }
            StackMapFrame::SameExtended { offset_delta } => {
                put_u8(out, 251);
                put_u16(out, *offset_delta);
            }
            StackMapFrame::Append {
                offset_delta,
                locals,
            } => {
                let n = u8::try_from(locals.len())
                    .ok()
                    .filter(|n| (1..=3).contains(n));
                let Some(n) = n else {
                    return Err(Error::InvalidValue {
                        what: "append frame must add 1 to 3 locals",
                    });
                };
                put_u8(out, 251 + n);
                put_u16(out, *offset_delta);
                for t in locals {
                    write_type(*t, out);
                }
            }
            StackMapFrame::Full {
                offset_delta,
                locals,
                stack,
            } => {
                put_u8(out, 255);
                put_u16(out, *offset_delta);
                put_u16(out, count_u16(locals.len(), "full frame locals")?);
                for t in locals {
                    write_type(*t, out);
                }
                put_u16(out, count_u16(stack.len(), "full frame stack items")?);
                for t in stack {
                    write_type(*t, out);
                }
            }
        }
    }
    Ok(())
}
