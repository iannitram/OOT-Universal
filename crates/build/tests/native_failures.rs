#[path = "native_failures/concurrent.rs"]
mod concurrent;

use oot_build_probe::{
    build_synthetic, verify_generation, BuildConfig, BuildError, VerifyError, SYNTHETIC_FIXTURE_ID,
};
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

fn native_compiler() -> PathBuf {
    let compiler = std::env::var_os("OOT_BUILD_TEST_COMPILER")
        .expect("set OOT_BUILD_TEST_COMPILER to an absolute native compiler path");
    let compiler = PathBuf::from(compiler);
    assert!(compiler.is_absolute(), "compiler path must be absolute");
    assert!(compiler.is_file(), "compiler path must exist");
    compiler
}

fn assert_probe_output(path: &Path) {
    let output = Command::new(path).output().expect("run probe");
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

fn assert_prior_generation(root: &Path, previous: &Path) {
    let entries: Vec<_> = fs::read_dir(root)
        .expect("read output root")
        .map(|entry| entry.expect("read output root entry").file_name())
        .collect();
    assert_eq!(
        entries,
        [previous.file_name().expect("generation name")],
        "only the prior generation should remain"
    );
    assert_probe_output(&previous.join(PROBE_NAME));
    verify_generation(previous).expect("prior generation manifest should still verify");
}

fn copy_generation(source: &Path, destination: &Path) {
    fs::create_dir(destination).expect("create copied generation");
    fs::copy(source.join(PROBE_NAME), destination.join(PROBE_NAME)).expect("copy executable");
    fs::copy(
        source.join("artifact.manifest"),
        destination.join("artifact.manifest"),
    )
    .expect("copy artifact manifest");
}

#[test]
#[ignore = "requires OOT_BUILD_TEST_COMPILER to name an absolute native compiler executable"]
fn native_success_compiler_failure_and_post_compile_cancellation() {
    let compiler = native_compiler();

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
    let record = verify_generation(&previous).expect("published manifest should verify");
    assert_eq!(record.format_version, 1);
    assert_eq!(record.fixture_id, SYNTHETIC_FIXTURE_ID);
    assert_eq!(
        record.executable_size,
        fs::metadata(&previous_executable).unwrap().len()
    );
    let manifest_text = fs::read_to_string(previous.join("artifact.manifest")).unwrap();
    assert!(!manifest_text.contains(&output_root.to_string_lossy().to_string()));

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
    assert_prior_generation(&output_root, &previous);

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
    assert_prior_generation(&output_root, &previous);

    // Cancellation after the manifest is written but before the staging rename
    // must leave the verified prior generation intact.
    let cancelled_before_publish = build_synthetic(&config, || {
        fs::read_dir(&output_root).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("staging-"))
                && path.join("artifact.manifest").exists()
        })
    });
    assert!(matches!(
        cancelled_before_publish,
        Err(BuildError::Cancelled)
    ));
    assert_prior_generation(&output_root, &previous);
}

#[test]
#[ignore = "requires OOT_BUILD_TEST_COMPILER to name an absolute native compiler executable"]
fn native_manifest_verifier_rejects_corrupt_and_truncated_records_and_artifacts() {
    let compiler = native_compiler();
    let temp = TempDir::new();
    let output_root = temp.0.join("output");
    let generation = build_synthetic(
        &BuildConfig {
            compiler,
            output_root,
        },
        || false,
    )
    .expect("native build should publish");

    let cases = temp.0.join("cases");
    fs::create_dir(&cases).unwrap();

    let malformed = cases.join("malformed-metadata");
    copy_generation(&generation, &malformed);
    fs::write(malformed.join("artifact.manifest"), "format_version=2\n")
        .expect("write altered metadata");
    assert!(matches!(
        verify_generation(&malformed),
        Err(VerifyError::MalformedMetadata(_))
    ));

    let truncated_metadata = cases.join("truncated-metadata");
    copy_generation(&generation, &truncated_metadata);
    let manifest = fs::read(truncated_metadata.join("artifact.manifest")).unwrap();
    fs::write(
        truncated_metadata.join("artifact.manifest"),
        &manifest[..manifest.len() / 2],
    )
    .expect("truncate metadata");
    assert!(matches!(
        verify_generation(&truncated_metadata),
        Err(VerifyError::MalformedMetadata(_))
    ));

    let corrupt_metadata = cases.join("corrupt-metadata");
    copy_generation(&generation, &corrupt_metadata);
    let mut manifest = fs::read(corrupt_metadata.join("artifact.manifest")).unwrap();
    let digest_start = manifest
        .windows(7)
        .position(|window| window == b"sha256=")
        .unwrap()
        + 7;
    manifest[digest_start] = if manifest[digest_start] == b'0' {
        b'1'
    } else {
        b'0'
    };
    fs::write(corrupt_metadata.join("artifact.manifest"), manifest).expect("corrupt digest");
    assert!(matches!(
        verify_generation(&corrupt_metadata),
        Err(VerifyError::DigestMismatch)
    ));

    let truncated_artifact = cases.join("truncated-artifact");
    copy_generation(&generation, &truncated_artifact);
    let executable = truncated_artifact.join(PROBE_NAME);
    let bytes = fs::read(&executable).unwrap();
    fs::write(&executable, &bytes[..bytes.len() - 1]).expect("truncate executable");
    assert!(matches!(
        verify_generation(&truncated_artifact),
        Err(VerifyError::SizeMismatch { .. })
    ));

    let corrupt_artifact = cases.join("corrupt-artifact");
    copy_generation(&generation, &corrupt_artifact);
    let executable = corrupt_artifact.join(PROBE_NAME);
    let mut bytes = fs::read(&executable).unwrap();
    bytes[0] ^= 0x01;
    fs::write(&executable, bytes).expect("corrupt executable");
    assert!(matches!(
        verify_generation(&corrupt_artifact),
        Err(VerifyError::DigestMismatch)
    ));

    verify_generation(&generation).expect("original published generation remains valid");
}

