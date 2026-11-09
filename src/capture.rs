//! Write one lossless, unmodified RGBA frame before SteamVR receives it.
//! Minimal PNG encoder with zlib stored (uncompressed) DEFLATE blocks. It adds
//! no runtime dependency and preserves all four 8-bit channels exactly.

use std::path::Path;

pub fn save_rgba_png(path: &Path, pixels: &[u8], width: usize, height: usize) -> Result<(), String> {
    let expected = width.checked_mul(height).and_then(|n| n.checked_mul(4))
        .ok_or("Image dimensions overflow")?;
    if pixels.len() != expected || width == 0 || height == 0
        || width > u32::MAX as usize || height > u32::MAX as usize {
        return Err("Invalid RGBA frame dimensions".into());
    }
    let mut scanlines = Vec::with_capacity(expected + height);
    for row in pixels.chunks_exact(width * 4) {
        scanlines.push(0); // PNG filter None, preserve each source component.
        scanlines.extend_from_slice(row);
    }

    // RFC 1950 zlib stream, containing RFC 1951 stored DEFLATE blocks.
    let mut zlib = vec![0x78, 0x01];
    for (index, block) in scanlines.chunks(65535).enumerate() {
        let last = (index + 1) * 65535 >= scanlines.len();
        zlib.push(u8::from(last)); // BFINAL, BTYPE=00; byte aligned.
        let len = block.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.extend_from_slice(block);
    }
    zlib.extend_from_slice(&adler32(&scanlines).to_be_bytes());

    let mut png = Vec::with_capacity(zlib.len() + 128);
    png.extend_from_slice(b"\x89PNG\r\n\x1a\n");
    let mut header = [0_u8; 13];
    header[..4].copy_from_slice(&(width as u32).to_be_bytes());
    header[4..8].copy_from_slice(&(height as u32).to_be_bytes());
    header[8] = 8; // 8-bit channels.
    header[9] = 6; // RGBA truecolour with alpha.
    append_chunk(&mut png, b"IHDR", &header);
    append_chunk(&mut png, b"IDAT", &zlib);
    append_chunk(&mut png, b"IEND", &[]);
    std::fs::write(path, png)
        .map_err(|e| format!("Cannot save RGBA capture to {}: {e}", path.display()))
}

fn append_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc = 0xffff_ffff_u32;
    for b in kind.iter().chain(data) {
        crc ^= u32::from(*b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in data {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checks_dimensions_and_checksums() {
        assert!(save_rgba_png(Path::new("/unused"), &[1, 2, 3], 1, 1).is_err());
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
        let mut out = Vec::new();
        append_chunk(&mut out, b"IEND", &[]);
        assert_eq!(&out[..8], b"\0\0\0\0IEND");
        assert_eq!(&out[8..], &[0xae, 0x42, 0x60, 0x82]);
    }
    #[test]
    fn preserves_straight_rgba_bytes() {
        let path = std::env::temp_dir().join(format!("vrperf-rgba-test-{}.png", std::process::id()));
        save_rgba_png(&path, &[8, 10, 14, 128, 255, 0, 50, 255], 2, 1).unwrap();
        let data = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert!(data.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert_eq!(&data[12..16], b"IHDR");
        assert_eq!(data[25], 6); // RGBA colour type.
        assert!(data.windows(8).any(|chunk| chunk == [8, 10, 14, 128, 255, 0, 50, 255]));
    }
}
