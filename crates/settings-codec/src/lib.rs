use oot_contracts::{FrameCap, Resolution, Settings, Upscaler};
use serde::{Deserialize, Serialize};
use std::{fmt, num::NonZeroU32};

/// Maximum accepted encoded settings size, in bytes.
pub const MAX_SETTINGS_BYTES: usize = 4096;

/// Failure returned when settings cannot be represented or decoded.
#[derive(Debug)]
pub enum CodecError {
    TooLarge { actual: usize, maximum: usize },
    Json(serde_json::Error),
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { actual, maximum } => {
                write!(f, "settings JSON is {actual} bytes; maximum is {maximum}")
            }
            Self::Json(error) => write!(f, "invalid settings JSON: {error}"),
        }
    }
}

impl std::error::Error for CodecError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::TooLarge { .. } => None,
            Self::Json(error) => Some(error),
        }
    }
}

impl From<serde_json::Error> for CodecError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsDocument {
    schema_version: u32,
    resolution: ResolutionDocument,
    frame_cap: u32,
    upscaler: UpscalerDocument,
    extras_enabled: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ResolutionDocument {
    Native,
    Explicit { width: u32, height: u32 },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum UpscalerDocument {
    Off,
    PlatformDefault,
}

/// Encodes every setting explicitly to a bounded JSON byte vector.
pub fn encode(settings: &Settings) -> Result<Vec<u8>, CodecError> {
    let resolution = match settings.resolution() {
        Resolution::Native => ResolutionDocument::Native,
        Resolution::Explicit { width, height } => ResolutionDocument::Explicit {
            width: width.get(),
            height: height.get(),
        },
    };
    let document = SettingsDocument {
        schema_version: settings.schema_version(),
        resolution,
        frame_cap: settings.frame_cap().fps(),
        upscaler: match settings.upscaler() {
            Upscaler::Off => UpscalerDocument::Off,
            Upscaler::PlatformDefault => UpscalerDocument::PlatformDefault,
        },
        extras_enabled: settings.extras_enabled(),
    };
    let bytes = serde_json::to_vec(&document)?;
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(CodecError::TooLarge {
            actual: bytes.len(),
            maximum: MAX_SETTINGS_BYTES,
        });
    }
    Ok(bytes)
}

/// Decodes bounded JSON into the dependency-free in-memory settings contract.
pub fn decode(bytes: &[u8]) -> Result<Settings, CodecError> {
    if bytes.len() > MAX_SETTINGS_BYTES {
        return Err(CodecError::TooLarge {
            actual: bytes.len(),
            maximum: MAX_SETTINGS_BYTES,
        });
    }
    let document: SettingsDocument = serde_json::from_slice(bytes)?;
    let resolution = match document.resolution {
        ResolutionDocument::Native => Resolution::Native,
        ResolutionDocument::Explicit { width, height } => Resolution::Explicit {
            width: NonZeroU32::new(width)
                .ok_or_else(|| invalid("resolution.width must be nonzero"))?,
            height: NonZeroU32::new(height)
                .ok_or_else(|| invalid("resolution.height must be nonzero"))?,
        },
    };
    let frame_cap = match document.frame_cap {
        30 => FrameCap::Fps30,
        60 => FrameCap::Fps60,
        _ => return Err(invalid("frame_cap must be 30 or 60")),
    };
    let upscaler = match document.upscaler {
        UpscalerDocument::Off => Upscaler::Off,
        UpscalerDocument::PlatformDefault => Upscaler::PlatformDefault,
    };
    Settings::new(
        document.schema_version,
        resolution,
        frame_cap,
        upscaler,
        document.extras_enabled,
    )
    .map_err(|error| invalid(error.to_string()))
}

fn invalid(message: impl fmt::Display) -> CodecError {
    CodecError::Json(<serde_json::Error as serde::de::Error>::custom(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(
        resolution: Resolution,
        cap: FrameCap,
        upscaler: Upscaler,
        extras: bool,
    ) -> Settings {
        Settings::new(1, resolution, cap, upscaler, extras).unwrap()
    }

    #[test]
    fn default_values_are_all_stored_and_round_trip() {
        let defaults = Settings::default();
        let encoded = encode(&defaults).unwrap();
        let text = std::str::from_utf8(&encoded).unwrap();
        assert!(text.contains("\"resolution\":{\"kind\":\"native\"}"));
        assert!(text.contains("\"frame_cap\":60"));
        assert!(text.contains("\"upscaler\":\"off\""));
        assert!(text.contains("\"extras_enabled\":false"));
        assert_eq!(decode(&encoded).unwrap(), defaults);
    }

    #[test]
    fn round_trips_explicit_dimensions_both_caps_and_upscaler() {
        for cap in [FrameCap::Fps30, FrameCap::Fps60] {
            for upscaler in [Upscaler::Off, Upscaler::PlatformDefault] {
                let value = settings(
                    Resolution::Explicit {
                        width: NonZeroU32::new(u32::MAX).unwrap(),
                        height: NonZeroU32::new(720).unwrap(),
                    },
                    cap,
                    upscaler,
                    true,
                );
                assert_eq!(decode(&encode(&value).unwrap()).unwrap(), value);
            }
        }
    }

    #[test]
    fn rejects_oversized_input() {
        let input = vec![b' '; MAX_SETTINGS_BYTES + 1];
        assert!(matches!(decode(&input), Err(CodecError::TooLarge { .. })));
    }

    #[test]
    fn rejects_invalid_versions_values_unknown_fields_and_malformed_json() {
        for input in [
            br#"{"schema_version":2,"resolution":{"kind":"native"},"frame_cap":60,"upscaler":"off","extras_enabled":false}"#.as_slice(),
            br#"{"schema_version":1,"resolution":{"kind":"explicit","width":0,"height":10},"frame_cap":60,"upscaler":"off","extras_enabled":false}"#,
            br#"{"schema_version":1,"resolution":{"kind":"native"},"frame_cap":24,"upscaler":"off","extras_enabled":false}"#,
            br#"{"schema_version":1,"resolution":{"kind":"native"},"frame_cap":60,"upscaler":"future","extras_enabled":false}"#,
            br#"{"schema_version":1,"resolution":{"kind":"native"},"frame_cap":60,"upscaler":"off","extras_enabled":false,"future":true}"#,
            b"{",
        ] {
            assert!(decode(input).is_err(), "accepted {}", String::from_utf8_lossy(input));
        }
    }
}
