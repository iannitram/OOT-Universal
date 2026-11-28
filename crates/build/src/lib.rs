use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);
const FIXTURE: &str = include_str!("../fixtures/synthetic.c");
pub const SYNTHETIC_FIXTURE_ID: &str = "oot-build-probe.synthetic-c.v1";
const MANIFEST_NAME: &str = "artifact.manifest";
const MAX_MANIFEST_BYTES: usize = 256;
const MAX_EXECUTABLE_BYTES: u64 = 64 * 1024 * 1024;
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

/// A strictly parsed record for a successfully published synthetic executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub format_version: u32,
    pub fixture_id: String,
    pub executable_size: u64,
    pub sha256: String,
}

/// Failure while validating a generation's bounded artifact record and executable.
#[derive(Debug)]
pub enum VerifyError {
    Io(io::Error),
    MetadataTooLarge,
    MalformedMetadata(&'static str),
    FixtureMismatch,
    SizeMismatch { expected: u64, actual: u64 },
    DigestMismatch,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "artifact verification I/O failed: {error}"),
            Self::MetadataTooLarge => write!(f, "artifact metadata exceeds the size limit"),
            Self::MalformedMetadata(reason) => write!(f, "malformed artifact metadata: {reason}"),
            Self::FixtureMismatch => write!(f, "artifact fixture identifier does not match"),
            Self::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "artifact size mismatch: expected {expected}, got {actual}"
                )
            }
            Self::DigestMismatch => write!(f, "artifact SHA-256 does not match metadata"),
        }
    }
}

impl std::error::Error for VerifyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for VerifyError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Verify a published generation using its strict, bounded metadata record.
///
/// The verifier accepts only format version 1 and the bundled synthetic fixture
/// identifier. It expects a trusted directory without concurrent changes and
/// regular manifest and executable files. Metadata is capped at 256 bytes and
/// executable hashing at 64 MiB.
pub fn verify_generation(generation: &Path) -> Result<ArtifactRecord, VerifyError> {
    let manifest = open_regular_file(&generation.join(MANIFEST_NAME))?;
    let metadata = manifest.metadata()?;
    if metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(VerifyError::MetadataTooLarge);
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    manifest
        .take((MAX_MANIFEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(VerifyError::MetadataTooLarge);
    }
    let record = parse_manifest(&bytes)?;
    if record.fixture_id != SYNTHETIC_FIXTURE_ID {
        return Err(VerifyError::FixtureMismatch);
    }

    let file = open_regular_file(&generation.join(binary_name()))?;
    let actual_size = file.metadata()?.len();
    if actual_size > MAX_EXECUTABLE_BYTES {
        return Err(VerifyError::MalformedMetadata(
            "executable exceeds the verification limit",
        ));
    }
    if record.executable_size != actual_size {
        return Err(VerifyError::SizeMismatch {
            expected: record.executable_size,
            actual: actual_size,
        });
    }
    let (read_size, digest) = hash_bounded(file)?;
    if read_size != record.executable_size {
        return Err(VerifyError::SizeMismatch {
            expected: record.executable_size,
            actual: read_size,
        });
    }
    if encode_hex(&digest) != record.sha256 {
        return Err(VerifyError::DigestMismatch);
    }
    Ok(record)
}

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
        .and_then(|()| check_cancelled(&mut cancelled))
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

    require_regular_file(&fs::symlink_metadata(&executable)?)?;
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
    write_manifest(staging, &executable)?;
    Ok(())
}

fn write_manifest(staging: &Path, executable: &Path) -> Result<(), BuildError> {
    use std::io::Write;

    let file = open_regular_file(executable).map_err(BuildError::Publication)?;
    let (size, digest) = hash_bounded(file).map_err(BuildError::Publication)?;
    let manifest = format!(
        "format_version=1\nfixture_id={SYNTHETIC_FIXTURE_ID}\nexecutable_size={size}\nsha256={}\n",
        encode_hex(&digest)
    );
    fs::File::create_new(staging.join(MANIFEST_NAME))
        .and_then(|mut file| file.write_all(manifest.as_bytes()))
        .map_err(BuildError::Publication)
}

fn open_regular_file(path: &Path) -> io::Result<fs::File> {
    require_regular_file(&fs::symlink_metadata(path)?)?;
    let file = fs::File::open(path)?;
    require_regular_file(&file.metadata()?)?;
    Ok(file)
}

fn require_regular_file(metadata: &fs::Metadata) -> io::Result<()> {
    if metadata.is_file() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "artifact entry is not a regular file",
    ))
}

