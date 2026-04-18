//! PXI — compact indexed atlas image loader.
//!
//! Decodes `.pxi` files into [`PxIndexedImage`] assets. Supports both raw
//! and deflate-compressed 4bpp payloads. See `asset_pipeline::pxi` for the
//! encoder and full format documentation.
//!
//! # Format layout (version 1)
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

use std::{
    error::Error,
    io::{Cursor, Read as _},
};

use bevy_asset::{AssetLoader, io::Reader};
use bevy_reflect::TypePath;
use flate2::bufread::DeflateDecoder;

use crate::image::PxImage;
use crate::prelude::*;

/// File magic identifying a PXI file.
const MAGIC: [u8; 4] = *b"PXAI";

/// Expected format version.
const VERSION: u8 = 1;

/// Format byte: raw (uncompressed) 4bpp nibble data.
const FORMAT_RAW_4BPP: u8 = 0;

/// Format byte: deflate-compressed 4bpp nibble data.
const FORMAT_DEFLATE_4BPP: u8 = 1;

/// Size of the fixed header in bytes.
const HEADER_SIZE: usize = 10;

/// A pre-indexed atlas image loaded from a `.pxi` file.
///
/// Contains palette indices ready for direct use by the rendering pipeline,
/// without any PNG decode or palette lookup.
#[derive(Asset, Clone, Reflect, Debug)]
pub(crate) struct PxIndexedImage {
    pub(crate) image: PxImage,
}

/// Asset loader for `.pxi` files.
#[derive(TypePath)]
pub(crate) struct PxiLoader;

impl AssetLoader for PxiLoader {
    type Asset = PxIndexedImage;
    type Settings = ();
    type Error = Box<dyn Error + Send + Sync>;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        (): &(),
        _load_context: &mut bevy_asset::LoadContext<'_>,
    ) -> Result<PxIndexedImage, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let (width, _height, indices) = decode(&bytes)?;
        Ok(PxIndexedImage {
            image: PxImage::new(indices, width as usize),
        })
    }

    fn extensions(&self) -> &[&str] {
        &["pxi"]
    }
}

