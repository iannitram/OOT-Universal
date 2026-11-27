// SPDX-License-Identifier: GPL-3.0-or-later
// Quoting adapted from RedTunic contributors' desktop_exec() in
// builder/linux/hookshot_builder.py at bc199725b73f5c2a1d5c2723978379a511a78e66.
// RedTunic inherits Longshot and Hookshot (Copyright (C) 2026 NotRexed
// and the Hookshot contributors; Copyright (C) 2026 the Longshot contributors).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecPathError {
    NotAbsolute,
    InvalidCharacter,
}

impl std::fmt::Display for ExecPathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotAbsolute => f.write_str("desktop executable path must be absolute"),
            Self::InvalidCharacter => f.write_str(
                "desktop executable path must use ASCII without controls or equals signs",
            ),
        }
    }
}

impl std::error::Error for ExecPathError {}

/// Encodes one absolute executable path as the entire value of an XDG Exec key.
///
/// Desktop Entry value escaping applies before argument unquoting, and a percent
/// needs escaping for field-code expansion. The executable position disallows '='.
/// The Desktop Entry specification limits Exec strings to ASCII.
pub fn desktop_exec(path: &str) -> Result<String, ExecPathError> {
    if !path.starts_with('/') {
        return Err(ExecPathError::NotAbsolute);
    }
    if !path.is_ascii() || path.bytes().any(|b| b == b'=' || b.is_ascii_control()) {
        return Err(ExecPathError::InvalidCharacter);
    }

    let mut value = String::from("\"");
    for ch in path.chars() {
        match ch {
            '\\' => value.push_str(r"\\\\"),
            '"' | '$' | '`' => {
                value.push_str(r"\\");
                value.push(ch);
            }
            '%' => value.push_str("%%"),
            _ => value.push(ch),
        }
    }
    value.push('"');
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_the_path_at_both_escaping_layers() {
        assert_eq!(
            desktop_exec(r#"/opt/My "100%" Games/\$HOME/`tool`"#).unwrap(),
            r#""/opt/My \\"100%%\\" Games/\\\\\\$HOME/\\`tool\\`""#
        );
    }

    #[test]
    fn rejects_paths_that_cannot_be_executables_in_the_key() {
        assert_eq!(desktop_exec("bin/tool"), Err(ExecPathError::NotAbsolute));
        for path in [
            "/opt/a=b",
            "/opt/x\ny",
            "/opt/x\0y",
            "/opt/é",
            "/opt/a\u{7f}",
        ] {
            assert_eq!(
                desktop_exec(path),
                Err(ExecPathError::InvalidCharacter),
                "{path:?}"
            );
        }
    }
}
