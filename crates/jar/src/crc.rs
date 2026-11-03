//! CRC-32 as zip uses it (IEEE 802.3, reflected polynomial `0xEDB88320`).

const TABLE: [u32; 256] = make_table();

const fn make_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

/// The CRC-32 of `data`, as stored in zip headers.
///
/// ```
/// assert_eq!(jarrow_jar::crc32(b"123456789"), 0xCBF4_3926);
/// ```
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &byte in data {
        c = TABLE[((c ^ u32::from(byte)) & 0xFF) as usize] ^ (c >> 8);
    }
    !c
}

#[cfg(test)]
mod tests {
    use super::crc32;

    #[test]
    fn known_vectors() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414F_A339
        );
    }
}
