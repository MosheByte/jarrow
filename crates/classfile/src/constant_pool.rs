//! The constant pool (JVMS 4.4): typed entries, lookups and get-or-append helpers.

use std::borrow::Cow;

use crate::Error;
use crate::bytes::{Reader, capacity, put_u8, put_u16, put_u32, put_u64};
use crate::mutf8;

/// Constant-pool tags (JVMS table 4.4-B).
pub mod tag {
    /// `CONSTANT_Utf8`
    pub const UTF8: u8 = 1;
    /// `CONSTANT_Integer`
    pub const INTEGER: u8 = 3;
    /// `CONSTANT_Float`
    pub const FLOAT: u8 = 4;
    /// `CONSTANT_Long` (takes two slots)
    pub const LONG: u8 = 5;
    /// `CONSTANT_Double` (takes two slots)
    pub const DOUBLE: u8 = 6;
    /// `CONSTANT_Class`
    pub const CLASS: u8 = 7;
    /// `CONSTANT_String`
    pub const STRING: u8 = 8;
    /// `CONSTANT_Fieldref`
    pub const FIELDREF: u8 = 9;
    /// `CONSTANT_Methodref`
    pub const METHODREF: u8 = 10;
    /// `CONSTANT_InterfaceMethodref`
    pub const INTERFACE_METHODREF: u8 = 11;
    /// `CONSTANT_NameAndType`
    pub const NAME_AND_TYPE: u8 = 12;
    /// `CONSTANT_MethodHandle`
    pub const METHOD_HANDLE: u8 = 15;
    /// `CONSTANT_MethodType`
    pub const METHOD_TYPE: u8 = 16;
    /// `CONSTANT_Dynamic`
    pub const DYNAMIC: u8 = 17;
    /// `CONSTANT_InvokeDynamic`
    pub const INVOKE_DYNAMIC: u8 = 18;
    /// `CONSTANT_Module`
    pub const MODULE: u8 = 19;
    /// `CONSTANT_Package`
    pub const PACKAGE: u8 = 20;
}

/// The reference kind of a `CONSTANT_MethodHandle` (JVMS table 5.4.3.5-A).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ReferenceKind {
    /// `REF_getField`
    GetField = 1,
    /// `REF_getStatic`
    GetStatic = 2,
    /// `REF_putField`
    PutField = 3,
    /// `REF_putStatic`
    PutStatic = 4,
    /// `REF_invokeVirtual`
    InvokeVirtual = 5,
    /// `REF_invokeStatic`
    InvokeStatic = 6,
    /// `REF_invokeSpecial`
    InvokeSpecial = 7,
    /// `REF_newInvokeSpecial`
    NewInvokeSpecial = 8,
    /// `REF_invokeInterface`
    InvokeInterface = 9,
}

impl ReferenceKind {
    /// The kind for a `reference_kind` byte, if it is one of 1..=9.
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::GetField,
            2 => Self::GetStatic,
            3 => Self::PutField,
            4 => Self::PutStatic,
            5 => Self::InvokeVirtual,
            6 => Self::InvokeStatic,
            7 => Self::InvokeSpecial,
            8 => Self::NewInvokeSpecial,
            9 => Self::InvokeInterface,
            _ => return None,
        })
    }

    /// The `reference_kind` byte.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// One constant-pool entry.
