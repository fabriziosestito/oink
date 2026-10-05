//! Mutable character state and change events.

use crate::model::{CreationPools, Rulebook};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Something that changed on the character sheet. The UI can render these
/// as notices or log lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateChange {
    ItemAdded(String),
    ItemRemoved(String),
    PerkAdded(String),
    PerkRemoved(String),
    ConditionAdded(String),
    ConditionRefreshed(String),
    ConditionRemoved(String),
    EnvironmentEntered(String),
    EnvironmentCleared(String),
    TagAdded(String),
    TagRemoved(String),
    AbilityGranted(String),
    ResourceChanged { id: String, from: i32, to: i32 },
}

impl fmt::Display for StateChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateChange::ItemAdded(id) => write!(f, "item added: {id}"),
            StateChange::ItemRemoved(id) => write!(f, "item removed: {id}"),
            StateChange::PerkAdded(id) => write!(f, "perk added: {id}"),
            StateChange::PerkRemoved(id) => write!(f, "perk removed: {id}"),
            StateChange::ConditionAdded(id) => write!(f, "condition added: {id}"),
            StateChange::ConditionRefreshed(id) => write!(f, "condition refreshed: {id}"),
            StateChange::ConditionRemoved(id) => write!(f, "condition removed: {id}"),
            StateChange::EnvironmentEntered(id) => write!(f, "environment entered: {id}"),
            StateChange::EnvironmentCleared(id) => write!(f, "environment cleared: {id}"),
            StateChange::TagAdded(id) => write!(f, "tag added: {id}"),
            StateChange::TagRemoved(id) => write!(f, "tag removed: {id}"),
            StateChange::AbilityGranted(id) => write!(f, "ability granted: {id}"),
            StateChange::ResourceChanged { id, from, to } => {
                write!(f, "resource {id}: {from} -> {to}")
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Character {
    characteristics: BTreeMap<String, i32>,
    abilities: BTreeMap<String, i32>,
    perks: BTreeSet<String>,
    conditions: BTreeMap<String, Option<u32>>,
    inventory: BTreeSet<String>,
    tags: BTreeSet<String>,
    environments: BTreeSet<String>,
    resources: BTreeMap<String, i32>,
    creation_spent: CreationSpent,
    level: u32,
    xp: u32,
    level_bonus: CreationPools,
}

/// Which creation pool a point comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreationPointKind {
    Characteristic,
    Ability,
    Perk,
}

impl CreationPointKind {
    pub fn parse(kind: &str) -> Option<Self> {
        match kind {
            "characteristic" => Some(Self::Characteristic),
            "ability" => Some(Self::Ability),
            "perk" => Some(Self::Perk),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Characteristic => "characteristic",
            Self::Ability => "ability",
            Self::Perk => "perk",
        }
    }
}

/// Points spent from each creation pool.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CreationSpent {
    pub characteristics: u32,
    pub abilities: u32,
    pub perks: u32,
}

impl Character {
    /// Build the starting character from the rulebook and apply tag grants
    /// until the sheet stops changing.
    pub fn from_starting(rulebook: &Rulebook) -> Self {
        let starting = &rulebook.starting_character;
        let mut character = Self {
            level: 1,
            ..Default::default()
        };

        for (id, definition) in &rulebook.characteristics {
            let value = starting
                .characteristics
                .get(id)
                .copied()
                .or(definition.default)
                .unwrap_or(definition.min);
            character
                .characteristics
                .insert(id.clone(), value.clamp(definition.min, definition.max));
        }

        for (id, level) in &starting.abilities {
            if rulebook.abilities.contains_key(id) {
                character.abilities.insert(id.clone(), *level);
            }
        }

        character.tags = starting.tags.iter().cloned().collect();
        character.perks = starting.perks.iter().cloned().collect();
        character.inventory = starting.inventory.iter().cloned().collect();

        for (id, definition) in &rulebook.resources {
            let max = definition.derived_max(&character.characteristics);
            let value = starting
                .resources
                .get(id)
                .copied()
                .unwrap_or(if definition.start_full {
                    max
                } else {
                    definition.min
                });
            character
                .resources
                .insert(id.clone(), value.clamp(definition.min, max));
        }

        character.apply_grants(rulebook);
        character
    }

    pub fn has_item(&self, id: &str) -> bool {
        self.inventory.contains(id)
    }

