//! The deterministic zip writer.
//!
//! Output is a pure function of the entries and the options: fixed DOS timestamp
//! (1980-01-01 00:00:00), entries in the order given with `META-INF/MANIFEST.MF` moved
//! first, a fixed deflate level, no extra fields (except the zip64 field, which only archives
//! over 4 GiB or with more than 65,534 entries need), no comments, no Unix modes, no data
//! descriptors.

use std::borrow::Cow;
use std::collections::BTreeSet;

use miniz_oxide::deflate::compress_to_vec;

use crate::Jar;
use crate::crc::crc32;
use crate::entry::{Entry, Method};
use crate::error::{Error, Result};
use crate::name::check_name;
use crate::read::{
    CENTRAL_SIG, EOCD_SIG, LOCAL_SIG, U16_MARK, U32_MARK, Z64_EOCD_SIG, Z64_LOCATOR_SIG,
    ZIP64_EXTRA_ID,
};

/// DOS time of 00:00:00.
pub(crate) const DOS_TIME: u16 = 0;
/// DOS date of 1980-01-01: (year - 1980) << 9 | month << 5 | day.
pub(crate) const DOS_DATE: u16 = (1 << 5) | 1;

const FLAG_UTF8: u16 = 1 << 11;
const VERSION_STORED: u16 = 10;
const VERSION_DEFLATE_OR_DIR: u16 = 20;
const VERSION_ZIP64: u16 = 45;
const ATTR_DIRECTORY: u32 = 0x10;

/// Which compression the writer uses.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Compression {
    /// Each entry's own [`Entry::method`]. Entries read from an archive keep the method they
    /// had there; new entries ([`Entry::file`]) are deflated.
    #[default]
    PerEntry,
    /// Deflate every file.
    Deflate,
    /// Store every file uncompressed.
    Store,
}

/// Options for [`Jar::write`].
///
/// Directories and empty files are always stored, whatever the options say.
/// The struct is `#[non_exhaustive]`: start from [`WriteOptions::default`] and set fields.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WriteOptions {
    /// Which compression to use. Default [`Compression::PerEntry`].
    pub compression: Compression,
    /// Deflate level, 0 (no compression) to 9 (smallest). Default 6.
    pub deflate_level: u8,
}

impl WriteOptions {
    /// Default for [`WriteOptions::deflate_level`].
    pub const DEFAULT_DEFLATE_LEVEL: u8 = 6;
    /// Highest accepted [`WriteOptions::deflate_level`].
    pub const MAX_DEFLATE_LEVEL: u8 = 9;
}

impl Default for WriteOptions {
    fn default() -> WriteOptions {
        WriteOptions {
            compression: Compression::PerEntry,
            deflate_level: WriteOptions::DEFAULT_DEFLATE_LEVEL,
        }
    }
}

/// From which size and entry count the writer switches to zip64 fields. Only tests lower
/// them, to exercise the zip64 path without 4 GiB of data.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Thresholds {
    pub(crate) size: u64,
    pub(crate) count: u64,
}

pub(crate) const STANDARD: Thresholds = Thresholds {
    size: U32_MARK as u64,
    count: U16_MARK as u64,
};

fn put16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// A 32-bit header field: the value, or the zip64 marker when it reaches the threshold.
fn field32(v: u64, threshold: u64) -> u32 {
    if v >= threshold {
        U32_MARK
    } else {
        u32::try_from(v).unwrap_or(U32_MARK)
    }
}

/// Index of the manifest: `META-INF/MANIFEST.MF` exactly, else the first file whose name
/// matches it ignoring ASCII case (the JVM looks it up the same way).
pub(crate) fn manifest_index(entries: &[Entry]) -> Option<usize> {
    let path = crate::manifest::MANIFEST_PATH;
    entries.iter().position(|e| e.name == path).or_else(|| {
        entries
            .iter()
            .position(|e| e.name.eq_ignore_ascii_case(path))
    })
}

