//! One file or directory in a JAR.

/// How an entry's bytes are stored in the archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Method {
    /// Method 0: the bytes as they are.
    Stored,
    /// Method 8: raw DEFLATE.
    Deflated,
}

impl Method {
    /// The zip method id (0 or 8).
    pub fn id(self) -> u16 {
        match self {
            Method::Stored => 0,
            Method::Deflated => 8,
        }
    }

    pub(crate) fn from_id(id: u16) -> Option<Method> {
        match id {
            0 => Some(Method::Stored),
            8 => Some(Method::Deflated),
            _ => None,
        }
    }
}

/// How an entry looked in the archive it was read from.
///
/// These values describe the input and are kept for the report. The writer ignores them: it
/// derives sizes and CRC from [`Entry::data`] and writes no extra fields and no Unix mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Original {
    /// Compression method in the input.
    pub method: Method,
    /// CRC-32 the input declared (the reader checked that it matches the data).
    pub crc32: u32,
    /// Size of the stored bytes in the input.
    pub compressed_size: u64,
    /// Uncompressed size the input declared (the reader checked that it matches the data).
    pub uncompressed_size: u64,
    /// Whether the local header or the central-directory record carried extra fields.
    pub has_extra_field: bool,
    /// Unix mode bits (`st_mode`), when the archive was made on a Unix-like host and set them.
    pub unix_mode: Option<u32>,
    /// Whether zip64 fields were needed to describe the entry.
    pub zip64: bool,
}

/// One file or directory of a [`crate::Jar`].
///
/// A name ending in `/` is a directory, which carries no data. Names use `/` as the only
/// separator and are checked by [`crate::check_name`] on read and on write.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Entry {
    /// Full path inside the archive, for example `net/example/Mod.class` or `META-INF/`.
    pub name: String,
    /// The uncompressed bytes. Empty for directories.
    pub data: Vec<u8>,
    /// How the writer stores the entry when [`crate::Compression::PerEntry`] is used.
    /// Entries read from an archive keep the method they had there.
    pub method: Method,
    /// How the entry looked in the archive it was read from; `None` for new entries.
    pub original: Option<Original>,
}

impl Entry {
    /// A new file entry, deflated when written with [`crate::Compression::PerEntry`].
    pub fn file(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Entry {
        Entry {
            name: name.into(),
            data: data.into(),
            method: Method::Deflated,
            original: None,
        }
    }

    /// A new directory entry. A `/` is appended to `name` when it does not end in one.
    pub fn directory(name: impl Into<String>) -> Entry {
        let mut name = name.into();
        if !name.ends_with('/') {
            name.push('/');
        }
        Entry {
            name,
            data: Vec::new(),
            method: Method::Stored,
            original: None,
        }
    }

    /// Whether this entry is a directory (its name ends in `/`).
    pub fn is_dir(&self) -> bool {
        self.name.ends_with('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors() {
        let f = Entry::file("a/b.txt", b"hi".to_vec());
        assert!(!f.is_dir());
        assert_eq!(f.method, Method::Deflated);
        let d = Entry::directory("a");
        assert_eq!(d.name, "a/");
        assert!(d.is_dir());
        assert_eq!(Entry::directory("a/").name, "a/");
    }

    #[test]
    fn method_ids() {
        assert_eq!(Method::from_id(Method::Stored.id()), Some(Method::Stored));
        assert_eq!(
            Method::from_id(Method::Deflated.id()),
            Some(Method::Deflated)
        );
        assert_eq!(Method::from_id(12), None);
    }
}
