//! Active and passive check resolution with generic dice pools.
//!
//! Every active check compares a score with a target. Over adds dice plus
//! contribution against difficulty. Under rolls dice alone against
//! contribution plus difficulty. One ordered outcome table per profile
//! decides criticals and normal results.
//!
//! Totals use saturating arithmetic, so extreme modifiers clamp at the `i32`
//! limits instead of overflowing.

use crate::dice::Dice;
use crate::model::{Degrees, DiceProfile, Die, Direction, Keep, Rulebook};
use crate::modifiers::{apply_entity, Breakdown};
use crate::state::Character;
use std::collections::BTreeSet;
use std::fmt;

pub use crate::model::Outcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    UnknownAbility(String),
    UnknownPool(String),
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::UnknownAbility(id) => write!(f, "unknown ability `{id}`"),
            CheckError::UnknownPool(id) => write!(f, "unknown dice profile `{id}`"),
        }
    }
}

impl std::error::Error for CheckError {}

/// How a roll was made: a sum of kept dice, or a percentile digit pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollKind {
    Sum,
    Percentile,
}

/// The dice log for one active check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceRoll {
    pub kind: RollKind,
    pub dice: Vec<u16>,
    pub kept: Vec<bool>,
    pub total: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub outcome: Outcome,
    pub pool: String,
    pub score: i32,
    pub target: i32,
    pub margin: i32,
    pub degrees: i32,
    pub roll: DiceRoll,
    pub breakdown: Breakdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassiveResult {
    pub passed: bool,
    pub value: i32,
    pub difficulty: i32,
    pub breakdown: Breakdown,
}

/// What the story asks for. Tags may be empty, the modifier is the one-off
/// modifier, and pool optionally names an explicit dice profile. None starts
/// from the rulebook default and permits state advantage or disadvantage.
#[derive(Debug, Clone, Copy)]
pub struct CheckRequest<'a> {
    pub ability: &'a str,
    pub difficulty: i32,
    pub tags: &'a [&'a str],
    pub modifier: i32,
    pub pool: Option<&'a str>,
}