#[test]
#[ignore = "requires OOT_BUILD_TEST_COMPILER to name an absolute native compiler executable"]
fn native_probe_failures_preserve_previous_generation() {
    let compiler = native_compiler();
    let temp = TempDir::new();
    let output_root = temp.0.join("output");
    let config = BuildConfig {
        compiler: compiler.clone(),
        output_root: output_root.clone(),
    };
    let previous = build_synthetic(&config, || false).expect("establish previous generation");

    // The stand-in runs inside the staging directory and copies our precompiled
    // fixture from two levels up to the output filename.
    let wrapper_source = temp.0.join("wrapper.c");
    fs::write(
        &wrapper_source,
        r#"
#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#ifndef _WIN32
#include <sys/stat.h>
#endif
int main(int argc, char **argv) {
    if (argc < 2) return 1;
    FILE *in = fopen("../../probe-fixture", "rb");
    FILE *out = fopen(argv[argc - 1], "wb");
    if (!in || !out) return 2;
    int c;
    while ((c = fgetc(in)) != EOF) if (fputc(c, out) == EOF) return 3;
    if (ferror(in) || fclose(in) || fclose(out)) return 4;
#ifndef _WIN32
    if (chmod(argv[argc - 1], 0700)) return 5;
#endif
    return 0;
}
"#,
    )
    .expect("write wrapper source");
    let build = |source: &Path, output: &Path| {
        let result = Command::new(&compiler)
            .args(["-std=c11", "-Wall", "-Wextra", "-Werror", "-O2"])
            .arg(source)
            .arg("-o")
            .arg(output)
            .output()
            .expect("run native compiler");
        assert!(
            result.status.success(),
            "compile failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    let wrapper = temp.0.join(if cfg!(windows) {
        "wrapper.exe"
    } else {
        "wrapper"
    });
    build(&wrapper_source, &wrapper);
    let fixture_source = temp.0.join("bad_fixture.c");
    let fixture = temp.0.join("probe-fixture");
    let bad_config = BuildConfig {
        compiler: wrapper,
        output_root: output_root.clone(),
    };

    fs::write(
        &fixture_source,
        "#include <stdio.h>\nint main(void) { fputs(\"probe failure\", stderr); return 23; }\n",
    )
    .unwrap();
    build(&fixture_source, &fixture);
    match build_synthetic(&bad_config, || false) {
        Err(BuildError::ProbeFailed { status, stderr }) => {
            assert_eq!(status, Some(23));
            assert_eq!(stderr, "probe failure");
        }
        other => panic!("expected probe failure, got {other:?}"),
    }
    assert_prior_generation(&output_root, &previous);

    fs::write(
        &fixture_source,
        "#include <stdio.h>\nint main(void) { fputs(\"wrong bytes\", stdout); return 0; }\n",
    )
    .unwrap();
    build(&fixture_source, &fixture);
    match build_synthetic(&bad_config, || false) {
        Err(BuildError::UnexpectedOutput(output)) => assert_eq!(output, "wrong bytes"),
        other => panic!("expected unexpected output, got {other:?}"),
    }
    assert_prior_generation(&output_root, &previous);
    // The marker is written by the child itself, which then waits long enough
    // to prove that cancellation is observed before normal exit.
    fs::write(
        &fixture_source,
        r#"
#define _CRT_SECURE_NO_WARNINGS
#include <stdio.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#endif
int main(void) {
    FILE *marker = fopen("running-marker", "wb");
    if (!marker || fclose(marker)) return 1;
#ifdef _WIN32
    Sleep(30000);
#else
    sleep(30);
#endif
    return 2;
}
"#,
    )
    .expect("write running fixture");
    build(&fixture_source, &fixture);
    let started = std::time::Instant::now();
    let mut observed_start = false;
    let cancelled = build_synthetic(&bad_config, || {
        observed_start = fs::read_dir(&output_root).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("staging-"))
                && path.join("running-marker").exists()
        });
        observed_start || started.elapsed() >= std::time::Duration::from_secs(20)
    });
    assert!(observed_start, "probe did not start");
    assert!(started.elapsed() < std::time::Duration::from_secs(30));
    assert!(matches!(cancelled, Err(BuildError::Cancelled)));
    assert_prior_generation(&output_root, &previous);
}
