//! The instruction decoder (JVMS chapter 6) and a matching encoder.

use std::fmt;

use crate::Error;

/// A JVM opcode. Only the 202 opcodes JVMS defines for class files (`0x00..=0xC9`) exist;
/// the reserved `breakpoint`, `impdep1` and `impdep2` are not valid in a class file.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Opcode(u8);

macro_rules! opcodes {
    ($($name:ident = $value:literal, $mnemonic:literal;)*) => {
        impl Opcode {
            $(
                #[doc = concat!("`", $mnemonic, "`")]
                pub const $name: Opcode = Opcode($value);
            )*
        }

        /// Mnemonics, indexed by opcode.
        const MNEMONICS: [&str; 202] = [$($mnemonic),*];
    };
}

opcodes! {
    NOP = 0x00, "nop"; ACONST_NULL = 0x01, "aconst_null"; ICONST_M1 = 0x02, "iconst_m1";
    ICONST_0 = 0x03, "iconst_0"; ICONST_1 = 0x04, "iconst_1"; ICONST_2 = 0x05, "iconst_2";
    ICONST_3 = 0x06, "iconst_3"; ICONST_4 = 0x07, "iconst_4"; ICONST_5 = 0x08, "iconst_5";
    LCONST_0 = 0x09, "lconst_0"; LCONST_1 = 0x0A, "lconst_1"; FCONST_0 = 0x0B, "fconst_0";
    FCONST_1 = 0x0C, "fconst_1"; FCONST_2 = 0x0D, "fconst_2"; DCONST_0 = 0x0E, "dconst_0";
    DCONST_1 = 0x0F, "dconst_1"; BIPUSH = 0x10, "bipush"; SIPUSH = 0x11, "sipush";
    LDC = 0x12, "ldc"; LDC_W = 0x13, "ldc_w"; LDC2_W = 0x14, "ldc2_w";
    ILOAD = 0x15, "iload"; LLOAD = 0x16, "lload"; FLOAD = 0x17, "fload";
    DLOAD = 0x18, "dload"; ALOAD = 0x19, "aload"; ILOAD_0 = 0x1A, "iload_0";
    ILOAD_1 = 0x1B, "iload_1"; ILOAD_2 = 0x1C, "iload_2"; ILOAD_3 = 0x1D, "iload_3";
    LLOAD_0 = 0x1E, "lload_0"; LLOAD_1 = 0x1F, "lload_1"; LLOAD_2 = 0x20, "lload_2";
    LLOAD_3 = 0x21, "lload_3"; FLOAD_0 = 0x22, "fload_0"; FLOAD_1 = 0x23, "fload_1";
    FLOAD_2 = 0x24, "fload_2"; FLOAD_3 = 0x25, "fload_3"; DLOAD_0 = 0x26, "dload_0";
    DLOAD_1 = 0x27, "dload_1"; DLOAD_2 = 0x28, "dload_2"; DLOAD_3 = 0x29, "dload_3";
    ALOAD_0 = 0x2A, "aload_0"; ALOAD_1 = 0x2B, "aload_1"; ALOAD_2 = 0x2C, "aload_2";
    ALOAD_3 = 0x2D, "aload_3"; IALOAD = 0x2E, "iaload"; LALOAD = 0x2F, "laload";
    FALOAD = 0x30, "faload"; DALOAD = 0x31, "daload"; AALOAD = 0x32, "aaload";
    BALOAD = 0x33, "baload"; CALOAD = 0x34, "caload"; SALOAD = 0x35, "saload";
    ISTORE = 0x36, "istore"; LSTORE = 0x37, "lstore"; FSTORE = 0x38, "fstore";
    DSTORE = 0x39, "dstore"; ASTORE = 0x3A, "astore"; ISTORE_0 = 0x3B, "istore_0";
    ISTORE_1 = 0x3C, "istore_1"; ISTORE_2 = 0x3D, "istore_2"; ISTORE_3 = 0x3E, "istore_3";
    LSTORE_0 = 0x3F, "lstore_0"; LSTORE_1 = 0x40, "lstore_1"; LSTORE_2 = 0x41, "lstore_2";
    LSTORE_3 = 0x42, "lstore_3"; FSTORE_0 = 0x43, "fstore_0"; FSTORE_1 = 0x44, "fstore_1";
    FSTORE_2 = 0x45, "fstore_2"; FSTORE_3 = 0x46, "fstore_3"; DSTORE_0 = 0x47, "dstore_0";
    DSTORE_1 = 0x48, "dstore_1"; DSTORE_2 = 0x49, "dstore_2"; DSTORE_3 = 0x4A, "dstore_3";
    ASTORE_0 = 0x4B, "astore_0"; ASTORE_1 = 0x4C, "astore_1"; ASTORE_2 = 0x4D, "astore_2";
    ASTORE_3 = 0x4E, "astore_3"; IASTORE = 0x4F, "iastore"; LASTORE = 0x50, "lastore";
    FASTORE = 0x51, "fastore"; DASTORE = 0x52, "dastore"; AASTORE = 0x53, "aastore";
    BASTORE = 0x54, "bastore"; CASTORE = 0x55, "castore"; SASTORE = 0x56, "sastore";
    POP = 0x57, "pop"; POP2 = 0x58, "pop2"; DUP = 0x59, "dup";
    DUP_X1 = 0x5A, "dup_x1"; DUP_X2 = 0x5B, "dup_x2"; DUP2 = 0x5C, "dup2";
    DUP2_X1 = 0x5D, "dup2_x1"; DUP2_X2 = 0x5E, "dup2_x2"; SWAP = 0x5F, "swap";
    IADD = 0x60, "iadd"; LADD = 0x61, "ladd"; FADD = 0x62, "fadd";
    DADD = 0x63, "dadd"; ISUB = 0x64, "isub"; LSUB = 0x65, "lsub";
    FSUB = 0x66, "fsub"; DSUB = 0x67, "dsub"; IMUL = 0x68, "imul";
    LMUL = 0x69, "lmul"; FMUL = 0x6A, "fmul"; DMUL = 0x6B, "dmul";
    IDIV = 0x6C, "idiv"; LDIV = 0x6D, "ldiv"; FDIV = 0x6E, "fdiv";
    DDIV = 0x6F, "ddiv"; IREM = 0x70, "irem"; LREM = 0x71, "lrem";
    FREM = 0x72, "frem"; DREM = 0x73, "drem"; INEG = 0x74, "ineg";
    LNEG = 0x75, "lneg"; FNEG = 0x76, "fneg"; DNEG = 0x77, "dneg";
    ISHL = 0x78, "ishl"; LSHL = 0x79, "lshl"; ISHR = 0x7A, "ishr";
    LSHR = 0x7B, "lshr"; IUSHR = 0x7C, "iushr"; LUSHR = 0x7D, "lushr";
    IAND = 0x7E, "iand"; LAND = 0x7F, "land"; IOR = 0x80, "ior";
    LOR = 0x81, "lor"; IXOR = 0x82, "ixor"; LXOR = 0x83, "lxor";
    IINC = 0x84, "iinc"; I2L = 0x85, "i2l"; I2F = 0x86, "i2f";
    I2D = 0x87, "i2d"; L2I = 0x88, "l2i"; L2F = 0x89, "l2f";
    L2D = 0x8A, "l2d"; F2I = 0x8B, "f2i"; F2L = 0x8C, "f2l";
    F2D = 0x8D, "f2d"; D2I = 0x8E, "d2i"; D2L = 0x8F, "d2l";
    D2F = 0x90, "d2f"; I2B = 0x91, "i2b"; I2C = 0x92, "i2c";
    I2S = 0x93, "i2s"; LCMP = 0x94, "lcmp"; FCMPL = 0x95, "fcmpl";
    FCMPG = 0x96, "fcmpg"; DCMPL = 0x97, "dcmpl"; DCMPG = 0x98, "dcmpg";
    IFEQ = 0x99, "ifeq"; IFNE = 0x9A, "ifne"; IFLT = 0x9B, "iflt";
    IFGE = 0x9C, "ifge"; IFGT = 0x9D, "ifgt"; IFLE = 0x9E, "ifle";
    IF_ICMPEQ = 0x9F, "if_icmpeq"; IF_ICMPNE = 0xA0, "if_icmpne"; IF_ICMPLT = 0xA1, "if_icmplt";
    IF_ICMPGE = 0xA2, "if_icmpge"; IF_ICMPGT = 0xA3, "if_icmpgt"; IF_ICMPLE = 0xA4, "if_icmple";
    IF_ACMPEQ = 0xA5, "if_acmpeq"; IF_ACMPNE = 0xA6, "if_acmpne"; GOTO = 0xA7, "goto";
    JSR = 0xA8, "jsr"; RET = 0xA9, "ret"; TABLESWITCH = 0xAA, "tableswitch";
    LOOKUPSWITCH = 0xAB, "lookupswitch"; IRETURN = 0xAC, "ireturn"; LRETURN = 0xAD, "lreturn";
    FRETURN = 0xAE, "freturn"; DRETURN = 0xAF, "dreturn"; ARETURN = 0xB0, "areturn";
    RETURN = 0xB1, "return"; GETSTATIC = 0xB2, "getstatic"; PUTSTATIC = 0xB3, "putstatic";
    GETFIELD = 0xB4, "getfield"; PUTFIELD = 0xB5, "putfield"; INVOKEVIRTUAL = 0xB6, "invokevirtual";
    INVOKESPECIAL = 0xB7, "invokespecial"; INVOKESTATIC = 0xB8, "invokestatic";
    INVOKEINTERFACE = 0xB9, "invokeinterface"; INVOKEDYNAMIC = 0xBA, "invokedynamic";
    NEW = 0xBB, "new"; NEWARRAY = 0xBC, "newarray"; ANEWARRAY = 0xBD, "anewarray";
    ARRAYLENGTH = 0xBE, "arraylength"; ATHROW = 0xBF, "athrow"; CHECKCAST = 0xC0, "checkcast";
    INSTANCEOF = 0xC1, "instanceof"; MONITORENTER = 0xC2, "monitorenter"; MONITOREXIT = 0xC3, "monitorexit";
    WIDE = 0xC4, "wide"; MULTIANEWARRAY = 0xC5, "multianewarray"; IFNULL = 0xC6, "ifnull";
    IFNONNULL = 0xC7, "ifnonnull"; GOTO_W = 0xC8, "goto_w"; JSR_W = 0xC9, "jsr_w";
}

