use oot_platform_support::desktop_exec::desktop_exec;

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next();
    let path = path.as_deref().and_then(std::ffi::OsStr::to_str);
    match (path.map(desktop_exec), arguments.next()) {
        (Some(Ok(exec)), None) => {
            println!("[Desktop Entry]\nType=Application\nName=Platform Probe\nExec={exec}");
            std::process::ExitCode::SUCCESS
        }
        _ => {
            eprintln!("Usage: desktop_entry <absolute ASCII executable path>");
            std::process::ExitCode::FAILURE
        }
    }
}
