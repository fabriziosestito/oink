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

/// How a characteristic contributes to a check.
///
/// A threshold table converts the stored value through `BonusTable`.
/// The `direct` marker passes the stored value untouched, for percentile
/// systems where the characteristic is the target base.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum CharacteristicBonus {
    Direct(DirectMarker),
    Table(BonusTable),
}

impl Default for CharacteristicBonus {
    fn default() -> Self {
        Self::Table(BonusTable::default())
    }
}

impl CharacteristicBonus {
    pub fn bonus_for(&self, value: i32) -> i32 {
        match self {
            Self::Direct(_) => value,
            Self::Table(table) => table.bonus_for(value),
        }
    }

    pub fn is_empty_table(&self) -> bool {
        match self {
            Self::Direct(_) => false,
            Self::Table(table) => table.thresholds.is_empty(),
        }
    }

    pub fn sort_thresholds(&mut self) {
        if let Self::Table(table) = self {
            table
                .thresholds
                .sort_by_key(|threshold| std::cmp::Reverse(threshold.at));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectMarker {
    Direct,
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
    pub bonus: CharacteristicBonus,
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
    #[serde(default)]
    pub advantage: bool,
    #[serde(default)]
    pub disadvantage: bool,
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
    #[serde(default)]
    pub advantage: bool,
    #[serde(default)]
    pub disadvantage: bool,
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
    #[serde(default)]
    pub max: i32,
    #[serde(default = "default_true")]
    pub start_full: bool,
    #[serde(default)]
    pub max_from: Vec<MaxFromEntry>,
}

impl Resource {
    /// The effective maximum: the plain `max` when no derivation is
    /// configured, otherwise the sum of every `max_from` entry. Derivation
    /// reads stored characteristic values only, so temporary conditions
    /// never move resource maxima. The result never drops below `min`, so
    /// every caller shares one bound.
    pub fn derived_max(&self, characteristics: &BTreeMap<String, i32>) -> i32 {
        if self.max_from.is_empty() {
            return self.max.max(self.min);
        }
        let mut total: i64 = 0;
        for entry in &self.max_from {
            let value = characteristics
                .get(&entry.characteristic)
                .copied()
                .unwrap_or(0);
            let part: i64 = match entry.mode {
                MaxFromMode::Thresholds => entry
                    .thresholds
                    .iter()
                    .find(|threshold| value >= threshold.at)
                    .or_else(|| entry.thresholds.last())
                    .map(|threshold| i64::from(threshold.value))
                    .unwrap_or(0),
                MaxFromMode::PerPoint => {
                    i64::from(entry.base) + i64::from(entry.value_per_point) * i64::from(value)
                }
            };
            total = total.saturating_add(part);
        }
        (total.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32).max(self.min)
    }

    pub fn sort_thresholds(&mut self) {
        for entry in &mut self.max_from {
            entry
                .thresholds
                .sort_by_key(|threshold| std::cmp::Reverse(threshold.at));
        }
    }
}

/// One characteristic contribution to a derived resource maximum.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MaxFromEntry {
    pub characteristic: String,
    pub mode: MaxFromMode,
    #[serde(default)]
    pub thresholds: Vec<MaxFromThreshold>,
    #[serde(default)]
    pub base: i32,
    #[serde(default)]
    pub value_per_point: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaxFromMode {
    Thresholds,
    PerPoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MaxFromThreshold {
    pub at: i32,
    pub value: i32,
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

/// Which side of the comparison the character contribution joins.
///
/// `Over` adds dice plus contribution against difficulty.
/// `Under` rolls dice alone against contribution plus difficulty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Over,
    Under,
}

/// How to compute degrees (margin of success) for a check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Degrees {
    #[default]
    Margin,
    Tens,
    None,
}

/// One die size supported by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Die {
    D2,
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
    Percentile,
}

impl Die {
    pub fn sides(self) -> u16 {
        match self {
            Self::D2 => 2,
            Self::D4 => 4,
            Self::D6 => 6,
            Self::D8 => 8,
            Self::D10 => 10,
            Self::D12 => 12,
            Self::D20 => 20,
            Self::Percentile => 10,
        }
    }

    pub fn from_faces(faces: u16) -> Option<Self> {
        match faces {
            2 => Some(Self::D2),
            4 => Some(Self::D4),
            6 => Some(Self::D6),
            8 => Some(Self::D8),
            10 => Some(Self::D10),
            12 => Some(Self::D12),
            20 => Some(Self::D20),
            _ => None,
        }
    }
}

/// Which dice of a pool count toward the total.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Keep {
    #[default]
    All,
    Highest(u16),
    Lowest(u16),
}

/// A parsed dice pool, from notation such as `2d6` or `2d20kh1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DicePool {
    pub count: u16,
    pub die: Die,
    pub keep: Keep,
}

impl Default for DicePool {
    fn default() -> Self {
        Self {
            count: 2,
            die: Die::D6,
            keep: Keep::All,
        }
    }
}

/// One row of a profile outcome table. Fields in one row combine with AND.
/// The first matching row wins. A row without tests always matches.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct OutcomeRule {
    pub all_max: bool,
    pub all_min: bool,
    pub any_max: bool,
    pub any_min: bool,
    pub doubles: bool,
    pub score_at_least: Option<i32>,
    pub score_at_most: Option<i32>,
    pub margin_at_least: Option<i32>,
    pub margin_at_most: Option<i32>,
    pub degrees_at_least: Option<i32>,
    pub degrees_at_most: Option<i32>,
    pub target_at_least: Option<i32>,
    pub target_at_most: Option<i32>,
    pub degrees_min: Option<i32>,
    pub degrees_max: Option<i32>,
    pub outcome: Outcome,
}