fn validate(jar: &Jar, options: &WriteOptions) -> Result<()> {
    if options.deflate_level > WriteOptions::MAX_DEFLATE_LEVEL {
        return Err(Error::InvalidOption {
            what: "deflate_level must be 0 to 9",
        });
    }
    let mut names = BTreeSet::new();
    for entry in &jar.entries {
        check_name(&entry.name).map_err(|reason| Error::UnsafeName {
            name: entry.name.clone(),
            reason,
        })?;
        if entry.name.len() > usize::from(U16_MARK) {
            return Err(Error::malformed(format!(
                "entry name of {} bytes is longer than zip allows (65,535)",
                entry.name.len()
            )));
        }
        if entry.is_dir() && !entry.data.is_empty() {
            return Err(Error::DirectoryWithData {
                name: entry.name.clone(),
            });
        }
        if !names.insert(entry.name.as_str()) {
            return Err(Error::DuplicateName {
                name: entry.name.clone(),
            });
        }
    }
    Ok(())
}

pub(crate) fn write_archive(jar: &Jar, options: &WriteOptions, t: Thresholds) -> Result<Vec<u8>> {
    validate(jar, options)?;

    let manifest = manifest_index(&jar.entries);
    let order = manifest
        .into_iter()
        .chain((0..jar.entries.len()).filter(|&i| Some(i) != manifest));

    let mut out = Vec::new();
    let mut central = Vec::new();
    let mut count: u64 = 0;
    for index in order {
        let entry = &jar.entries[index];
        let method = if entry.is_dir() || entry.data.is_empty() {
            Method::Stored
        } else {
            match options.compression {
                Compression::PerEntry => entry.method,
                Compression::Deflate => Method::Deflated,
                Compression::Store => Method::Stored,
            }
        };
        let stored: Cow<'_, [u8]> = match method {
            Method::Stored => Cow::Borrowed(&entry.data),
            Method::Deflated => Cow::Owned(compress_to_vec(&entry.data, options.deflate_level)),
        };
        let crc = crc32(&entry.data);
        let size = entry.data.len() as u64;
        let csize = stored.len() as u64;
        let offset = out.len() as u64;
        let name = entry.name.as_bytes();
        let name_len = u16::try_from(name.len()).unwrap_or(U16_MARK);
        let flags = if entry.name.is_ascii() { 0 } else { FLAG_UTF8 };

        let local_zip64 = size >= t.size || csize >= t.size;
        let mut central_extra = Vec::new();
        if size >= t.size {
            put64(&mut central_extra, size);
        }
        if csize >= t.size {
            put64(&mut central_extra, csize);
        }
        if offset >= t.size {
            put64(&mut central_extra, offset);
        }
        let version = if local_zip64 || !central_extra.is_empty() {
            VERSION_ZIP64
        } else if method == Method::Deflated || entry.is_dir() {
            VERSION_DEFLATE_OR_DIR
        } else {
            VERSION_STORED
        };

        put32(&mut out, LOCAL_SIG);
        put16(&mut out, version);
        put16(&mut out, flags);
        put16(&mut out, method.id());
        put16(&mut out, DOS_TIME);
        put16(&mut out, DOS_DATE);
        put32(&mut out, crc);
        if local_zip64 {
            put32(&mut out, U32_MARK);
            put32(&mut out, U32_MARK);
        } else {
            put32(&mut out, field32(csize, t.size));
            put32(&mut out, field32(size, t.size));
        }
        put16(&mut out, name_len);
        put16(&mut out, if local_zip64 { 20 } else { 0 });
        out.extend_from_slice(name);
        if local_zip64 {
            put16(&mut out, ZIP64_EXTRA_ID);
            put16(&mut out, 16);
            put64(&mut out, size);
            put64(&mut out, csize);
        }
        out.extend_from_slice(&stored);

        put32(&mut central, CENTRAL_SIG);
        put16(&mut central, version); // made by: MS-DOS host, so no Unix mode
        put16(&mut central, version);
        put16(&mut central, flags);
        put16(&mut central, method.id());
        put16(&mut central, DOS_TIME);
        put16(&mut central, DOS_DATE);
        put32(&mut central, crc);
        put32(&mut central, field32(csize, t.size));
        put32(&mut central, field32(size, t.size));
        put16(&mut central, name_len);
        let extra_len = if central_extra.is_empty() {
            0
        } else {
            central_extra.len() as u16 + 4
        };
        put16(&mut central, extra_len);
        put16(&mut central, 0); // comment length
        put16(&mut central, 0); // disk number start
        put16(&mut central, 0); // internal attributes
        put32(
            &mut central,
            if entry.is_dir() { ATTR_DIRECTORY } else { 0 },
        );
        put32(&mut central, field32(offset, t.size));
        central.extend_from_slice(name);
        if !central_extra.is_empty() {
            put16(&mut central, ZIP64_EXTRA_ID);
            put16(&mut central, central_extra.len() as u16);
            central.extend_from_slice(&central_extra);
        }
        count += 1;
    }

    let cd_offset = out.len() as u64;
    let cd_size = central.len() as u64;
    out.extend_from_slice(&central);

    let zip64 = count >= t.count || cd_offset >= t.size || cd_size >= t.size;
    if zip64 {
        let record = out.len() as u64;
        put32(&mut out, Z64_EOCD_SIG);
        put64(&mut out, 44); // size of the rest of this record
        put16(&mut out, VERSION_ZIP64);
        put16(&mut out, VERSION_ZIP64);
        put32(&mut out, 0); // this disk
        put32(&mut out, 0); // disk of the central directory
        put64(&mut out, count);
        put64(&mut out, count);
        put64(&mut out, cd_size);
        put64(&mut out, cd_offset);
        put32(&mut out, Z64_LOCATOR_SIG);
        put32(&mut out, 0); // disk of the zip64 record
        put64(&mut out, record);
        put32(&mut out, 1); // total disks
    }
    let count16 = if count >= t.count {
        U16_MARK
    } else {
        u16::try_from(count).unwrap_or(U16_MARK)
    };
    put32(&mut out, EOCD_SIG);
    put16(&mut out, 0); // this disk
    put16(&mut out, 0); // disk of the central directory
    put16(&mut out, count16);
    put16(&mut out, count16);
    put32(&mut out, field32(cd_size, t.size));
    put32(&mut out, field32(cd_offset, t.size));
    put16(&mut out, 0); // comment length
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ReadOptions;

    fn small() -> Thresholds {
        Thresholds { size: 8, count: 2 }
    }

    #[test]
    fn dos_date_is_1980_01_01() {
        assert_eq!(DOS_DATE, 0x0021);
        assert_eq!(DOS_TIME, 0);
    }

    #[test]
    fn forced_zip64_round_trips() {
        let mut jar = Jar::new();
        jar.entries
            .push(Entry::file("a.txt", b"hello, zip64 world".to_vec()));
        jar.entries.push(Entry::directory("dir"));
        let mut stored = Entry::file("dir/b.bin", vec![7u8; 40]);
        stored.method = Method::Stored;
        jar.entries.push(stored);
        let bytes = write_archive(&jar, &WriteOptions::default(), small()).unwrap();
        let back = Jar::read_with(&bytes, &ReadOptions::default()).unwrap();
        assert_eq!(
            back.names().collect::<Vec<_>>(),
            ["a.txt", "dir/", "dir/b.bin"]
        );
        assert_eq!(back.entries[0].data, b"hello, zip64 world");
        assert_eq!(back.entries[2].data, vec![7u8; 40]);
        assert!(back.entries[0].original.unwrap().zip64);
        assert!(back.entries[0].original.unwrap().has_extra_field);
        // the directory sits past offset 8, so its central record needs a zip64 offset
        assert!(back.entries[1].original.unwrap().zip64);
        // writing again with forced zip64 gives the same bytes
        assert_eq!(
            write_archive(&back, &WriteOptions::default(), small()).unwrap(),
            bytes
        );
    }

    #[test]
    fn manifest_index_prefers_exact_case() {
        let entries = vec![
            Entry::file("meta-inf/manifest.mf", b"x".to_vec()),
            Entry::file("META-INF/MANIFEST.MF", b"y".to_vec()),
        ];
        assert_eq!(manifest_index(&entries), Some(1));
        assert_eq!(manifest_index(&entries[..1]), Some(0));
        assert_eq!(manifest_index(&[]), None);
    }
}
