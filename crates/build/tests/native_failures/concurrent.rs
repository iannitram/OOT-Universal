use super::{assert_probe_output, native_compiler, BuildConfig, TempDir, PROBE_NAME};
use std::fs;
use std::sync::{Arc, Barrier};

#[test]
#[ignore = "requires OOT_BUILD_TEST_COMPILER to name an absolute native compiler executable"]
fn concurrent_native_builds_publish_distinct_complete_generations() {
    let compiler = native_compiler();

    let temp = TempDir::new();
    let output_root = temp.0.join("output");
    fs::create_dir(&output_root).expect("create output root");

    let barrier = Arc::new(Barrier::new(2));
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let config = BuildConfig {
                compiler: compiler.clone(),
                output_root: output_root.clone(),
            };
            std::thread::spawn(move || {
                barrier.wait();
                oot_build_probe::build_synthetic(&config, || false)
                    .expect("concurrent native build should succeed")
            })
        })
        .collect();

    let generations: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("native build thread should not panic"))
        .collect();

    assert_ne!(generations[0], generations[1]);
    for generation in &generations {
        assert!(generation.is_dir(), "published generation should exist");
        assert_probe_output(&generation.join(PROBE_NAME));
    }

    let mut entries: Vec<_> = fs::read_dir(&output_root)
        .expect("read output root")
        .map(|entry| entry.expect("read output root entry").file_name())
        .collect();
    entries.sort();
    let mut expected: Vec<_> = generations
        .iter()
        .map(|generation| generation.file_name().expect("generation name").to_owned())
        .collect();
    expected.sort();
    assert_eq!(entries, expected, "only both generations should remain");
    assert!(entries
        .iter()
        .all(|path| { path.to_string_lossy().starts_with("generation-") }));
}
