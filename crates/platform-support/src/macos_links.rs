// SPDX-License-Identifier: MIT
// Adapted from NX2Apple frontend/tools/check_bundle.sh at
// 8aad909bb5d0bfee39583dabf1d15d924b9f156c. See licenses/nx2apple-MIT.txt.

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidReport {
    pub line: usize,
}

/// Finds load names that fall outside the donor's system and dyld-relative policy.
///
/// Use the successful output of `otool -arch arm64 -L` for a single binary.
/// This checks path spelling. Bundle validation must also resolve dyld-relative
/// names against the actual executable, loader and run paths.
pub fn unexpected_load_names(report: &str) -> Result<Vec<&str>, InvalidReport> {
    let mut lines = report.lines();
    if !lines
        .next()
        .is_some_and(|line| line.ends_with(':') && line.len() > 1)
    {
        return Err(InvalidReport { line: 1 });
    }
    let mut unexpected = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let Some((name, versions)) = line.trim_start().rsplit_once(" (compatibility version ")
        else {
            return Err(InvalidReport { line: line_number });
        };
        if name.is_empty() || !line.starts_with(char::is_whitespace) || !valid_versions(versions) {
            return Err(InvalidReport { line: line_number });
        }
        if !allowed(name) {
            unexpected.push(name);
        }
    }
    Ok(unexpected)
}

fn valid_versions(text: &str) -> bool {
    let Some((compatibility, current)) = text
        .strip_suffix(')')
        .and_then(|body| body.split_once(", current version "))
    else {
        return false;
    };
    let current = match current.split_once(", ") {
        Some((version, "weak" | "reexport" | "upward" | "lazy")) => version,
        Some(_) => return false,
        None => current,
    };
    fn numeric_triplet(version: &str) -> bool {
        let mut parts = version.split('.');
        [u16::MAX, u8::MAX.into(), u8::MAX.into()]
            .into_iter()
            .all(|max| {
                parts.next().is_some_and(|part| {
                    part.bytes().all(|byte| byte.is_ascii_digit())
                        && part.parse::<u16>().is_ok_and(|value| value <= max)
                })
            })
            && parts.next().is_none()
    }
    numeric_triplet(compatibility) && numeric_triplet(current)
}

fn allowed(name: &str) -> bool {
    let prefixes = [
        "/usr/lib/",
        "/System/Library/",
        "@rpath/",
        "@loader_path/",
        "@executable_path/",
    ];
    if name.chars().any(char::is_control) {
        return false;
    }
    for prefix in prefixes {
        if let Some(suffix) = name.strip_prefix(prefix) {
            if suffix.is_empty()
                || suffix.split('/').any(|part| {
                    part.is_empty() || (prefix.starts_with('/') && matches!(part, "." | ".."))
                })
            {
                return false;
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(names: &[&str]) -> String {
        let mut output = String::from("Example.app/Contents/MacOS/Example:\n");
        for name in names {
            output.push_str(&format!(
                "\t{name} (compatibility version 1.0.0, current version 1.2.3)\n"
            ));
        }
        output
    }

    #[test]
    fn preserves_spaces_and_flags_machine_specific_dependencies() {
        let report = report(&[
            "/usr/lib/libSystem.B.dylib",
            "/System/Library/Frameworks/Metal.framework/Metal",
            "@rpath/My Library.dylib",
            "@loader_path/../Frameworks/Example.dylib",
            "@executable_path/lib.dylib",
            "/opt/homebrew/opt/Example/lib/libExample.dylib",
            "/usr/local/lib/libOther.dylib",
            "/usr/lib/../../tmp/libOutside.dylib",
        ]);
        assert_eq!(
            unexpected_load_names(&report).unwrap(),
            [
                "/opt/homebrew/opt/Example/lib/libExample.dylib",
                "/usr/local/lib/libOther.dylib",
                "/usr/lib/../../tmp/libOutside.dylib",
            ]
        );
    }

    #[test]
    fn accepts_architecture_header_and_annotated_dylibs() {
        let report = concat!(
            "Example.app/Contents/MacOS/Example (architecture arm64):\n",
            "\t/System/Library/Frameworks/Metal.framework/Metal (compatibility version 1.0.0, current version 1.2.3, weak)\n",
            "\t@rpath/Framework.dylib (compatibility version 0.0.0, current version 1.2.3, reexport)\n",
        );
        assert!(unexpected_load_names(report).unwrap().is_empty());
    }

    #[test]
    fn rejects_empty_tool_output_and_malformed_rows() {
        assert_eq!(unexpected_load_names(""), Err(InvalidReport { line: 1 }));
        assert_eq!(
            unexpected_load_names("sample:\ninvalid"),
            Err(InvalidReport { line: 2 })
        );
        let malformed = "sample:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version garbage)";
        assert_eq!(
            unexpected_load_names(malformed),
            Err(InvalidReport { line: 2 })
        );
    }
}
