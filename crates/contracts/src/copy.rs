use std::fmt;

/// A syntactically valid copy identity, normalized for equality and matching.
///
/// Construction validates shape only; it does not decide whether the identity
/// is supported. Program IDs are 16 hexadecimal digits, versions are `v`
/// followed by decimal digits, and build IDs are 8 hexadecimal digits. Hex
/// input is case-insensitive. Leading zeroes in the decimal version are
/// accepted and normalized away (`v0012` and `v12` compare equal).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CopyIdentity {
    program_id: u64,
    version: u32,
    build_id: [u8; 4],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CopyIdentityError {
    InvalidProgramId,
    InvalidVersion,
    InvalidBuildId,
    VersionOutOfRange,
}

impl fmt::Display for CopyIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProgramId => f.write_str("program ID must be 16 hexadecimal digits"),
            Self::InvalidVersion => f.write_str("version must be v followed by decimal digits"),
            Self::InvalidBuildId => f.write_str("build ID must be 8 hexadecimal digits"),
            Self::VersionOutOfRange => f.write_str("numeric version is out of range"),
        }
    }
}

impl std::error::Error for CopyIdentityError {}

impl CopyIdentity {
    pub fn new(program_id: &str, version: &str, build_id: &str) -> Result<Self, CopyIdentityError> {
        if program_id.len() != 16 || !program_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(CopyIdentityError::InvalidProgramId);
        }
        if build_id.len() != 8 || !build_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(CopyIdentityError::InvalidBuildId);
        }
        let digits = version
            .strip_prefix('v')
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or(CopyIdentityError::InvalidVersion)?;
        let version = digits
            .parse::<u32>()
            .map_err(|_| CopyIdentityError::VersionOutOfRange)?;
        let build = u32::from_str_radix(build_id, 16)
            .map_err(|_| CopyIdentityError::InvalidBuildId)?
            .to_be_bytes();
        Ok(Self {
            program_id: u64::from_str_radix(program_id, 16)
                .expect("validated fixed-width hexadecimal program ID"),
            version,
            build_id: build,
        })
    }

    pub const fn program_id(&self) -> u64 {
        self.program_id
    }

    pub const fn version(&self) -> u32 {
        self.version
    }

    pub const fn build_id(&self) -> [u8; 4] {
        self.build_id
    }

    pub fn program_id_hex(&self) -> String {
        format!("{:016X}", self.program_id)
    }

    pub fn version_tag(&self) -> String {
        format!("v{}", self.version)
    }

    pub fn build_id_hex(&self) -> String {
        self.build_id
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_syntactic_identity() {
        let identity = CopyIdentity::new("0400486024DEC000", "v00131072", "E0F3CCFA").unwrap();
        assert_eq!(identity.program_id_hex(), "0400486024DEC000");
        assert_eq!(identity.version(), 131_072);
        assert_eq!(identity.version_tag(), "v131072");
        assert_eq!(identity.build_id_hex(), "e0f3ccfa");
        assert_eq!(identity.build_id(), [0xe0, 0xf3, 0xcc, 0xfa]);
    }

    #[test]
    fn identity_validation_checks_shape_but_not_supported_identity() {
        assert!(CopyIdentity::new("0000000000000000", "v1", "00000000").is_ok());
        assert_eq!(
            CopyIdentity::new("ABC", "v1", "00000000"),
            Err(CopyIdentityError::InvalidProgramId)
        );
        assert_eq!(
            CopyIdentity::new("0000000000000000", "1", "00000000"),
            Err(CopyIdentityError::InvalidVersion)
        );
        assert_eq!(
            CopyIdentity::new("0000000000000000", "v4294967296", "00000000"),
            Err(CopyIdentityError::VersionOutOfRange)
        );
        assert_eq!(
            CopyIdentity::new("0000000000000000", "v1", "xyz"),
            Err(CopyIdentityError::InvalidBuildId)
        );
    }
}
