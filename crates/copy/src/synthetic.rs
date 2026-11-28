//! Synthetic-only validation of fixed, authored acceptance fixture bytes.
//!
//! This module does not inspect game copies, paths, archives, or extracted files.

use crate::{validate_identity, SupportedIdentity, UnsupportedIdentity};
use oot_contracts::CopyIdentity;
use std::{fmt, io};

/// Maximum number of named assets accepted for the synthetic fixture.
pub const MAX_ASSET_COUNT: usize = 2;
/// Maximum number of bytes in any one fixture asset.
pub const MAX_ASSET_BYTES: usize = 64;
/// Maximum number of bytes in any asset name.
pub const MAX_ASSET_NAME_BYTES: usize = 32;

const MANIFEST_NAME: &str = "fixture/manifest.bin";
const PAYLOAD_NAME: &str = "fixture/payload.bin";
const MANIFEST_BYTES: &[u8] = b"oot-synthetic-fixture-v1\n";
const PAYLOAD_BYTES: &[u8] = &[0x4f, 0x4f, 0x54, 0x00, 0x01, 0x7f, 0x80, 0xff];

/// A borrowed name and byte slice supplied by a synthetic fixture caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedAsset<'a> {
    name: &'a str,
    bytes: &'a [u8],
}

impl<'a> NamedAsset<'a> {
    /// Creates a borrowed named asset. Validation is performed by
    /// [`validate_synthetic_fixture`].
    pub const fn new(name: &'a str, bytes: &'a [u8]) -> Self {
        Self { name, bytes }
    }

    pub const fn name(&self) -> &'a str {
        self.name
    }

    pub const fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// A borrowed name and stream used to construct an owned synthetic snapshot.
///
/// The stream is consumed only after identity and descriptor validation. Each
/// stream is read through a `MAX_ASSET_BYTES + 1` byte limit.
pub struct NamedAssetReader<'a> {
    name: &'a str,
    reader: &'a mut dyn io::Read,
}

impl<'a> NamedAssetReader<'a> {
    pub fn new(name: &'a str, reader: &'a mut dyn io::Read) -> Self {
        Self { name, reader }
    }

    pub const fn name(&self) -> &'a str {
        self.name
    }
}

/// Opaque proof that identity and the exact authored synthetic fixture passed.
/// The token keeps borrows to the same immutable slices that were validated.
#[derive(Debug)]
pub struct SyntheticAcceptance<'a> {
    _identity: SupportedIdentity,
    manifest: &'a [u8],
    payload: &'a [u8],
}

impl<'a> SyntheticAcceptance<'a> {
    /// Returns the exact validated manifest bytes.
    pub const fn manifest(&self) -> &'a [u8] {
        self.manifest
    }

    /// Returns the exact validated payload bytes.
    pub const fn payload(&self) -> &'a [u8] {
        self.payload
    }

    /// Looks up one of the two exact fixture names.
    pub fn asset(&self, name: &str) -> Option<&'a [u8]> {
        match name {
            MANIFEST_NAME => Some(self.manifest),
            PAYLOAD_NAME => Some(self.payload),
            _ => None,
        }
    }
}

/// Owned proof that identity and the exact authored synthetic fixture passed.
/// The validated asset bytes remain available independently of their readers.
#[derive(Debug, Eq, PartialEq)]
pub struct OwnedSyntheticAcceptance {
    _identity: SupportedIdentity,
    manifest: Vec<u8>,
    payload: Vec<u8>,
}

impl OwnedSyntheticAcceptance {
    /// Returns the exact validated manifest bytes.
    pub fn manifest(&self) -> &[u8] {
        &self.manifest
    }

    /// Returns the exact validated payload bytes.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Looks up one of the two exact fixture names.
    pub fn asset(&self, name: &str) -> Option<&[u8]> {
        match name {
            MANIFEST_NAME => Some(&self.manifest),
            PAYLOAD_NAME => Some(&self.payload),
            _ => None,
        }
    }
}

