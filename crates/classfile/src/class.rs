//! The class file itself (JVMS 4.1), its fields and methods.

use std::borrow::Cow;

use crate::Error;
use crate::attribute::{
    Attribute, AttributeBody, Code, Context, parse_attributes, write_attributes,
};
use crate::bytes::{Reader, capacity, count_u16, put_u16, put_u32};
use crate::constant_pool::ConstantPool;

/// The magic number every class file starts with.
pub const MAGIC: u32 = 0xCAFE_BABE;

/// A parsed class file.
///
/// [`ClassFile::parse`] followed by [`ClassFile::to_bytes`] reproduces the input byte for
/// byte. Fields are public: change what you need, and keep constant-pool indices valid
/// (append new entries with the [`ConstantPool`] `get_or_add_*` helpers).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassFile {
    /// `minor_version`.
    pub minor_version: u16,
    /// `major_version` (65 is Java 21).
    pub major_version: u16,
    /// The constant pool.
    pub constant_pool: ConstantPool,
    /// Class access flags; see [`crate::access`].
    pub access_flags: u16,
    /// `Class`: this class.
    pub this_class: u16,
    /// `Class`: the superclass, or 0 for `java/lang/Object` and `module-info`.
    pub super_class: u16,
    /// `Class` entries of the direct superinterfaces.
    pub interfaces: Vec<u16>,
    /// Fields, in file order.
    pub fields: Vec<Field>,
    /// Methods, in file order.
    pub methods: Vec<Method>,
    /// Class attributes, in file order.
    pub attributes: Vec<Attribute>,
}

/// A field (`field_info`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Field {
    /// Field access flags; see [`crate::access`].
    pub access_flags: u16,
    /// `Utf8`: the field name.
    pub name_index: u16,
    /// `Utf8`: the field descriptor.
    pub descriptor_index: u16,
    /// Field attributes, in file order.
    pub attributes: Vec<Attribute>,
}

/// A method (`method_info`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Method {
    /// Method access flags; see [`crate::access`].
    pub access_flags: u16,
    /// `Utf8`: the method name.
    pub name_index: u16,
    /// `Utf8`: the method descriptor.
    pub descriptor_index: u16,
    /// Method attributes, in file order.
    pub attributes: Vec<Attribute>,
}

impl Method {
    /// The method's first `Code` attribute, if it has one (abstract and native methods do not).
    pub fn code(&self) -> Option<&Code> {
        self.attributes.iter().find_map(|a| match &a.body {
            AttributeBody::Code(code) => Some(code),
            _ => None,
        })
    }

    /// Mutable access to the method's first `Code` attribute.
    pub fn code_mut(&mut self) -> Option<&mut Code> {
        self.attributes.iter_mut().find_map(|a| match &mut a.body {
            AttributeBody::Code(code) => Some(code),
            _ => None,
        })
    }
}

fn parse_member(
    r: &mut Reader<'_>,
    pool: &ConstantPool,
    ctx: Context,
) -> Result<(u16, u16, u16, Vec<Attribute>), Error> {
    let access_flags = r.u16()?;
    let name_index = r.u16()?;
    let descriptor_index = r.u16()?;
    pool.expect_utf8(name_index, "member name")?;
    pool.expect_utf8(descriptor_index, "member descriptor")?;
    let attributes = parse_attributes(r, pool, ctx)?;
    Ok((access_flags, name_index, descriptor_index, attributes))
}

impl ClassFile {
    /// Parses a class file.
    ///
    /// Fails, never panics, on malformed input: a bad magic number, a truncated
    /// structure, an unknown constant tag, a constant-pool reference to the wrong kind of
    /// entry, a known attribute whose contents do not match its declared length, or bytes
    /// left over after the last attribute. Unknown attributes are kept raw.
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let mut r = Reader::new(bytes);
        let magic = r.u32()?;
        if magic != MAGIC {
            return Err(Error::BadMagic { found: magic });
        }
        let minor_version = r.u16()?;
        let major_version = r.u16()?;
        let constant_pool = ConstantPool::parse(&mut r)?;
        let access_flags = r.u16()?;
        let this_class = r.u16()?;
        let super_class = r.u16()?;
        constant_pool.expect_class(this_class, "this_class")?;
        if super_class != 0 {
            constant_pool.expect_class(super_class, "super_class")?;
        }
        let n = usize::from(r.u16()?);
        let mut interfaces = Vec::with_capacity(capacity(n, r.remaining(), 2));
        for _ in 0..n {
            let index = r.u16()?;
            constant_pool.expect_class(index, "interfaces")?;
            interfaces.push(index);
        }
        let n = usize::from(r.u16()?);
        let mut fields = Vec::with_capacity(capacity(n, r.remaining(), 8));
        for _ in 0..n {
            let (access_flags, name_index, descriptor_index, attributes) =
                parse_member(&mut r, &constant_pool, Context::Field)?;
            fields.push(Field {
                access_flags,
                name_index,
                descriptor_index,
                attributes,
            });
        }
        let n = usize::from(r.u16()?);
        let mut methods = Vec::with_capacity(capacity(n, r.remaining(), 8));
        for _ in 0..n {
            let (access_flags, name_index, descriptor_index, attributes) =
                parse_member(&mut r, &constant_pool, Context::Method)?;
            methods.push(Method {
                access_flags,
                name_index,
                descriptor_index,
                attributes,
            });
        }
        let attributes = parse_attributes(&mut r, &constant_pool, Context::Class)?;
        if r.remaining() != 0 {
            return Err(Error::TrailingBytes {
                offset: r.offset(),
                count: r.remaining(),
            });
        }
        Ok(Self {
            minor_version,
            major_version,
            constant_pool,
            access_flags,
            this_class,
            super_class,
            interfaces,
            fields,
            methods,
            attributes,
        })
    }

    /// Writes the class file. For a class from [`ClassFile::parse`] that was not changed,
    /// the result equals the parsed bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        self.write_to(&mut out)?;
        Ok(out)
    }

    /// Appends the class file to `out`. On error, `out` may hold a partial class.
    pub fn write_to(&self, out: &mut Vec<u8>) -> Result<(), Error> {
        put_u32(out, MAGIC);
        put_u16(out, self.minor_version);
        put_u16(out, self.major_version);
        self.constant_pool.write(out);
        put_u16(out, self.access_flags);
        put_u16(out, self.this_class);
        put_u16(out, self.super_class);
        put_u16(out, count_u16(self.interfaces.len(), "interfaces")?);
        for index in &self.interfaces {
            put_u16(out, *index);
        }
        put_u16(out, count_u16(self.fields.len(), "fields")?);
        for f in &self.fields {
            put_u16(out, f.access_flags);
            put_u16(out, f.name_index);
            put_u16(out, f.descriptor_index);
            write_attributes(&f.attributes, out)?;
        }
        put_u16(out, count_u16(self.methods.len(), "methods")?);
        for m in &self.methods {
            put_u16(out, m.access_flags);
            put_u16(out, m.name_index);
            put_u16(out, m.descriptor_index);
            write_attributes(&m.attributes, out)?;
        }
        write_attributes(&self.attributes, out)
    }

    /// The internal name of this class (`com/example/Foo`).
    pub fn this_class_name(&self) -> Result<Cow<'_, str>, Error> {
        self.constant_pool.class_name(self.this_class)
    }

    /// The internal name of the superclass, or `None` when `super_class` is 0.
    pub fn super_class_name(&self) -> Result<Option<Cow<'_, str>>, Error> {
        if self.super_class == 0 {
            return Ok(None);
        }
        self.constant_pool.class_name(self.super_class).map(Some)
    }
}
