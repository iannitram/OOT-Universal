use oot_build_probe::{build_synthetic, BuildConfig, BuildError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("oot-native-build-{nonce}-{id}"));
        fs::create_dir(&path).expect("create temporary test directory");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(windows)]
const PROBE_NAME: &str = "probe.exe";
#[cfg(not(windows))]
const PROBE_NAME: &str = "probe";

fn run_probe(path: &Path) -> std::process::Output {
    Command::new(path).output().expect("run published probe")
}

fn assert_probe_output(path: &Path) {
    let output = run_probe(path);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        if cfg!(windows) {
            "synthetic-build-ok\r\n"
        } else {
            "synthetic-build-ok\n"
        }
    );
}

fn assert_no_staging(root: &Path) {
    for entry in fs::read_dir(root).expect("read output root") {
        let name = entry.unwrap().file_name();
        assert!(!name.to_string_lossy().starts_with("staging-"));
    }
}

#[test]
#[ignore = "requires OOT_BUILD_TEST_COMPILER to name an absolute native compiler executable"]
fn native_success_compiler_failure_and_post_compile_cancellation() {
    let compiler = std::env::var_os("OOT_BUILD_TEST_COMPILER")
        .expect("set OOT_BUILD_TEST_COMPILER to an absolute native compiler path");
    let compiler = PathBuf::from(compiler);
    assert!(compiler.is_absolute(), "compiler path must be absolute");
    assert!(compiler.is_file(), "compiler path must exist");

    let temp = TempDir::new();
    let output_root = temp.0.join("output");
    fs::create_dir(&output_root).expect("create output root");
    let config = BuildConfig {
        compiler: compiler.clone(),
        output_root: output_root.clone(),
    };

    // A real compile, execution, and publication establishes the good generation.
    let previous = build_synthetic(&config, || false).expect("native build should succeed");
    let previous_executable = previous.join(PROBE_NAME);
    assert_probe_output(&previous_executable);

    // Compile a tiny native compiler stand-in. It receives the build's normal
    // compiler arguments, writes diagnostic text, and exits unsuccessfully.
    let failing_source = temp.0.join("failing_compiler.c");
    let failing_compiler = temp.0.join(if cfg!(windows) {
        "failing_compiler.exe"
    } else {
        "failing_compiler"
    });
    fs::write(
        &failing_source,
        "#include <stdio.h>\nint main(void) { fputs(\"synthetic compiler failure\\n\", stderr); return 23; }\n",
    )
    .expect("write compiler stand-in source");
    let standin_build = Command::new(&compiler)
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-O2")
        .arg(&failing_source)
        .arg("-o")
        .arg(&failing_compiler)
        .output()
        .expect("compile compiler stand-in");
    assert!(
        standin_build.status.success(),
        "stand-in compilation failed: {}",
        String::from_utf8_lossy(&standin_build.stderr)
    );
    let failed = build_synthetic(
        &BuildConfig {
            compiler: failing_compiler,
            output_root: output_root.clone(),
        },
        || false,
    );
    match failed {
        Err(BuildError::CompilerFailed { status, stderr }) => {
            assert_eq!(status, Some(23));
            assert!(stderr.contains("synthetic compiler failure"));
        }
        other => panic!("expected compiler failure, got {other:?}"),
    }
    assert_probe_output(&previous_executable);
    assert_eq!(
        fs::read_dir(&output_root).unwrap().count(),
        1,
        "only the prior generation should remain after compiler failure"
    );
    assert_no_staging(&output_root);

    // Cancellation is triggered only by the check after the real compiler has
    // returned. The staged executable is run in that check, proving compilation
    // completed; cancellation then cleans staging and keeps the prior generation.
    let cancelled = build_synthetic(&config, || {
        assert_probe_output(&previous_executable);
        let staging_executable = fs::read_dir(&output_root)
            .expect("read output root during cancellation check")
            .map(|entry| entry.expect("read output root entry").path())
            .find_map(|path| {
                let is_staging = path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("staging-"));
                if is_staging {
                    let executable = path.join(PROBE_NAME);
                    executable.exists().then_some(executable)
                } else {
                    None
                }
            });
        if let Some(staging_executable) = staging_executable {
            assert_probe_output(&staging_executable);
            true
        } else {
            false
        }
    });
    assert!(matches!(cancelled, Err(BuildError::Cancelled)));
    assert!(run_probe(&previous_executable).status.success());
    assert_eq!(
        fs::read_dir(&output_root).unwrap().count(),
        1,
        "only the prior generation should remain"
    );
    assert_no_staging(&output_root);
}
