//! Renameable display labels.
//!
//! Internal section keys stay stable, while the `names` block in
//! `rulebook.yaml` sets what the UI and the story show.

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Names {
    pub characteristics: String,
    pub abilities: String,
    pub perks: String,
    pub conditions: String,
    pub items: String,
    pub resources: String,
    pub spells: String,
    pub tags: String,
    pub environments: String,
}

impl Default for Names {
    fn default() -> Self {
        Self {
            characteristics: "Characteristics".to_string(),
            abilities: "Abilities".to_string(),
            perks: "Perks".to_string(),
            conditions: "Conditions".to_string(),
            items: "Items".to_string(),
            resources: "Resources".to_string(),
            spells: "Spells".to_string(),
            tags: "Tags".to_string(),
            environments: "Environments".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Characteristics,
    Abilities,
    Perks,
    Conditions,
    Items,
    Resources,
    Spells,
    Tags,
    Environments,
}

impl Names {
    pub fn label(&self, section: Section) -> &str {
        match section {
            Section::Characteristics => &self.characteristics,
            Section::Abilities => &self.abilities,
            Section::Perks => &self.perks,
            Section::Conditions => &self.conditions,
            Section::Items => &self.items,
            Section::Resources => &self.resources,
            Section::Spells => &self.spells,
            Section::Tags => &self.tags,
            Section::Environments => &self.environments,
        }
    }
}