/// Why the synthetic identity-plus-assets acceptance failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntheticAcceptanceError {
    UnsupportedIdentity(UnsupportedIdentity),
    TooManyAssets {
        found: usize,
        maximum: usize,
    },
    AssetNameTooLong {
        found: usize,
        maximum: usize,
    },
    AssetTooLarge {
        name: &'static str,
        found: usize,
        maximum: usize,
    },
    UnknownAsset,
    DuplicateAsset(&'static str),
    MissingAsset(&'static str),
    CorruptAsset(&'static str),
}

impl fmt::Display for SyntheticAcceptanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedIdentity(error) => write!(f, "{error}"),
            Self::TooManyAssets { found, maximum } => {
                write!(
                    f,
                    "synthetic fixture has {found} assets; maximum is {maximum}"
                )
            }
            Self::AssetNameTooLong { found, maximum } => {
                write!(
                    f,
                    "synthetic asset name is {found} bytes; maximum is {maximum}"
                )
            }
            Self::AssetTooLarge {
                name,
                found,
                maximum,
            } => {
                write!(
                    f,
                    "synthetic asset {name} is {found} bytes; maximum is {maximum}"
                )
            }
            Self::UnknownAsset => f.write_str("synthetic fixture contains an unknown asset"),
            Self::DuplicateAsset(name) => write!(f, "synthetic fixture duplicates {name}"),
            Self::MissingAsset(name) => write!(f, "synthetic fixture is missing {name}"),
            Self::CorruptAsset(name) => write!(f, "synthetic fixture asset {name} does not match"),
        }
    }
}

impl std::error::Error for SyntheticAcceptanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnsupportedIdentity(error) => Some(error),
            _ => None,
        }
    }
}

/// Why owned synthetic stream acceptance failed.
#[derive(Debug)]
pub enum OwnedSyntheticAcceptanceError {
    Validation(SyntheticAcceptanceError),
    Io {
        name: &'static str,
        source: io::Error,
    },
}

impl fmt::Display for OwnedSyntheticAcceptanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => write!(f, "{error}"),
            Self::Io { name, source } => {
                write!(f, "could not read synthetic asset {name}: {source}")
            }
        }
    }
}

impl std::error::Error for OwnedSyntheticAcceptanceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Validation(error) => Some(error),
            Self::Io { source, .. } => Some(source),
        }
    }
}

impl From<SyntheticAcceptanceError> for OwnedSyntheticAcceptanceError {
    fn from(error: SyntheticAcceptanceError) -> Self {
        Self::Validation(error)
    }
}

