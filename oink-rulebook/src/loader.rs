//! YAML loading and validation.

use crate::model::{DifficultyRef, Rulebook};
use std::fmt;

#[derive(Debug)]
pub enum LoadError {
    Yaml(serde_yaml::Error),
    Validation(Vec<String>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Yaml(error) => write!(f, "failed to parse rulebook YAML: {error}"),
            LoadError::Validation(errors) => {
                write!(f, "invalid rulebook: {}", errors.join("; "))
            }
        }
    }
}

impl std::error::Error for LoadError {}

impl From<serde_yaml::Error> for LoadError {
    fn from(error: serde_yaml::Error) -> Self {
        LoadError::Yaml(error)
    }
}

/// A loaded rulebook plus the warnings from validation.
#[derive(Debug)]
pub struct Loaded {
    pub rulebook: Rulebook,
    pub warnings: Vec<String>,
}

impl Rulebook {
    pub fn load(source: &str) -> Result<Loaded, LoadError> {
        let mut rulebook: Rulebook = serde_yaml::from_str(source)?;
        rulebook.normalize();
        let mut errors = Vec::new();
        let warnings = rulebook.validate(&mut errors);
        if errors.is_empty() {
            Ok(Loaded { rulebook, warnings })
        } else {
            Err(LoadError::Validation(errors))
        }
    }

    fn normalize(&mut self) {
        for characteristic in self.characteristics.values_mut() {
            characteristic
                .bonus
                .thresholds
                .sort_by_key(|threshold| std::cmp::Reverse(threshold.at));
        }
    }

    fn validate(&self, errors: &mut Vec<String>) -> Vec<String> {
        let mut warnings = Vec::new();

        for (id, characteristic) in &self.characteristics {
            if characteristic.min > characteristic.max {
                errors.push(format!(
                    "characteristic `{id}`: min {} is greater than max {}",
                    characteristic.min, characteristic.max
                ));
            }
            if let Some(default) = characteristic.default {
                if default < characteristic.min || default > characteristic.max {
                    errors.push(format!(
                        "characteristic `{id}`: default {default} is outside {}..={}",
                        characteristic.min, characteristic.max
                    ));
                }
            }
            if characteristic.bonus.thresholds.is_empty() {
                warnings.push(format!(
                    "characteristic `{id}` has no bonus thresholds, so checks add 0"
                ));
            }
        }

        for (id, ability) in &self.abilities {
            if !self.characteristics.contains_key(&ability.characteristic) {
                errors.push(format!(
                    "ability `{id}` references unknown characteristic `{}`",
                    ability.characteristic
                ));
            }
            for tag in &ability.tags {
                self.warn_tag(&mut warnings, tag, &format!("ability `{id}`"));
            }
        }

        for (id, tag) in &self.tags {
            for ability in &tag.grants.abilities {
                if !self.abilities.contains_key(ability) {
                    errors.push(format!("tag `{id}` grants unknown ability `{ability}`"));
                }
            }
            for perk in &tag.grants.perks {
                if !self.perks.contains_key(perk) {
                    errors.push(format!("tag `{id}` grants unknown perk `{perk}`"));
                }
            }
        }

        for (id, perk) in &self.perks {
            for tag in &perk.grants_tags {
                self.warn_tag(&mut warnings, tag, &format!("perk `{id}`"));
            }
            for tag in perk.modifiers.tags.keys() {
                self.warn_tag(&mut warnings, tag, &format!("perk `{id}`"));
            }
        }

        for (id, condition) in &self.conditions {
            for tag in &condition.grants_tags {
                self.warn_tag(&mut warnings, tag, &format!("condition `{id}`"));
            }
            for tag in condition.modifiers.tags.keys() {
                self.warn_tag(&mut warnings, tag, &format!("condition `{id}`"));
            }
        }

        for (id, environment) in &self.environments {
            for tag in &environment.tags {
                self.warn_tag(&mut warnings, tag, &format!("environment `{id}`"));
            }
            for tag in environment.modifiers.tags.keys() {
                self.warn_tag(&mut warnings, tag, &format!("environment `{id}`"));
            }
        }

        for (id, item) in &self.items {
            for condition in &item.applies_conditions {
                if !self.conditions.contains_key(condition) {
                    errors.push(format!(
                        "item `{id}` applies unknown condition `{condition}`"
                    ));
                }
            }
            for tag in item.modifiers.tags.keys() {
                self.warn_tag(&mut warnings, tag, &format!("item `{id}`"));
            }
        }

        for (id, resource) in &self.resources {
            if resource.min > resource.max {
                errors.push(format!(
                    "resource `{id}`: min {} is greater than max {}",
                    resource.min, resource.max
                ));
            }
        }

        for (id, spell) in &self.spells {
            if spell.cost.amount < 0 {
                errors.push(format!("spell `{id}` has a negative resource cost"));
            }
            if !self.abilities.contains_key(&spell.ability) {
                errors.push(format!(
                    "spell `{id}` references unknown ability `{}`",
                    spell.ability
                ));
            }
            if !self.resources.contains_key(&spell.cost.resource) {
                errors.push(format!(
                    "spell `{id}` costs unknown resource `{}`",
                    spell.cost.resource
                ));
            }
            if let DifficultyRef::Named(name) = &spell.check.difficulty {
                if !self.difficulties.contains_key(name) {
                    errors.push(format!(
                        "spell `{id}` references unknown difficulty `{name}`"
                    ));
                }
            }
            for tag in &spell.check.tags {
                self.warn_tag(&mut warnings, tag, &format!("spell `{id}`"));
            }
        }

        let starting = &self.starting_character;
        for id in starting.characteristics.keys() {
            if !self.characteristics.contains_key(id) {
                errors.push(format!(
                    "starting character has unknown characteristic `{id}`"
                ));
            }
        }
        for id in starting.abilities.keys() {
            if !self.abilities.contains_key(id) {
                errors.push(format!("starting character has unknown ability `{id}`"));
            }
        }
        for tag in &starting.tags {
            self.warn_tag(&mut warnings, tag, "starting character");
        }
        for id in &starting.perks {
            if !self.perks.contains_key(id) {
                errors.push(format!("starting character has unknown perk `{id}`"));
            }
        }
        for id in &starting.inventory {
            if !self.items.contains_key(id) {
                errors.push(format!("starting character has unknown item `{id}`"));
            }
        }
        for id in starting.resources.keys() {
            if !self.resources.contains_key(id) {
                errors.push(format!("starting character has unknown resource `{id}`"));
            }
        }

        warnings
    }

    fn warn_tag(&self, warnings: &mut Vec<String>, tag: &str, context: &str) {
        if !self.tags.contains_key(tag) {
            warnings.push(format!(
                "{context} uses tag `{tag}`, which is not in the tags registry"
            ));
        }
    }
}