impl Opcode {
    /// The opcode for a byte, if the byte is a valid class-file opcode.
    pub fn from_u8(byte: u8) -> Option<Self> {
        (byte <= 0xC9).then_some(Self(byte))
    }

    /// The opcode byte.
    pub fn as_u8(self) -> u8 {
        self.0
    }

    /// The JVMS mnemonic, such as `"invokevirtual"`.
    pub fn mnemonic(self) -> &'static str {
        MNEMONICS.get(usize::from(self.0)).copied().unwrap_or("?")
    }

    /// `invokevirtual`, `invokespecial`, `invokestatic`, `invokeinterface` or `invokedynamic`.
    pub fn is_invoke(self) -> bool {
        (0xB6..=0xBA).contains(&self.0)
    }

    /// `getstatic`, `putstatic`, `getfield` or `putfield`.
    pub fn is_field_access(self) -> bool {
        (0xB2..=0xB5).contains(&self.0)
    }

    /// `ldc`, `ldc_w` or `ldc2_w`.
    pub fn is_ldc(self) -> bool {
        (0x12..=0x14).contains(&self.0)
    }

    /// `new`, `anewarray`, `checkcast`, `instanceof` or `multianewarray`: the instructions
    /// whose operand is a `Class` entry.
    pub fn is_type_instruction(self) -> bool {
        matches!(self.0, 0xBB | 0xBD | 0xC0 | 0xC1 | 0xC5)
    }

    fn format(self) -> Format {
        match self.0 {
            0x10 => Format::Byte,
            0x11 => Format::Short,
            0x12 => Format::ConstantU8,
            0x13 | 0x14 | 0xB2..=0xB8 | 0xBB | 0xBD | 0xC0 | 0xC1 => Format::Constant,
            0x15..=0x19 | 0x36..=0x3A | 0xA9 => Format::Local,
            0x84 => Format::Iinc,
            0x99..=0xA8 | 0xC6 | 0xC7 => Format::Branch16,
            0xC8 | 0xC9 => Format::Branch32,
            0xAA => Format::TableSwitch,
            0xAB => Format::LookupSwitch,
            0xB9 => Format::InvokeInterface,
            0xBA => Format::InvokeDynamic,
            0xBC => Format::NewArray,
            0xC4 => Format::Wide,
            0xC5 => Format::MultiANewArray,
            _ => Format::None,
        }
    }
}

