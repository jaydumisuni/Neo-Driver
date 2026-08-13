use crate::{Sha256Digest, VaultError, VaultSegment};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

pub const SOURCE_MAP_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePackageKind {
    DriverPack,
    TechnicianComponent,
    RuntimePack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverSource {
    pub id: VaultSegment,
    pub family: VaultSegment,
    pub kind: SourcePackageKind,
    pub repository: String,
    pub release_tag: String,
    pub asset_name: String,
    pub sha256: Sha256Digest,
    pub size_bytes: u64,
}

impl DriverSource {
    pub fn validate(&self) -> Result<(), VaultError> {
        require_text("repository", &self.repository)?;
        require_text("release_tag", &self.release_tag)?;
        require_text("asset_name", &self.asset_name)?;
        if self.repository.matches('/').count() != 1
            || self.repository.starts_with('/')
            || self.repository.ends_with('/')
            || self.repository.chars().any(char::is_whitespace)
        {
            return Err(VaultError::InvalidRepository(self.repository.clone()));
        }
        if self.asset_name.contains('/')
            || self.asset_name.contains('\\')
            || self.asset_name == "."
            || self.asset_name == ".."
        {
            return Err(VaultError::InvalidSegment(self.asset_name.clone()));
        }
        Ok(())
    }

    pub fn asset_identity(&self) -> String {
        format!(
            "{}@{}:{}",
            self.repository.to_ascii_lowercase(),
            self.release_tag.to_ascii_lowercase(),
            self.asset_name.to_ascii_lowercase()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriverSourceMap {
    pub schema_version: u32,
    pub sources: Vec<DriverSource>,
}

impl DriverSourceMap {
    pub fn new(sources: Vec<DriverSource>) -> Result<Self, VaultError> {
        let value = Self {
            schema_version: SOURCE_MAP_SCHEMA_VERSION,
            sources,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), VaultError> {
        if self.schema_version != SOURCE_MAP_SCHEMA_VERSION {
            return Err(VaultError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.sources.is_empty() {
            return Err(VaultError::EmptySourceMap);
        }
        let mut ids = BTreeSet::new();
        let mut assets = BTreeSet::new();
        for source in &self.sources {
            source.validate()?;
            let id = source.id.as_str().to_ascii_lowercase();
            if !ids.insert(id.clone()) {
                return Err(VaultError::DuplicateSourceId(id));
            }
            let asset = source.asset_identity();
            if !assets.insert(asset.clone()) {
                return Err(VaultError::DuplicateSourceAsset(asset));
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, VaultError> {
        let wire: SourceMapWire = serde_json::from_str(input)?;
        wire.try_into()
    }
}

#[derive(Debug, Deserialize)]
struct SourceMapWire {
    schema_version: u32,
    sources: Vec<DriverSource>,
}

impl TryFrom<SourceMapWire> for DriverSourceMap {
    type Error = VaultError;

    fn try_from(wire: SourceMapWire) -> Result<Self, Self::Error> {
        let value = Self {
            schema_version: wire.schema_version,
            sources: wire.sources,
        };
        value.validate()?;
        Ok(value)
    }
}

impl<'de> Deserialize<'de> for DriverSourceMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SourceMapWire::deserialize(deserializer)?;
        wire.try_into().map_err(D::Error::custom)
    }
}

fn require_text(field: &'static str, value: &str) -> Result<(), VaultError> {
    if value.trim().is_empty() {
        Err(VaultError::BlankSourceField(field))
    } else {
        Ok(())
    }
}
