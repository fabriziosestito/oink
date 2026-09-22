//! Static game data loaded from YAML: items, perks, engine config.

use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub weight: u32,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Perk {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Stat modifiers, e.g. { agility: 2 }
    #[serde(default)]
    pub modifiers: HashMap<String, i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: String,
    /// Dice expression used for skill checks, e.g. "2d6".
    #[serde(default = "default_dice")]
    pub dice: String,
    /// Starting stats, e.g. { skill: 8, stamina: 20 }
    #[serde(default)]
    pub stats: HashMap<String, i32>,
    /// Item ids the player starts with.
    #[serde(default)]
    pub starting_inventory: Vec<String>,
}

fn default_dice() -> String {
    "2d6".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameData {
    pub config: Config,
    #[serde(default)]
    pub items: HashMap<String, Item>,
    #[serde(default)]
    pub perks: HashMap<String, Perk>,
}

impl GameData {
    pub fn from_yaml(config: &str, items: &str, perks: &str) -> Result<Self, serde_yaml::Error> {
        Ok(Self {
            config: serde_yaml::from_str(config)?,
            items: serde_yaml::from_str(items)?,
            perks: serde_yaml::from_str(perks)?,
        })
    }
}
