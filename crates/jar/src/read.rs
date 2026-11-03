//! The zip reader: central directory first, every structure validated before anything is
//! decompressed.

use std::collections::BTreeSet;

use miniz_oxide::inflate::{TINFLStatus, decompress_to_vec_with_limit};

use crate::Jar;
use crate::crc::crc32;
use crate::entry::{Entry, Method, Original};
use crate::error::{Error, Limit, Result};
use crate::name::check_name;

pub(crate) const EOCD_SIG: u32 = 0x0605_4b50;
pub(crate) const EOCD_LEN: usize = 22;
pub(crate) const Z64_LOCATOR_SIG: u32 = 0x0706_4b50;
pub(crate) const Z64_LOCATOR_LEN: usize = 20;
pub(crate) const Z64_EOCD_SIG: u32 = 0x0606_4b50;
pub(crate) const Z64_EOCD_LEN: usize = 56;
pub(crate) const CENTRAL_SIG: u32 = 0x0201_4b50;
pub(crate) const CENTRAL_LEN: usize = 46;
pub(crate) const LOCAL_SIG: u32 = 0x0403_4b50;
pub(crate) const ZIP64_EXTRA_ID: u16 = 0x0001;
pub(crate) const U16_MARK: u16 = 0xFFFF;
pub(crate) const U32_MARK: u32 = 0xFFFF_FFFF;

const MAX_COMMENT: usize = 0xFFFF;
const FLAG_ENCRYPTED: u16 = 1;
const FLAG_STRONG_ENCRYPTION: u16 = 1 << 6;
const FLAG_MASKED_HEADERS: u16 = 1 << 13;
const HOST_UNIX: u16 = 3;
const HOST_DARWIN: u16 = 19;

/// The most nesting levels [`crate::Jar::read_nested`] ever follows, whatever
/// [`ReadOptions::max_nesting_depth`] says. It keeps recursion off the end of the stack.
pub const HARD_MAX_NESTING_DEPTH: u32 = 64;

/// Limits for reading untrusted archives (zip-bomb guards).
///
/// Every limit is checked against the sizes the archive declares, before anything is
/// decompressed, and the decompressor is capped at the declared size, so an archive that
/// lies about its sizes fails instead of filling memory. For nested jars
/// ([`crate::Jar::read_nested`]) the entry and size limits apply to the whole tree together.
///
/// The struct is `#[non_exhaustive]`: start from [`ReadOptions::default`] and set fields.
///
/// ```
/// let mut options = jarrow_jar::ReadOptions::default();
/// options.max_total_size = 64 << 20;
/// assert!(jarrow_jar::Jar::read_with(b"not a zip", &options).is_err());
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReadOptions {
    /// Most entries (files and directories) in one archive. Default 100,000.
    pub max_entries: usize,
    /// Largest uncompressed size of one entry, in bytes. Default 256 MiB.
    pub max_entry_size: u64,
    /// Largest uncompressed size of all entries together, in bytes. Default 1 GiB.
    pub max_total_size: u64,
    /// Deepest nesting [`crate::Jar::read_nested`] follows; the outer jar is level 0.
    /// Default 8, never more than [`HARD_MAX_NESTING_DEPTH`].
    pub max_nesting_depth: u32,
}

impl ReadOptions {
    /// Default for [`ReadOptions::max_entries`].
    pub const DEFAULT_MAX_ENTRIES: usize = 100_000;
    /// Default for [`ReadOptions::max_entry_size`].
    pub const DEFAULT_MAX_ENTRY_SIZE: u64 = 256 << 20;
    /// Default for [`ReadOptions::max_total_size`].
    pub const DEFAULT_MAX_TOTAL_SIZE: u64 = 1 << 30;
    /// Default for [`ReadOptions::max_nesting_depth`].
    pub const DEFAULT_MAX_NESTING_DEPTH: u32 = 8;
}

impl Default for ReadOptions {
    fn default() -> ReadOptions {
        ReadOptions {
            max_entries: ReadOptions::DEFAULT_MAX_ENTRIES,
            max_entry_size: ReadOptions::DEFAULT_MAX_ENTRY_SIZE,
            max_total_size: ReadOptions::DEFAULT_MAX_TOTAL_SIZE,
            max_nesting_depth: ReadOptions::DEFAULT_MAX_NESTING_DEPTH,
        }
    }
}

/// What is left of the entry and size limits; shared across a nested-jar tree.
pub(crate) struct Budget {
    entries_used: u64,
    bytes_used: u64,
    max_entries: u64,
    max_bytes: u64,
}

