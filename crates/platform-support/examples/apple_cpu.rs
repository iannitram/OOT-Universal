//! Read the output of a Clang predefined-macro probe from stdin.
use std::io::{self, Read};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut macros = String::new();
    if let Err(error) = io::stdin().read_to_string(&mut macros) {
        eprintln!("Could not read Clang macro report: {error}");
        return ExitCode::FAILURE;
    }
    let audit = oot_platform_support::apple_cpu::audit_m1_macros(&macros);
    if audit.passes() {
        println!("Apple ARM64 M1 compiler feature check passed");
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "Missing macros: {:?}\nForbidden macros: {:?}",
            audit.missing, audit.forbidden
        );
        ExitCode::FAILURE
    }
}
