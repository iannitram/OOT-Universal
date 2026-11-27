use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
const FIXTURE: &str = include_str!("../fixtures/synthetic.c");
const EXPECTED_OUTPUT: &str = "synthetic-build-ok\n";

#[derive(Debug)]
pub enum BuildError {
    Cancelled,
    Io(io::Error),
    CompilerFailed { status: Option<i32>, stderr: String },
    ProbeFailed { status: Option<i32>, stderr: String },
    UnexpectedOutput(String),
    Publication(io::Error),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "build cancelled"),
            Self::Io(error) => write!(f, "build I/O failed: {error}"),
            Self::CompilerFailed { status, stderr } => {
                write!(f, "compiler failed ({status:?}): {stderr}")
            }
            Self::ProbeFailed { status, stderr } => {
                write!(f, "synthetic executable failed ({status:?}): {stderr}")
            }
            Self::UnexpectedOutput(output) => write!(f, "unexpected probe output: {output:?}"),
            Self::Publication(error) => write!(f, "could not publish build generation: {error}"),
        }
    }
}

impl std::error::Error for BuildError {}

impl From<io::Error> for BuildError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Configuration for the bundled, synthetic C build probe.
///
/// The compiler is invoked directly with fixed arguments and is never interpreted by a shell.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub compiler: PathBuf,
    pub output_root: PathBuf,
}

/// Compile and execute the bundled fixture, then atomically publish a new generation.
///
/// Cancellation is checked between operations. A running compiler or executable is not
/// interrupted; the predicate is checked again when that child exits.
pub fn build_synthetic<F>(config: &BuildConfig, mut cancelled: F) -> Result<PathBuf, BuildError>
where
    F: FnMut() -> bool,
{
    check_cancelled(&mut cancelled)?;
    if !config.compiler.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "compiler must be an explicit absolute path",
        )
        .into());
    }
    fs::create_dir_all(&config.output_root)?;
    let staging = create_staging_dir(&config.output_root)?;
    let outcome = build_in_staging(config, &staging, &mut cancelled);
    match outcome {
        Ok(()) => {
            let generation = match new_generation_path(&config.output_root) {
                Ok(path) => path,
                Err(error) => {
                    let _ = fs::remove_dir_all(&staging);
                    return Err(error);
                }
            };
            match fs::rename(&staging, &generation) {
                Ok(()) => Ok(generation),
                Err(error) => {
                    let _ = fs::remove_dir_all(&staging);
                    Err(BuildError::Publication(error))
                }
            }
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            Err(error)
        }
    }
}

fn build_in_staging<F>(
    config: &BuildConfig,
    staging: &Path,
    cancelled: &mut F,
) -> Result<(), BuildError>
where
    F: FnMut() -> bool,
{
    let source = staging.join("fixture.c");
    let executable = staging.join(binary_name());
    fs::write(&source, FIXTURE)?;
    check_cancelled(cancelled)?;

    let compile = Command::new(&config.compiler)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-O2")
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()?;
    check_cancelled(cancelled)?;
    if !compile.status.success() {
        return Err(BuildError::CompilerFailed {
            status: compile.status.code(),
            stderr: String::from_utf8_lossy(&compile.stderr).into_owned(),
        });
    }

    let run = Command::new(&executable).output()?;
    check_cancelled(cancelled)?;
    if !run.status.success() {
        return Err(BuildError::ProbeFailed {
            status: run.status.code(),
            stderr: String::from_utf8_lossy(&run.stderr).into_owned(),
        });
    }
    let output = String::from_utf8_lossy(&run.stdout).into_owned();
    if output != EXPECTED_OUTPUT {
        return Err(BuildError::UnexpectedOutput(output));
    }
    Ok(())
}

fn check_cancelled<F: FnMut() -> bool>(cancelled: &mut F) -> Result<(), BuildError> {
    if cancelled() {
        Err(BuildError::Cancelled)
    } else {
        Ok(())
    }
}

fn create_staging_dir(root: &Path) -> Result<PathBuf, BuildError> {
    for _ in 0..100 {
        let path = unique_path(root, "staging");
        let created = create_private_dir(&path);
        match created {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate unique staging directory",
    )
    .into())
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> io::Result<()> {
    fs::create_dir(path)
}

fn unique_path(root: &Path, prefix: &str) -> PathBuf {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    root.join(format!("{prefix}-{}-{nonce}-{id}", std::process::id()))
}

fn new_generation_path(root: &Path) -> Result<PathBuf, BuildError> {
    for _ in 0..100 {
        let candidate = unique_path(root, "generation");
        match fs::symlink_metadata(&candidate) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique generation path",
    )
    .into())
}

fn binary_name() -> &'static std::ffi::OsStr {
    #[cfg(windows)]
    {
        std::ffi::OsStr::new("probe.exe")
    }
    #[cfg(not(windows))]
    {
        std::ffi::OsStr::new("probe")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let n = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("oot-build-test-{}-{n}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn prior_generation(root: &Path) -> PathBuf {
        let previous = root.join("generation-existing");
        fs::create_dir(&previous).unwrap();
        fs::write(previous.join("keep.txt"), "previous-good-output").unwrap();
        previous
    }

    fn generation_count(root: &Path) -> usize {
        fs::read_dir(root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("generation-")
            })
            .count()
    }

    #[test]
    fn cancellation_keeps_existing_generation_and_cleans_staging() {
        let temp = TempDir::new();
        let previous = prior_generation(&temp.0);
        let config = BuildConfig {
            compiler: PathBuf::from("/usr/bin/cc"),
            output_root: temp.0.clone(),
        };
        let mut checks = 0;
        let result = build_synthetic(&config, || {
            checks += 1;
            checks == 2
        });
        assert!(matches!(result, Err(BuildError::Cancelled)));
        assert_eq!(
            fs::read_to_string(previous.join("keep.txt")).unwrap(),
            "previous-good-output"
        );
        assert_eq!(generation_count(&temp.0), 1);
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 1);
    }

    #[test]
    fn compiler_failure_keeps_existing_generation_and_cleans_staging() {
        let temp = TempDir::new();
        let previous = prior_generation(&temp.0);
        let config = BuildConfig {
            compiler: temp.0.join("missing-compiler"),
            output_root: temp.0.clone(),
        };
        assert!(matches!(
            build_synthetic(&config, || false),
            Err(BuildError::Io(_))
        ));
        assert_eq!(
            fs::read_to_string(previous.join("keep.txt")).unwrap(),
            "previous-good-output"
        );
        assert_eq!(generation_count(&temp.0), 1);
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 1);
    }
}
