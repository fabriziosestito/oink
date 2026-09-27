//! Resource definitions loaded from `rulebook.yaml`.

use crate::names::Names;
use serde::Deserialize;
use std::collections::BTreeMap;

/// A set of flat modifiers to characteristics, abilities, and check tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Modifiers {
    pub characteristics: BTreeMap<String, i32>,
    pub abilities: BTreeMap<String, i32>,
    pub tags: BTreeMap<String, i32>,
}

impl Modifiers {
    pub fn is_empty(&self) -> bool {
        self.characteristics.is_empty() && self.abilities.is_empty() && self.tags.is_empty()
    }
}

/// A configurable bonus table for a characteristic.
///
/// Thresholds are sorted by the loader. `bonus_for` picks the highest
/// threshold that the value reaches and clamps at both ends.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct BonusTable {
    pub thresholds: Vec<BonusThreshold>,
}

impl BonusTable {
    pub fn bonus_for(&self, value: i32) -> i32 {
        self.thresholds
            .iter()
            .find(|threshold| value >= threshold.at)
            .or_else(|| self.thresholds.last())
            .map(|threshold| threshold.bonus)
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BonusThreshold {
    pub at: i32,
    pub bonus: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Characteristic {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_min")]
    pub min: i32,
    #[serde(default = "default_max")]
    pub max: i32,
    #[serde(default)]
    pub default: Option<i32>,
    #[serde(default)]
    pub bonus: BonusTable,
}

impl Characteristic {
    pub fn bonus_for(&self, value: i32) -> i32 {
        self.bonus.bonus_for(value)
    }
}

fn default_min() -> i32 {
    1
}

fn default_max() -> i32 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct Ability {
    pub name: String,
    pub characteristic: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub grants: TagGrants,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct TagGrants {
    pub abilities: Vec<String>,
    pub perks: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Perk {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub modifiers: Modifiers,
    #[serde(default)]
    pub grants_tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Condition {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub modifiers: Modifiers,
    #[serde(default)]
    pub grants_tags: Vec<String>,
    #[serde(default)]
    pub duration: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Environment {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub weight: u32,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub modifiers: Modifiers,
    #[serde(default)]
    pub consumable: bool,
    #[serde(default)]
    pub applies_conditions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Resource {
    pub name: String,
    #[serde(default)]
    pub min: i32,
    pub max: i32,
    #[serde(default = "default_true")]
    pub start_full: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct Spell {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub ability: String,
    pub cost: Cost,
    pub check: SpellCheck,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Cost {
    pub resource: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpellCheck {
    pub difficulty: DifficultyRef,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// A difficulty written as a number or as a name from the difficulty table.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum DifficultyRef {
    Value(i32),
    Named(String),
}

impl DifficultyRef {
    pub fn resolve(&self, rulebook: &Rulebook) -> Option<i32> {
        match self {
            DifficultyRef::Value(value) => Some(*value),
            DifficultyRef::Named(name) => rulebook.difficulties.get(name).copied(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct StartingCharacter {
    pub characteristics: BTreeMap<String, i32>,
    pub abilities: BTreeMap<String, i32>,
    pub tags: Vec<String>,
    pub perks: Vec<String>,
    pub inventory: Vec<String>,
    pub resources: BTreeMap<String, i32>,
}

/// The full rulebook: every resource definition plus the starting character.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Rulebook {
    pub names: Names,
    pub difficulties: BTreeMap<String, i32>,
    pub characteristics: BTreeMap<String, Characteristic>,
    pub abilities: BTreeMap<String, Ability>,
    pub tags: BTreeMap<String, Tag>,
    pub perks: BTreeMap<String, Perk>,
    pub conditions: BTreeMap<String, Condition>,
    pub environments: BTreeMap<String, Environment>,
    pub items: BTreeMap<String, Item>,
    pub resources: BTreeMap<String, Resource>,
    pub spells: BTreeMap<String, Spell>,
    pub starting_character: StartingCharacter,
}