impl<'a> CheckRequest<'a> {
    pub fn new(ability: &'a str, difficulty: i32) -> Self {
        Self {
            ability,
            difficulty,
            tags: &[],
            modifier: 0,
            pool: None,
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

    pub fn with_pool(mut self, pool: Option<&'a str>) -> Self {
        self.pool = pool;
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
        let contribution = breakdown.total();

        let (profile_id, profile) = self.resolve_profile(request.pool)?;
        let (used_id, used) = if request.pool.is_some() {
            (profile_id, profile)
        } else {
            self.pick_advantage_pool(&profile_id, profile)
        };

        let roll = roll_pool(dice, &used.pool);
        let (score, target) = match used.direction {
            Direction::Over => (roll.total.saturating_add(contribution), request.difficulty),
            Direction::Under => (roll.total, contribution.saturating_add(request.difficulty)),
        };
        let margin = match used.direction {
            Direction::Over => score.saturating_sub(target),
            Direction::Under => target.saturating_sub(score),
        };
        let mut degrees = match used.degrees {
            Degrees::Margin => margin,
            Degrees::None => 0,
            Degrees::Tens => match used.direction {
                Direction::Over => tens(score).saturating_sub(tens(target)),
                Direction::Under => tens(target).saturating_sub(tens(score)),
            },
        };

        let (outcome, degrees_min, degrees_max) =
            evaluate_outcomes(used, &roll, score, target, margin, degrees);
        if let Some(min) = degrees_min {
            degrees = degrees.max(min);
        }
        if let Some(max) = degrees_max {
            degrees = degrees.min(max);
        }

        Ok(CheckResult {
            outcome,
            pool: used_id,
            score,
            target,
            margin,
            degrees,
            roll,
            breakdown,
        })
    }

    pub fn passive(&self, request: &CheckRequest<'_>) -> Result<PassiveResult, CheckError> {
        let mut breakdown = self.breakdown(request.ability, request.tags, request.modifier)?;
        let constant = self
            .rulebook
            .dice
            .profiles
            .get(&self.rulebook.dice.default)
            .and_then(|profile| profile.passive)
            .unwrap_or(0);
        breakdown.push("passive constant", constant);
        let value = breakdown.total();
        Ok(PassiveResult {
            passed: value >= request.difficulty,
            value,
            difficulty: request.difficulty,
            breakdown,
        })
    }

    fn resolve_profile(&self, pool: Option<&str>) -> Result<(String, &DiceProfile), CheckError> {
        if let Some(name) = pool {
            let profile = self
                .rulebook
                .dice
                .profiles
                .get(name)
                .ok_or_else(|| CheckError::UnknownPool(name.to_string()))?;
            Ok((name.to_string(), profile))
        } else {
            let name = &self.rulebook.dice.default;
            let profile = self
                .rulebook
                .dice
                .profiles
                .get(name)
                .ok_or_else(|| CheckError::UnknownPool(name.clone()))?;
            Ok((name.clone(), profile))
        }
    }

    fn pick_advantage_pool<'x>(
        &'x self,
        base_id: &str,
        base: &'x DiceProfile,
    ) -> (String, &'x DiceProfile) {
        // Story-named pools ignore state flags. The caller skips this when
        // the request names a pool, so here only state flags apply.
        let advantage = self.has_advantage();
        let disadvantage = self.has_disadvantage();
        match (advantage, disadvantage) {
            (true, false) => {
                if let Some(name) = &base.advantage {
                    if let Some(pool) = self.rulebook.dice.profiles.get(name) {
                        return (name.clone(), pool);
                    }
                }
            }
            (false, true) => {
                if let Some(name) = &base.disadvantage {
                    if let Some(pool) = self.rulebook.dice.profiles.get(name) {
                        return (name.clone(), pool);
                    }
                }
            }
            _ => {}
        }
        // Look up the base again to return an owned id plus reference.
        // The base reference is already valid, so return it directly.
        (base_id.to_string(), base)
    }

    fn has_advantage(&self) -> bool {
        for id in self.character.perk_ids() {
            if self
                .rulebook
                .perks
                .get(id)
                .is_some_and(|perk| perk.advantage)
            {
                return true;
            }
        }
        for id in self.character.condition_ids() {
            if self
                .rulebook
                .conditions
                .get(id)
                .is_some_and(|condition| condition.advantage)
            {
                return true;
            }
        }
        false
    }

    fn has_disadvantage(&self) -> bool {
        for id in self.character.perk_ids() {
            if self
                .rulebook
                .perks
                .get(id)
                .is_some_and(|perk| perk.disadvantage)
            {
                return true;
            }
        }
        for id in self.character.condition_ids() {
            if self
                .rulebook
                .conditions
                .get(id)
                .is_some_and(|condition| condition.disadvantage)
            {
                return true;
            }
        }
        false
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

/// The tens digit of a value, rounding toward negative infinity so that
/// negative scores and targets keep a well-defined digit (`tens(-15)` is -2).
/// Positive values behave like truncating division.
fn tens(value: i32) -> i32 {
    value.div_euclid(10)
}

fn roll_pool<D: Dice + ?Sized>(dice: &mut D, pool: &crate::model::DicePool) -> DiceRoll {
    if pool.die == Die::Percentile {
        let tens_roll = dice.roll(10);
        let units_roll = dice.roll(10);
        let tens_digit = tens_roll % 10;
        let units_digit = units_roll % 10;
        let value = i32::from(tens_digit) * 10 + i32::from(units_digit);
        let total = if value == 0 { 100 } else { value };
        return DiceRoll {
            kind: RollKind::Percentile,
            dice: vec![tens_digit, units_digit],
            kept: vec![true, true],
            total,
        };
    }

    let sides = pool.die.sides();
    let mut faces: Vec<u16> = Vec::with_capacity(pool.count as usize);
    for _ in 0..pool.count {
        faces.push(dice.roll(sides));
    }
    let kept = keep_mask(&faces, &pool.keep);
    let mut total: i32 = 0;
    for (face, keep) in faces.iter().zip(kept.iter()) {
        if *keep {
            total = total.saturating_add(i32::from(*face));
        }
    }
    DiceRoll {
        kind: RollKind::Sum,
        dice: faces,
        kept,
        total,
    }
}

fn keep_mask(faces: &[u16], keep: &Keep) -> Vec<bool> {
    match keep {
        Keep::All => vec![true; faces.len()],
        Keep::Highest(n) | Keep::Lowest(n) => {
            let n = (*n as usize).min(faces.len());
            let mut indices: Vec<usize> = (0..faces.len()).collect();
            match keep {
                Keep::Highest(_) => {
                    indices.sort_by(|&a, &b| faces[b].cmp(&faces[a]).then(a.cmp(&b)));
                }
                Keep::Lowest(_) => {
                    indices.sort_by(|&a, &b| faces[a].cmp(&faces[b]).then(a.cmp(&b)));
                }
                Keep::All => unreachable!(),
            }
            let mut kept = vec![false; faces.len()];
            for index in indices.into_iter().take(n) {
                kept[index] = true;
            }
            kept
        }
    }
}

fn evaluate_outcomes(
    profile: &DiceProfile,
    roll: &DiceRoll,
    score: i32,
    target: i32,
    margin: i32,
    degrees: i32,
) -> (Outcome, Option<i32>, Option<i32>) {
    let kept_faces: Vec<u16> = roll
        .dice
        .iter()
        .zip(roll.kept.iter())
        .filter_map(|(face, keep)| keep.then_some(*face))
        .collect();

    let (max_face, min_face) = match profile.pool.die {
        Die::Percentile => (9, 0),
        die => (die.sides(), 1),
    };

    let is_all_max = !kept_faces.is_empty() && kept_faces.iter().all(|face| *face == max_face);
    let is_all_min = !kept_faces.is_empty() && kept_faces.iter().all(|face| *face == min_face);
    let is_any_max = kept_faces.contains(&max_face);
    let is_any_min = kept_faces.contains(&min_face);
    let is_doubles = has_doubles(&kept_faces);

    for rule in &profile.outcomes {
        if rule.all_max && !is_all_max {
            continue;
        }
        if rule.all_min && !is_all_min {
            continue;
        }
        if rule.any_max && !is_any_max {
            continue;
        }
        if rule.any_min && !is_any_min {
            continue;
        }
        if rule.doubles && !is_doubles {
            continue;
        }
        if rule.score_at_least.is_some_and(|bound| score < bound) {
            continue;
        }
        if rule.score_at_most.is_some_and(|bound| score > bound) {
            continue;
        }
        if rule.margin_at_least.is_some_and(|bound| margin < bound) {
            continue;
        }
        if rule.margin_at_most.is_some_and(|bound| margin > bound) {
            continue;
        }
        if rule.degrees_at_least.is_some_and(|bound| degrees < bound) {
            continue;
        }
        if rule.degrees_at_most.is_some_and(|bound| degrees > bound) {
            continue;
        }
        if rule.target_at_least.is_some_and(|bound| target < bound) {
            continue;
        }
        if rule.target_at_most.is_some_and(|bound| target > bound) {
            continue;
        }
        return (rule.outcome, rule.degrees_min, rule.degrees_max);
    }
    (Outcome::Failure, None, None)
}

fn has_doubles(faces: &[u16]) -> bool {
    if faces.len() < 2 {
        return false;
    }
    let mut sorted = faces.to_vec();
    sorted.sort_unstable();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::ScriptedDice;
    use crate::model::Rulebook;
    use crate::state::Character;

    const OVER_FIXTURE: &str = r#"
characteristics:
  intellect:
    name: Intellect
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  logic:
    name: Logic
    characteristic: intellect
dice:
  default: standard
  profiles:
    standard:
      notation: "2d6"
      direction: over
      degrees: margin
      passive: 6
      outcomes:
        - { all_max: true, outcome: critical_success }
        - { all_min: true, outcome: critical_failure }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { intellect: 5 }
  abilities: { logic: 2 }
"#;

    const WFRP_FIXTURE: &str = r#"
characteristics:
  weapon_skill:
    name: Weapon Skill
    min: 1
    max: 100
    bonus: direct
abilities:
  melee_basic:
    name: Melee Basic
    characteristic: weapon_skill
conditions:
  tired:
    name: Tired
    modifiers:
      abilities: { melee_basic: -5 }
dice:
  default: wfrp
  profiles:
    wfrp:
      notation: "d%"
      direction: under
      degrees: tens
      outcomes:
        - { score_at_most: 5, outcome: critical_success, degrees_min: 1 }
        - { score_at_least: 96, outcome: critical_failure, degrees_max: -1 }
        - { doubles: true, margin_at_least: 0, outcome: critical_success }
        - { doubles: true, margin_at_most: -1, outcome: critical_failure }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { weapon_skill: 42 }
  abilities: { melee_basic: 5 }
"#;

    fn load(yaml: &str) -> Rulebook {
        Rulebook::load(yaml).expect("fixture loads").rulebook
    }

    #[test]
    fn over_adds_contribution_to_dice() {
        let rulebook = load(OVER_FIXTURE);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // intellect 5 -> 0, logic 2, so contribution 2.
        let request = CheckRequest::new("logic", 10);
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.roll.total, 7);
        assert_eq!(result.score, 9);
        assert_eq!(result.target, 10);
        assert_eq!(result.margin, -1);
        assert_eq!(result.degrees, -1);
        assert_eq!(result.outcome, Outcome::Failure);
    }

    #[test]
    fn under_adds_difficulty_to_contribution() {
        let rulebook = load(WFRP_FIXTURE);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // target = 42 + 5 + 20 = 67. Roll 50 -> margin 17, SL tens(67)-tens(50)=6-5=1.
        let request = CheckRequest::new("melee_basic", 20);
        let mut dice = ScriptedDice::new(&[5, 10]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.roll.total, 50);
        assert_eq!(result.score, 50);
        assert_eq!(result.target, 67);
        assert_eq!(result.margin, 17);
        assert_eq!(result.degrees, 1);
        assert_eq!(result.outcome, Outcome::Success);
    }

    #[test]
    fn direct_contribution_passes_value_untouched() {
        let rulebook = load(WFRP_FIXTURE);
        let mut character = Character::from_starting(&rulebook);
        character.add_condition(&rulebook, "tired");
        let checks = Checks::new(&rulebook, &character);
        let breakdown = checks.breakdown("melee_basic", &[], 0).unwrap();
        // 42 direct + 5 ability -5 condition = 42.
        assert_eq!(breakdown.total(), 42);
    }

    #[test]
    fn wfrp_example_gives_three_sl() {
        let rulebook = load(WFRP_FIXTURE);
        let mut character = Character::from_starting(&rulebook);
        character.add_condition(&rulebook, "tired");
        let checks = Checks::new(&rulebook, &character);
        // WS 42 + Melee 5 - Tired 5 + Average 20 = 62. Roll 34 -> margin 28, SL 6-3=3.
        let request = CheckRequest::new("melee_basic", 20);
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.target, 62);
        assert_eq!(result.score, 34);
        assert_eq!(result.margin, 28);
        assert_eq!(result.degrees, 3);
        assert_eq!(result.outcome, Outcome::Success);
    }

    #[test]
    fn wfrp_automatic_ranges_and_doubles() {
        let rulebook = load(WFRP_FIXTURE);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);

        // 03 is a critical success via the 01-05 range.
        let mut dice = ScriptedDice::new(&[10, 3]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", 20))
            .unwrap();
        assert_eq!(result.roll.total, 3);
        assert_eq!(result.outcome, Outcome::CriticalSuccess);

        // 67 vs 67 gives 0 SL (equal tens, margin 0).
        let mut dice = ScriptedDice::new(&[6, 7]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", 0))
            .unwrap();
        // target 47, roll 67 -> margin -20, tens 4-6 = -2.
        assert_eq!(result.margin, -20);

        // 68 vs 67 fails with 0 degrees: use target 68? Build exact case:
        // target 67 (42+5+20), roll 68 -> margin -1, tens 6-6=0.
        let mut dice = ScriptedDice::new(&[6, 8]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", 20))
            .unwrap();
        assert_eq!(result.target, 67);
        assert_eq!(result.score, 68);
        assert_eq!(result.margin, -1);
        assert_eq!(result.degrees, 0);
        assert_eq!(result.outcome, Outcome::Failure);

        // 97 and 99 are critical failures via the 96-00 range.
        for roll in [[9, 7], [9, 9]] {
            let mut dice = ScriptedDice::new(&roll);
            let result = checks
                .active(&mut dice, &CheckRequest::new("melee_basic", 20))
                .unwrap();
            assert_eq!(result.outcome, Outcome::CriticalFailure);
        }

        // 11 on a success is a critical (doubles + margin>=0).
        let mut dice = ScriptedDice::new(&[1, 1]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", 20))
            .unwrap();
        assert_eq!(result.outcome, Outcome::CriticalSuccess);

        // 11 on a failure is a fumble (doubles + margin<0): target low.
        let mut dice = ScriptedDice::new(&[1, 1]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", -50))
            .unwrap();
        assert_eq!(result.outcome, Outcome::CriticalFailure);
    }

    #[test]
    fn degrees_bounds_clamp_sl() {
        let rulebook = load(WFRP_FIXTURE);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // 04 vs low target: tens would be negative, but 01-05 row bounds to +1.
        let mut dice = ScriptedDice::new(&[10, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", -50))
            .unwrap();
        assert_eq!(result.roll.total, 4);
        assert_eq!(result.outcome, Outcome::CriticalSuccess);
        assert_eq!(result.degrees, 1);

        // 99 bounds to -1 via the 96-00 row when tens would be positive:
        // target 100 (42+5+53), roll 99 -> tens 10-9=1, clamped to -1.
        let mut dice = ScriptedDice::new(&[9, 9]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", 53))
            .unwrap();
        assert_eq!(result.target, 100);
        assert_eq!(result.outcome, Outcome::CriticalFailure);
        assert_eq!(result.degrees, -1);
    }

    #[test]
    fn degrees_formulas() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: m
  profiles:
    m:
      notation: "2d6"
      direction: over
      degrees: margin
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    t:
      notation: "2d6"
      direction: over
      degrees: tens
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    n:
      notation: "2d6"
      direction: over
      degrees: none
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // contribution 0, dice 3+4=7 vs 5: margin 2.
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 5).with_pool(Some("m")))
            .unwrap();
        assert_eq!(result.degrees, 2);

        // tens: score 7 vs 5 -> tens 0-0=0.
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 5).with_pool(Some("t")))
            .unwrap();
        assert_eq!(result.degrees, 0);

        // none stays 0.
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 5).with_pool(Some("n")))
            .unwrap();
        assert_eq!(result.degrees, 0);
    }

    #[test]
    fn tens_digits_round_toward_negative_infinity() {
        assert_eq!(tens(34), 3);
        assert_eq!(tens(5), 0);
        assert_eq!(tens(0), 0);
        assert_eq!(tens(-1), -1);
        assert_eq!(tens(-15), -2);
    }

    #[test]
    fn tens_degrees_work_with_negative_targets() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: t
  profiles:
    t:
      notation: "2d6"
      direction: over
      degrees: tens
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // score 7 vs target -15: tens 0 - (-2) = 2.
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", -15))
            .unwrap();
        assert_eq!(result.degrees, 2);
        assert_eq!(result.outcome, Outcome::Success);
    }

