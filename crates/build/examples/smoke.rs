use std::path::PathBuf;

use oot_build_probe::{build_synthetic, BuildConfig};

fn main() {
    let mut args = std::env::args_os().skip(1);
    let compiler = args.next().map(PathBuf::from).expect("pass compiler path");
    let output_root = args.next().map(PathBuf::from).expect("pass output root");
    assert!(args.next().is_none(), "expected exactly two arguments");
    let generation = build_synthetic(
        &BuildConfig {
            compiler,
            output_root,
        },
        || false,
    )
    .expect("synthetic build failed");
    println!("published {}", generation.display());
}