impl fmt::Debug for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.mnemonic())
    }
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.mnemonic())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    None,
    Byte,
    Short,
    ConstantU8,
    Constant,
    Local,
    Iinc,
    Branch16,
    Branch32,
    TableSwitch,
    LookupSwitch,
    InvokeInterface,
    InvokeDynamic,
    NewArray,
    Wide,
    MultiANewArray,
}

/// The operands of one instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Operands {
    /// No operands.
    None,
    /// `bipush`.
    Byte(i8),
    /// `sipush`.
    Short(i16),
    /// A constant-pool index: `ldc` (widened from one byte), `ldc_w`, `ldc2_w`, the field
    /// and method instructions except `invokeinterface` and `invokedynamic`, `new`,
    /// `anewarray`, `checkcast` and `instanceof`.
    Constant(u16),
    /// A local variable slot: the load and store instructions with an operand, and `ret`.
    Local(u16),
    /// `iinc`.
    Iinc {
        /// Local variable slot.
        index: u16,
        /// The increment.
        delta: i16,
    },
    /// A branch, relative to the instruction's own offset: `if*`, `goto`, `jsr`, `goto_w`,
    /// `jsr_w`.
    Branch(i32),
    /// `tableswitch`; offsets are relative to the instruction.
    TableSwitch {
        /// Default target.
        default: i32,
        /// Lowest key.
        low: i32,
        /// Highest key.
        high: i32,
        /// Targets for `low..=high`.
        offsets: Vec<i32>,
    },
    /// `lookupswitch`; offsets are relative to the instruction.
    LookupSwitch {
        /// Default target.
        default: i32,
        /// `(match, offset)` pairs, in file order.
        pairs: Vec<(i32, i32)>,
    },
    /// `invokeinterface`.
    InvokeInterface {
        /// `InterfaceMethodref` (or `Methodref`) index.
        index: u16,
        /// The argument count byte.
        count: u8,
    },
    /// `invokedynamic`.
    InvokeDynamic {
        /// `InvokeDynamic` index.
        index: u16,
    },
    /// `newarray`: the primitive array type code (4 = boolean .. 11 = long).
    NewArray(u8),
    /// `multianewarray`.
    MultiANewArray {
        /// `Class` index of the array type.
        index: u16,
        /// Number of dimensions to create.
        dimensions: u8,
    },
}

