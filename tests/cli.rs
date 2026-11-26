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
    assert!(Command::new(exe)
        .arg(&good)
        .output()
        .unwrap()
        .status
        .success());
    let mixed = Command::new(exe).args([&good, &bad]).output().unwrap();
    assert_eq!(mixed.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&mixed.stderr).contains("frame is truncated"));
    let folder = Command::new(exe).arg(&dir).output().unwrap();
    assert_eq!(folder.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&folder.stderr).contains("expected a regular file"));
    assert_eq!(Command::new(exe).output().unwrap().status.code(), Some(2));
    fs::remove_dir_all(dir).unwrap();
}