impl Budget {
    pub(crate) fn new(options: &ReadOptions) -> Budget {
        Budget {
            entries_used: 0,
            bytes_used: 0,
            max_entries: u64::try_from(options.max_entries).unwrap_or(u64::MAX),
            max_bytes: options.max_total_size,
        }
    }

    fn take_entries(&mut self, n: u64) -> Result<()> {
        let requested = self.entries_used.saturating_add(n);
        if requested > self.max_entries {
            return Err(Error::LimitExceeded {
                limit: Limit::Entries,
                requested,
                max: self.max_entries,
            });
        }
        self.entries_used = requested;
        Ok(())
    }

    fn take_bytes(&mut self, n: u64) -> Result<()> {
        let requested = self.bytes_used.saturating_add(n);
        if requested > self.max_bytes {
            return Err(Error::LimitExceeded {
                limit: Limit::TotalSize,
                requested,
                max: self.max_bytes,
            });
        }
        self.bytes_used = requested;
        Ok(())
    }
}

/// A bounds-checked little-endian cursor over a byte slice.
struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn at(data: &'a [u8], pos: usize) -> Cursor<'a> {
        Cursor { data, pos }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let bytes = self.data.get(self.pos..end)?;
        self.pos = end;
        Some(bytes)
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2)
            .and_then(|b| b.try_into().ok())
            .map(u16::from_le_bytes)
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .and_then(|b| b.try_into().ok())
            .map(u32::from_le_bytes)
    }

    fn u64(&mut self) -> Option<u64> {
        self.take(8)
            .and_then(|b| b.try_into().ok())
            .map(u64::from_le_bytes)
    }
}

fn truncated(what: &str) -> Error {
    Error::malformed(format!("{what} is truncated"))
}

fn to_usize(value: u64, what: &str) -> Result<usize> {
    usize::try_from(value).map_err(|_| Error::malformed(format!("{what} does not fit in memory")))
}

/// Where the central directory is, as the end records describe it.
struct Directory {
    entries: u64,
    /// Absolute position of the first central-directory record.
    start: usize,
    size: usize,
    /// Added to every local-header offset (bytes prepended to the archive).
    base: usize,
}

fn find_eocd(data: &[u8]) -> Result<usize> {
    if data.len() < EOCD_LEN {
        return Err(Error::NotAZip);
    }
    let last = data.len() - EOCD_LEN;
    let first = last.saturating_sub(MAX_COMMENT);
    for pos in (first..=last).rev() {
        let mut c = Cursor::at(data, pos);
        if c.u32() != Some(EOCD_SIG) {
            continue;
        }
        let mut c = Cursor::at(data, pos + 20);
        let comment_len = usize::from(c.u16().ok_or(Error::NotAZip)?);
        if pos + EOCD_LEN + comment_len == data.len() {
            return Ok(pos);
        }
    }
    Err(Error::NotAZip)
}