///
/// Numbers are stored as their raw bits, so a `Float` or `Double` NaN payload survives a
/// round trip. `Utf8` holds the raw Modified UTF-8 bytes; decode them with
/// [`ConstantPool::utf8_str`] or the [`mutf8`] module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Constant {
    /// `CONSTANT_Utf8`: raw Modified UTF-8 bytes.
    Utf8(Vec<u8>),
    /// `CONSTANT_Integer`.
    Integer(i32),
    /// `CONSTANT_Float`: the IEEE 754 bits (use `f32::from_bits`).
    Float(u32),
    /// `CONSTANT_Long`; the next slot is unusable.
    Long(i64),
    /// `CONSTANT_Double`: the IEEE 754 bits (use `f64::from_bits`); the next slot is unusable.
    Double(u64),
    /// `CONSTANT_Class`.
    Class {
        /// `Utf8`: the internal name (`java/lang/Object`) or an array descriptor.
        name_index: u16,
    },
    /// `CONSTANT_String`.
    String {
        /// `Utf8`: the string value.
        string_index: u16,
    },
    /// `CONSTANT_Fieldref`.
    Fieldref {
        /// `Class`: the owner.
        class_index: u16,
        /// `NameAndType`: name and field descriptor.
        name_and_type_index: u16,
    },
    /// `CONSTANT_Methodref`.
    Methodref {
        /// `Class`: the owner.
        class_index: u16,
        /// `NameAndType`: name and method descriptor.
        name_and_type_index: u16,
    },
    /// `CONSTANT_InterfaceMethodref`.
    InterfaceMethodref {
        /// `Class`: the owning interface.
        class_index: u16,
        /// `NameAndType`: name and method descriptor.
        name_and_type_index: u16,
    },
    /// `CONSTANT_NameAndType`.
    NameAndType {
        /// `Utf8`: the member name.
        name_index: u16,
        /// `Utf8`: the field or method descriptor.
        descriptor_index: u16,
    },
    /// `CONSTANT_MethodHandle`.
    MethodHandle {
        /// What the handle does.
        reference_kind: ReferenceKind,
        /// `Fieldref`, `Methodref` or `InterfaceMethodref`, depending on the kind.
        reference_index: u16,
    },
    /// `CONSTANT_MethodType`.
    MethodType {
        /// `Utf8`: a method descriptor.
        descriptor_index: u16,
    },
    /// `CONSTANT_Dynamic` (a dynamically computed constant).
    Dynamic {
        /// Index into the `BootstrapMethods` attribute.
        bootstrap_method_attr_index: u16,
        /// `NameAndType`: name and field descriptor.
        name_and_type_index: u16,
    },
    /// `CONSTANT_InvokeDynamic`.
    InvokeDynamic {
        /// Index into the `BootstrapMethods` attribute.
        bootstrap_method_attr_index: u16,
        /// `NameAndType`: name and method descriptor.
        name_and_type_index: u16,
    },
    /// `CONSTANT_Module`.
    Module {
        /// `Utf8`: the module name.
        name_index: u16,
    },
    /// `CONSTANT_Package`.
    Package {
        /// `Utf8`: the package name in internal form.
        name_index: u16,
    },
}

impl Constant {
    /// The tag byte of this entry.
    pub fn tag(&self) -> u8 {
        match self {
            Self::Utf8(_) => tag::UTF8,
            Self::Integer(_) => tag::INTEGER,
            Self::Float(_) => tag::FLOAT,
            Self::Long(_) => tag::LONG,
            Self::Double(_) => tag::DOUBLE,
            Self::Class { .. } => tag::CLASS,
            Self::String { .. } => tag::STRING,
            Self::Fieldref { .. } => tag::FIELDREF,
            Self::Methodref { .. } => tag::METHODREF,
            Self::InterfaceMethodref { .. } => tag::INTERFACE_METHODREF,
            Self::NameAndType { .. } => tag::NAME_AND_TYPE,
            Self::MethodHandle { .. } => tag::METHOD_HANDLE,
            Self::MethodType { .. } => tag::METHOD_TYPE,
            Self::Dynamic { .. } => tag::DYNAMIC,
            Self::InvokeDynamic { .. } => tag::INVOKE_DYNAMIC,
            Self::Module { .. } => tag::MODULE,
            Self::Package { .. } => tag::PACKAGE,
        }
    }

