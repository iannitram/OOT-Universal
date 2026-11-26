use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn checks_local_files_and_returns_failure_for_bad_input() {
    let dir = std::env::temp_dir().join(format!(
        "oot-frame-check-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&dir).unwrap();
    let good = dir.join("good.zs");
    let bad = dir.join("bad.zs");
    fs::write(&good, [0x28, 0xb5, 0x2f, 0xfd, 0x20, 0, 1, 0, 0]).unwrap();
    fs::write(&bad, [0x28, 0xb5]).unwrap();
    let exe = env!("CARGO_BIN_EXE_oot-frame-check");
    assert!(Command::new(exe).arg(&good).status().unwrap().success());
    assert_eq!(
        Command::new(exe)
            .args([&good, &bad])
            .status()
            .unwrap()
            .code(),
        Some(1)
    );
    assert_eq!(Command::new(exe).status().unwrap().code(), Some(2));
    fs::remove_dir_all(dir).unwrap();
}
