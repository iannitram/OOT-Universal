// SPDX-License-Identifier: MIT
// Copyright (c) 2026 seedds
// Adapted from nxnative dm/ios/tests/test_cpu_target.py at
// b2743d11be3bf81f0bb5e0c37ef2b95742151c78. See licenses/nxnative-MIT.txt.
use std::collections::BTreeSet;

const REQUIRED: [&str; 3] = ["__APPLE__", "__aarch64__", "__ARM_FEATURE_ATOMICS"];
const FORBIDDEN: [&str; 3] = [
    "__ARM_FEATURE_MATMUL_INT8",
    "__ARM_FEATURE_BF16_VECTOR_ARITHMETIC",
    "__ARM_FEATURE_SME",
];

#[derive(Debug, PartialEq, Eq)]
pub struct AppleCpuAudit {
    pub missing: Vec<&'static str>,
    pub forbidden: Vec<&'static str>,
}

impl AppleCpuAudit {
    pub fn passes(&self) -> bool {
        self.missing.is_empty() && self.forbidden.is_empty()
    }
}

/// Checks the donor's M1 floor against Clang's predefined-macro report.
/// This audits compiler promises. It cannot prove a linked app runs on a device.
pub fn audit_m1_macros(report: &str) -> AppleCpuAudit {
    let macros: BTreeSet<_> = report
        .lines()
        .filter_map(|line| {
            let mut words = line.split_ascii_whitespace();
            (words.next()? == "#define").then(|| words.next()).flatten()
        })
        .collect();

    AppleCpuAudit {
        missing: REQUIRED
            .into_iter()
            .filter(|name| !macros.contains(name))
            .collect(),
        forbidden: FORBIDDEN
            .into_iter()
            .filter(|name| macros.contains(name))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const M1: &str =
        "#define __APPLE__ 1\n#define __aarch64__ 1\n#define __ARM_FEATURE_ATOMICS 1\n";

    #[test]
    fn accepts_the_m1_feature_floor() {
        assert!(audit_m1_macros(M1).passes());
    }

    #[test]
    fn rejects_missing_required_and_newer_cpu_features() {
        let audit = audit_m1_macros(&format!("{M1}#define __ARM_FEATURE_MATMUL_INT8 1\n"));
        assert_eq!(audit.forbidden, ["__ARM_FEATURE_MATMUL_INT8"]);
        assert_eq!(
            audit_m1_macros("#define __aarch64__ 1").missing,
            ["__APPLE__", "__ARM_FEATURE_ATOMICS"]
        );
    }
}
