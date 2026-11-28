//! Bounded settings file persistence.
//!
//! The caller selects the destination and is responsible for providing a trusted
//! directory. This module does not choose platform settings directories or impose
//! a path sandbox. Writes stage a sibling file and rename it over the destination;
//! the file contents are synced, but portable `std` APIs do not sync the parent
//! directory, so a power loss may still lose the directory entry update.

use crate::{decode, encode, CodecError, MAX_SETTINGS_BYTES};
use oot_contracts::Settings;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const CREATE_ATTEMPTS: usize = 128;

/// Failure while reading or writing a settings file.
#[derive(Debug)]
pub enum FileError {
    Io(io::Error),
    Codec(CodecError),
}

impl std::fmt::Display for FileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "settings file I/O error: {error}"),
            Self::Codec(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Codec(error) => Some(error),
        }
    }
}

impl From<io::Error> for FileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<CodecError> for FileError {
    fn from(error: CodecError) -> Self {
        Self::Codec(error)
    }
}

/// Reads at most `MAX_SETTINGS_BYTES + 1` bytes from a regular file, then
/// validates the settings JSON.
pub fn read(path: impl AsRef<Path>) -> Result<Settings, FileError> {
    let path = path.as_ref();
    require_regular_file(&fs::metadata(path)?)?;
    let file = File::open(path)?;
    require_regular_file(&file.metadata()?)?;
    let mut bytes = Vec::with_capacity(MAX_SETTINGS_BYTES + 1);
    file.take((MAX_SETTINGS_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    Ok(decode(&bytes)?)
}

/// Encodes settings and atomically replaces `destination` using a sibling staging file.
///
/// `destination` and its parent directory are caller-controlled trusted paths. The
/// staging file is created with `create_new`, synced, and closed before rename. If
/// staging or rename fails, the temporary file is cleaned up and no destination
/// removal fallback is attempted.
pub fn write_atomic(destination: impl AsRef<Path>, settings: &Settings) -> Result<(), FileError> {
    let destination = destination.as_ref();
    let encoded = encode(settings)?;
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let destination_name = destination.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name")
    })?;

    let (temporary_path, mut temporary) = create_temporary(parent, destination_name)?;
    let write_result = temporary
        .write_all(&encoded)
        .and_then(|()| temporary.sync_all());
    drop(temporary);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(FileError::Io(error));
    }

    if let Err(error) = fs::rename(&temporary_path, destination) {
        let _ = fs::remove_file(&temporary_path);
        return Err(FileError::Io(error));
    }
    Ok(())
}

fn require_regular_file(metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.is_file() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidInput,
        "settings path does not refer to a regular file",
    ))
}

fn create_temporary(
    parent: &Path,
    destination_name: &std::ffi::OsStr,
) -> io::Result<(PathBuf, File)> {
    for _ in 0..CREATE_ATTEMPTS {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary_name = format!(".oot-settings.{}.{}.tmp", std::process::id(), sequence);
        // A caller can choose a name resembling one of our staging files.
        // Use an ASCII case-insensitive comparison for case-insensitive volumes.
        if destination_name
            .to_string_lossy()
            .trim_end_matches(['.', ' '])
            .eq_ignore_ascii_case(&temporary_name)
        {
            continue;
        }
        let path = parent.join(temporary_name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique settings staging file",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static TEST_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "oot-settings-codec-{}-{}-{sequence}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn staged_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "tmp"))
            .collect()
    }

    #[test]
    fn saves_reloads_and_replaces_existing_file() {
        let directory = TestDir::new();
        let destination = directory.0.join("settings.json");
        let first = Settings::default();
        write_atomic(&destination, &first).unwrap();
        assert_eq!(read(&destination).unwrap(), first);

        let second = Settings::new(
            1,
            oot_contracts::Resolution::Explicit {
                width: std::num::NonZeroU32::new(1280).unwrap(),
                height: std::num::NonZeroU32::new(720).unwrap(),
            },
            oot_contracts::FrameCap::Fps30,
            oot_contracts::Upscaler::PlatformDefault,
            true,
        )
        .unwrap();
        write_atomic(&destination, &second).unwrap();
        assert_eq!(read(&destination).unwrap(), second);
        assert!(staged_files(&directory.0).is_empty());
    }

    #[test]
    fn concurrent_writers_use_distinct_staging_files() {
        let directory = TestDir::new();
        let workers: Vec<_> = (0..8)
            .map(|index| {
                let path = directory.0.join(format!("settings-{index}.json"));
                std::thread::spawn(move || {
                    let settings = Settings::default();
                    write_atomic(&path, &settings).unwrap();
                    assert_eq!(read(&path).unwrap(), settings);
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        assert!(staged_files(&directory.0).is_empty());
    }

    #[test]
    fn staging_name_does_not_grow_with_destination_name() {
        let directory = TestDir::new();
        let destination = directory.0.join(format!("{}.json", "s".repeat(180)));
        write_atomic(&destination, &Settings::default()).unwrap();
        assert_eq!(read(&destination).unwrap(), Settings::default());
        assert!(staged_files(&directory.0).is_empty());
    }

    #[test]
    fn rejects_malformed_and_oversized_files() {
        let directory = TestDir::new();
        let destination = directory.0.join("settings.json");
        fs::write(&destination, b"{").unwrap();
        assert!(matches!(
            read(&destination),
            Err(FileError::Codec(CodecError::Json(_)))
        ));
        fs::write(&destination, vec![b' '; MAX_SETTINGS_BYTES + 1]).unwrap();
        assert!(matches!(
            read(&destination),
            Err(FileError::Codec(CodecError::TooLarge { .. }))
        ));
    }

    #[test]
    fn rejects_non_regular_settings_path() {
        let directory = TestDir::new();
        assert!(matches!(
            read(&directory.0),
            Err(FileError::Io(error)) if error.kind() == io::ErrorKind::InvalidInput
        ));
    }

    #[test]
    fn failed_replacement_preserves_destination_and_cleans_temporary_file() {
        let directory = TestDir::new();
        let destination = directory.0.join("as-directory");
        fs::create_dir(&destination).unwrap();
        let child = destination.join("keep");
        fs::write(&child, b"prior data").unwrap();

        assert!(write_atomic(&destination, &Settings::default()).is_err());
        assert_eq!(fs::read(&child).unwrap(), b"prior data");
        assert_eq!(staged_files(&directory.0).len(), 0);
    }
}