    pub fn has_perk(&self, id: &str) -> bool {
        self.perks.contains(id)
    }

    pub fn has_condition(&self, id: &str) -> bool {
        self.conditions.contains_key(id)
    }

    pub fn has_ability(&self, id: &str) -> bool {
        self.abilities.contains_key(id)
    }

    pub fn has_environment(&self, id: &str) -> bool {
        self.environments.contains(id)
    }

    pub fn has_tag(&self, rulebook: &Rulebook, tag: &str) -> bool {
        self.effective_tags(rulebook).contains(tag)
    }

    pub fn characteristic(&self, id: &str) -> Option<i32> {
        self.characteristics.get(id).copied()
    }

    pub fn ability_level(&self, id: &str) -> i32 {
        self.abilities.get(id).copied().unwrap_or(0)
    }

    pub fn resource(&self, id: &str) -> Option<i32> {
        self.resources.get(id).copied()
    }

    pub fn resource_max(&self, rulebook: &Rulebook, id: &str) -> Option<i32> {
        rulebook
            .resources
            .get(id)
            .map(|definition| definition.derived_max(&self.characteristics))
    }

    pub fn perk_ids(&self) -> impl Iterator<Item = &String> {
        self.perks.iter()
    }

    pub fn item_ids(&self) -> impl Iterator<Item = &String> {
        self.inventory.iter()
    }

    pub fn condition_ids(&self) -> impl Iterator<Item = &String> {
        self.conditions.keys()
    }

    pub fn environment_ids(&self) -> impl Iterator<Item = &String> {
        self.environments.iter()
    }

    pub fn ability_ids(&self) -> impl Iterator<Item = &String> {
        self.abilities.keys()
    }

    pub fn tag_ids(&self) -> impl Iterator<Item = &String> {
        self.tags.iter()
    }

    /// The tags of the character: the ones on the sheet, plus the tags
    /// granted by perks, conditions, and carried items.
    pub fn effective_tags(&self, rulebook: &Rulebook) -> BTreeSet<String> {
        let mut tags = self.tags.clone();
        for id in &self.perks {
            if let Some(perk) = rulebook.perks.get(id) {
                tags.extend(perk.grants_tags.iter().cloned());
            }
        }
        for id in self.conditions.keys() {
            if let Some(condition) = rulebook.conditions.get(id) {
                tags.extend(condition.grants_tags.iter().cloned());
            }
        }
        for id in &self.inventory {
            if let Some(item) = rulebook.items.get(id) {
                tags.extend(item.tags.iter().cloned());
            }
        }
        tags
    }

