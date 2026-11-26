// SPDX-License-Identifier: GPL-3.0-only
// Adapted to Rust on 2026-11-26 from ProjectReLink contributors' check()
// in pixl27/ProjectReLink, tools/dump_check.py, at commit
// 54af18cda249233a9a9aafcddf5dc317cb3fad42 (GPL-3.0).
use std::io::{self, Read, Seek, SeekFrom};

#[derive(Debug, PartialEq, Eq)]
pub struct FrameInfo {
    pub content_size: Option<u64>,
    pub dictionary_id: Option<u32>,
    pub has_checksum: bool,
    pub blocks: u64,
}

fn bytes<R: Read>(reader: &mut R, n: usize) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    reader.read_exact(&mut buf[..n]).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            io::Error::new(io::ErrorKind::UnexpectedEof, "frame is truncated")
        } else {
            error
        }
    })?;
    Ok(u64::from_le_bytes(buf))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Check framing from the start of a seekable input containing one standard frame.
/// The content size and dictionary ID are declared values, and the checksum flag
/// records presence only. The payload is skipped.
pub fn check_frame<R: Read + Seek>(reader: &mut R) -> io::Result<FrameInfo> {
    let len = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;
    if bytes(reader, 4)? != 0xfd2f_b528 {
        return Err(invalid("not a standard Zstandard frame"));
    }
    let descriptor = bytes(reader, 1)? as u8;
    if descriptor & 0x08 != 0 {
        return Err(invalid("reserved frame header bit is set"));
    }
    let single = descriptor & 0x20 != 0;
    let window_size = if single {
        None
    } else {
        let descriptor = bytes(reader, 1)?;
        let base = 1_u64 << (10 + (descriptor >> 3));
        Some(base + (base / 8) * (descriptor & 7))
    };
    let dict_len = [0, 1, 2, 4][(descriptor & 3) as usize];
    let dictionary_id = if dict_len == 0 {
        None
    } else {
        Some(bytes(reader, dict_len)? as u32)
    };
    let size_len = match descriptor >> 6 {
        0 if single => 1,
        0 => 0,
        1 => 2,
        2 => 4,
        _ => 8,
    };
    let content_size = if size_len == 0 {
        None
    } else {
        Some(bytes(reader, size_len)? + if size_len == 2 { 256 } else { 0 })
    };
    // RFC 8878 defines a single-segment frame's window as its content size.
    let block_limit = window_size
        .or(content_size)
        .unwrap_or_default()
        .min(128 * 1024);
    let mut info = FrameInfo {
        content_size,
        dictionary_id,
        has_checksum: descriptor & 4 != 0,
        blocks: 0,
    };
    loop {
        let header = bytes(reader, 3)?;
        let last = header & 1 != 0;
        let kind = (header >> 1) & 3;
        let size = header >> 3;
        if kind == 3 {
            return Err(invalid("reserved block type"));
        }
        if size > block_limit {
            return Err(invalid("block size exceeds the frame limit"));
        }
        let payload = if kind == 1 { 1 } else { size };
        let pos = reader.stream_position()?;
        let end = pos
            .checked_add(payload)
            .ok_or_else(|| invalid("block size overflow"))?;
        if end > len {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "block payload truncated",
            ));
        }
        reader.seek(SeekFrom::Start(end))?;
        info.blocks += 1;
        if last {
            break;
        }
    }
    if info.has_checksum {
        bytes(reader, 4)?;
    }
    if reader.stream_position()? != len {
        return Err(invalid("trailing bytes after frame"));
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn check(data: &[u8]) -> io::Result<FrameInfo> {
        check_frame(&mut Cursor::new(data))
    }
    #[test]
    fn single_frame_with_checksum_and_two_blocks() {
        // Header: single segment, 1-byte content size and checksum; raw "abc", then RLE.
        let data = [
            0x28, 0xb5, 0x2f, 0xfd, 0x24, 5, 24, 0, 0, b'a', b'b', b'c', 19, 0, 0, b'x', 0, 0, 0, 0,
        ];
        assert_eq!(
            check(&data).unwrap(),
            FrameInfo {
                content_size: Some(5),
                dictionary_id: None,
                has_checksum: true,
                blocks: 2
            }
        );
        for n in 0..data.len() {
            assert!(check(&data[..n]).is_err(), "accepted truncation at {n}");
        }
    }
    #[test]
    fn dictionary_and_extended_size() {
        // The unused descriptor bit is ignored. A raw last block contains 258 bytes.
        let mut data = vec![
            0x28, 0xb5, 0x2f, 0xfd, 0x53, 0, 1, 2, 3, 4, 2, 0, 0x11, 8, 0,
        ];
        data.resize(data.len() + 258, b'a');
        let info = check(&data).unwrap();
        assert_eq!(info.dictionary_id, Some(0x04030201));
        assert_eq!(info.content_size, Some(258));
    }
    #[test]
    fn rle_block_size_respects_window_and_format_limit() {
        for (header, limit) in [([0x20, 3], 3_u32), ([0, 1], 1152), ([0, 255], 131_072)] {
            for size in [limit, limit + 1] {
                let block_header = ((size << 3) | 3).to_le_bytes();
                let mut data = vec![0x28, 0xb5, 0x2f, 0xfd];
                data.extend_from_slice(&header);
                data.extend_from_slice(&block_header[..3]);
                data.push(b'x');
                assert_eq!(
                    check(&data).is_ok(),
                    size == limit,
                    "size {size}, limit {limit}"
                );
            }
        }
    }
    #[test]
    fn rejects_bad_magic_reserved_fields_and_trailing_bytes() {
        for data in [
            vec![0; 9],
            vec![0x28, 0xb5, 0x2f, 0xfd, 8],
            vec![0x28, 0xb5, 0x2f, 0xfd, 0x20, 0, 7, 0, 0],
            vec![0x28, 0xb5, 0x2f, 0xfd, 0x20, 0, 1, 0, 0, 0],
        ] {
            assert!(check(&data).is_err());
        }
    }
}