    #[test]
    fn outcome_table_first_match_wins_and_falls_back() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: s
  profiles:
    s:
      notation: "1d20"
      direction: over
      outcomes:
        - { any_max: true, outcome: critical_success }
        - { any_min: true, outcome: critical_failure }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    empty:
      notation: "1d20"
      direction: over
      outcomes: []
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);

        // Natural 20.
        let mut dice = ScriptedDice::new(&[20]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 30))
            .unwrap();
        assert_eq!(result.outcome, Outcome::CriticalSuccess);

        // Natural 1.
        let mut dice = ScriptedDice::new(&[1]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0))
            .unwrap();
        assert_eq!(result.outcome, Outcome::CriticalFailure);

        // Empty table falls back to failure.
        let mut dice = ScriptedDice::new(&[20]);
        let result = checks
            .active(
                &mut dice,
                &CheckRequest::new("a", 0).with_pool(Some("empty")),
            )
            .unwrap();
        assert_eq!(result.outcome, Outcome::Failure);
    }

    #[test]
    fn target_tests_gate_rows() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: s
  profiles:
    s:
      notation: "2d6"
      direction: over
      outcomes:
        - { target_at_least: 10, margin_at_least: 0, outcome: critical_success }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // target 12, dice 7 -> margin -5 -> failure (first row needs target>=10 AND margin>=0).
        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 12))
            .unwrap();
        assert_eq!(result.outcome, Outcome::Failure);
        // target 12, dice 12 (6+6) -> margin 0 -> critical via first row.
        let mut dice = ScriptedDice::new(&[6, 6]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 12))
            .unwrap();
        assert_eq!(result.outcome, Outcome::CriticalSuccess);
        // target 5, dice 12 -> margin 7 but target too low -> success, not critical.
        let mut dice = ScriptedDice::new(&[6, 6]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 5))
            .unwrap();
        assert_eq!(result.outcome, Outcome::Success);
    }

    #[test]
    fn doubles_need_two_kept_dice() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: pair
  profiles:
    pair:
      notation: "2d6"
      direction: over
      outcomes:
        - { doubles: true, outcome: critical_success }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    single:
      notation: "2d20kh1"
      direction: over
      outcomes:
        - { doubles: true, outcome: critical_success }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);

        for dice in [[1, 1], [6, 6]] {
            let mut d = ScriptedDice::new(&dice);
            let result = checks.active(&mut d, &CheckRequest::new("a", 30)).unwrap();
            assert_eq!(result.outcome, Outcome::CriticalSuccess);
        }
        // Single kept die never counts even when both rolled faces match,
        // because only one die is kept.
        let mut d = ScriptedDice::new(&[5, 5]);
        let result = checks
            .active(&mut d, &CheckRequest::new("a", 0).with_pool(Some("single")))
            .unwrap();
        assert_ne!(result.outcome, Outcome::CriticalSuccess);

        // d% doubles: 11 and 22 give criticals via doubles+success.
        // 99 and 00 hit the 96-00 auto range first, so they are critical
        // failures. A doubles-only profile proves 99 and 00 count as doubles.
        let wfrp = load(WFRP_FIXTURE);
        let wchar = Character::from_starting(&wfrp);
        let wchecks = Checks::new(&wfrp, &wchar);
        for digits in [[1, 1], [2, 2]] {
            let mut d = ScriptedDice::new(&digits);
            let result = wchecks
                .active(&mut d, &CheckRequest::new("melee_basic", 60))
                .unwrap();
            assert_eq!(
                result.outcome,
                Outcome::CriticalSuccess,
                "digits {digits:?}"
            );
        }
        for digits in [[9, 9], [10, 10]] {
            let mut d = ScriptedDice::new(&digits);
            let result = wchecks
                .active(&mut d, &CheckRequest::new("melee_basic", 60))
                .unwrap();
            assert_eq!(
                result.outcome,
                Outcome::CriticalFailure,
                "digits {digits:?}"
            );
        }

        let doubles_yaml = r#"