/// One decoded instruction.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Instruction {
    /// Bytecode offset of the first byte (the `wide` prefix, if there is one).
    pub offset: u32,
    /// The opcode. For a `wide` instruction this is the modified opcode (`iload`, `iinc`
    /// and so on), and [`Instruction::wide`] is true.
    pub opcode: Opcode,
    /// Whether the instruction carries a `wide` prefix.
    pub wide: bool,
    /// The operands.
    pub operands: Operands,
    /// Length in bytes, including the prefix and any switch padding.
    pub length: u32,
}

impl Instruction {
    /// The constant-pool index the instruction refers to, if any: `ldc*`, field and method
    /// instructions, `invokeinterface`, `invokedynamic`, `new`, `anewarray`, `checkcast`,
    /// `instanceof` and `multianewarray`.
    pub fn constant_index(&self) -> Option<u16> {
        match self.operands {
            Operands::Constant(index)
            | Operands::InvokeInterface { index, .. }
            | Operands::InvokeDynamic { index }
            | Operands::MultiANewArray { index, .. } => Some(index),
            _ => None,
        }
    }

    /// Appends the instruction's encoding to `out`.
    ///
    /// Switch padding is computed from [`Instruction::offset`] and written as zero bytes, so
    /// set `offset` to where the instruction will sit. Fails when the operands do not fit
    /// the opcode (a branch too far for a 16-bit offset, `ldc` with an index above 255, a
    /// local index above 255 without `wide`, and so on).
    pub fn encode(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        let mismatch = Error::OperandMismatch {
            offset: self.offset,
            opcode: self.opcode.0,
        };
        let format = self.opcode.format();
        if self.wide {
            if !matches!(format, Format::Local | Format::Iinc) {
                return Err(mismatch);
            }
            out.push(Opcode::WIDE.0);
        }
        out.push(self.opcode.0);
        match (format, &self.operands) {
            (Format::None, Operands::None) => {}
            (Format::Byte, Operands::Byte(v)) => out.extend_from_slice(&v.to_be_bytes()),
            (Format::Short, Operands::Short(v)) => out.extend_from_slice(&v.to_be_bytes()),
            (Format::ConstantU8, Operands::Constant(index)) => {
                out.push(u8::try_from(*index).map_err(|_| mismatch)?);
            }
            (Format::Constant, Operands::Constant(index)) => {
                out.extend_from_slice(&index.to_be_bytes())
            }
            (Format::Local, Operands::Local(index)) => {
                if self.wide {
                    out.extend_from_slice(&index.to_be_bytes());
                } else {
                    out.push(u8::try_from(*index).map_err(|_| mismatch)?);
                }
            }
            (Format::Iinc, Operands::Iinc { index, delta }) => {
                if self.wide {
                    out.extend_from_slice(&index.to_be_bytes());
                    out.extend_from_slice(&delta.to_be_bytes());
                } else {
                    out.push(u8::try_from(*index).map_err(|_| mismatch.clone())?);
                    out.extend_from_slice(
                        &i8::try_from(*delta).map_err(|_| mismatch)?.to_be_bytes(),
                    );
                }
            }
            (Format::Branch16, Operands::Branch(rel)) => {
                out.extend_from_slice(&i16::try_from(*rel).map_err(|_| mismatch)?.to_be_bytes());
            }
            (Format::Branch32, Operands::Branch(rel)) => out.extend_from_slice(&rel.to_be_bytes()),
            (
                Format::TableSwitch,
                Operands::TableSwitch {
                    default,
                    low,
                    high,
                    offsets,
                },
            ) => {
                let expected = i64::from(*high) - i64::from(*low) + 1;
                if expected < 1 || i64::try_from(offsets.len()).ok() != Some(expected) {
                    return Err(mismatch);
                }
                pad(self.offset, out);
                for v in [*default, *low, *high] {
                    out.extend_from_slice(&v.to_be_bytes());
                }
                for v in offsets {
                    out.extend_from_slice(&v.to_be_bytes());
                }
            }
            (Format::LookupSwitch, Operands::LookupSwitch { default, pairs }) => {
                let npairs = i32::try_from(pairs.len()).map_err(|_| mismatch)?;
                pad(self.offset, out);
                out.extend_from_slice(&default.to_be_bytes());
                out.extend_from_slice(&npairs.to_be_bytes());
                for (key, target) in pairs {
                    out.extend_from_slice(&key.to_be_bytes());
                    out.extend_from_slice(&target.to_be_bytes());
                }
            }
            (Format::InvokeInterface, Operands::InvokeInterface { index, count }) => {
                out.extend_from_slice(&index.to_be_bytes());
                out.push(*count);
                out.push(0);
            }
            (Format::InvokeDynamic, Operands::InvokeDynamic { index }) => {
                out.extend_from_slice(&index.to_be_bytes());
                out.extend_from_slice(&[0, 0]);
            }
            (Format::NewArray, Operands::NewArray(atype)) => out.push(*atype),
            (Format::MultiANewArray, Operands::MultiANewArray { index, dimensions }) => {
                out.extend_from_slice(&index.to_be_bytes());
                out.push(*dimensions);
            }
            _ => return Err(mismatch),
        }
        Ok(())
    }
}