fn locate_directory(data: &[u8]) -> Result<Directory> {
    let eocd = find_eocd(data)?;
    let mut c = Cursor::at(data, eocd + 4);
    let eocd_err = || truncated("end-of-central-directory record");
    let disk = c.u16().ok_or_else(eocd_err)?;
    let cd_disk = c.u16().ok_or_else(eocd_err)?;
    let entries_disk = c.u16().ok_or_else(eocd_err)?;
    let entries_total = c.u16().ok_or_else(eocd_err)?;
    let cd_size = c.u32().ok_or_else(eocd_err)?;
    let cd_offset = c.u32().ok_or_else(eocd_err)?;

    let locator = eocd
        .checked_sub(Z64_LOCATOR_LEN)
        .filter(|&pos| Cursor::at(data, pos).u32() == Some(Z64_LOCATOR_SIG));

    if let Some(locator) = locator {
        let mut c = Cursor::at(data, locator + 4);
        let loc_err = || truncated("zip64 end-of-central-directory locator");
        let z64_disk = c.u32().ok_or_else(loc_err)?;
        let z64_pos = c.u64().ok_or_else(loc_err)?;
        let disks = c.u32().ok_or_else(loc_err)?;
        if z64_disk != 0 || disks > 1 {
            return Err(Error::MultiDisk);
        }
        let z64_pos = to_usize(z64_pos, "zip64 end-of-central-directory offset")?;
        if z64_pos
            .checked_add(Z64_EOCD_LEN)
            .is_none_or(|end| end > locator)
        {
            return Err(Error::malformed(
                "zip64 end-of-central-directory record is not before its locator",
            ));
        }
        let mut c = Cursor::at(data, z64_pos);
        let rec_err = || truncated("zip64 end-of-central-directory record");
        if c.u32().ok_or_else(rec_err)? != Z64_EOCD_SIG {
            return Err(Error::malformed(
                "zip64 end-of-central-directory record has a bad signature",
            ));
        }
        let record_size = c.u64().ok_or_else(rec_err)?;
        let _made_by = c.u16().ok_or_else(rec_err)?;
        let _needed = c.u16().ok_or_else(rec_err)?;
        let disk = c.u32().ok_or_else(rec_err)?;
        let cd_disk = c.u32().ok_or_else(rec_err)?;
        let entries_disk = c.u64().ok_or_else(rec_err)?;
        let entries = c.u64().ok_or_else(rec_err)?;
        let size = c.u64().ok_or_else(rec_err)?;
        let offset = c.u64().ok_or_else(rec_err)?;
        if record_size < 44 {
            return Err(Error::malformed(
                "zip64 end-of-central-directory record is too short",
            ));
        }
        if disk != 0 || cd_disk != 0 || entries_disk != entries {
            return Err(Error::MultiDisk);
        }
        let end = offset
            .checked_add(size)
            .ok_or_else(|| Error::malformed("central directory offset overflows"))?;
        if end > z64_pos as u64 {
            return Err(Error::malformed(
                "central directory overlaps the zip64 end records",
            ));
        }
        return Ok(Directory {
            entries,
            start: to_usize(offset, "central directory offset")?,
            size: to_usize(size, "central directory size")?,
            base: 0,
        });
    }

    if disk != 0 || cd_disk != 0 || entries_disk != entries_total {
        return Err(Error::MultiDisk);
    }
    let size = cd_size as usize;
    let offset = cd_offset as usize;
    let end = offset
        .checked_add(size)
        .ok_or_else(|| Error::malformed("central directory offset overflows"))?;
    // Bytes between the end of the directory and the end record are bytes prepended to the
    // archive (as with a launcher script); offsets in the archive are relative to its start.
    let base = eocd.checked_sub(end).ok_or_else(|| {
        Error::malformed("central directory extends past the end-of-central-directory record")
    })?;
    Ok(Directory {
        entries: u64::from(entries_total),
        start: base + offset,
        size,
        base,
    })
}

/// One central-directory record, with zip64 values already applied.
struct Record {
    name: String,
    method: Method,
    crc32: u32,
    compressed_size: u64,
    uncompressed_size: u64,
    /// Absolute position of the local header.
    local: usize,
    has_extra: bool,
    unix_mode: Option<u32>,
    zip64: bool,
}

/// Reads the zip64 extended-information extra field for the values that are marked.
fn apply_zip64(
    name: &str,
    extra: &[u8],
    uncompressed: &mut u64,
    compressed: &mut u64,
    offset: &mut u64,
    disk: &mut u32,
) -> Result<()> {
    let need_u = *uncompressed == u64::from(U32_MARK);
    let need_c = *compressed == u64::from(U32_MARK);
    let need_o = *offset == u64::from(U32_MARK);
    let need_d = *disk == u32::from(U16_MARK);
    let missing = || {
        Error::malformed(format!(
            "entry {name:?} needs a zip64 extra field that is missing or too short"
        ))
    };
    let mut c = Cursor::at(extra, 0);
    while let (Some(id), Some(len)) = (c.u16(), c.u16()) {
        let body = c.take(usize::from(len)).ok_or_else(missing)?;
        if id != ZIP64_EXTRA_ID {
            continue;
        }
        let mut f = Cursor::at(body, 0);
        if need_u {
            *uncompressed = f.u64().ok_or_else(missing)?;
        }
        if need_c {
            *compressed = f.u64().ok_or_else(missing)?;
        }
        if need_o {
            *offset = f.u64().ok_or_else(missing)?;
        }
        if need_d {
            *disk = f.u32().ok_or_else(missing)?;
        }
        return Ok(());
    }
    Err(missing())
}