/// Reads and validates exactly the authored fixture into an owned snapshot.
///
/// Identity, asset count, names, duplicate names, and required names are
/// validated before any stream is read. Each stream is bounded to at most one
/// byte beyond [`MAX_ASSET_BYTES`] so oversized input can be rejected without
/// reading an unbounded source. Validation uses the same byte-slice validator
/// as [`validate_synthetic_fixture`]. The result owns the validated bytes and
/// does not retain a borrow of any reader.
pub fn read_synthetic_fixture<'a>(
    identity: &CopyIdentity,
    assets: &mut [NamedAssetReader<'a>],
) -> Result<OwnedSyntheticAcceptance, OwnedSyntheticAcceptanceError> {
    let supported =
        validate_identity(identity).map_err(SyntheticAcceptanceError::UnsupportedIdentity)?;
    if assets.len() > MAX_ASSET_COUNT {
        return Err(SyntheticAcceptanceError::TooManyAssets {
            found: assets.len(),
            maximum: MAX_ASSET_COUNT,
        }
        .into());
    }

    let (mut has_manifest, mut has_payload) = (false, false);
    for asset in assets.iter() {
        if asset.name.len() > MAX_ASSET_NAME_BYTES {
            return Err(SyntheticAcceptanceError::AssetNameTooLong {
                found: asset.name.len(),
                maximum: MAX_ASSET_NAME_BYTES,
            }
            .into());
        }
        match asset.name {
            MANIFEST_NAME if !has_manifest => has_manifest = true,
            MANIFEST_NAME => {
                return Err(SyntheticAcceptanceError::DuplicateAsset(MANIFEST_NAME).into())
            }
            PAYLOAD_NAME if !has_payload => has_payload = true,
            PAYLOAD_NAME => {
                return Err(SyntheticAcceptanceError::DuplicateAsset(PAYLOAD_NAME).into())
            }
            _ => return Err(SyntheticAcceptanceError::UnknownAsset.into()),
        }
    }
    if !has_manifest {
        return Err(SyntheticAcceptanceError::MissingAsset(MANIFEST_NAME).into());
    }
    if !has_payload {
        return Err(SyntheticAcceptanceError::MissingAsset(PAYLOAD_NAME).into());
    }

    let mut buffers = Vec::with_capacity(assets.len());
    for asset in assets.iter_mut() {
        let bytes = read_bounded(&mut *asset.reader).map_err(|source| {
            OwnedSyntheticAcceptanceError::Io {
                name: known_asset_name(asset.name),
                source,
            }
        })?;
        if bytes.len() > MAX_ASSET_BYTES {
            return Err(SyntheticAcceptanceError::AssetTooLarge {
                name: known_asset_name(asset.name),
                found: bytes.len(),
                maximum: MAX_ASSET_BYTES,
            }
            .into());
        }
        buffers.push(bytes);
    }

    let borrowed: Vec<_> = assets
        .iter()
        .zip(&buffers)
        .map(|(asset, bytes)| NamedAsset::new(asset.name, bytes))
        .collect();
    validate_synthetic_fixture(identity, &borrowed)?;

    let manifest_index = assets
        .iter()
        .position(|asset| asset.name == MANIFEST_NAME)
        .expect("preflight requires the manifest");
    let payload_index = assets
        .iter()
        .position(|asset| asset.name == PAYLOAD_NAME)
        .expect("preflight requires the payload");
    let mut owned: Vec<Option<Vec<u8>>> = buffers.into_iter().map(Some).collect();
    Ok(OwnedSyntheticAcceptance {
        _identity: supported,
        manifest: owned[manifest_index]
            .take()
            .expect("asset buffer is present"),
        payload: owned[payload_index]
            .take()
            .expect("asset buffer is present"),
    })
}

/// Validates supported identity and exactly the two authored synthetic assets.
///
/// The fixture consists of `fixture/manifest.bin` with the bytes
/// `oot-synthetic-fixture-v1\n` and `fixture/payload.bin` with the eight
/// bytes `4f 4f 54 00 01 7f 80 ff`. Names are compared as exact labels.
///
/// Count, name length, and byte length bounds are checked before asset-name or
/// content comparisons. The returned token borrows the caller's original
/// immutable slices. This is synthetic fixture acceptance only; it says
/// nothing about game metadata, filesystem extraction, or a real installation.
pub fn validate_synthetic_fixture<'a>(
    identity: &CopyIdentity,
    assets: &[NamedAsset<'a>],
) -> Result<SyntheticAcceptance<'a>, SyntheticAcceptanceError> {
    if assets.len() > MAX_ASSET_COUNT {
        return Err(SyntheticAcceptanceError::TooManyAssets {
            found: assets.len(),
            maximum: MAX_ASSET_COUNT,
        });
    }
    for asset in assets {
        if asset.name.len() > MAX_ASSET_NAME_BYTES {
            return Err(SyntheticAcceptanceError::AssetNameTooLong {
                found: asset.name.len(),
                maximum: MAX_ASSET_NAME_BYTES,
            });
        }
        if asset.bytes.len() > MAX_ASSET_BYTES {
            return Err(SyntheticAcceptanceError::AssetTooLarge {
                name: known_asset_name(asset.name),
                found: asset.bytes.len(),
                maximum: MAX_ASSET_BYTES,
            });
        }
    }

    let supported =
        validate_identity(identity).map_err(SyntheticAcceptanceError::UnsupportedIdentity)?;
    let (mut manifest, mut payload): (Option<&'a [u8]>, Option<&'a [u8]>) = (None, None);
    for asset in assets {
        match asset.name {
            MANIFEST_NAME => {
                if manifest.replace(asset.bytes).is_some() {
                    return Err(SyntheticAcceptanceError::DuplicateAsset(MANIFEST_NAME));
                }
            }
            PAYLOAD_NAME => {
                if payload.replace(asset.bytes).is_some() {
                    return Err(SyntheticAcceptanceError::DuplicateAsset(PAYLOAD_NAME));
                }
            }
            _ => return Err(SyntheticAcceptanceError::UnknownAsset),
        }
    }

    let manifest = manifest.ok_or(SyntheticAcceptanceError::MissingAsset(MANIFEST_NAME))?;
    let payload = payload.ok_or(SyntheticAcceptanceError::MissingAsset(PAYLOAD_NAME))?;
    if manifest != MANIFEST_BYTES {
        return Err(SyntheticAcceptanceError::CorruptAsset(MANIFEST_NAME));
    }
    if payload != PAYLOAD_BYTES {
        return Err(SyntheticAcceptanceError::CorruptAsset(PAYLOAD_NAME));
    }
    Ok(SyntheticAcceptance {
        _identity: supported,
        manifest,
        payload,
    })
}

