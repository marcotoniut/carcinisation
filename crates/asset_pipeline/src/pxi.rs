//! PXI — compact indexed atlas image format.
//!
//! Why PXI exists: runtime wants engine-native palette indices (`Vec<u8>`), not
//! PNG-decoded RGBA. PNG, including indexed PNG, still expands through RGBA in
//! Bevy/image and would force a reverse palette lookup we do not need.
//!
//! Why deflate: the packed 4bpp payload compresses well on atlas transparency
//! and padding, typically yielding ~6–8× smaller files for negligible load-time
//! cost. Compression is a payload concern only; the format stays "header + 4bpp
//! indices", with format byte `0` = raw and `1` = deflate.
//!
//! Polymorphism: intentionally avoided. The format space is small and closed
//! (raw 4bpp, deflate 4bpp, maybe future 8bpp), so a format byte plus `match`
//! is clearer than trait/generic codecs. That keeps behaviour explicit and
//! future extensions simple: add a new format value, then handle it directly.
//!
//! # Format layout
//!
//! ```text
//! Offset  Size  Field
//! ------  ----  -----
//!      0     4  magic: b"PXAI"
//!      4     1  version: 1
//!      5     1  format: 0 = raw 4bpp, 1 = deflate 4bpp
//!      6     2  width:  u16 LE
//!      8     2  height: u16 LE
//!     10     …  payload (raw or deflate-compressed packed nibbles)
//! ```
//!
//! The header is always 10 bytes. Pixel data is packed 2 per byte, high nibble
//! first, row-major. Index 0 = transparent, 1–15 = palette entries.

use anyhow::{Result, bail, ensure};
use flate2::{Compression, write::DeflateEncoder};
use std::io::Write;

/// File magic identifying a PXI file.
pub const MAGIC: [u8; 4] = *b"PXAI";

/// Current format version.
pub const VERSION: u8 = 1;

/// Format byte: raw (uncompressed) 4bpp nibble data.
pub const FORMAT_RAW_4BPP: u8 = 0;

/// Format byte: deflate-compressed 4bpp nibble data.
pub const FORMAT_DEFLATE_4BPP: u8 = 1;

/// Size of the fixed header in bytes.
pub const HEADER_SIZE: usize = 10;

/// Fixed compression level for deterministic output.
const DEFLATE_LEVEL: u32 = 6;

/// Validate and pack palette indices into 4bpp nibble buffer.
fn pack_nibbles(width: u32, height: u32, indices: &[u8]) -> Result<Vec<u8>> {
    let pixel_count = width as usize * height as usize;
    ensure!(
        indices.len() == pixel_count,
        "index count {} does not match {}×{} = {}",
        indices.len(),
        width,
        height,
        pixel_count,
    );
    ensure!(
        u16::try_from(width).is_ok() && u16::try_from(height).is_ok(),
        "dimensions {width}×{height} exceed u16 range",
    );

    for (i, &idx) in indices.iter().enumerate() {
        if idx > 15 {
            bail!("palette index {idx} at pixel {i} exceeds 4-bit range (0..=15)",);
        }
    }

    let packed_len = pixel_count.div_ceil(2);
    let mut packed = Vec::with_capacity(packed_len);
    for pair in indices.chunks(2) {
        let hi = pair[0];
        let lo = if pair.len() == 2 { pair[1] } else { 0 };
        packed.push((hi << 4) | lo);
    }
    Ok(packed)
}

/// Write the 10-byte PXI header.
#[allow(clippy::cast_possible_truncation)] // dimensions validated by callers
fn write_header(buf: &mut Vec<u8>, format: u8, width: u32, height: u32) {
    buf.extend_from_slice(&MAGIC);
    buf.push(VERSION);
    buf.push(format);
    buf.extend_from_slice(&(width as u16).to_le_bytes());
    buf.extend_from_slice(&(height as u16).to_le_bytes());
}

/// Encode palette indices as raw (uncompressed) PXI.
///
/// # Errors
///
/// Returns an error if the indices length doesn't match width × height.
pub fn encode(width: u32, height: u32, indices: &[u8]) -> Result<Vec<u8>> {
    let packed = pack_nibbles(width, height, indices)?;
    let mut buf = Vec::with_capacity(HEADER_SIZE + packed.len());
    write_header(&mut buf, FORMAT_RAW_4BPP, width, height);
    buf.extend_from_slice(&packed);
    Ok(buf)
}

