//! Pure identity validation for OOT Universal.
//!
//! This crate validates the supported program, update version, and build.
//! [`synthetic`] adds a bounded validator for an authored two-asset fixture;
//! it is not game metadata inspection, filesystem extraction, or real install
//! acceptance.

use std::fmt;

use oot_contracts::CopyIdentity;

const SUPPORTED_PROGRAM_ID: u64 = 0x0400_4860_24de_c000;
const SUPPORTED_VERSION: u32 = 131_072;
const SUPPORTED_BUILD_ID: [u8; 4] = [0xe0, 0xf3, 0xcc, 0xfa];

/// Opaque proof that an identity matches the one supported by the spec.
///
/// Its private field prevents callers from constructing the token without
/// passing [`validate_identity`]. It does not prove that any copy assets were
/// inspected or validated.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupportedIdentity {
    _private: (),
}

/// The first identity field that differs from the supported tuple.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IdentityMismatch {
    /// The program ID differs from `0400486024DEC000`.
    ProgramId { found: u64 },
    /// The update version differs from `v131072`.
    Version { found: u32 },
    /// The build ID differs from `e0f3ccfa`.
    BuildId { found: [u8; 4] },
}

/// An identity that does not match the single supported tuple.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedIdentity {
    mismatch: IdentityMismatch,
}

impl UnsupportedIdentity {
    /// Returns the first mismatching field and its normalized value.
    pub const fn mismatch(&self) -> IdentityMismatch {
        self.mismatch
    }
}

impl fmt::Display for UnsupportedIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mismatch {
            IdentityMismatch::ProgramId { found } => write!(
                f,
                "unsupported program ID {found:016X}; expected 0400486024DEC000"
            ),
            IdentityMismatch::Version { found } => {
                write!(f, "unsupported update version v{found}; expected v131072")
            }
            IdentityMismatch::BuildId { found } => write!(
                f,
                "unsupported build ID {:02x}{:02x}{:02x}{:02x}; expected e0f3ccfa",
                found[0], found[1], found[2], found[3]
            ),
        }
    }
}

impl std::error::Error for UnsupportedIdentity {}

/// Accepts only the supported program `0400486024DEC000`, update `v131072`,
/// and build `e0f3ccfa`.
///
/// Comparison uses the normalized numeric identity from `oot-contracts`, so
/// hexadecimal case and leading zeroes in the version text do not affect the
/// result. When several fields differ, the first reported mismatch is program
/// ID, then version, then build ID.
pub fn validate_identity(
    identity: &CopyIdentity,
) -> Result<SupportedIdentity, UnsupportedIdentity> {
    if identity.program_id() != SUPPORTED_PROGRAM_ID {
        return Err(UnsupportedIdentity {
            mismatch: IdentityMismatch::ProgramId {
                found: identity.program_id(),
            },
        });
    }
    if identity.version() != SUPPORTED_VERSION {
        return Err(UnsupportedIdentity {
            mismatch: IdentityMismatch::Version {
                found: identity.version(),
            },
        });
    }
    if identity.build_id() != SUPPORTED_BUILD_ID {
        return Err(UnsupportedIdentity {
            mismatch: IdentityMismatch::BuildId {
                found: identity.build_id(),
            },
        });
    }
    Ok(SupportedIdentity { _private: () })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(program_id: &str, version: &str, build_id: &str) -> CopyIdentity {
        CopyIdentity::new(program_id, version, build_id).unwrap()
    }

    #[test]
    fn accepts_only_the_supported_identity_tuple() {
        let supported = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        assert_eq!(
            validate_identity(&supported),
            Ok(SupportedIdentity { _private: () })
        );

        // Hex case and leading zeroes normalize to the same exact tuple.
        let normalized = identity("0400486024dec000", "v00131072", "E0F3CCFA");
        assert!(validate_identity(&normalized).is_ok());
    }

    #[test]
    fn rejects_wrong_program_id_independently() {
        let found = identity("0400486024DEC001", "v131072", "e0f3ccfa");
        assert_eq!(
            validate_identity(&found).unwrap_err().mismatch(),
            IdentityMismatch::ProgramId {
                found: 0x0400_4860_24de_c001,
            }
        );
    }

    #[test]
    fn rejects_wrong_version_independently() {
        let found = identity("0400486024DEC000", "v131073", "e0f3ccfa");
        assert_eq!(
            validate_identity(&found).unwrap_err().mismatch(),
            IdentityMismatch::Version { found: 131_073 }
        );
    }

    #[test]
    fn rejects_wrong_build_id_independently() {
        let found = identity("0400486024DEC000", "v131072", "e0f3ccfb");
        assert_eq!(
            validate_identity(&found).unwrap_err().mismatch(),
            IdentityMismatch::BuildId {
                found: [0xe0, 0xf3, 0xcc, 0xfb],
            }
        );
    }
}

pub mod synthetic;
