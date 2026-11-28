use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
const FIXTURE: &str = include_str!("../fixtures/synthetic.c");
#[cfg(not(windows))]
const EXPECTED_OUTPUT: &str = "synthetic-build-ok\n";
#[cfg(windows)]
const EXPECTED_OUTPUT: &str = "synthetic-build-ok\r\n";

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
/// Use a trusted compiler and a trusted, private output root. The compiler is
/// invoked directly with fixed arguments in staging. It inherits the caller's
/// environment and can run other tools.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub compiler: PathBuf,
    pub output_root: PathBuf,
}

/// Compile and execute the bundled fixture, then atomically publish a new generation.
///
/// A running compiler is not interrupted. Cancellation kills and reaps the direct
/// synthetic executable child; it does not manage any descendants.
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
    let root = config.output_root.canonicalize()?;
    let (staging, generation) = create_staging_dir(&root)?;
    let outcome = build_in_staging(config, &staging, &mut cancelled)
        .and_then(|()| fs::rename(&staging, &generation).map_err(BuildError::Publication));
    if outcome.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    outcome.map(|()| generation)
}

fn build_in_staging<F>(
    config: &BuildConfig,
    staging: &Path,
    cancelled: &mut F,
) -> Result<(), BuildError>
where
    F: FnMut() -> bool,
{
    let source = "fixture.c";
    let executable = staging.join(binary_name());
    fs::write(staging.join(source), FIXTURE)?;
    check_cancelled(cancelled)?;

    let compile = Command::new(&config.compiler)
        .current_dir(staging)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-O2")
        .arg(source)
        .arg("-o")
        .arg(binary_name())
        .output()?;
    check_cancelled(cancelled)?;
    if !compile.status.success() {
        return Err(BuildError::CompilerFailed {
            status: compile.status.code(),
            stderr: String::from_utf8_lossy(&compile.stderr).into_owned(),
        });
    }

    let run = run_probe(&executable, staging, cancelled)?;
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

fn run_probe<F>(
    executable: &Path,
    staging: &Path,
    cancelled: &mut F,
) -> Result<std::process::Output, BuildError>
where
    F: FnMut() -> bool,
{
    let capture_id = next_id();
    let stdout_path = staging.join(format!("stdout-{capture_id}"));
    let stderr_path = staging.join(format!("stderr-{capture_id}"));
    let stdout = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stdout_path)?;
    let stderr = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&stderr_path)?;
    let mut child = RunningProbe(
        Command::new(executable)
            .current_dir(staging)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()?,
    );
    let status = loop {
        if let Some(status) = child.0.try_wait()? {
            break status;
        }
        if cancelled() {
            child.terminate()?;
            return Err(BuildError::Cancelled);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    check_cancelled(cancelled)?;
    let output = std::process::Output {
        status,
        stdout: fs::read(&stdout_path)?,
        stderr: fs::read(&stderr_path)?,
    };
    fs::remove_file(stdout_path)?;
    fs::remove_file(stderr_path)?;
    Ok(output)
}

struct RunningProbe(Child);

impl RunningProbe {
    fn terminate(&mut self) -> io::Result<()> {
        if let Err(error) = self.0.kill() {
            // On Windows an exit racing with kill can return an error.
            return match self.0.try_wait()? {
                Some(_) => Ok(()),
                None => Err(error),
            };
        }
        self.0.wait()?;
        Ok(())
    }
}

impl Drop for RunningProbe {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.terminate();
        }
    }
}

fn check_cancelled<F: FnMut() -> bool>(cancelled: &mut F) -> Result<(), BuildError> {
    if cancelled() {
        Err(BuildError::Cancelled)
    } else {
        Ok(())
    }
}

fn create_staging_dir(root: &Path) -> Result<(PathBuf, PathBuf), BuildError> {
    for _ in 0..100 {
        let id = next_id();
        let staging = root.join(format!("staging-{id}"));
        let generation = root.join(format!("generation-{id}"));
        match create_private_dir(&staging) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }

        // Holding the staging name reserves this ID among probe instances.
        // A previous process may already have promoted the same ID.
        match fs::symlink_metadata(&generation) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok((staging, generation));
            }
            Ok(_) => fs::remove_dir(&staging)?,
            Err(error) => {
                let _ = fs::remove_dir(&staging);
                return Err(BuildError::Publication(error));
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not reserve a unique build generation",
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

fn next_id() -> String {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{nonce}-{id}", std::process::id())
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

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("oot-build-test-{}", next_id()));
            create_private_dir(&path).unwrap();
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
            compiler: temp.0.join("unused-compiler"),
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
