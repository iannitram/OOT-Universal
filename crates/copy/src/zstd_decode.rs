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
/// Dictionary frames are rejected. If the frame declares its content size,
/// that size is checked against the output cap before decompression and must
/// match the number of bytes actually produced. Frames with checksums are
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
            .map_err(|e| invalid_box("invalid Zstandard frame", e))?;

    // Read at most one byte past the limit, so the result vector cannot grow
    // beyond the configured bound even for frames without a declared size.
    let mut output = Vec::with_capacity(
        info.content_size
            .unwrap_or(0)
            .min(limits.output_bytes as u64) as usize,
    );
    let mut chunk = [0u8; 8192];
    loop {
        let allowed = limits.output_bytes.saturating_sub(output.len());
        let request = chunk.len().min(allowed.saturating_add(1));
        let read = decoder
            .read(&mut chunk[..request])
            .map_err(|e| invalid_box("failed to decode Zstandard frame", e))?;
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

fn invalid_box(message: &'static str, _cause: impl std::fmt::Display) -> io::Error {
    invalid(message)
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
    fn decodes_real_compressed_fixture_and_checksum() {
        let original = b"authored synthetic frame with repeated repeated repeated data";
        let frame = compressed(original);
        assert_eq!(decode_frame(&frame, limits()).unwrap(), original);
        assert!(
            frame[4] & 4 != 0,
            "hash feature should emit checksummed frames"
        );
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
    fn rejects_incorrect_checksum() {
        let mut frame = compressed(b"checksum fixture");
        *frame.last_mut().unwrap() ^= 1;
        assert!(decode_frame(&frame, limits()).is_err());
    }

    #[test]
    fn rejects_window_above_configured_limit() {
        // Non-single-segment descriptor with a 16 KiB window.
        let frame = [
            0x28, 0xb5, 0x2f, 0xfd, 0x00, 0x20, 0x19, 0, 0, b'a', b'b', b'c',
        ];
        let mut bounded = limits();
        bounded.window_bytes = 8 * 1024;
        assert!(decode_frame(&frame, bounded).is_err());
    }

    #[test]
    fn rejects_oversized_output_and_declared_size_mismatch() {
        let frame = compressed(b"0123456789abcdef");
        let mut bounded = limits();
        bounded.output_bytes = 8;
        assert!(decode_frame(&frame, bounded).is_err());

        // Same valid raw block but header declares four bytes for three output bytes.
        let mismatch = [
            0x28, 0xb5, 0x2f, 0xfd, 0x20, 4, 0x19, 0, 0, b'a', b'b', b'c',
        ];
        assert!(decode_frame(&mismatch, limits()).is_err());
    }

    #[test]
    fn rejects_oversized_compressed_input_before_parse() {
        let mut bounded = limits();
        bounded.compressed_bytes = 1;
        assert!(decode_frame(&[0; 2], bounded).is_err());
    }

    #[test]
    fn rejects_dictionaries_and_trailing_frames() {
        let dict_frame = [0x28, 0xb5, 0x2f, 0xfd, 0x21, 1, 0, 0, 0, 0x19, 0, 0, b'a'];
        assert!(decode_frame(&dict_frame, limits()).is_err());

        let mut concatenated = compressed(b"one");
        concatenated.extend_from_slice(&compressed(b"two"));
        assert!(decode_frame(&concatenated, limits()).is_err());
    }
}