    /// The JVMS name of this entry's kind, such as `"Methodref"`.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Utf8(_) => "Utf8",
            Self::Integer(_) => "Integer",
            Self::Float(_) => "Float",
            Self::Long(_) => "Long",
            Self::Double(_) => "Double",
            Self::Class { .. } => "Class",
            Self::String { .. } => "String",
            Self::Fieldref { .. } => "Fieldref",
            Self::Methodref { .. } => "Methodref",
            Self::InterfaceMethodref { .. } => "InterfaceMethodref",
            Self::NameAndType { .. } => "NameAndType",
            Self::MethodHandle { .. } => "MethodHandle",
            Self::MethodType { .. } => "MethodType",
            Self::Dynamic { .. } => "Dynamic",
            Self::InvokeDynamic { .. } => "InvokeDynamic",
            Self::Module { .. } => "Module",
            Self::Package { .. } => "Package",
        }
    }

    /// Whether the entry takes two slots (`Long` and `Double`).
    pub fn is_wide(&self) -> bool {
        matches!(self, Self::Long(_) | Self::Double(_))
    }

    /// How many slots the entry takes: 2 for `Long` and `Double`, else 1.
    pub fn width(&self) -> u16 {
        if self.is_wide() { 2 } else { 1 }
    }
}

/// A field, method or interface method reference, resolved to its names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberRef<'a> {
    /// The entry's tag: [`tag::FIELDREF`], [`tag::METHODREF`] or [`tag::INTERFACE_METHODREF`].
    pub tag: u8,
    /// Internal name of the owner class.
    pub class: Cow<'a, str>,
    /// Member name.
    pub name: Cow<'a, str>,
    /// Field or method descriptor.
    pub descriptor: Cow<'a, str>,
}

/// The constant pool of a class.
///
/// Indices are the class file's own: 1-based, with `Long` and `Double` taking two slots.
/// [`ConstantPool::get`] returns `None` for index 0, for the unusable slot after a wide
/// entry, and past the end.
///
/// The pool only grows at the end. The `get_or_add_*` helpers return the first existing
/// entry that matches (lowest index), or append a new one, so they never renumber an
/// existing entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConstantPool {
    /// `slots[0]` and the slot after each wide entry are `None`.
    slots: Vec<Option<Constant>>,
}

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}

/// The largest `constant_pool_count`.
const MAX_COUNT: usize = u16::MAX as usize;

impl ConstantPool {
    /// An empty pool (`constant_pool_count` = 1).
    pub fn new() -> Self {
        Self { slots: vec![None] }
    }

    /// `constant_pool_count` as written in the class file: one more than the highest index.
    pub fn count(&self) -> u16 {
        // The slot vector never grows past MAX_COUNT (see `push`), and parse reads a u16.
        u16::try_from(self.slots.len()).unwrap_or(u16::MAX)
    }

    /// The entry at `index`, if that slot holds one.
    pub fn get(&self, index: u16) -> Option<&Constant> {
        self.slots.get(usize::from(index)).and_then(Option::as_ref)
    }