fn read_bounded<R: io::Read>(reader: R) -> io::Result<Vec<u8>> {
    let mut limited = io::Read::take(reader, (MAX_ASSET_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut limited, &mut bytes)?;
    Ok(bytes)
}

fn known_asset_name(name: &str) -> &'static str {
    match name {
        MANIFEST_NAME => MANIFEST_NAME,
        PAYLOAD_NAME => PAYLOAD_NAME,
        _ => "<unknown>",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    fn identity(program: &str, version: &str, build: &str) -> CopyIdentity {
        CopyIdentity::new(program, version, build).unwrap()
    }

    fn fixture<'a>() -> [NamedAsset<'a>; 2] {
        [
            NamedAsset::new(MANIFEST_NAME, MANIFEST_BYTES),
            NamedAsset::new(PAYLOAD_NAME, PAYLOAD_BYTES),
        ]
    }

    #[test]
    fn accepts_exact_fixture_and_returns_borrowed_bytes() {
        let assets = fixture();
        let accepted = validate_synthetic_fixture(
            &identity("0400486024DEC000", "v131072", "e0f3ccfa"),
            &assets,
        )
        .unwrap();
        assert_eq!(accepted.manifest(), assets[0].bytes());
        assert_eq!(accepted.payload(), assets[1].bytes());
        assert!(std::ptr::eq(
            accepted.manifest().as_ptr(),
            assets[0].bytes().as_ptr()
        ));
        assert_eq!(accepted.asset(MANIFEST_NAME), Some(assets[0].bytes()));
        assert_eq!(accepted.asset("unknown"), None);
    }

    #[test]
    fn token_outlives_temporary_descriptor_array() {
        let accepted = {
            let descriptors = fixture();
            validate_synthetic_fixture(
                &identity("0400486024DEC000", "v131072", "e0f3ccfa"),
                &descriptors,
            )
            .unwrap()
        };
        assert_eq!(accepted.manifest(), MANIFEST_BYTES);
        assert_eq!(accepted.payload(), PAYLOAD_BYTES);
    }

    #[test]
    fn rejects_wrong_identity() {
        let assets = fixture();
        let error = validate_synthetic_fixture(
            &identity("0400486024DEC001", "v131072", "e0f3ccfa"),
            &assets,
        )
        .unwrap_err();
        assert!(matches!(
            error,
            SyntheticAcceptanceError::UnsupportedIdentity(_)
        ));
    }

    #[test]
    fn rejects_missing_unknown_duplicate_oversized_and_corrupt_assets() {
        let id = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        let both = fixture();
        assert!(matches!(
            validate_synthetic_fixture(&id, &both[..1]),
            Err(SyntheticAcceptanceError::MissingAsset(PAYLOAD_NAME))
        ));

        let unknown = [both[0], NamedAsset::new("extra", b"x")];
        assert_eq!(
            validate_synthetic_fixture(&id, &unknown).unwrap_err(),
            SyntheticAcceptanceError::UnknownAsset
        );

        let duplicate = [both[0], both[0]];
        assert_eq!(
            validate_synthetic_fixture(&id, &duplicate).unwrap_err(),
            SyntheticAcceptanceError::DuplicateAsset(MANIFEST_NAME)
        );

        let oversized = [
            both[0],
            NamedAsset::new(PAYLOAD_NAME, &[0; MAX_ASSET_BYTES + 1]),
        ];
        assert!(matches!(
            validate_synthetic_fixture(&id, &oversized),
            Err(SyntheticAcceptanceError::AssetTooLarge { .. })
        ));

        let corrupt = [both[0], NamedAsset::new(PAYLOAD_NAME, b"changed!")];
        assert_eq!(
            validate_synthetic_fixture(&id, &corrupt).unwrap_err(),
            SyntheticAcceptanceError::CorruptAsset(PAYLOAD_NAME)
        );
    }

    #[test]
    fn rejects_count_and_name_bounds_before_asset_scanning() {
        let id = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        let over_count = [
            NamedAsset::new("bad", &[]),
            NamedAsset::new("bad", &[]),
            NamedAsset::new("bad", &[]),
        ];
        assert!(matches!(
            validate_synthetic_fixture(&id, &over_count),
            Err(SyntheticAcceptanceError::TooManyAssets { .. })
        ));
        let over_name = [NamedAsset::new("abcdefghijklmnopqrstuvwxyz1234567", &[])];
        assert!(matches!(
            validate_synthetic_fixture(&id, &over_name),
            Err(SyntheticAcceptanceError::AssetNameTooLong { .. })
        ));
    }

    #[test]
    fn reads_bounded_streams_into_owned_snapshot() {
        let id = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        let mut manifest_source = Cursor::new(MANIFEST_BYTES.to_vec());
        let mut payload_source = Cursor::new(PAYLOAD_BYTES.to_vec());
        let accepted = {
            let mut readers = [
                NamedAssetReader::new(MANIFEST_NAME, &mut manifest_source),
                NamedAssetReader::new(PAYLOAD_NAME, &mut payload_source),
            ];
            read_synthetic_fixture(&id, &mut readers).unwrap()
        };

        manifest_source.get_mut().fill(0);
        payload_source.get_mut().fill(0);
        assert_eq!(accepted.manifest(), MANIFEST_BYTES);
        assert_eq!(accepted.payload(), PAYLOAD_BYTES);
        assert_eq!(accepted.asset(MANIFEST_NAME), Some(MANIFEST_BYTES));
        assert_eq!(accepted.asset(PAYLOAD_NAME), Some(PAYLOAD_BYTES));
        assert_eq!(accepted.asset("unknown"), None);
    }

    #[test]
    fn rejects_truncated_and_oversized_streams() {
        let id = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        let mut short_manifest = Cursor::new(&MANIFEST_BYTES[..MANIFEST_BYTES.len() - 1]);
        let mut payload = Cursor::new(PAYLOAD_BYTES);
        let mut truncated = [
            NamedAssetReader::new(MANIFEST_NAME, &mut short_manifest),
            NamedAssetReader::new(PAYLOAD_NAME, &mut payload),
        ];
        assert!(matches!(
            read_synthetic_fixture(&id, &mut truncated),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::CorruptAsset(MANIFEST_NAME)
            ))
        ));

        struct PanicReader;
        impl Read for PanicReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                panic!("later asset should not be read after oversized input")
            }
        }

        let mut oversized_manifest = Cursor::new(vec![0; MAX_ASSET_BYTES + 100]);
        let mut payload = PanicReader;
        let mut oversized = [
            NamedAssetReader::new(MANIFEST_NAME, &mut oversized_manifest),
            NamedAssetReader::new(PAYLOAD_NAME, &mut payload),
        ];
        assert!(matches!(
            read_synthetic_fixture(&id, &mut oversized),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::AssetTooLarge {
                    name: MANIFEST_NAME,
                    found,
                    maximum
                }
            )) if found == MAX_ASSET_BYTES + 1 && maximum == MAX_ASSET_BYTES
        ));
        assert_eq!(oversized_manifest.position(), (MAX_ASSET_BYTES + 1) as u64);
    }

    #[test]
    fn rejects_invalid_descriptors_and_identity_before_reading() {
        struct PanicReader;
        impl Read for PanicReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                panic!("invalid input should be rejected before reading")
            }
        }

        let id = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        let mut missing_reader = PanicReader;
        let mut missing = [NamedAssetReader::new(MANIFEST_NAME, &mut missing_reader)];
        assert!(matches!(
            read_synthetic_fixture(&id, &mut missing),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::MissingAsset(PAYLOAD_NAME)
            ))
        ));

        let mut count_reader_a = PanicReader;
        let mut count_reader_b = PanicReader;
        let mut count_reader_c = PanicReader;
        let mut over_count = [
            NamedAssetReader::new("one", &mut count_reader_a),
            NamedAssetReader::new("two", &mut count_reader_b),
            NamedAssetReader::new("three", &mut count_reader_c),
        ];
        assert!(matches!(
            read_synthetic_fixture(&id, &mut over_count),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::TooManyAssets { .. }
            ))
        ));

        let mut name_reader = PanicReader;
        let mut long_name = [NamedAssetReader::new(
            "abcdefghijklmnopqrstuvwxyz123456789",
            &mut name_reader,
        )];
        assert!(matches!(
            read_synthetic_fixture(&id, &mut long_name),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::AssetNameTooLong { .. }
            ))
        ));

        let mut duplicate_manifest = PanicReader;
        let mut duplicate_other = PanicReader;
        let mut duplicate = [
            NamedAssetReader::new(MANIFEST_NAME, &mut duplicate_manifest),
            NamedAssetReader::new(MANIFEST_NAME, &mut duplicate_other),
        ];
        assert!(matches!(
            read_synthetic_fixture(&id, &mut duplicate),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::DuplicateAsset(MANIFEST_NAME)
            ))
        ));

        let wrong_id = identity("0400486024DEC001", "v131072", "e0f3ccfa");
        let mut wrong_manifest = PanicReader;
        let mut wrong_payload = PanicReader;
        let mut both = [
            NamedAssetReader::new(MANIFEST_NAME, &mut wrong_manifest),
            NamedAssetReader::new(PAYLOAD_NAME, &mut wrong_payload),
        ];
        assert!(matches!(
            read_synthetic_fixture(&wrong_id, &mut both),
            Err(OwnedSyntheticAcceptanceError::Validation(
                SyntheticAcceptanceError::UnsupportedIdentity(_)
            ))
        ));
    }

    #[test]
    fn reports_stream_io_errors() {
        struct FailingReader;
        impl Read for FailingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("synthetic read failure"))
            }
        }

        let id = identity("0400486024DEC000", "v131072", "e0f3ccfa");
        let mut failing = FailingReader;
        let mut payload = Cursor::new(PAYLOAD_BYTES);
        let mut readers = [
            NamedAssetReader::new(MANIFEST_NAME, &mut failing),
            NamedAssetReader::new(PAYLOAD_NAME, &mut payload),
        ];
        let error = read_synthetic_fixture(&id, &mut readers).unwrap_err();
        assert!(matches!(
            error,
            OwnedSyntheticAcceptanceError::Io {
                name: MANIFEST_NAME,
                ..
            }
        ));
        assert_eq!(
            std::error::Error::source(&error).unwrap().to_string(),
            "synthetic read failure"
        );
    }
}
