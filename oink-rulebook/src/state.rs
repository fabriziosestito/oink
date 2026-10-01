//! Mutable character state and change events.

use crate::model::Rulebook;
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
}

impl Character {
    /// Build the starting character from the rulebook and apply tag grants
    /// until the sheet stops changing.
    pub fn from_starting(rulebook: &Rulebook) -> Self {
        let starting = &rulebook.starting_character;
        let mut character = Self::default();

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
        let next = (i64::from(current) + i64::from(delta)).clamp(
            i64::from(definition.min),
            i64::from(definition.derived_max(&self.characteristics)),
        ) as i32;
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
