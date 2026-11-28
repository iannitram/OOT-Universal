use std::fmt;
use std::num::NonZeroU32;

/// Version of the in-memory settings contract.
pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

/// Resolution requested by the user. Explicit dimensions cannot be zero.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Resolution {
    Native,
    Explicit {
        width: NonZeroU32,
        height: NonZeroU32,
    },
}

/// Supported frame rate caps.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FrameCap {
    Fps30,
    Fps60,
}

impl FrameCap {
    pub const fn fps(self) -> u32 {
        match self {
            Self::Fps30 => 30,
            Self::Fps60 => 60,
        }
    }
}

/// Portable upscaler setting. Platform-specific implementations remain open.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Upscaler {
    Off,
    PlatformDefault,
}

/// In-memory settings values for the current schema.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Settings {
    schema_version: u32,
    resolution: Resolution,
    frame_cap: FrameCap,
    upscaler: Upscaler,
    extras_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsError {
    UnsupportedSchemaVersion(u32),
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported settings schema version: {version}")
            }
        }
    }
}

impl std::error::Error for SettingsError {}

impl Settings {
    /// Validates a versioned set of values against the supported schema.
    pub fn new(
        schema_version: u32,
        resolution: Resolution,
        frame_cap: FrameCap,
        upscaler: Upscaler,
        extras_enabled: bool,
    ) -> Result<Self, SettingsError> {
        if schema_version != SETTINGS_SCHEMA_VERSION {
            return Err(SettingsError::UnsupportedSchemaVersion(schema_version));
        }
        Ok(Self {
            schema_version,
            resolution,
            frame_cap,
            upscaler,
            extras_enabled,
        })
    }

    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    pub const fn resolution(self) -> Resolution {
        self.resolution
    }

    pub const fn frame_cap(self) -> FrameCap {
        self.frame_cap
    }

    pub const fn upscaler(self) -> Upscaler {
        self.upscaler
    }

    pub const fn extras_enabled(self) -> bool {
        self.extras_enabled
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            resolution: Resolution::Native,
            frame_cap: FrameCap::Fps60,
            upscaler: Upscaler::Off,
            extras_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_native_sixty_fps_and_feature_off() {
        let settings = Settings::default();
        assert_eq!(settings.schema_version(), SETTINGS_SCHEMA_VERSION);
        assert_eq!(settings.resolution(), Resolution::Native);
        assert_eq!(settings.frame_cap(), FrameCap::Fps60);
        assert_eq!(settings.upscaler(), Upscaler::Off);
        assert!(!settings.extras_enabled());
    }

    #[test]
    fn supports_only_the_fixed_frame_caps_and_nonzero_dimensions() {
        assert_eq!(FrameCap::Fps30.fps(), 30);
        assert_eq!(FrameCap::Fps60.fps(), 60);
        let resolution = Resolution::Explicit {
            width: NonZeroU32::new(1).unwrap(),
            height: NonZeroU32::new(u32::MAX).unwrap(),
        };
        assert_eq!(
            Settings::new(
                1,
                resolution,
                FrameCap::Fps30,
                Upscaler::PlatformDefault,
                true
            )
            .unwrap()
            .resolution(),
            resolution
        );
        assert_eq!(NonZeroU32::new(0), None);
    }

    #[test]
    fn rejects_unknown_schema_versions() {
        assert_eq!(
            Settings::new(2, Resolution::Native, FrameCap::Fps60, Upscaler::Off, false),
            Err(SettingsError::UnsupportedSchemaVersion(2))
        );
    }
}
