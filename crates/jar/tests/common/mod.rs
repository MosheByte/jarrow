//! A raw zip builder for tests: it writes whatever it is told, including the hostile
//! archives that `Jar::write` refuses to produce.
#![allow(dead_code)]

use jarrow_jar::crc32;

/// One entry, field by field. Start from [`Raw::stored`] or [`Raw::deflated`].
#[derive(Clone, Debug)]
pub struct Raw {
    pub name: Vec<u8>,
    /// Name in the local header, when it should differ from the central one.
    pub local_name: Option<Vec<u8>>,
    /// The bytes as stored (already compressed for method 8).
    pub payload: Vec<u8>,
    pub method: u16,
    pub local_method: Option<u16>,
    pub crc: u32,
    pub csize: u32,
    pub usize: u32,
    pub flags: u16,
    pub made_by: u16,
    pub external: u32,
    pub time: u16,
    pub date: u16,
    pub central_extra: Vec<u8>,
    pub local_extra: Vec<u8>,
    pub comment: Vec<u8>,
    /// Write sizes and CRC as zero in the local header and append a data descriptor.
    pub descriptor: bool,
    /// Skip the local header and data; the central record points at `offset`.
    pub write_local: bool,
    /// Local-header offset to put in the central record instead of the real one.
    pub offset: Option<u32>,
}

impl Raw {
    pub fn stored(name: &str, data: &[u8]) -> Raw {
        Raw {
            name: name.as_bytes().to_vec(),
            local_name: None,
            payload: data.to_vec(),
            method: 0,
            local_method: None,
            crc: crc32(data),
            csize: data.len() as u32,
            usize: data.len() as u32,
            flags: 0,
            made_by: 20,
            external: 0,
            time: 0x6000,
            date: 0x5a21,
            central_extra: Vec::new(),
            local_extra: Vec::new(),
            comment: Vec::new(),
            descriptor: false,
            write_local: true,
            offset: None,
        }
    }

    pub fn deflated(name: &str, data: &[u8]) -> Raw {
        let payload = miniz_oxide::deflate::compress_to_vec(data, 6);
        let mut raw = Raw::stored(name, data);
        raw.csize = payload.len() as u32;
        raw.payload = payload;
        raw.method = 8;
        raw
    }

    pub fn name_bytes(name: &[u8], data: &[u8]) -> Raw {
        let mut raw = Raw::stored("x", data);
        raw.name = name.to_vec();
        raw
    }
}

fn put16(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn put64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// The local header plus data (plus descriptor) of one entry.
pub fn local(raw: &Raw) -> Vec<u8> {
    let mut out = Vec::new();
    let name = raw.local_name.as_ref().unwrap_or(&raw.name);
    put32(&mut out, 0x0403_4b50);
    put16(&mut out, 20);
    put16(
        &mut out,
        raw.flags | if raw.descriptor { 1 << 3 } else { 0 },
    );
    put16(&mut out, raw.local_method.unwrap_or(raw.method));
    put16(&mut out, raw.time);
    put16(&mut out, raw.date);
    if raw.descriptor {
        put32(&mut out, 0);
        put32(&mut out, 0);
        put32(&mut out, 0);
    } else {
        put32(&mut out, raw.crc);
        put32(&mut out, raw.csize);
        put32(&mut out, raw.usize);
    }
    put16(&mut out, name.len() as u16);
    put16(&mut out, raw.local_extra.len() as u16);
    out.extend_from_slice(name);
    out.extend_from_slice(&raw.local_extra);
    out.extend_from_slice(&raw.payload);
    if raw.descriptor {
        put32(&mut out, 0x0807_4b50);
        put32(&mut out, raw.crc);
        put32(&mut out, raw.csize);
        put32(&mut out, raw.usize);
    }
    out
}

/// Options for the end of the archive.
#[derive(Clone, Debug, Default)]
pub struct End {
    pub prefix: Vec<u8>,
    pub comment: Vec<u8>,
    /// Entry count to declare instead of the real one.
    pub declared_entries: Option<u16>,
    /// Write a zip64 end record and locator (with the real values) and mark the EOCD.
    pub zip64: bool,
}

pub fn build(entries: &[Raw]) -> Vec<u8> {
    build_with(entries, &End::default())
}

pub fn build_with(entries: &[Raw], end: &End) -> Vec<u8> {
    let mut body = Vec::new();
    let mut offsets = Vec::new();
    for raw in entries {
        offsets.push(body.len() as u32);
        if raw.write_local {
            body.extend_from_slice(&local(raw));
        }
    }
    let cd_offset = body.len();
    let mut central = Vec::new();
    for (raw, offset) in entries.iter().zip(offsets) {
        put32(&mut central, 0x0201_4b50);
        put16(&mut central, raw.made_by);
        put16(&mut central, 20);
        put16(
            &mut central,
            raw.flags | if raw.descriptor { 1 << 3 } else { 0 },
        );
        put16(&mut central, raw.method);
        put16(&mut central, raw.time);
        put16(&mut central, raw.date);
        put32(&mut central, raw.crc);
        put32(&mut central, raw.csize);
        put32(&mut central, raw.usize);
        put16(&mut central, raw.name.len() as u16);
        put16(&mut central, raw.central_extra.len() as u16);
        put16(&mut central, raw.comment.len() as u16);
        put16(&mut central, 0);
        put16(&mut central, 0);
        put32(&mut central, raw.external);
        put32(&mut central, raw.offset.unwrap_or(offset));
        central.extend_from_slice(&raw.name);
        central.extend_from_slice(&raw.central_extra);
        central.extend_from_slice(&raw.comment);
    }
    let mut out = end.prefix.clone();
    out.extend_from_slice(&body);
    out.extend_from_slice(&central);
    let count = end.declared_entries.unwrap_or(entries.len() as u16);
    if end.zip64 {
        let record = out.len() as u64;
        put32(&mut out, 0x0606_4b50);
        put64(&mut out, 44);
        put16(&mut out, 45);
        put16(&mut out, 45);
        put32(&mut out, 0);
        put32(&mut out, 0);
        put64(&mut out, entries.len() as u64);
        put64(&mut out, entries.len() as u64);
        put64(&mut out, central.len() as u64);
        put64(&mut out, cd_offset as u64);
        put32(&mut out, 0x0706_4b50);
        put32(&mut out, 0);
        put64(&mut out, record);
        put32(&mut out, 1);
    }
    put32(&mut out, 0x0605_4b50);
    put16(&mut out, 0);
    put16(&mut out, 0);
    put16(&mut out, if end.zip64 { 0xFFFF } else { count });
    put16(&mut out, if end.zip64 { 0xFFFF } else { count });
    put32(&mut out, central.len() as u32);
    put32(
        &mut out,
        if end.zip64 {
            0xFFFF_FFFF
        } else {
            cd_offset as u32
        },
    );
    put16(&mut out, end.comment.len() as u16);
    out.extend_from_slice(&end.comment);
    out
}

/// A zip64 extended-information extra field holding `values` (in spec order).
pub fn zip64_extra(values: &[u64]) -> Vec<u8> {
    let mut out = Vec::new();
    put16(&mut out, 0x0001);
    put16(&mut out, (values.len() * 8) as u16);
    for v in values {
        put64(&mut out, *v);
    }
    out
}

/// Little-endian u16 at `pos`.
pub fn u16_at(bytes: &[u8], pos: usize) -> u16 {
    u16::from_le_bytes([bytes[pos], bytes[pos + 1]])
}

/// Little-endian u32 at `pos`.
pub fn u32_at(bytes: &[u8], pos: usize) -> u32 {
    u32::from_le_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
}