/// Active-check outcome, with Ink-compatible snake_case names.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    CriticalFailure,
    #[default]
    Failure,
    Success,
    CriticalSuccess,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::CriticalFailure => "critical_failure",
            Outcome::Failure => "failure",
            Outcome::Success => "success",
            Outcome::CriticalSuccess => "critical_success",
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Outcome::Success | Outcome::CriticalSuccess)
    }
}

/// One named dice profile: notation plus direction, degrees, passive,
/// advantage pools, and the ordered outcome table.
#[derive(Debug, Clone, Deserialize)]
pub struct DiceProfile {
    pub notation: String,
    #[serde(default)]
    pub direction: Direction,
    #[serde(default)]
    pub degrees: Degrees,
    #[serde(default)]
    pub passive: Option<i32>,
    #[serde(default)]
    pub advantage: Option<String>,
    #[serde(default)]
    pub disadvantage: Option<String>,
    #[serde(skip, default)]
    pub pool: DicePool,
    #[serde(default)]
    pub outcomes: Vec<OutcomeRule>,
}

/// Dice section of the rulebook: default profile plus named profiles.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DiceConfig {
    pub default: String,
    pub profiles: BTreeMap<String, DiceProfile>,
}

impl Default for DiceConfig {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "standard".to_string(),
            DiceProfile {
                notation: "2d6".to_string(),
                direction: Direction::Over,
                degrees: Degrees::Margin,
                passive: Some(6),
                advantage: None,
                disadvantage: None,
                pool: DicePool {
                    count: 2,
                    die: Die::D6,
                    keep: Keep::All,
                },
                outcomes: vec![
                    OutcomeRule {
                        all_max: true,
                        outcome: Outcome::CriticalSuccess,
                        ..Default::default()
                    },
                    OutcomeRule {
                        all_min: true,
                        outcome: Outcome::CriticalFailure,
                        ..Default::default()
                    },
                    OutcomeRule {
                        margin_at_least: Some(5),
                        outcome: Outcome::CriticalSuccess,
                        ..Default::default()
                    },
                    OutcomeRule {
                        margin_at_least: Some(0),
                        outcome: Outcome::Success,
                        ..Default::default()
                    },
                    OutcomeRule {
                        margin_at_least: Some(-4),
                        outcome: Outcome::Failure,
                        ..Default::default()
                    },
                    OutcomeRule {
                        outcome: Outcome::CriticalFailure,
                        ..Default::default()
                    },
                ],
            },
        );
        Self {
            default: "standard".to_string(),
            profiles,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreationMode {
    #[default]
    Pool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CreationValidation {
    #[default]
    AllPointsSpent,
}

/// Free placement applied before creation points are spent.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct CreationBase {
    pub characteristics: BTreeMap<String, i32>,
    pub abilities: BTreeMap<String, i32>,
}

/// Points to spend on top of the base.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct CreationPools {
    pub characteristic_points: u32,
    pub ability_points: u32,
    pub perk_points: u32,
}

fn default_point_cost() -> u32 {
    1
}

/// Flat point costs (see Q1: flat for v1, rising costs stay an open question).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreationCosts {
    #[serde(default = "default_point_cost")]
    pub characteristics: u32,
    #[serde(default = "default_point_cost")]
    pub abilities: u32,
}

impl Default for CreationCosts {
    fn default() -> Self {
        Self {
            characteristics: 1,
            abilities: 1,
        }
    }
}

/// A named starting sheet. The story can apply it and skip point spending.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreationPreset {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub characteristics: BTreeMap<String, i32>,
    #[serde(default)]
    pub abilities: BTreeMap<String, i32>,
    #[serde(default)]
    pub perks: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Data-driven character creation. Absent means stories use
/// `starting_character` directly and creation functions stay unused.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CharacterCreation {
    #[serde(default)]
    pub mode: CreationMode,
    #[serde(default)]
    pub base: CreationBase,
    #[serde(default)]
    pub pools: CreationPools,
    #[serde(default)]
    pub costs: CreationCosts,
    #[serde(default)]
    pub validate: CreationValidation,
    #[serde(default)]
    pub presets: BTreeMap<String, CreationPreset>,
}

/// Minimum values gating one perk, ability, item, or spell by id.
/// Characteristics read stored values, abilities read owned levels
/// (0 when absent), resources read current balances.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Requirement {
    pub characteristics: BTreeMap<String, i32>,
    pub abilities: BTreeMap<String, i32>,
    pub resources: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Prerequisite {
    pub requires: Requirement,
}

/// XP needed to reach one level.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LevelThreshold {
    pub level: u32,
    pub xp: u32,
}

/// Points granted at every level.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct LevelPoints {
    pub characteristic_points: u32,
    pub ability_points: u32,
}

/// Extra points granted every N levels. Absent means no interval rewards.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct LevelInterval {
    pub every: u32,
    pub perk_points: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct LevelRewards {
    pub per_level: LevelPoints,
    #[serde(default)]
    pub interval: Option<LevelInterval>,
}

/// XP curve, level cap, and rewards. Absent means no levelling: XP banks
/// without effect and level checks stay false.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Levelling {
    pub max_level: u32,
    #[serde(default)]
    pub xp_curve: Vec<LevelThreshold>,
    #[serde(default)]
    pub rewards: LevelRewards,
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
    pub dice: DiceConfig,
    pub starting_character: StartingCharacter,
    #[serde(default, rename = "character_creation")]
    pub creation: Option<CharacterCreation>,
    #[serde(default)]
    pub levelling: Option<Levelling>,
    #[serde(default)]
    pub prerequisites: BTreeMap<String, Prerequisite>,
}