characteristics:
  c:
    name: C
    min: 1
    max: 100
    bonus: direct
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: pct
  profiles:
    pct:
      notation: "d%"
      direction: under
      degrees: tens
      outcomes:
        - { doubles: true, margin_at_least: 0, outcome: critical_success }
        - { doubles: true, margin_at_most: -1, outcome: critical_failure }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 50 }
"#;
        let doubles_book = load(doubles_yaml);
        let doubles_char = Character::from_starting(&doubles_book);
        let doubles_checks = Checks::new(&doubles_book, &doubles_char);
        for digits in [[1, 1], [2, 2], [9, 9], [10, 10]] {
            let mut d = ScriptedDice::new(&digits);
            let result = doubles_checks
                .active(&mut d, &CheckRequest::new("a", 0))
                .unwrap();
            // target 50, rolls 11/22 succeed crit, 99/00 fail crit.
            assert!(
                matches!(
                    result.outcome,
                    Outcome::CriticalSuccess | Outcome::CriticalFailure
                ),
                "digits {digits:?} gave {:?}",
                result.outcome
            );
        }
    }

    #[test]
    fn percentile_reads_zero_as_one_hundred() {
        let rulebook = load(WFRP_FIXTURE);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        let mut dice = ScriptedDice::new(&[10, 10]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("melee_basic", 0))
            .unwrap();
        assert_eq!(result.roll.dice, vec![0, 0]);
        assert_eq!(result.roll.kept, vec![true, true]);
        assert_eq!(result.roll.total, 100);
        assert_eq!(result.score, 100);
    }

    #[test]
    fn keep_pools_sum_kept_dice_only() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: high
  profiles:
    high:
      notation: "2d20kh1"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    low:
      notation: "2d20kl1"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    four:
      notation: "4d6kh3"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);

        let mut dice = ScriptedDice::new(&[17, 3]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0))
            .unwrap();
        assert_eq!(result.roll.total, 17);
        assert_eq!(result.roll.kept, vec![true, false]);

        let mut dice = ScriptedDice::new(&[17, 3]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0).with_pool(Some("low")))
            .unwrap();
        assert_eq!(result.roll.total, 3);
        assert_eq!(result.roll.kept, vec![false, true]);

        let mut dice = ScriptedDice::new(&[6, 6, 6, 1]);
        let result = checks
            .active(
                &mut dice,
                &CheckRequest::new("a", 0).with_pool(Some("four")),
            )
            .unwrap();
        assert_eq!(result.roll.total, 18);
    }

    #[test]
    fn advantage_comes_from_state_unless_story_names_pool() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
