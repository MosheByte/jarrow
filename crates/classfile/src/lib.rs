//! Java class-file parser and writer for Jarrow.
//!
//! - [`ClassFile::parse`] reads a class file into typed structures: a [`ConstantPool`] with
//!   every tag, fields, methods and [`Attribute`]s. The attributes remapping needs are
//!   typed ([`AttributeBody`]); every other attribute is kept as raw bytes.
//! - [`ClassFile::to_bytes`] writes it back. For an unmodified class the output equals the
//!   input byte for byte; the JDK round-trip test checks this on every class in
//!   `$JAVA_HOME/jmods`.
//! - [`Code::instructions`] decodes bytecode into [`Instruction`]s (offset, opcode,
//!   operands), covering every opcode including switch padding and `wide`.
//! - The [`ConstantPool`] `get_or_add_*` helpers find or append `Utf8`, `Class`,
//!   `String`, `NameAndType` and member-reference entries without renumbering anything.
//! - [`mutf8`] encodes and decodes Modified UTF-8, keeping lone surrogates as raw bytes.
//!
//! Malformed input returns [`Error`]; the parser never panics. The crate uses no I/O,
//! threads or `unsafe`, and builds for `wasm32-unknown-unknown`. It reads bytecode only;
//! it does not decompile.
//!
//! ```
//! use jarrow_classfile::{ClassFile, ConstantPool, Opcode};
//!
//! # fn main() -> Result<(), jarrow_classfile::Error> {
//! // Build a tiny class in memory, write it, and read it back.
//! let mut pool = ConstantPool::new();
//! let this_class = pool.get_or_add_class("com/example/Hello")?;
//! let super_class = pool.get_or_add_class("java/lang/Object")?;
//! let class = ClassFile {
//!     minor_version: 0,
//!     major_version: 65,
//!     constant_pool: pool,
//!     access_flags: jarrow_classfile::access::PUBLIC | jarrow_classfile::access::SUPER,
//!     this_class,
//!     super_class,
//!     interfaces: vec![],
//!     fields: vec![],
//!     methods: vec![],
//!     attributes: vec![],
//! };
//! let bytes = class.to_bytes()?;
//! let parsed = ClassFile::parse(&bytes)?;
//! assert_eq!(parsed.this_class_name()?, "com/example/Hello");
//! assert_eq!(parsed.to_bytes()?, bytes);
//!
//! // Decode bytecode: aload_0, invokespecial #7, return.
//! let code = [0x2A, 0xB7, 0x00, 0x07, 0xB1];
//! let ops: Vec<Opcode> = jarrow_classfile::Instructions::new(&code)
//!     .map(|insn| insn.map(|i| i.opcode))
//!     .collect::<Result<_, _>>()?;
//! assert_eq!(ops, [Opcode::ALOAD_0, Opcode::INVOKESPECIAL, Opcode::RETURN]);
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

pub mod access;
mod annotation;
mod attribute;
mod bytes;
mod class;
mod constant_pool;
mod error;
mod insn;
pub mod mutf8;
mod stack_map;

pub use annotation::{
    Annotation, ConstKind, ElementValue, ElementValuePair, LocalVarTarget, MAX_NESTING, TargetInfo,
    TypeAnnotation, TypePathEntry,
};
pub use attribute::{
    Attribute, AttributeBody, BootstrapMethod, Code, ExceptionHandler, InnerClass, LineNumber,
    LocalVariable, LocalVariableType, MethodParameter, RecordComponent,
};
pub use class::{ClassFile, Field, MAGIC, Method};
pub use constant_pool::{Constant, ConstantPool, MemberRef, ReferenceKind, tag};
pub use error::Error;
pub use insn::{Instruction, Instructions, Opcode, Operands, decode_at as decode_instruction};
pub use stack_map::{StackMapFrame, VerificationType};
