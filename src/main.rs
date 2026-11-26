// SPDX-License-Identifier: GPL-3.0-only
use std::{
    env, fs,
    io::{self, BufReader},
    process::ExitCode,
};

fn main() -> ExitCode {
    let paths: Vec<_> = env::args_os().skip(1).collect();
    if paths.is_empty() {
        eprintln!("Usage: oot-frame-check <frame.zs> [frame.zs ...]\nChecks framing only. It does not decompress or verify checksums or game versions.");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for path in paths {
        let result = fs::metadata(&path).and_then(|metadata| {
            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "expected a regular file",
                ));
            }
            let file = fs::File::open(&path)?;
            oot_frame_check::check_frame(&mut BufReader::new(file))
        });
        match result {
            Ok(info) => println!(
                "{}: framing OK ({} blocks, checksum {})",
                path.to_string_lossy(),
                info.blocks,
                if info.has_checksum {
                    "present, not verified"
                } else {
                    "absent"
                }
            ),
            Err(error) => {
                eprintln!("{}: {error}", path.to_string_lossy());
                failed = true;
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