perks:
  lucky:
    name: Lucky
    advantage: true
conditions:
  shaken:
    name: Shaken
    disadvantage: true
dice:
  default: standard
  profiles:
    standard:
      notation: "1d20"
      direction: over
      advantage: advantage_pool
      disadvantage: disadvantage_pool
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    advantage_pool:
      notation: "2d20kh1"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    disadvantage_pool:
      notation: "2d20kl1"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);

        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        let mut dice = ScriptedDice::new(&[10]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0))
            .unwrap();
        assert_eq!(result.pool, "standard");

        let mut advantaged = Character::from_starting(&rulebook);
        advantaged.add_perk(&rulebook, "lucky");
        let checks = Checks::new(&rulebook, &advantaged);
        let mut dice = ScriptedDice::new(&[10, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0))
            .unwrap();
        assert_eq!(result.pool, "advantage_pool");
        assert_eq!(result.roll.total, 10);

        let mut disadvantaged = Character::from_starting(&rulebook);
        disadvantaged.add_condition(&rulebook, "shaken");
        let checks = Checks::new(&rulebook, &disadvantaged);
        let mut dice = ScriptedDice::new(&[10, 4]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0))
            .unwrap();
        assert_eq!(result.pool, "disadvantage_pool");
        assert_eq!(result.roll.total, 4);

        // Both cancel to the base profile.
        advantaged.add_condition(&rulebook, "shaken");
        let checks = Checks::new(&rulebook, &advantaged);
        let mut dice = ScriptedDice::new(&[10]);
        let result = checks
            .active(&mut dice, &CheckRequest::new("a", 0))
            .unwrap();
        assert_eq!(result.pool, "standard");

        // A story argument beats the state flags.
        let checks = Checks::new(&rulebook, &advantaged);
        let mut dice = ScriptedDice::new(&[7]);
        let result = checks
            .active(
                &mut dice,
                &CheckRequest::new("a", 0).with_pool(Some("disadvantage_pool")),
            )
            .unwrap();
        assert_eq!(result.pool, "disadvantage_pool");
    }

    #[test]
    fn passive_uses_profile_constant() {
        let yaml = r#"
characteristics:
  c:
    name: C
    bonus:
      thresholds:
        - { at: 1, bonus: 2 }
abilities:
  a:
    name: A
    characteristic: c
dice:
  default: d20p
  profiles:
    d20p:
      notation: "1d20"
      direction: over
      passive: 10
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    nopass:
      notation: "1d20"
      direction: under
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
"#;
        let rulebook = load(yaml);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        // contribution 2, passive 10 -> 12.
        let result = checks.passive(&CheckRequest::new("a", 12)).unwrap();
        assert_eq!(result.value, 12);
        assert!(result.passed);

        // A profile without a constant contributes 0: switch default.
        let mut no_pass = rulebook.clone();
        no_pass.dice.default = "nopass".to_string();
        let checks = Checks::new(&no_pass, &character);
        let result = checks.passive(&CheckRequest::new("a", 2)).unwrap();
        assert_eq!(result.value, 2);
    }

    #[test]
    fn unknown_pool_is_an_error() {
        let rulebook = load(OVER_FIXTURE);
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        let mut dice = ScriptedDice::new(&[3, 4]);
        let error = checks
            .active(
                &mut dice,
                &CheckRequest::new("logic", 0).with_pool(Some("nope")),
            )
            .unwrap_err();
        assert_eq!(error, CheckError::UnknownPool("nope".to_string()));
    }
}