/// Encode palette indices as deflate-compressed PXI.
///
/// # Errors
///
/// Returns an error if the indices length doesn't match width × height
/// or if deflate compression fails.
pub fn encode_compressed(width: u32, height: u32, indices: &[u8]) -> Result<Vec<u8>> {
    let packed = pack_nibbles(width, height, indices)?;
    let mut buf = Vec::with_capacity(HEADER_SIZE + packed.len());
    write_header(&mut buf, FORMAT_DEFLATE_4BPP, width, height);

    let mut encoder = DeflateEncoder::new(&mut buf, Compression::new(DEFLATE_LEVEL));
    encoder.write_all(&packed)?;
    encoder.finish()?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_simple_2x2() {
        let indices = [1u8, 2, 3, 4];
        let data = encode(2, 2, &indices).unwrap();
        assert_eq!(&data[..4], b"PXAI");
        assert_eq!(data[4], VERSION);
        assert_eq!(data[5], FORMAT_RAW_4BPP);
        assert_eq!(u16::from_le_bytes([data[6], data[7]]), 2);
        assert_eq!(u16::from_le_bytes([data[8], data[9]]), 2);
        assert_eq!(data[10], 0x12);
        assert_eq!(data[11], 0x34);
        assert_eq!(data.len(), HEADER_SIZE + 2);
    }

    #[test]
    fn encode_odd_pixel_count_pads_low_nibble() {
        let indices = [5u8, 10, 15];
        let data = encode(3, 1, &indices).unwrap();
        assert_eq!(data[10], 0x5A);
        assert_eq!(data[11], 0xF0);
    }

    #[test]
    fn encode_rejects_index_above_15() {
        let err = encode(2, 1, &[0, 16]).unwrap_err();
        assert!(
            err.to_string().contains("exceeds 4-bit range"),
            "got: {err}"
        );
    }

    #[test]
    fn encode_rejects_wrong_length() {
        let err = encode(2, 2, &[0, 1, 2]).unwrap_err();
        assert!(err.to_string().contains("does not match"), "got: {err}");
    }

    #[test]
    fn encode_empty_image() {
        let data = encode(0, 0, &[]).unwrap();
        assert_eq!(data.len(), HEADER_SIZE);
    }

    #[test]
    fn encode_all_transparent() {
        let indices = vec![0u8; 100];
        let data = encode(10, 10, &indices).unwrap();
        assert!(data[HEADER_SIZE..].iter().all(|&b| b == 0));
    }

    #[test]
    fn encode_deterministic() {
        let indices = vec![3u8; 64];
        let a = encode(8, 8, &indices).unwrap();
        let b = encode(8, 8, &indices).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn encode_compressed_has_deflate_format_byte() {
        let indices = vec![0u8; 64];
        let data = encode_compressed(8, 8, &indices).unwrap();
        assert_eq!(&data[..4], b"PXAI");
        assert_eq!(data[5], FORMAT_DEFLATE_4BPP);
    }

    #[test]
    fn encode_compressed_smaller_than_raw() {
        // Atlas-like data: mostly transparent with some opaque regions.
        let mut indices = vec![0u8; 4096];
        for (i, pixel) in indices.iter_mut().enumerate().take(200) {
            *pixel = (i % 15 + 1) as u8;
        }
        let raw = encode(64, 64, &indices).unwrap();
        let compressed = encode_compressed(64, 64, &indices).unwrap();
        assert!(
            compressed.len() < raw.len(),
            "compressed {} should be < raw {}",
            compressed.len(),
            raw.len()
        );
    }

    #[test]
    fn encode_compressed_deterministic() {
        let indices: Vec<u8> = (0..256).map(|i| (i % 16) as u8).collect();
        let a = encode_compressed(16, 16, &indices).unwrap();
        let b = encode_compressed(16, 16, &indices).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn encode_compressed_rejects_invalid_indices() {
        let err = encode_compressed(1, 1, &[16]).unwrap_err();
        assert!(
            err.to_string().contains("exceeds 4-bit range"),
            "got: {err}"
        );
    }
}
