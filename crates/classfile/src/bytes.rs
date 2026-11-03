//! Big-endian reading with bounds checks, and the matching write helpers.

use crate::Error;

/// A cursor over a byte slice. Every read checks bounds and fails with
/// [`Error::UnexpectedEof`]; nothing here can panic.
#[derive(Debug, Clone)]
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Offset of `data[0]` in the whole class file, so errors carry absolute offsets.
    base: usize,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            base: 0,
        }
    }

    /// Absolute offset of the next byte.
    pub(crate) fn offset(&self) -> usize {
        self.base.saturating_add(self.pos)
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let eof = Error::UnexpectedEof {
            offset: self.offset(),
        };
        let end = self.pos.checked_add(n).ok_or(eof.clone())?;
        let slice = self.data.get(self.pos..end).ok_or(eof)?;
        self.pos = end;
        Ok(slice)
    }

    /// Everything not read yet.
    pub(crate) fn rest(&mut self) -> &'a [u8] {
        let slice = self.data.get(self.pos..).unwrap_or(&[]);
        self.pos = self.data.len();
        slice
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let slice = self.bytes(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(slice);
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    /// Splits off the next `n` bytes as their own reader, keeping absolute offsets.
    pub(crate) fn sub(&mut self, n: usize) -> Result<Reader<'a>, Error> {
        let base = self.offset();
        let data = self.bytes(n)?;
        Ok(Reader { data, pos: 0, base })
    }
}

/// A safe initial capacity for a list whose count comes from untrusted input: never more
/// entries than the remaining bytes could hold, so a lying count cannot force a large
/// allocation.
pub(crate) fn capacity(count: usize, remaining: usize, min_entry_size: usize) -> usize {
    count.min(remaining / min_entry_size.max(1))
}

pub(crate) fn put_u8(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}

pub(crate) fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

pub(crate) fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

/// A list length as a `u16` count field, or [`Error::TooMany`].
pub(crate) fn count_u16(len: usize, what: &'static str) -> Result<u16, Error> {
    u16::try_from(len).map_err(|_| Error::TooMany {
        what,
        count: len,
        max: usize::from(u16::MAX),
    })
}

/// A list length as a `u8` count field, or [`Error::TooMany`].
pub(crate) fn count_u8(len: usize, what: &'static str) -> Result<u8, Error> {
    u8::try_from(len).map_err(|_| Error::TooMany {
        what,
        count: len,
        max: usize::from(u8::MAX),
    })
}

/// Writes a `u32` length placeholder and returns where it is, for [`patch_len`].
pub(crate) fn begin_len(out: &mut Vec<u8>) -> usize {
    let at = out.len();
    put_u32(out, 0);
    at
}

/// Fills in the placeholder written by [`begin_len`] with the bytes written since.
pub(crate) fn patch_len(out: &mut [u8], at: usize, what: &'static str) -> Result<(), Error> {
    let body = out.len().saturating_sub(at.saturating_add(4));
    let len = u32::try_from(body).map_err(|_| Error::TooMany {
        what,
        count: body,
        max: usize::try_from(u32::MAX).unwrap_or(usize::MAX),
    })?;
    let slot = out
        .get_mut(at..at.saturating_add(4))
        .ok_or(Error::InvalidValue { what })?;
    slot.copy_from_slice(&len.to_be_bytes());
    Ok(())
}
