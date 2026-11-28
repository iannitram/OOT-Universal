//! Bounded, synthetic-only decoder for one Zstandard frame.
//!
//! The frame checker first proves the input contains exactly one structurally
//! complete standard frame. This module then decodes it with explicit memory
//! limits and verifies the optional RFC 8878 content checksum.

use std::io::{self, Cursor, Read};

/// Explicit resource limits for synthetic frame decoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeLimits {
    /// Maximum encoded frame size accepted before parsing.
    pub compressed_bytes: usize,
    /// Maximum window accepted by ruzstd.
    pub window_bytes: u64,
    /// Maximum decoded output retained by this API.
    pub output_bytes: usize,
}

/// Small limits intended for authored fixtures, not real game assets.
pub const SYNTHETIC_LIMITS: DecodeLimits = DecodeLimits {
    compressed_bytes: 64 * 1024,
    window_bytes: 128 * 1024,
    output_bytes: 1024 * 1024,
};

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Decode one complete standard frame under caller-supplied resource limits.
///
/// Frames carrying a dictionary ID field are rejected. A declared content size
/// is checked against the output cap before decompression and must match the
/// number of bytes actually produced. Frames with checksums are
/// checked against ruzstd's XXH64 digest truncated to the RFC-required low 32
/// bits. Trailing bytes and concatenated frames are rejected by `check_frame`.
pub fn decode_frame(input: &[u8], limits: DecodeLimits) -> io::Result<Vec<u8>> {
    if input.len() > limits.compressed_bytes {
        return Err(invalid("compressed frame exceeds configured limit"));
    }

    let info = oot_frame_check::check_frame(&mut Cursor::new(input))?;
    if info.dictionary_id.is_some() {
        return Err(invalid("dictionary frames are not supported"));
    }
    if info
        .content_size
        .is_some_and(|n| n > limits.output_bytes as u64)
    {
        return Err(invalid("declared content size exceeds configured limit"));
    }

    let mut decoder =
        ruzstd::decoding::StreamingDecoder::new_with_max_window_size(input, limits.window_bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    // Collect at most one byte past the limit, without retaining it in the
    // result. The decoder processes whole blocks in its scratch buffer.
    let mut output = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let allowed = limits.output_bytes.saturating_sub(output.len());
        let request = chunk.len().min(allowed.saturating_add(1));
        let read = decoder
            .read(&mut chunk[..request])
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if read == 0 {
            break;
        }
        if read > allowed {
            return Err(invalid("decoded output exceeds configured limit"));
        }
        output.extend_from_slice(&chunk[..read]);
    }

    if info.content_size.is_some_and(|n| n != output.len() as u64) {
        return Err(invalid("decoded size does not match declared content size"));
    }

    let frame_decoder = decoder.into_frame_decoder();
    if info.has_checksum {
        let expected = frame_decoder
            .get_checksum_from_data()
            .ok_or_else(|| invalid("frame checksum was not decoded"))?;
        let actual = frame_decoder
            .get_calculated_checksum()
            .ok_or_else(|| invalid("frame checksum could not be calculated"))?;
        if actual != expected {
            return Err(invalid("Zstandard content checksum mismatch"));
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruzstd::encoding::{compress_to_vec, CompressionLevel};

    fn limits() -> DecodeLimits {
        DecodeLimits {
            compressed_bytes: 4096,
            window_bytes: 128 * 1024,
            output_bytes: 4096,
        }
    }

    fn compressed(data: &[u8]) -> Vec<u8> {
        compress_to_vec(data, CompressionLevel::Fastest)
    }

    #[test]
    fn decodes_compressed_block_and_bounds_output_without_declared_size() {
        let original = b"authored synthetic frame with repeating data\n".repeat(64);
        let frame = compressed(&original);
        let info = oot_frame_check::check_frame(&mut Cursor::new(&frame)).unwrap();
        assert_eq!(info.content_size, None);
        assert!(info.has_checksum);
        // The descriptor supplies only a window byte, so the block starts at offset 6.
        assert_eq!((frame[6] >> 1) & 3, 2, "expected a compressed block");
        assert!(frame.len() < original.len());

        let mut bounded = limits();
        bounded.output_bytes = original.len();
        assert_eq!(decode_frame(&frame, bounded).unwrap(), original);
        bounded.output_bytes -= 1;
        let error = decode_frame(&frame, bounded).unwrap_err();
        assert_eq!(error.to_string(), "decoded output exceeds configured limit");
    }

    #[test]
    fn decodes_tiny_raw_fixture() {
        // Single-segment frame, content-size 3, one final raw block "abc".
        let frame = [
            0x28, 0xb5, 0x2f, 0xfd, 0x20, 3, 0x19, 0, 0, b'a', b'b', b'c',
        ];
        assert_eq!(decode_frame(&frame, limits()).unwrap(), b"abc");
    }

    #[test]
    fn verifies_standard_checksum() {
        // XXH64("abc", 0) is 0x44bc2cf5ad770999. The frame stores its low 32 bits.
        let mut frame = [
            0x28, 0xb5, 0x2f, 0xfd, 0x24, 3, 0x19, 0, 0, b'a', b'b', b'c', 0x99, 0x09, 0x77, 0xad,
        ];
        assert_eq!(decode_frame(&frame, limits()).unwrap(), b"abc");
        *frame.last_mut().unwrap() ^= 1;
        let error = decode_frame(&frame, limits()).unwrap_err();
        assert_eq!(error.to_string(), "Zstandard content checksum mismatch");
    }

    #[test]
    fn rejects_window_above_configured_limit() {
        // Non-single-segment descriptor with a 16 KiB window.
        let frame = [
            0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x20, 0x19, 0, 0, b'a', b'b', b'c',
        ];
        let mut bounded = limits();
        bounded.window_bytes = 8 * 1024;
        let error = decode_frame(&frame, bounded).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error
            .get_ref()
            .unwrap()
            .is::<ruzstd::decoding::errors::FrameDecoderError>());
    }

    #[test]
    fn checks_declared_output_size_before_and_after_decoding() {
        // Same valid raw block but header declares four bytes for three output bytes.
        let mismatch = [
            0x28, 0xb5, 0x2f, 0xfd, 0x20, 4, 0x19, 0, 0, b'a', b'b', b'c',
        ];
        let mut bounded = limits();
        bounded.output_bytes = 3;
        let error = decode_frame(&mismatch, bounded).unwrap_err();
        assert_eq!(
            error.to_string(),
            "declared content size exceeds configured limit"
        );
        let error = decode_frame(&mismatch, limits()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "decoded size does not match declared content size"
        );
    }

    #[test]
    fn rejects_oversized_compressed_input_before_parse() {
        let mut bounded = limits();
        bounded.compressed_bytes = 1;
        assert!(decode_frame(&[0; 2], bounded).is_err());
    }

    #[test]
    fn rejects_malformed_compressed_payload_after_structural_preflight() {
        // A final compressed block containing only an empty literals header.
        let frame = [0x28, 0xb5, 0x2f, 0xfd, 0, 0, 0x0d, 0, 0, 0];
        assert!(oot_frame_check::check_frame(&mut Cursor::new(frame)).is_ok());
        let error = decode_frame(&frame, limits()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.get_ref().is_some());
    }

    #[test]
    fn rejects_dictionaries_and_trailing_frames() {
        // One-byte dictionary ID 1, content-size 3, one final raw block.
        let dict_frame = [
            0x28, 0xb5, 0x2f, 0xfd, 0x21, 1, 3, 0x19, 0, 0, b'a', b'b', b'c',
        ];
        let error = decode_frame(&dict_frame, limits()).unwrap_err();
        assert_eq!(error.to_string(), "dictionary frames are not supported");

        let mut concatenated = compressed(b"one");
        let truncated = &concatenated[..concatenated.len() - 1];
        assert_eq!(
            decode_frame(truncated, limits()).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        concatenated.extend_from_slice(&compressed(b"two"));
        let error = decode_frame(&concatenated, limits()).unwrap_err();
        assert_eq!(error.to_string(), "trailing bytes after frame");
    }
}
