use super::TweakDefinition;
use crate::StatePlanError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TweakCatalogueWire")]
pub struct TweakCatalogue {
    pub tweaks: Vec<TweakDefinition>,
}

#[derive(Debug, Deserialize)]
struct TweakCatalogueWire {
    tweaks: Vec<TweakDefinition>,
}

impl TryFrom<TweakCatalogueWire> for TweakCatalogue {
    type Error = StatePlanError;

    fn try_from(value: TweakCatalogueWire) -> Result<Self, Self::Error> {
        Self::new(value.tweaks)
    }
}

impl TweakCatalogue {
    pub fn new(tweaks: Vec<TweakDefinition>) -> Result<Self, StatePlanError> {
        let catalogue = Self { tweaks };
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn validate(&self) -> Result<(), StatePlanError> {
        let mut ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for tweak in &self.tweaks {
            tweak.validate()?;
            if !ids.insert(tweak.id.as_str()) {
                return Err(StatePlanError::DuplicateId(tweak.id.clone()));
            }
            let target = tweak.target.canonical_key()?;
            if !targets.insert(target.clone()) {
                return Err(StatePlanError::DuplicateTarget(target));
            }
        }
        Ok(())
    }

    pub fn from_json_str(input: &str) -> Result<Self, StatePlanError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn read_json(path: impl AsRef<Path>) -> Result<Self, StatePlanError> {
        Self::from_json_str(&std::fs::read_to_string(path)?)
    }

    pub fn get(&self, id: &str) -> Option<&TweakDefinition> {
        self.tweaks.iter().find(|tweak| tweak.id == id)
    }
}
