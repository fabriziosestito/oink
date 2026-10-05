//! Spell casting: spend a resource, then run the spell's active check.
//!
//! The story applies the consequences of the returned outcome itself.

use crate::check::{CheckError, CheckRequest, CheckResult, Checks, Outcome};
use crate::dice::Dice;
use crate::model::{DifficultyRef, Rulebook};
use crate::state::{Character, StateChange};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellError {
    InvalidCost(i32),
    UnknownSpell(String),
    UnknownDifficulty(String),
    UnknownResource(String),
    NotEnough {
        resource: String,
        required: i32,
        available: i32,
    },
    Check(CheckError),
}

impl fmt::Display for SpellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpellError::InvalidCost(amount) => {
                write!(f, "spell cost must be nonnegative: {amount}")
            }
            SpellError::UnknownSpell(id) => write!(f, "unknown spell `{id}`"),
            SpellError::UnknownDifficulty(name) => {
                write!(f, "spell references unknown difficulty `{name}`")
            }
            SpellError::UnknownResource(id) => {
                write!(f, "spell costs unknown resource `{id}`")
            }
            SpellError::NotEnough {
                resource,
                required,
                available,
            } => write!(
                f,
                "not enough {resource}: need {required}, have {available}"
            ),
            SpellError::Check(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for SpellError {}

impl From<CheckError> for SpellError {
    fn from(error: CheckError) -> Self {
        SpellError::Check(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CastResult {
    pub outcome: Outcome,
    pub check: CheckResult,
    pub spent: i32,
    pub changes: Vec<StateChange>,
}

/// Spend the spell's cost and resolve its active check.
pub fn cast<D: Dice + ?Sized>(
    rulebook: &Rulebook,
    character: &mut Character,
    spell_id: &str,
    dice: &mut D,
) -> Result<CastResult, SpellError> {
    let spell = rulebook
        .spells
        .get(spell_id)
        .ok_or_else(|| SpellError::UnknownSpell(spell_id.to_string()))?;

    let difficulty = match spell.check.difficulty.resolve(rulebook) {
        Some(value) => value,
        None => {
            let described = match &spell.check.difficulty {
                DifficultyRef::Named(name) => name.clone(),
                DifficultyRef::Value(value) => value.to_string(),
            };
            return Err(SpellError::UnknownDifficulty(described));
        }
    };

    let available = character
        .resource(&spell.cost.resource)
        .ok_or_else(|| SpellError::UnknownResource(spell.cost.resource.clone()))?;
    if spell.cost.amount < 0 {
        return Err(SpellError::InvalidCost(spell.cost.amount));
    }
    if !character.can_spend_resource(rulebook, &spell.cost.resource, spell.cost.amount) {
        return Err(SpellError::NotEnough {
            resource: spell.cost.resource.clone(),
            required: spell.cost.amount,
            available,
        });
    }

    let tags: Vec<&str> = spell.check.tags.iter().map(String::as_str).collect();
    // Spells always use the default profile. Naming it explicitly skips the
    // advantage and disadvantage pools that character state would select.
    let request = CheckRequest {
        ability: spell.ability.as_str(),
        difficulty,
        tags: &tags,
        modifier: 0,
        pool: Some(rulebook.dice.default.as_str()),
    };
    let checks = Checks::new(rulebook, character);
    // Validate the check before spending or advancing the dice source.
    checks.breakdown(request.ability, request.tags, request.modifier)?;
    let check = checks.active(dice, &request)?;
    let changes = character.spend_resource(rulebook, &spell.cost.resource, spell.cost.amount);
    Ok(CastResult {
        outcome: check.outcome,
        check,
        spent: spell.cost.amount,
        changes,
    })
}
