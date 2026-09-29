//! Active and passive check resolution.
//!
//! Active checks roll `2d6 + characteristic bonus + ability level + modifiers`
//! against a difficulty. Double ones always fail, double sixes always succeed.
//!
//! Passive checks roll nothing. Their value is
//! `characteristic bonus + ability level + modifiers + 6`.
//!
//! Totals use saturating arithmetic, so extreme modifiers clamp at the `i32`
//! limits instead of overflowing.

use crate::dice::Dice;
use crate::model::Rulebook;
use crate::modifiers::{apply_entity, Breakdown};
use crate::state::Character;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    CriticalFailure,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    UnknownAbility(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::UnknownAbility(id) => write!(f, "unknown ability `{id}`"),
        }
    }
}

impl std::error::Error for CheckError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub outcome: Outcome,
    pub total: i32,
    pub dice: [u8; 2],
    pub breakdown: Breakdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassiveResult {
    pub passed: bool,
    pub value: i32,
    pub difficulty: i32,
    pub breakdown: Breakdown,
}

/// What the story asks for. Tags may be empty, and the modifier is the
/// one-off modifier.
#[derive(Debug, Clone, Copy)]
pub struct CheckRequest<'a> {
    pub ability: &'a str,
    pub difficulty: i32,
    pub tags: &'a [&'a str],
    pub modifier: i32,
}

impl<'a> CheckRequest<'a> {
    pub fn new(ability: &'a str, difficulty: i32) -> Self {
        Self {
            ability,
            difficulty,
            tags: &[],
            modifier: 0,
        }
    }

    pub fn with_tags(mut self, tags: &'a [&'a str]) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_modifier(mut self, modifier: i32) -> Self {
        self.modifier = modifier;
        self
    }
}

/// Resolves checks for one rulebook and one character.
pub struct Checks<'a> {
    rulebook: &'a Rulebook,
    character: &'a Character,
}

impl<'a> Checks<'a> {
    pub fn new(rulebook: &'a Rulebook, character: &'a Character) -> Self {
        Self {
            rulebook,
            character,
        }
    }

    /// The full modifier list for a check, sources included.
    pub fn breakdown(
        &self,
        ability: &str,
        tags: &[&str],
        modifier: i32,
    ) -> Result<Breakdown, CheckError> {
        let ability_def = self
            .rulebook
            .abilities
            .get(ability)
            .ok_or_else(|| CheckError::UnknownAbility(ability.to_string()))?;
        let characteristic = ability_def.characteristic.as_str();
        let characteristic_value = self.character.characteristic(characteristic).unwrap_or(0);

        let mut breakdown = Breakdown::default();
        if let Some(definition) = self.rulebook.characteristics.get(characteristic) {
            breakdown.push(
                format!("{characteristic} bonus"),
                definition.bonus_for(characteristic_value),
            );
        }
        breakdown.push(
            format!("{ability} ability"),
            self.character.ability_level(ability),
        );

        let check_tags = self.check_tags(tags);
        for id in self.character.perk_ids() {
            if let Some(perk) = self.rulebook.perks.get(id) {
                apply_entity(
                    &mut breakdown,
                    "perk",
                    id,
                    &perk.modifiers,
                    Some(characteristic),
                    ability,
                    &check_tags,
                );
            }
        }
        for id in self.character.condition_ids() {
            if let Some(condition) = self.rulebook.conditions.get(id) {
                apply_entity(
                    &mut breakdown,
                    "condition",
                    id,
                    &condition.modifiers,
                    Some(characteristic),
                    ability,
                    &check_tags,
                );
            }
        }
        for id in self.character.item_ids() {
            if let Some(item) = self.rulebook.items.get(id) {
                apply_entity(
                    &mut breakdown,
                    "item",
                    id,
                    &item.modifiers,
                    Some(characteristic),
                    ability,
                    &check_tags,
                );
            }
        }
        for id in self.character.environment_ids() {
            if let Some(environment) = self.rulebook.environments.get(id) {
                apply_entity(
                    &mut breakdown,
                    "environment",
                    id,
                    &environment.modifiers,
                    Some(characteristic),
                    ability,
                    &check_tags,
                );
            }
        }
        breakdown.push("one-off", modifier);
        Ok(breakdown)
    }

    pub fn active<D: Dice + ?Sized>(
        &self,
        dice: &mut D,
        request: &CheckRequest<'_>,
    ) -> Result<CheckResult, CheckError> {
        let breakdown = self.breakdown(request.ability, request.tags, request.modifier)?;
        let rolls = [dice.roll_d6(), dice.roll_d6()];
        let dice_total = i32::from(rolls[0]).saturating_add(i32::from(rolls[1]));
        let total = dice_total.saturating_add(breakdown.total());
        let outcome = match rolls {
            [1, 1] => Outcome::CriticalFailure,
            [6, 6] => Outcome::CriticalSuccess,
            _ if total >= request.difficulty => Outcome::Success,
            _ => Outcome::Failure,
        };
        Ok(CheckResult {
            outcome,
            total,
            dice: rolls,
            breakdown,
        })
    }

    pub fn passive(&self, request: &CheckRequest<'_>) -> Result<PassiveResult, CheckError> {
        let mut breakdown = self.breakdown(request.ability, request.tags, request.modifier)?;
        breakdown.push("passive constant", 6);
        let value = breakdown.total();
        Ok(PassiveResult {
            passed: value >= request.difficulty,
            value,
            difficulty: request.difficulty,
            breakdown,
        })
    }

    /// The tags of a check: the tags the story passed, plus the tags of every
    /// active environment.
    fn check_tags(&self, tags: &[&str]) -> BTreeSet<String> {
        let mut set: BTreeSet<String> = tags.iter().map(|tag| (*tag).to_string()).collect();
        for id in self.character.environment_ids() {
            if let Some(environment) = self.rulebook.environments.get(id) {
                set.extend(environment.tags.iter().cloned());
            }
        }
        set
    }
}