/// Zero bytes after a switch opcode at `offset` so the next byte sits at a multiple of 4.
fn pad(offset: u32, out: &mut Vec<u8>) {
    for _ in 0..padding(offset) {
        out.push(0);
    }
}

fn padding(offset: u32) -> u32 {
    (4 - (offset.wrapping_add(1) % 4)) % 4
}

/// An iterator over the instructions of a code array, yielding each instruction or the
/// first decoding error (after which it stops).
#[derive(Debug, Clone)]
pub struct Instructions<'a> {
    code: &'a [u8],
    pos: usize,
    failed: bool,
}

impl<'a> Instructions<'a> {
    /// Decodes `code`, the contents of a `Code` attribute's code array.
    pub fn new(code: &'a [u8]) -> Self {
        Self {
            code,
            pos: 0,
            failed: false,
        }
    }
}

impl Iterator for Instructions<'_> {
    type Item = Result<Instruction, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.pos >= self.code.len() {
            return None;
        }
        let result = decode_at(self.code, self.pos);
        match &result {
            Ok(insn) => self.pos += insn.length as usize,
            Err(_) => self.failed = true,
        }
        Some(result)
    }
}

impl std::iter::FusedIterator for Instructions<'_> {}

/// Reads big-endian operands of the instruction at `start`, failing with
/// [`Error::TruncatedInstruction`].
struct Cursor<'a> {
    code: &'a [u8],
    pos: usize,
    offset: u32,
}

impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let end = self.pos.checked_add(N);
        let slice = end.and_then(|end| self.code.get(self.pos..end));
        let slice = slice.ok_or(Error::TruncatedInstruction {
            offset: self.offset,
        })?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        self.pos += N;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.take()?))
    }

    fn i32(&mut self) -> Result<i32, Error> {
        Ok(i32::from_be_bytes(self.take()?))
    }

    fn remaining(&self) -> usize {
        self.code.len().saturating_sub(self.pos)
    }
}