/// Decode a PXI file into width, height, and palette indices.
///
/// Returns one `u8` per pixel in row-major order. Index 0 = transparent,
/// 1–15 = palette entries.
fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), Box<dyn Error + Send + Sync>> {
    if bytes.len() < HEADER_SIZE {
        return Err(format!(
            "PXI file too short: {} bytes, need at least {}",
            bytes.len(),
            HEADER_SIZE
        )
        .into());
    }

    if bytes[0..4] != MAGIC {
        return Err("PXI file has invalid magic (expected b\"PXAI\")".into());
    }

    let version = bytes[4];
    if version != VERSION {
        return Err(format!("PXI version {version} is not supported (expected {VERSION})").into());
    }

    let format = bytes[5];
    let width = u32::from(u16::from_le_bytes([bytes[6], bytes[7]]));
    let height = u32::from(u16::from_le_bytes([bytes[8], bytes[9]]));
    let pixel_count = (width * height) as usize;
    let expected_packed_len = pixel_count.div_ceil(2);
    let payload = &bytes[HEADER_SIZE..];

    let packed = match format {
        FORMAT_RAW_4BPP => {
            if payload.len() != expected_packed_len {
                return Err(format!(
                    "PXI raw payload size mismatch: {} bytes, expected {} for {}×{}",
                    payload.len(),
                    expected_packed_len,
                    width,
                    height,
                )
                .into());
            }
            payload[..expected_packed_len].to_vec()
        }
        FORMAT_DEFLATE_4BPP => {
            let mut inflated = Vec::with_capacity(expected_packed_len);
            let mut decoder = DeflateDecoder::new(Cursor::new(payload));
            decoder.read_to_end(&mut inflated)?;
            let consumed = decoder.into_inner().position() as usize;
            if consumed != payload.len() {
                return Err(format!(
                    "PXI deflate payload has trailing data: consumed {} of {} bytes",
                    consumed,
                    payload.len(),
                )
                .into());
            }
            if inflated.len() != expected_packed_len {
                return Err(format!(
                    "PXI inflated payload size mismatch: {} bytes, expected {} for {}×{}",
                    inflated.len(),
                    expected_packed_len,
                    width,
                    height,
                )
                .into());
            }
            inflated
        }
        _ => {
            return Err(format!(
                "PXI format {format} is not supported (expected 0 = raw or 1 = deflate)"
            )
            .into());
        }
    };

    let mut indices = Vec::with_capacity(pixel_count);
    for &byte in &packed[..expected_packed_len] {
        indices.push(byte >> 4);
        indices.push(byte & 0x0F);
    }
    indices.truncate(pixel_count);

    Ok((width, height, indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use asset_pipeline::pxi::{encode as encode_raw_pxi, encode_compressed as encode_deflate_pxi};
    use flate2::{Compression, write::DeflateEncoder};
    use std::io::Write;

    fn make_pxi_raw(width: u16, height: u16, packed: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + packed.len());
        buf.extend_from_slice(&MAGIC);
        buf.push(VERSION);
        buf.push(FORMAT_RAW_4BPP);
        buf.extend_from_slice(&width.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());
        buf.extend_from_slice(packed);
        buf
    }

    fn make_pxi_deflate(width: u16, height: u16, packed: &[u8]) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&MAGIC);
        buf.push(VERSION);
        buf.push(FORMAT_DEFLATE_4BPP);
        buf.extend_from_slice(&width.to_le_bytes());
        buf.extend_from_slice(&height.to_le_bytes());
        let mut encoder = DeflateEncoder::new(&mut buf, Compression::new(6));
        encoder.write_all(packed).unwrap();
        encoder.finish().unwrap();
        buf
    }

    // --- Raw format tests ---

    #[test]
    fn decode_raw_2x2() {
        let data = make_pxi_raw(2, 2, &[0x12, 0x34]);
        let (w, h, indices) = decode(&data).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(indices, vec![1, 2, 3, 4]);
    }

    #[test]
    fn decode_raw_odd_pixel_count() {
        let data = make_pxi_raw(3, 1, &[0x5A, 0xF0]);
        let (_, _, indices) = decode(&data).unwrap();
        assert_eq!(indices, vec![5, 10, 15]);
    }

    #[test]
    fn decode_raw_empty() {
        let data = make_pxi_raw(0, 0, &[]);
        let (w, h, indices) = decode(&data).unwrap();
        assert_eq!((w, h), (0, 0));
        assert!(indices.is_empty());
    }

    #[test]
    fn decode_raw_all_transparent() {
        let data = make_pxi_raw(4, 4, &[0x00; 8]);
        let (_, _, indices) = decode(&data).unwrap();
        assert!(indices.iter().all(|&i| i == 0));
        assert_eq!(indices.len(), 16);
    }

    #[test]
    fn decode_raw_short_payload() {
        let data = make_pxi_raw(4, 4, &[0x00]);
        let err = decode(&data).unwrap_err();
        assert!(
            err.to_string().contains("payload size mismatch"),
            "got: {err}"
        );
    }

    #[test]
    fn decode_raw_rejects_trailing_payload_bytes() {
        let data = make_pxi_raw(2, 2, &[0x12, 0x34, 0x56]);
        let err = decode(&data).unwrap_err();
        assert!(
            err.to_string().contains("payload size mismatch"),
            "got: {err}"
        );
    }

    // --- Deflate format tests ---

    #[test]
    fn decode_deflate_2x2() {
        let data = make_pxi_deflate(2, 2, &[0x12, 0x34]);
        let (w, h, indices) = decode(&data).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(indices, vec![1, 2, 3, 4]);
    }

    #[test]
    fn decode_deflate_all_transparent() {
        let data = make_pxi_deflate(8, 8, &[0x00; 32]);
        let (_, _, indices) = decode(&data).unwrap();
        assert!(indices.iter().all(|&i| i == 0));
        assert_eq!(indices.len(), 64);
    }

    #[test]
    fn decode_deflate_truncated_payload_errors() {
        // A valid header claiming 4×4 but with an empty payload after the header.
        let mut data = Vec::new();
        data.extend_from_slice(&MAGIC);
        data.push(VERSION);
        data.push(FORMAT_DEFLATE_4BPP);
        data.extend_from_slice(&4u16.to_le_bytes());
        data.extend_from_slice(&4u16.to_le_bytes());
        // No compressed data at all — deflate decoder should fail.
        let err = decode(&data).unwrap_err();
        assert!(!err.to_string().is_empty(), "should produce an error");
    }

    #[test]
    fn decode_deflate_rejects_oversized_inflated_payload() {
        let data = make_pxi_deflate(2, 2, &[0x12, 0x34, 0x56]);
        let err = decode(&data).unwrap_err();
        assert!(
            err.to_string().contains("payload size mismatch"),
            "got: {err}"
        );
    }

    #[test]
    fn decode_deflate_rejects_trailing_compressed_garbage() {
        let mut data = make_pxi_deflate(2, 2, &[0x12, 0x34]);
        data.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
        let err = decode(&data).unwrap_err();
        assert!(err.to_string().contains("trailing data"), "got: {err}");
    }

    #[test]
    fn raw_and_deflate_produce_identical_indices() {
        let packed = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
        let raw = make_pxi_raw(4, 4, &packed);
        let deflated = make_pxi_deflate(4, 4, &packed);

        let (_, _, raw_indices) = decode(&raw).unwrap();
        let (_, _, deflate_indices) = decode(&deflated).unwrap();
        assert_eq!(raw_indices, deflate_indices);
    }

    // --- Header validation ---

    #[test]
    fn decode_rejects_short_header() {
        let err = decode(&[0; 5]).unwrap_err();
        assert!(err.to_string().contains("too short"), "got: {err}");
    }

    #[test]
    fn decode_rejects_bad_magic() {
        let mut data = make_pxi_raw(1, 1, &[0x10]);
        data[0] = b'X';
        let err = decode(&data).unwrap_err();
        assert!(err.to_string().contains("invalid magic"), "got: {err}");
    }

    #[test]
    fn decode_rejects_unknown_version() {
        let mut data = make_pxi_raw(1, 1, &[0x10]);
        data[4] = 99;
        let err = decode(&data).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn decode_rejects_unknown_format() {
        let mut data = make_pxi_raw(1, 1, &[0x10]);
        data[5] = 99;
        let err = decode(&data).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    // --- Cross-format roundtrip ---

    #[test]
    fn roundtrip_raw() {
        let width = 5u16;
        let height = 3u16;
        let indices: Vec<u8> = (0..15).map(|i| i % 16).collect();

        let mut packed = Vec::new();
        for pair in indices.chunks(2) {
            let hi = pair[0];
            let lo = if pair.len() == 2 { pair[1] } else { 0 };
            packed.push((hi << 4) | lo);
        }
        let data = make_pxi_raw(width, height, &packed);

        let (w, h, decoded) = decode(&data).unwrap();
        assert_eq!((w, h), (u32::from(width), u32::from(height)));
        assert_eq!(decoded, indices);
    }

    #[test]
    fn roundtrip_deflate() {
        let width = 5u16;
        let height = 3u16;
        let indices: Vec<u8> = (0..15).map(|i| i % 16).collect();

        let mut packed = Vec::new();
        for pair in indices.chunks(2) {
            let hi = pair[0];
            let lo = if pair.len() == 2 { pair[1] } else { 0 };
            packed.push((hi << 4) | lo);
        }
        let data = make_pxi_deflate(width, height, &packed);

        let (w, h, decoded) = decode(&data).unwrap();
        assert_eq!((w, h), (u32::from(width), u32::from(height)));
        assert_eq!(decoded, indices);
    }

    #[test]
    fn exporter_and_runtime_roundtrip_raw() {
        let width = 7u32;
        let height = 3u32;
        let indices: Vec<u8> = (0..width * height).map(|i| (i % 16) as u8).collect();

        let bytes = encode_raw_pxi(width, height, &indices).expect("exporter should encode raw");
        let (decoded_width, decoded_height, decoded_indices) =
            decode(&bytes).expect("runtime should decode raw");

        assert_eq!((decoded_width, decoded_height), (width, height));
        assert_eq!(decoded_indices, indices);
    }

    #[test]
    fn exporter_and_runtime_roundtrip_deflate_is_deterministic() {
        let width = 9u32;
        let height = 5u32;
        let indices: Vec<u8> = (0..width * height)
            .map(|i| if i % 7 == 0 { 0 } else { (i % 16) as u8 })
            .collect();

        let first =
            encode_deflate_pxi(width, height, &indices).expect("exporter should encode deflate");
        let second =
            encode_deflate_pxi(width, height, &indices).expect("exporter should encode deflate");
        assert_eq!(first, second, "deflate output should be deterministic");

        let (decoded_width, decoded_height, decoded_indices) =
            decode(&first).expect("runtime should decode deflate");
        assert_eq!((decoded_width, decoded_height), (width, height));
        assert_eq!(decoded_indices, indices);
    }
}
