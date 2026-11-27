use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Architecture {
    X86_64,
    Aarch64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TargetPlatform {
    Linux,
    Windows,
    MacOS,
    IOS,
    Android,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TargetFamily {
    Desktop,
    Mobile,
}

/// An architecture/platform pair accepted by the project for development or probes.
///
/// This classification carries no Rust target triple, ABI, simulator/device
/// distinction, or runtime capability guarantee. Linux ARM64 is a portability
/// probe target. Choose an explicit compiler target for linking or packaging.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TargetProfile {
    architecture: Architecture,
    platform: TargetPlatform,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TargetProfileError {
    pub architecture: Architecture,
    pub platform: TargetPlatform,
}

impl fmt::Display for TargetProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsupported target pair: {:?} on {:?}",
            self.architecture, self.platform
        )
    }
}

impl std::error::Error for TargetProfileError {}

impl TargetProfile {
    pub fn new(
        architecture: Architecture,
        platform: TargetPlatform,
    ) -> Result<Self, TargetProfileError> {
        let supported = match platform {
            TargetPlatform::Linux => {
                matches!(architecture, Architecture::X86_64 | Architecture::Aarch64)
            }
            TargetPlatform::Windows => architecture == Architecture::X86_64,
            TargetPlatform::MacOS | TargetPlatform::IOS | TargetPlatform::Android => {
                architecture == Architecture::Aarch64
            }
        };
        if !supported {
            return Err(TargetProfileError {
                architecture,
                platform,
            });
        }
        Ok(Self {
            architecture,
            platform,
        })
    }

    pub const fn architecture(self) -> Architecture {
        self.architecture
    }

    pub const fn platform(self) -> TargetPlatform {
        self.platform
    }

    pub const fn family(self) -> TargetFamily {
        match self.platform {
            TargetPlatform::Linux | TargetPlatform::Windows | TargetPlatform::MacOS => {
                TargetFamily::Desktop
            }
            TargetPlatform::IOS | TargetPlatform::Android => TargetFamily::Mobile,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_architecture_and_platform_and_classifies_family() {
        let linux = TargetProfile::new(Architecture::Aarch64, TargetPlatform::Linux).unwrap();
        assert_eq!(linux.architecture(), Architecture::Aarch64);
        assert_eq!(linux.platform(), TargetPlatform::Linux);
        assert_eq!(linux.family(), TargetFamily::Desktop);

        let ios = TargetProfile::new(Architecture::Aarch64, TargetPlatform::IOS).unwrap();
        assert_eq!(ios.family(), TargetFamily::Mobile);
    }

    #[test]
    fn rejects_pairs_outside_current_target_matrix() {
        assert_eq!(
            TargetProfile::new(Architecture::X86_64, TargetPlatform::IOS),
            Err(TargetProfileError {
                architecture: Architecture::X86_64,
                platform: TargetPlatform::IOS
            })
        );
    }
}