/// Decodes the one instruction that starts at `pos` in `code`.
pub fn decode_at(code: &[u8], pos: usize) -> Result<Instruction, Error> {
    let offset =
        u32::try_from(pos).map_err(|_| Error::TruncatedInstruction { offset: u32::MAX })?;
    let mut c = Cursor { code, pos, offset };
    let byte = c.u8()?;
    let mut opcode = Opcode::from_u8(byte).ok_or(Error::BadOpcode {
        offset,
        opcode: byte,
    })?;
    let mut wide = false;
    let mut format = opcode.format();
    if format == Format::Wide {
        let inner = c.u8()?;
        opcode = Opcode::from_u8(inner).ok_or(Error::BadWide {
            offset,
            opcode: inner,
        })?;
        format = opcode.format();
        if !matches!(format, Format::Local | Format::Iinc) {
            return Err(Error::BadWide {
                offset,
                opcode: inner,
            });
        }
        wide = true;
    }
    let operands = match format {
        Format::None => Operands::None,
        Format::Byte => Operands::Byte(i8::from_be_bytes(c.take()?)),
        Format::Short => Operands::Short(i16::from_be_bytes(c.take()?)),
        Format::ConstantU8 => Operands::Constant(u16::from(c.u8()?)),
        Format::Constant => Operands::Constant(c.u16()?),
        Format::Local if wide => Operands::Local(c.u16()?),
        Format::Local => Operands::Local(u16::from(c.u8()?)),
        Format::Iinc if wide => Operands::Iinc {
            index: c.u16()?,
            delta: i16::from_be_bytes(c.take()?),
        },
        Format::Iinc => Operands::Iinc {
            index: u16::from(c.u8()?),
            delta: i16::from(i8::from_be_bytes(c.take()?)),
        },
        Format::Branch16 => Operands::Branch(i32::from(i16::from_be_bytes(c.take()?))),
        Format::Branch32 => Operands::Branch(c.i32()?),
        Format::TableSwitch => {
            c.pos += padding(offset) as usize;
            let default = c.i32()?;
            let low = c.i32()?;
            let high = c.i32()?;
            if low > high {
                return Err(Error::BadSwitch { offset });
            }
            let count = i64::from(high) - i64::from(low) + 1;
            let count = usize::try_from(count).map_err(|_| Error::BadSwitch { offset })?;
            if count > c.remaining() / 4 {
                return Err(Error::TruncatedInstruction { offset });
            }
            let mut offsets = Vec::with_capacity(count);
            for _ in 0..count {
                offsets.push(c.i32()?);
            }
            Operands::TableSwitch {
                default,
                low,
                high,
                offsets,
            }
        }
        Format::LookupSwitch => {
            c.pos += padding(offset) as usize;
            let default = c.i32()?;
            let npairs = usize::try_from(c.i32()?).map_err(|_| Error::BadSwitch { offset })?;
            if npairs > c.remaining() / 8 {
                return Err(Error::TruncatedInstruction { offset });
            }
            let mut pairs = Vec::with_capacity(npairs);
            for _ in 0..npairs {
                pairs.push((c.i32()?, c.i32()?));
            }
            Operands::LookupSwitch { default, pairs }
        }
        Format::InvokeInterface => {
            let index = c.u16()?;
            let count = c.u8()?;
            if c.u8()? != 0 {
                return Err(Error::NonZeroOperand { offset });
            }
            Operands::InvokeInterface { index, count }
        }
        Format::InvokeDynamic => {
            let index = c.u16()?;
            if c.u16()? != 0 {
                return Err(Error::NonZeroOperand { offset });
            }
            Operands::InvokeDynamic { index }
        }
        Format::NewArray => Operands::NewArray(c.u8()?),
        Format::MultiANewArray => Operands::MultiANewArray {
            index: c.u16()?,
            dimensions: c.u8()?,
        },
        // A `wide` after `wide` was rejected above.
        Format::Wide => {
            return Err(Error::BadWide {
                offset,
                opcode: byte,
            });
        }
    };
    let length = u32::try_from(c.pos - pos).map_err(|_| Error::TruncatedInstruction { offset })?;
    Ok(Instruction {
        offset,
        opcode,
        wide,
        operands,
        length,
    })
}