    /// Apply abilities and perks granted by the tags the character has.
    /// Grants apply once. Removing a tag does not take a grant back.
    ///
    /// The loop runs until the sheet stops changing. Every pass that changes
    /// the sheet adds at least one ability or perk, so the pass count is
    /// bounded by the rulebook definitions.
    pub fn apply_grants(&mut self, rulebook: &Rulebook) -> Vec<StateChange> {
        let mut changes = Vec::new();
        loop {
            let tags = self.effective_tags(rulebook);
            let mut changed = false;
            for tag in &tags {
                let Some(definition) = rulebook.tags.get(tag) else {
                    continue;
                };
                for ability in &definition.grants.abilities {
                    if rulebook.abilities.contains_key(ability)
                        && !self.abilities.contains_key(ability)
                    {
                        self.abilities.insert(ability.clone(), 0);
                        changes.push(StateChange::AbilityGranted(ability.clone()));
                        changed = true;
                    }
                }
                for perk in &definition.grants.perks {
                    if rulebook.perks.contains_key(perk) && self.perks.insert(perk.clone()) {
                        changes.push(StateChange::PerkAdded(perk.clone()));
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        changes
    }

    pub fn add_item(&mut self, rulebook: &Rulebook, id: &str) -> Vec<StateChange> {
        let mut changes = Vec::new();
        if self.inventory.insert(id.to_string()) {
            changes.push(StateChange::ItemAdded(id.to_string()));
        }
        changes.extend(self.apply_grants(rulebook));
        changes
    }

    pub fn remove_item(&mut self, id: &str) -> Vec<StateChange> {
        if self.inventory.remove(id) {
            vec![StateChange::ItemRemoved(id.to_string())]
        } else {
            Vec::new()
        }
    }

    pub fn add_perk(&mut self, rulebook: &Rulebook, id: &str) -> Vec<StateChange> {
        let mut changes = Vec::new();
        if self.perks.insert(id.to_string()) {
            changes.push(StateChange::PerkAdded(id.to_string()));
        }
        changes.extend(self.apply_grants(rulebook));
        changes
    }

    pub fn remove_perk(&mut self, id: &str) -> Vec<StateChange> {
        if self.perks.remove(id) {
            vec![StateChange::PerkRemoved(id.to_string())]
        } else {
            Vec::new()
        }
    }

    /// Add a condition with the duration from the rulebook, if any.
    pub fn add_condition(&mut self, rulebook: &Rulebook, id: &str) -> Vec<StateChange> {
        let duration = rulebook.conditions.get(id).and_then(|c| c.duration);
        self.add_timed_condition(rulebook, id, duration)
    }

    /// Add a condition with an explicit duration. `None` means the condition
    /// stays until the story removes it. Re-adding a condition with a
    /// different duration emits a refresh record.
    pub fn add_timed_condition(
        &mut self,
        rulebook: &Rulebook,
        id: &str,
        duration: Option<u32>,
    ) -> Vec<StateChange> {
        let mut changes = Vec::new();
        match self.conditions.insert(id.to_string(), duration) {
            None => changes.push(StateChange::ConditionAdded(id.to_string())),
            Some(previous) if previous != duration => {
                changes.push(StateChange::ConditionRefreshed(id.to_string()));
            }
            Some(_) => {}
        }
        changes.extend(self.apply_grants(rulebook));
        changes
    }

    pub fn remove_condition(&mut self, id: &str) -> Vec<StateChange> {
        if self.conditions.remove(id).is_some() {
            vec![StateChange::ConditionRemoved(id.to_string())]
        } else {
            Vec::new()
        }
    }

    pub fn enter_environment(&mut self, rulebook: &Rulebook, id: &str) -> Vec<StateChange> {
        let mut changes = Vec::new();
        if self.environments.insert(id.to_string()) {
            changes.push(StateChange::EnvironmentEntered(id.to_string()));
        }
        changes.extend(self.apply_grants(rulebook));
        changes
    }

    pub fn clear_environment(&mut self, id: &str) -> Vec<StateChange> {
        if self.environments.remove(id) {
            vec![StateChange::EnvironmentCleared(id.to_string())]
        } else {
            Vec::new()
        }
    }

    pub fn add_tag(&mut self, rulebook: &Rulebook, tag: &str) -> Vec<StateChange> {
        let mut changes = Vec::new();
        if self.tags.insert(tag.to_string()) {
            changes.push(StateChange::TagAdded(tag.to_string()));
        }
        changes.extend(self.apply_grants(rulebook));
        changes
    }

    pub fn remove_tag(&mut self, tag: &str) -> Vec<StateChange> {
        if self.tags.remove(tag) {
            vec![StateChange::TagRemoved(tag.to_string())]
        } else {
            Vec::new()
        }
    }

    /// Build a fresh sheet for character creation from the creation base.
    /// Characteristics and abilities come from the base, falling back to
    /// definition defaults. Everything else starts empty, resources start
    /// full, and spent counters start at zero.
    pub fn begin_creation(rulebook: &Rulebook) -> Self {
        let mut character = Self {
            level: 1,
            ..Default::default()
        };
        let base = rulebook.creation.as_ref().map(|creation| &creation.base);

        for (id, definition) in &rulebook.characteristics {
            let value = base
                .and_then(|base| base.characteristics.get(id).copied())
                .or(definition.default)
                .unwrap_or(definition.min);
            character
                .characteristics
                .insert(id.clone(), value.clamp(definition.min, definition.max));
        }

        if let Some(base) = base {
            for (id, level) in &base.abilities {
                if rulebook.abilities.contains_key(id) {
                    character.abilities.insert(id.clone(), *level);
                }
            }
        }

        for (id, definition) in &rulebook.resources {
            let max = definition.derived_max(&character.characteristics);
            let value = if definition.start_full {
                max
            } else {
                definition.min
            };
            character
                .resources
                .insert(id.clone(), value.clamp(definition.min, max));
        }

        character
    }

    /// Start creation over: rebuild the sheet from the creation base.
    pub fn reset_character(&mut self, rulebook: &Rulebook) {
        *self = Self::begin_creation(rulebook);
    }

    /// Points left in a creation pool, including level rewards. Zero
    /// without a creation section.
    pub fn points_available(&self, rulebook: &Rulebook, kind: CreationPointKind) -> u32 {
        let Some(creation) = rulebook.creation.as_ref() else {
            return 0;
        };
        let (pool, bonus, spent) = match kind {
            CreationPointKind::Characteristic => (
                creation.pools.characteristic_points,
                self.level_bonus.characteristic_points,
                self.creation_spent.characteristics,
            ),
            CreationPointKind::Ability => (
                creation.pools.ability_points,
                self.level_bonus.ability_points,
                self.creation_spent.abilities,
            ),
            CreationPointKind::Perk => (
                creation.pools.perk_points,
                self.level_bonus.perk_points,
                self.creation_spent.perks,
            ),
        };
        pool.saturating_add(bonus).saturating_sub(spent)
    }

    /// True when every requirement for an id holds. Ids without an entry
    /// always pass. Characteristic thresholds read stored values, abilities
    /// read owned levels (0 when absent), resources read current balances.
    pub fn meets_prerequisite(&self, rulebook: &Rulebook, id: &str) -> bool {
        let Some(entry) = rulebook.prerequisites.get(id) else {
            return true;
        };
        entry
            .requires
            .characteristics
            .iter()
            .all(|(id, need)| self.characteristics.get(id).copied().unwrap_or(0) >= *need)
            && entry
                .requires
                .abilities
                .iter()
                .all(|(id, need)| self.ability_level(id) >= *need)
            && entry
                .requires
                .resources
                .iter()
                .all(|(id, need)| self.resource(id).unwrap_or(0) >= *need)
    }

    /// Spend pool points and apply one pick. Characteristics and abilities
    /// rise by one for the flat cost, perks are granted for one point.
    /// Returns false without changing anything when points run out or the
    /// target is invalid. Grant side effects still apply on success.
    pub fn spend_point(&mut self, rulebook: &Rulebook, kind: CreationPointKind, id: &str) -> bool {
        let Some(creation) = rulebook.creation.as_ref() else {
            return false;
        };
        if !self.meets_prerequisite(rulebook, id) {
            return false;
        }
        match kind {
            CreationPointKind::Characteristic => {
                let Some(definition) = rulebook.characteristics.get(id) else {
                    return false;
                };
                let cost = creation.costs.characteristics;
                if self.points_available(rulebook, kind) < cost {
                    return false;
                }
                let value = self
                    .characteristics
                    .get(id)
                    .copied()
                    .unwrap_or(definition.min);
                if value >= definition.max {
                    return false;
                }
                self.characteristics
                    .insert(id.to_string(), (value + 1).min(definition.max));
                self.creation_spent.characteristics =
                    self.creation_spent.characteristics.saturating_add(cost);
                true
            }
            CreationPointKind::Ability => {
                if !rulebook.abilities.contains_key(id) {
                    return false;
                }
                let cost = creation.costs.abilities;
                if self.points_available(rulebook, kind) < cost {
                    return false;
                }
                let level = self.abilities.get(id).copied().unwrap_or(0);
                self.abilities.insert(id.to_string(), level + 1);
                self.creation_spent.abilities = self.creation_spent.abilities.saturating_add(cost);
                true
            }
            CreationPointKind::Perk => {
                if !rulebook.perks.contains_key(id) || self.perks.contains(id) {
                    return false;
                }
                if self.points_available(rulebook, kind) < 1 {
                    return false;
                }
                self.add_perk(rulebook, id);
                self.creation_spent.perks = self.creation_spent.perks.saturating_add(1);
                true
            }
        }
    }

    /// Apply a preset as the complete sheet: reset to the creation base,
    /// overlay preset values, add qualifying perks and tags, refresh derived
    /// resource maxima, and mark every pool spent. Preset perks whose
    /// prerequisites the preset sheet does not meet are skipped. Returns
    /// false for unknown presets or without a creation section.
    pub fn apply_preset(&mut self, rulebook: &Rulebook, id: &str) -> bool {
        let Some(creation) = rulebook.creation.as_ref() else {
            return false;
        };
        let Some(preset) = creation.presets.get(id).cloned() else {
            return false;
        };
        *self = Self::begin_creation(rulebook);
        for (id, value) in &preset.characteristics {
            if let Some(definition) = rulebook.characteristics.get(id) {
                self.characteristics
                    .insert(id.clone(), (*value).clamp(definition.min, definition.max));
            }
        }
        for (id, level) in &preset.abilities {
            if rulebook.abilities.contains_key(id) {
                self.abilities.insert(id.clone(), *level);
            }
        }
        for id in &preset.perks {
            if rulebook.perks.contains_key(id) && self.meets_prerequisite(rulebook, id) {
                self.add_perk(rulebook, id);
            }
        }
        for tag in &preset.tags {
            self.add_tag(rulebook, tag);
        }
        for (id, definition) in &rulebook.resources {
            let max = definition.derived_max(&self.characteristics);
            let value = if definition.start_full {
                max
            } else {
                self.resources.get(id).copied().unwrap_or(definition.min)
            };
            self.resources
                .insert(id.clone(), value.clamp(definition.min, max));
        }
        let pools = &rulebook
            .creation
            .as_ref()
            .expect("creation section checked above")
            .pools;
        self.creation_spent.characteristics = pools
            .characteristic_points
            .saturating_add(self.level_bonus.characteristic_points);
        self.creation_spent.abilities = pools
            .ability_points
            .saturating_add(self.level_bonus.ability_points);
        self.creation_spent.perks = pools
            .perk_points
            .saturating_add(self.level_bonus.perk_points);
        true
    }

    /// Set a stored characteristic directly, clamped to its bounds, without
    /// spending points. Lowering a value can shrink derived maxima, so
    /// balances above the new maxima clamp down silently. Returns false
    /// for unknown characteristics.
    pub fn set_characteristic(&mut self, rulebook: &Rulebook, id: &str, value: i32) -> bool {
        let Some(definition) = rulebook.characteristics.get(id) else {
            return false;
        };
        self.characteristics
            .insert(id.to_string(), value.clamp(definition.min, definition.max));
        for (id, definition) in &rulebook.resources {
            let max = definition.derived_max(&self.characteristics);
            if self.resources.get(id).copied().unwrap_or(0) > max {
                self.resources.insert(id.clone(), max);
            }
        }
        true
    }

    /// Set an ability level directly, floored at zero, without spending
    /// points. Returns false for unknown abilities.
    pub fn set_ability(&mut self, rulebook: &Rulebook, id: &str, level: i32) -> bool {
        if !rulebook.abilities.contains_key(id) {
            return false;
        }
        self.abilities.insert(id.to_string(), level.max(0));
        true
    }

    /// Check every pool, including level rewards, against its spent points.
    /// Empty means creation is complete under `validate: all_points_spent`.
    /// Stories gate on `points_available`; this is the Rust-side equivalent.
    pub fn validate_creation(&self, rulebook: &Rulebook) -> Vec<String> {
        let mut problems = Vec::new();
        let Some(creation) = rulebook.creation.as_ref() else {
            return problems;
        };
        for (kind, pool, spent) in [
            (
                CreationPointKind::Characteristic,
                creation
                    .pools
                    .characteristic_points
                    .saturating_add(self.level_bonus.characteristic_points),
                self.creation_spent.characteristics,
            ),
            (
                CreationPointKind::Ability,
                creation
                    .pools
                    .ability_points
                    .saturating_add(self.level_bonus.ability_points),
                self.creation_spent.abilities,
            ),
            (
                CreationPointKind::Perk,
                creation
                    .pools
                    .perk_points
                    .saturating_add(self.level_bonus.perk_points),
                self.creation_spent.perks,
            ),
        ] {
            if spent < pool {
                problems.push(format!(
                    "{}: {}/{} points spent",
                    kind.as_str(),
                    spent,
                    pool
                ));
            }
        }
        problems
    }

    /// Current level. Sheets start at 1.
    pub fn level(&self) -> u32 {
        self.level
    }

    /// Banked experience. Grows only through `add_xp`.
    pub fn xp(&self) -> u32 {
        self.xp
    }

    /// Bank experience without levelling. Level-ups happen one at a time
    /// through `level_up`, so stories control when the sheet changes.
    pub fn add_xp(&mut self, amount: u32) {
        self.xp = self.xp.saturating_add(amount);
    }

    /// True when banked XP reaches the next curve level below the cap.
    pub fn level_up_ready(&self, rulebook: &Rulebook) -> bool {
        let Some((_, need)) = Self::next_threshold(rulebook, self.level) else {
            return false;
        };
        self.xp >= need
    }

    /// Rise one level when ready, granting per-level rewards plus the
    /// interval perk points when the new level hits the interval.
    /// Returns false without changing anything otherwise.
    pub fn level_up(&mut self, rulebook: &Rulebook) -> bool {
        let Some(levelling) = rulebook.levelling.as_ref() else {
            return false;
        };
        if self.level >= levelling.max_level {
            return false;
        }
        if !self.level_up_ready(rulebook) {
            return false;
        }
        self.level += 1;
        let rewards = &levelling.rewards;
        self.level_bonus.characteristic_points = self
            .level_bonus
            .characteristic_points
            .saturating_add(rewards.per_level.characteristic_points);
        self.level_bonus.ability_points = self
            .level_bonus
            .ability_points
            .saturating_add(rewards.per_level.ability_points);
        if let Some(interval) = &rewards.interval {
            if interval.every > 0 && self.level.is_multiple_of(interval.every) {
                self.level_bonus.perk_points = self
                    .level_bonus
                    .perk_points
                    .saturating_add(interval.perk_points);
            }
        }
        true
    }

    /// The curve entry for the level right above the given one, if any.
    /// Each threshold applies only to the level it names: a curve without
    /// an entry for the next level blocks progression instead of borrowing
    /// a later threshold.
    fn next_threshold(rulebook: &Rulebook, level: u32) -> Option<(u32, u32)> {
        let levelling = rulebook.levelling.as_ref()?;
        if level >= levelling.max_level {
            return None;
        }
        let next = level.saturating_add(1);
        levelling
            .xp_curve
            .iter()
            .find(|entry| entry.level == next)
            .map(|entry| (entry.level, entry.xp))
    }

    pub fn spend_resource(
        &mut self,
        rulebook: &Rulebook,
        id: &str,
        amount: i32,
    ) -> Vec<StateChange> {
        if !self.can_spend_resource(rulebook, id, amount) {
            return Vec::new();
        }
        self.change_resource(rulebook, id, -amount)
    }

    /// A payment must fit above the resource floor. Negative amounts are invalid.
    pub fn can_spend_resource(&self, rulebook: &Rulebook, id: &str, amount: i32) -> bool {
        let Some(definition) = rulebook.resources.get(id) else {
            return false;
        };
        self.resource(id).is_some_and(|current| {
            amount >= 0 && i64::from(current) - i64::from(amount) >= i64::from(definition.min)
        })
    }

    pub fn restore_resource(
        &mut self,
        rulebook: &Rulebook,
        id: &str,
        amount: i32,
    ) -> Vec<StateChange> {
        if amount < 0 {
            return Vec::new();
        }
        self.change_resource(rulebook, id, amount)
    }

    /// Count one scene and expire timed conditions. Call this when the story
    /// moves from one scene to the next.
    pub fn on_scene_end(&mut self) -> Vec<StateChange> {
        let mut expired = Vec::new();
        for (id, remaining) in self.conditions.iter_mut() {
            if let Some(scenes) = remaining {
                if *scenes <= 1 {
                    expired.push(id.clone());
                } else {
                    *scenes -= 1;
                }
            }
        }
        let mut changes = Vec::new();
        for id in expired {
            self.conditions.remove(&id);
            changes.push(StateChange::ConditionRemoved(id));
        }
        changes
    }

    fn change_resource(&mut self, rulebook: &Rulebook, id: &str, delta: i32) -> Vec<StateChange> {
        let Some(definition) = rulebook.resources.get(id) else {
            return Vec::new();
        };
        let Some(current) = self.resources.get(id).copied() else {
            return Vec::new();
        };
        let ceiling = i64::from(definition.derived_max(&self.characteristics));
        let next = (i64::from(current) + i64::from(delta)).clamp(i64::from(definition.min), ceiling)
            as i32;
        if next == current {
            return Vec::new();
        }
        self.resources.insert(id.to_string(), next);
        vec![StateChange::ResourceChanged {
            id: id.to_string(),
            from: current,
            to: next,
        }]
    }
}