fn read_records(data: &[u8], dir: &Directory) -> Result<Vec<Record>> {
    let cd = dir
        .start
        .checked_add(dir.size)
        .and_then(|end| data.get(dir.start..end))
        .ok_or_else(|| Error::malformed("central directory lies outside the archive"))?;
    let min_size = dir
        .entries
        .checked_mul(CENTRAL_LEN as u64)
        .ok_or_else(|| Error::malformed("entry count overflows"))?;
    if min_size > cd.len() as u64 {
        return Err(Error::malformed(format!(
            "central directory of {} bytes cannot hold {} entries",
            cd.len(),
            dir.entries
        )));
    }
    let count = to_usize(dir.entries, "entry count")?;
    let mut records = Vec::with_capacity(count);
    let mut c = Cursor::at(cd, 0);
    for index in 0..count {
        let err = || truncated(&format!("central directory record #{index}"));
        if c.u32().ok_or_else(err)? != CENTRAL_SIG {
            return Err(Error::malformed(format!(
                "central directory record #{index} has a bad signature"
            )));
        }
        let made_by = c.u16().ok_or_else(err)?;
        let _needed = c.u16().ok_or_else(err)?;
        let flags = c.u16().ok_or_else(err)?;
        let method = c.u16().ok_or_else(err)?;
        let _time = c.u16().ok_or_else(err)?;
        let _date = c.u16().ok_or_else(err)?;
        let crc32 = c.u32().ok_or_else(err)?;
        let compressed32 = c.u32().ok_or_else(err)?;
        let uncompressed32 = c.u32().ok_or_else(err)?;
        let name_len = c.u16().ok_or_else(err)?;
        let extra_len = c.u16().ok_or_else(err)?;
        let comment_len = c.u16().ok_or_else(err)?;
        let disk16 = c.u16().ok_or_else(err)?;
        let _internal = c.u16().ok_or_else(err)?;
        let external = c.u32().ok_or_else(err)?;
        let offset32 = c.u32().ok_or_else(err)?;
        let name = c.take(usize::from(name_len)).ok_or_else(err)?;
        let extra = c.take(usize::from(extra_len)).ok_or_else(err)?;
        c.take(usize::from(comment_len)).ok_or_else(err)?;

        let name = String::from_utf8(name.to_vec()).map_err(|_| Error::NameNotUtf8 { index })?;
        check_name(&name).map_err(|reason| Error::UnsafeName {
            name: name.clone(),
            reason,
        })?;
        if flags & (FLAG_ENCRYPTED | FLAG_STRONG_ENCRYPTION | FLAG_MASKED_HEADERS) != 0 {
            return Err(Error::Encrypted { name });
        }
        let Some(method) = Method::from_id(method) else {
            return Err(Error::UnsupportedMethod { name, method });
        };

        let mut uncompressed = u64::from(uncompressed32);
        let mut compressed = u64::from(compressed32);
        let mut offset = u64::from(offset32);
        let mut disk = u32::from(disk16);
        let zip64 = uncompressed32 == U32_MARK
            || compressed32 == U32_MARK
            || offset32 == U32_MARK
            || disk16 == U16_MARK;
        if zip64 {
            apply_zip64(
                &name,
                extra,
                &mut uncompressed,
                &mut compressed,
                &mut offset,
                &mut disk,
            )?;
        }
        if disk != 0 {
            return Err(Error::MultiDisk);
        }
        if name.ends_with('/') && uncompressed != 0 {
            return Err(Error::DirectoryWithData { name });
        }
        let local = to_usize(offset, "local header offset")?
            .checked_add(dir.base)
            .ok_or_else(|| Error::malformed("local header offset overflows"))?;
        let host = made_by >> 8;
        let mode = external >> 16;
        let unix_mode = ((host == HOST_UNIX || host == HOST_DARWIN) && mode != 0).then_some(mode);
        records.push(Record {
            name,
            method,
            crc32,
            compressed_size: compressed,
            uncompressed_size: uncompressed,
            local,
            has_extra: extra_len != 0,
            unix_mode,
            zip64,
        });
    }
    Ok(records)
}

/// Byte range of one entry's stored data, and where its local header starts.
struct Span {
    local: usize,
    data: usize,
    end: usize,
}

fn read_local(data: &[u8], record: &Record, cd_start: usize) -> Result<(Span, bool)> {
    let name = &record.name;
    let mut c = Cursor::at(data, record.local);
    let err = || truncated(&format!("local header of entry {name:?}"));
    if c.u32().ok_or_else(err)? != LOCAL_SIG {
        return Err(Error::malformed(format!(
            "local header of entry {name:?} has a bad signature"
        )));
    }
    let _needed = c.u16().ok_or_else(err)?;
    let flags = c.u16().ok_or_else(err)?;
    let method = c.u16().ok_or_else(err)?;
    c.take(16).ok_or_else(err)?; // time, date, CRC-32 and sizes: the central directory rules
    let name_len = c.u16().ok_or_else(err)?;
    let extra_len = c.u16().ok_or_else(err)?;
    let local_name = c.take(usize::from(name_len)).ok_or_else(err)?;
    c.take(usize::from(extra_len)).ok_or_else(err)?;
    if local_name != name.as_bytes() {
        return Err(Error::LocalHeaderMismatch {
            name: name.clone(),
            field: "name",
        });
    }
    if method != record.method.id() {
        return Err(Error::LocalHeaderMismatch {
            name: name.clone(),
            field: "compression method",
        });
    }
    if flags & (FLAG_ENCRYPTED | FLAG_STRONG_ENCRYPTION) != 0 {
        return Err(Error::Encrypted { name: name.clone() });
    }
    let start = c.pos;
    let end = to_usize(record.compressed_size, "compressed size")
        .ok()
        .and_then(|size| start.checked_add(size))
        .filter(|&end| end <= cd_start)
        .ok_or_else(|| {
            Error::malformed(format!(
                "data of entry {name:?} runs into the central directory"
            ))
        })?;
    Ok((
        Span {
            local: record.local,
            data: start,
            end,
        },
        extra_len != 0,
    ))
}