fn hash_bounded(file: fs::File) -> io::Result<(u64, [u8; 32])> {
    let mut file = file.take(MAX_EXECUTABLE_BYTES + 1);
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        if size > MAX_EXECUTABLE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "executable exceeds the verification size limit",
            ));
        }
        hasher.update(&buffer[..read]);
    }
    Ok((size, hasher.finalize().into()))
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn parse_manifest(bytes: &[u8]) -> Result<ArtifactRecord, VerifyError> {
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(VerifyError::MetadataTooLarge);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| VerifyError::MalformedMetadata("metadata is not UTF-8"))?;
    let mut lines = text.split_terminator('\n');
    let version_line = lines
        .next()
        .ok_or(VerifyError::MalformedMetadata("missing version"))?;
    let fixture_line = lines
        .next()
        .ok_or(VerifyError::MalformedMetadata("missing fixture identifier"))?;
    let size_line = lines
        .next()
        .ok_or(VerifyError::MalformedMetadata("missing executable size"))?;
    let digest_line = lines
        .next()
        .ok_or(VerifyError::MalformedMetadata("missing SHA-256"))?;
    if lines.next().is_some() || !text.ends_with('\n') {
        return Err(VerifyError::MalformedMetadata(
            "unexpected lines or missing final newline",
        ));
    }
    if version_line != "format_version=1" {
        return Err(VerifyError::MalformedMetadata("unsupported version"));
    }
    let fixture_id =
        fixture_line
            .strip_prefix("fixture_id=")
            .ok_or(VerifyError::MalformedMetadata(
                "invalid fixture identifier field",
            ))?;
    if fixture_id.is_empty() || fixture_id.contains('\r') || fixture_id.contains('\0') {
        return Err(VerifyError::MalformedMetadata("invalid fixture identifier"));
    }
    let size_text =
        size_line
            .strip_prefix("executable_size=")
            .ok_or(VerifyError::MalformedMetadata(
                "invalid executable size field",
            ))?;
    if size_text.is_empty()
        || (size_text.len() > 1 && size_text.starts_with('0'))
        || !size_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(VerifyError::MalformedMetadata("invalid executable size"));
    }
    let executable_size = size_text
        .parse::<u64>()
        .map_err(|_| VerifyError::MalformedMetadata("executable size overflow"))?;
    if executable_size > MAX_EXECUTABLE_BYTES {
        return Err(VerifyError::MalformedMetadata(
            "executable exceeds the verification limit",
        ));
    }
    let sha256 = digest_line
        .strip_prefix("sha256=")
        .ok_or(VerifyError::MalformedMetadata("invalid SHA-256 field"))?;
    if sha256.len() != 64
        || !sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(VerifyError::MalformedMetadata("invalid SHA-256"));
    }
    Ok(ArtifactRecord {
        format_version: 1,
        fixture_id: fixture_id.to_owned(),
        executable_size,
        sha256: sha256.to_owned(),
    })
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

    #[test]
    fn manifest_parser_rejects_truncated_or_extra_metadata() {
        let valid = b"format_version=1\nfixture_id=oot-build-probe.synthetic-c.v1\nexecutable_size=1\nsha256=0000000000000000000000000000000000000000000000000000000000000000\n";
        assert!(matches!(
            parse_manifest(&valid[..valid.len() - 1]),
            Err(VerifyError::MalformedMetadata(_))
        ));
        let extra = [valid.as_slice(), b"extra\n".as_slice()].concat();
        assert!(matches!(
            parse_manifest(&extra),
            Err(VerifyError::MalformedMetadata(_))
        ));
        assert!(matches!(
            parse_manifest(&vec![b'x'; MAX_MANIFEST_BYTES + 1]),
            Err(VerifyError::MetadataTooLarge)
        ));
    }

    #[test]
    fn verifier_rejects_non_regular_manifest() {
        let temporary = TempDir::new();
        fs::create_dir(temporary.0.join(MANIFEST_NAME)).unwrap();
        assert!(matches!(
            verify_generation(&temporary.0),
            Err(VerifyError::Io(error)) if error.kind() == io::ErrorKind::InvalidData
        ));
    }

    #[cfg(unix)]
    #[test]
    fn verifier_rejects_symlinked_executable_even_when_bytes_match() {
        let temporary = TempDir::new();
        let source = temporary.0.join("source");
        let executable = temporary.0.join(binary_name());
        fs::write(&source, b"synthetic executable bytes").unwrap();
        std::os::unix::fs::symlink(&source, executable).unwrap();
        write_manifest(&temporary.0, &source).unwrap();
        assert!(matches!(
            verify_generation(&temporary.0),
            Err(VerifyError::Io(error)) if error.kind() == io::ErrorKind::InvalidData
        ));
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