    /// Every entry with its index, in index order.
    pub fn iter(&self) -> impl Iterator<Item = (u16, &Constant)> + '_ {
        self.slots.iter().enumerate().filter_map(|(i, slot)| {
            let index = u16::try_from(i).ok()?;
            slot.as_ref().map(|c| (index, c))
        })
    }

    /// Appends `constant` and returns its index. Never looks for an existing equal entry;
    /// see [`ConstantPool::get_or_add`] for that.
    ///
    /// Fails with [`Error::ConstantPoolFull`] when the entry does not fit in 65535 slots,
    /// and with [`Error::TooMany`] for a `Utf8` entry longer than 65535 bytes.
    pub fn push(&mut self, constant: Constant) -> Result<u16, Error> {
        check_length(&constant)?;
        let index = self.slots.len();
        let width = usize::from(constant.width());
        if index + width > MAX_COUNT {
            return Err(Error::ConstantPoolFull);
        }
        let wide = constant.is_wide();
        self.slots.push(Some(constant));
        if wide {
            self.slots.push(None);
        }
        u16::try_from(index).map_err(|_| Error::ConstantPoolFull)
    }

    /// Replaces the entry at `index` and returns the old one. The new entry must have the
    /// same width, so no other index moves.
    pub fn replace(&mut self, index: u16, constant: Constant) -> Result<Constant, Error> {
        check_length(&constant)?;
        let slot = self
            .slots
            .get_mut(usize::from(index))
            .and_then(Option::as_mut)
            .ok_or(Error::BadConstantIndex {
                index,
                expected: "entry",
                context: "replace",
            })?;
        if slot.is_wide() != constant.is_wide() {
            return Err(Error::WidthMismatch { index });
        }
        Ok(std::mem::replace(slot, constant))
    }

    /// The index of the first entry equal to `constant`.
    pub fn find(&self, constant: &Constant) -> Option<u16> {
        self.iter().find(|(_, c)| *c == constant).map(|(i, _)| i)
    }

    /// The index of the first entry equal to `constant`, appending it when there is none.
    pub fn get_or_add(&mut self, constant: Constant) -> Result<u16, Error> {
        match self.find(&constant) {
            Some(index) => Ok(index),
            None => self.push(constant),
        }
    }

    /// The index of the first `Utf8` entry holding exactly these Modified UTF-8 bytes,
    /// appended when there is none.
    pub fn get_or_add_utf8_bytes(&mut self, mutf8: &[u8]) -> Result<u16, Error> {
        let found = self
            .iter()
            .find(|(_, c)| matches!(c, Constant::Utf8(b) if b == mutf8));
        match found {
            Some((index, _)) => Ok(index),
            None => self.push(Constant::Utf8(mutf8.to_vec())),
        }
    }

    /// The index of the first `Utf8` entry for `text`, appended when there is none.
    pub fn get_or_add_utf8(&mut self, text: &str) -> Result<u16, Error> {
        self.get_or_add_utf8_bytes(&mutf8::encode(text))
    }

    /// The index of a `Class` entry for an internal name (`net/minecraft/world/item/Item`)
    /// or an array descriptor (`[Ljava/lang/String;`).
    pub fn get_or_add_class(&mut self, internal_name: &str) -> Result<u16, Error> {
        let name_index = self.get_or_add_utf8(internal_name)?;
        self.get_or_add(Constant::Class { name_index })
    }

    /// The index of a `String` entry for `value`.
    pub fn get_or_add_string(&mut self, value: &str) -> Result<u16, Error> {
        let string_index = self.get_or_add_utf8(value)?;
        self.get_or_add(Constant::String { string_index })
    }

    /// The index of a `NameAndType` entry.
    pub fn get_or_add_name_and_type(&mut self, name: &str, descriptor: &str) -> Result<u16, Error> {
        let name_index = self.get_or_add_utf8(name)?;
        let descriptor_index = self.get_or_add_utf8(descriptor)?;
        self.get_or_add(Constant::NameAndType {
            name_index,
            descriptor_index,
        })
    }

    /// The index of a `Fieldref` entry.
    pub fn get_or_add_fieldref(
        &mut self,
        class: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<u16, Error> {
        let (class_index, name_and_type_index) = self.ref_parts(class, name, descriptor)?;
        self.get_or_add(Constant::Fieldref {
            class_index,
            name_and_type_index,
        })
    }

    /// The index of a `Methodref` entry.
    pub fn get_or_add_methodref(
        &mut self,
        class: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<u16, Error> {
        let (class_index, name_and_type_index) = self.ref_parts(class, name, descriptor)?;
        self.get_or_add(Constant::Methodref {
            class_index,
            name_and_type_index,
        })
    }

    /// The index of an `InterfaceMethodref` entry.
    pub fn get_or_add_interface_methodref(
        &mut self,
        class: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<u16, Error> {
        let (class_index, name_and_type_index) = self.ref_parts(class, name, descriptor)?;
        self.get_or_add(Constant::InterfaceMethodref {
            class_index,
            name_and_type_index,
        })
    }

    fn ref_parts(
        &mut self,
        class: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<(u16, u16), Error> {
        let class_index = self.get_or_add_class(class)?;
        let name_and_type_index = self.get_or_add_name_and_type(name, descriptor)?;
        Ok((class_index, name_and_type_index))
    }

    /// The raw bytes of the `Utf8` entry at `index`.
    pub fn utf8(&self, index: u16) -> Result<&[u8], Error> {
        match self.get(index) {
            Some(Constant::Utf8(bytes)) => Ok(bytes),
            _ => Err(Error::BadConstantIndex {
                index,
                expected: "Utf8",
                context: "lookup",
            }),
        }
    }

    /// The `Utf8` entry at `index`, strictly decoded (see [`mutf8::decode`]).
    pub fn utf8_str(&self, index: u16) -> Result<Cow<'_, str>, Error> {
        mutf8::decode(self.utf8(index)?).map_err(|error| Error::Utf8 { index, error })
    }

    /// The name of the `Class` entry at `index`.
    pub fn class_name(&self, index: u16) -> Result<Cow<'_, str>, Error> {
        match self.get(index) {
            Some(Constant::Class { name_index }) => self.utf8_str(*name_index),
            _ => Err(Error::BadConstantIndex {
                index,
                expected: "Class",
                context: "lookup",
            }),
        }
    }

    /// The name and descriptor of the `NameAndType` entry at `index`.
    pub fn name_and_type(&self, index: u16) -> Result<(Cow<'_, str>, Cow<'_, str>), Error> {
        match self.get(index) {
            Some(Constant::NameAndType {
                name_index,
                descriptor_index,
            }) => Ok((
                self.utf8_str(*name_index)?,
                self.utf8_str(*descriptor_index)?,
            )),
            _ => Err(Error::BadConstantIndex {
                index,
                expected: "NameAndType",
                context: "lookup",
            }),
        }
    }

    /// The `Fieldref`, `Methodref` or `InterfaceMethodref` at `index`, resolved to names.
    pub fn member_ref(&self, index: u16) -> Result<MemberRef<'_>, Error> {
        let (tag, class_index, nat) = match self.get(index) {
            Some(
                c @ Constant::Fieldref {
                    class_index,
                    name_and_type_index,
                },
            )
            | Some(
                c @ Constant::Methodref {
                    class_index,
                    name_and_type_index,
                },
            )
            | Some(
                c @ Constant::InterfaceMethodref {
                    class_index,
                    name_and_type_index,
                },
            ) => (c.tag(), *class_index, *name_and_type_index),
            _ => {
                return Err(Error::BadConstantIndex {
                    index,
                    expected: "member reference",
                    context: "lookup",
                });
            }
        };
        let class = self.class_name(class_index)?;
        let (name, descriptor) = self.name_and_type(nat)?;
        Ok(MemberRef {
            tag,
            class,
            name,
            descriptor,
        })
    }

    /// Fails unless `index` holds an entry for which `accept` is true.
    pub(crate) fn expect(
        &self,
        index: u16,
        expected: &'static str,
        context: &'static str,
        accept: impl Fn(&Constant) -> bool,
    ) -> Result<(), Error> {
        match self.get(index) {
            Some(c) if accept(c) => Ok(()),
            _ => Err(Error::BadConstantIndex {
                index,
                expected,
                context,
            }),
        }
    }

    pub(crate) fn expect_utf8(&self, index: u16, context: &'static str) -> Result<(), Error> {
        self.expect(index, "Utf8", context, |c| matches!(c, Constant::Utf8(_)))
    }

    pub(crate) fn expect_class(&self, index: u16, context: &'static str) -> Result<(), Error> {
        self.expect(index, "Class", context, |c| {
            matches!(c, Constant::Class { .. })
        })
    }

    pub(crate) fn parse(r: &mut Reader<'_>) -> Result<Self, Error> {
        let count = usize::from(r.u16()?);
        if count == 0 {
            return Err(Error::EmptyConstantPoolCount);
        }
        let mut slots = Vec::with_capacity(capacity(count, r.remaining(), 3));
        slots.push(None);
        while slots.len() < count {
            let index = u16::try_from(slots.len()).map_err(|_| Error::ConstantPoolFull)?;
            let offset = r.offset();
            let constant = match r.u8()? {
                tag::UTF8 => {
                    let len = usize::from(r.u16()?);
                    Constant::Utf8(r.bytes(len)?.to_vec())
                }
                tag::INTEGER => Constant::Integer(r.u32()? as i32),
                tag::FLOAT => Constant::Float(r.u32()?),
                tag::LONG => Constant::Long(r.u64()? as i64),
                tag::DOUBLE => Constant::Double(r.u64()?),
                tag::CLASS => Constant::Class {
                    name_index: r.u16()?,
                },
                tag::STRING => Constant::String {
                    string_index: r.u16()?,
                },
                tag::FIELDREF => Constant::Fieldref {
                    class_index: r.u16()?,
                    name_and_type_index: r.u16()?,
                },
                tag::METHODREF => Constant::Methodref {
                    class_index: r.u16()?,
                    name_and_type_index: r.u16()?,
                },
                tag::INTERFACE_METHODREF => Constant::InterfaceMethodref {
                    class_index: r.u16()?,
                    name_and_type_index: r.u16()?,
                },
                tag::NAME_AND_TYPE => Constant::NameAndType {
                    name_index: r.u16()?,
                    descriptor_index: r.u16()?,
                },
                tag::METHOD_HANDLE => {
                    let kind = r.u8()?;
                    let reference_kind = ReferenceKind::from_u8(kind)
                        .ok_or(Error::BadReferenceKind { index, kind })?;
                    Constant::MethodHandle {
                        reference_kind,
                        reference_index: r.u16()?,
                    }
                }
                tag::METHOD_TYPE => Constant::MethodType {
                    descriptor_index: r.u16()?,
                },
                tag::DYNAMIC => Constant::Dynamic {
                    bootstrap_method_attr_index: r.u16()?,
                    name_and_type_index: r.u16()?,
                },
                tag::INVOKE_DYNAMIC => Constant::InvokeDynamic {
                    bootstrap_method_attr_index: r.u16()?,
                    name_and_type_index: r.u16()?,
                },
                tag::MODULE => Constant::Module {
                    name_index: r.u16()?,
                },
                tag::PACKAGE => Constant::Package {
                    name_index: r.u16()?,
                },
                other => {
                    return Err(Error::BadConstantTag {
                        index,
                        tag: other,
                        offset,
                    });
                }
            };
            let wide = constant.is_wide();
            if wide && slots.len() + 1 >= count {
                return Err(Error::WideConstantAtEnd { index });
            }
            slots.push(Some(constant));
            if wide {
                slots.push(None);
            }
        }
        let pool = Self { slots };
        pool.validate()?;
        Ok(pool)
    }

    /// Checks that every reference inside the pool points at an entry of the right kind.
    fn validate(&self) -> Result<(), Error> {
        let is_class = |c: &Constant| matches!(c, Constant::Class { .. });
        let is_nat = |c: &Constant| matches!(c, Constant::NameAndType { .. });
        for (_, constant) in self.iter() {
            match *constant {
                Constant::Utf8(_)
                | Constant::Integer(_)
                | Constant::Float(_)
                | Constant::Long(_)
                | Constant::Double(_) => {}
                Constant::Class { name_index } => self.expect_utf8(name_index, "Class name")?,
                Constant::String { string_index } => {
                    self.expect_utf8(string_index, "String value")?
                }
                Constant::Module { name_index } => self.expect_utf8(name_index, "Module name")?,
                Constant::Package { name_index } => self.expect_utf8(name_index, "Package name")?,
                Constant::MethodType { descriptor_index } => {
                    self.expect_utf8(descriptor_index, "MethodType descriptor")?;
                }
                Constant::NameAndType {
                    name_index,
                    descriptor_index,
                } => {
                    self.expect_utf8(name_index, "NameAndType name")?;
                    self.expect_utf8(descriptor_index, "NameAndType descriptor")?;
                }
                Constant::Fieldref {
                    class_index,
                    name_and_type_index,
                }
                | Constant::Methodref {
                    class_index,
                    name_and_type_index,
                }
                | Constant::InterfaceMethodref {
                    class_index,
                    name_and_type_index,
                } => {
                    self.expect(class_index, "Class", "member reference owner", is_class)?;
                    self.expect(
                        name_and_type_index,
                        "NameAndType",
                        "member reference",
                        is_nat,
                    )?;
                }
                Constant::Dynamic {
                    name_and_type_index,
                    ..
                }
                | Constant::InvokeDynamic {
                    name_and_type_index,
                    ..
                } => {
                    self.expect(
                        name_and_type_index,
                        "NameAndType",
                        "dynamic constant",
                        is_nat,
                    )?;
                }
                Constant::MethodHandle {
                    reference_kind,
                    reference_index,
                } => {
                    let (expected, accept): (&'static str, fn(&Constant) -> bool) =
                        match reference_kind {
                            ReferenceKind::GetField
                            | ReferenceKind::GetStatic
                            | ReferenceKind::PutField
                            | ReferenceKind::PutStatic => {
                                ("Fieldref", |c| matches!(c, Constant::Fieldref { .. }))
                            }
                            ReferenceKind::InvokeVirtual | ReferenceKind::NewInvokeSpecial => {
                                ("Methodref", |c| matches!(c, Constant::Methodref { .. }))
                            }
                            ReferenceKind::InvokeStatic | ReferenceKind::InvokeSpecial => {
                                ("Methodref or InterfaceMethodref", |c| {
                                    matches!(
                                        c,
                                        Constant::Methodref { .. }
                                            | Constant::InterfaceMethodref { .. }
                                    )
                                })
                            }
                            ReferenceKind::InvokeInterface => ("InterfaceMethodref", |c| {
                                matches!(c, Constant::InterfaceMethodref { .. })
                            }),
                        };
                    self.expect(reference_index, expected, "MethodHandle reference", accept)?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn write(&self, out: &mut Vec<u8>) {
        put_u16(out, self.count());
        for (_, constant) in self.iter() {
            put_u8(out, constant.tag());
            match constant {
                Constant::Utf8(bytes) => {
                    // `check_length` keeps every entry within 65535 bytes.
                    put_u16(out, u16::try_from(bytes.len()).unwrap_or(u16::MAX));
                    out.extend_from_slice(bytes);
                }
                Constant::Integer(v) => put_u32(out, *v as u32),
                Constant::Float(bits) => put_u32(out, *bits),
                Constant::Long(v) => put_u64(out, *v as u64),
                Constant::Double(bits) => put_u64(out, *bits),
                Constant::Class { name_index: a }
                | Constant::String { string_index: a }
                | Constant::MethodType {
                    descriptor_index: a,
                }
                | Constant::Module { name_index: a }
                | Constant::Package { name_index: a } => put_u16(out, *a),
                Constant::Fieldref {
                    class_index: a,
                    name_and_type_index: b,
                }
                | Constant::Methodref {
                    class_index: a,
                    name_and_type_index: b,
                }
                | Constant::InterfaceMethodref {
                    class_index: a,
                    name_and_type_index: b,
                }
                | Constant::NameAndType {
                    name_index: a,
                    descriptor_index: b,
                }
                | Constant::Dynamic {
                    bootstrap_method_attr_index: a,
                    name_and_type_index: b,
                }
                | Constant::InvokeDynamic {
                    bootstrap_method_attr_index: a,
                    name_and_type_index: b,
                } => {
                    put_u16(out, *a);
                    put_u16(out, *b);
                }
                Constant::MethodHandle {
                    reference_kind,
                    reference_index,
                } => {
                    put_u8(out, reference_kind.as_u8());
                    put_u16(out, *reference_index);
                }
            }
        }
    }
}

/// A `Utf8` entry's length is a `u16`, so longer byte strings cannot enter the pool. With
/// `push`, `replace` and `parse` the only ways in, every entry in a pool is writable.
fn check_length(constant: &Constant) -> Result<(), Error> {
    if let Constant::Utf8(bytes) = constant {
        crate::bytes::count_u16(bytes.len(), "bytes in a Utf8 constant")?;
    }
    Ok(())
}
