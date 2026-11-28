//! Dependency-free data contracts shared across build and runtime components.
//!
//! These types describe syntactically valid inputs and target capabilities. They
//! do not establish that a game copy is supported or that a target can run it.

mod copy;
mod settings;
mod target;

pub use copy::{CopyIdentity, CopyIdentityError};
pub use settings::{FrameCap, Resolution, Settings, SettingsError, Upscaler};
pub use target::{Architecture, TargetFamily, TargetPlatform, TargetProfile, TargetProfileError};