fn inflate(record: &Record, raw: &[u8]) -> Result<Vec<u8>> {
    let name = &record.name;
    let declared = record.uncompressed_size;
    match record.method {
        Method::Stored => {
            if raw.len() as u64 != declared {
                return Err(Error::SizeMismatch {
                    name: name.clone(),
                    declared,
                    actual: raw.len() as u64,
                });
            }
            Ok(raw.to_vec())
        }
        Method::Deflated => {
            // One byte over the declared size is enough to prove the declaration false.
            let limit = declared
                .checked_add(1)
                .and_then(|l| usize::try_from(l).ok())
                .ok_or(Error::LimitExceeded {
                    limit: Limit::EntrySize,
                    requested: declared,
                    max: usize::MAX as u64,
                })?;
            let out = match decompress_to_vec_with_limit(raw, limit) {
                Ok(out) => out,
                Err(e) if e.status == TINFLStatus::HasMoreOutput => {
                    return Err(Error::SizeMismatch {
                        name: name.clone(),
                        declared,
                        actual: declared.saturating_add(1),
                    });
                }
                Err(_) => return Err(Error::Inflate { name: name.clone() }),
            };
            if out.len() as u64 != declared {
                return Err(Error::SizeMismatch {
                    name: name.clone(),
                    declared,
                    actual: out.len() as u64,
                });
            }
            Ok(out)
        }
    }
}

/// Reads one archive, charging its entries and declared sizes to `budget`.
pub(crate) fn read_archive(data: &[u8], options: &ReadOptions, budget: &mut Budget) -> Result<Jar> {
    let dir = locate_directory(data)?;
    budget.take_entries(dir.entries)?;
    let records = read_records(data, &dir)?;

    let mut names = BTreeSet::new();
    for record in &records {
        if !names.insert(record.name.as_str()) {
            return Err(Error::DuplicateName {
                name: record.name.clone(),
            });
        }
        if record.uncompressed_size > options.max_entry_size {
            return Err(Error::LimitExceeded {
                limit: Limit::EntrySize,
                requested: record.uncompressed_size,
                max: options.max_entry_size,
            });
        }
        budget.take_bytes(record.uncompressed_size)?;
    }

    let mut spans = Vec::with_capacity(records.len());
    for record in &records {
        spans.push(read_local(data, record, dir.start)?);
    }

    // Overlapping entries are how "better zip bombs" multiply one compressed kernel; no
    // honest writer produces them.
    let mut order: Vec<usize> = (0..records.len()).collect();
    order.sort_by_key(|&i| spans[i].0.local);
    for pair in order.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if spans[b].0.local < spans[a].0.end {
            return Err(Error::OverlappingEntries {
                first: records[a].name.clone(),
                second: records[b].name.clone(),
            });
        }
    }

    let mut entries = Vec::with_capacity(records.len());
    for (record, (span, local_extra)) in records.into_iter().zip(spans) {
        let raw = data
            .get(span.data..span.end)
            .ok_or_else(|| truncated(&format!("data of entry {:?}", record.name)))?;
        let bytes = inflate(&record, raw)?;
        let actual = crc32(&bytes);
        if actual != record.crc32 {
            return Err(Error::CrcMismatch {
                name: record.name,
                declared: record.crc32,
                actual,
            });
        }
        entries.push(Entry {
            original: Some(Original {
                method: record.method,
                crc32: record.crc32,
                compressed_size: record.compressed_size,
                uncompressed_size: record.uncompressed_size,
                has_extra_field: record.has_extra || local_extra,
                unix_mode: record.unix_mode,
                zip64: record.zip64,
            }),
            name: record.name,
            data: bytes,
            method: record.method,
        });
    }
    Ok(Jar { entries })
}
