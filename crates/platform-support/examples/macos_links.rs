use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut report = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut report) {
        eprintln!("Could not read load report: {error}");
        return ExitCode::FAILURE;
    }
    match oot_platform_support::macos_links::unexpected_load_names(&report) {
        Ok(names) if names.is_empty() => ExitCode::SUCCESS,
        Ok(names) => {
            for name in names {
                eprintln!("Unexpected Mach-O load name: {name:?}");
            }
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("Invalid otool -L report at line {}", error.line);
            ExitCode::FAILURE
        }
    }
}
