//! oink-rulebook: the core rulebook for oink.
//!
//! The crate owns the game systems that turn an Ink story into a gamebook:
//! characteristics and abilities, perks, conditions, items, environments, and
//! active and passive check resolution. It has no display, IO, or Ink runtime
//! code, so it stays testable on the host.

pub mod check;
pub mod dice;
pub mod loader;
pub mod model;
pub mod modifiers;
pub mod names;
#[cfg(feature = "spells")]
pub mod spell;
pub mod state;

pub use check::{CheckError, CheckRequest, CheckResult, Checks, DiceRoll, PassiveResult, RollKind};
pub use dice::{parse_notation, Dice, DicePoolError, SeededDice, SystemDice};
pub use loader::{LoadError, Loaded};
pub use model::Outcome;
pub use model::{
    Ability, BonusTable, BonusThreshold, Characteristic, CharacteristicBonus, Condition, Cost,
    Degrees, DiceConfig, DicePool, DiceProfile, Die, DifficultyRef, DirectMarker, Direction,
    Environment, Item, Keep, MaxFromEntry, MaxFromMode, MaxFromThreshold, Modifiers, OutcomeRule,
    Perk, Resource, Rulebook, Spell, SpellCheck, StartingCharacter, Tag, TagGrants,
};
pub use modifiers::{Breakdown, BreakdownEntry};
pub use names::{Names, Section};
#[cfg(feature = "spells")]
pub use spell::{CastResult, SpellError};
pub use state::{Character, StateChange};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::ScriptedDice;

    const SAMPLE: &str = include_str!("../../assets/data/rulebook.yaml");

    const FIXTURE: &str = r#"
names:
  perks: Talents
difficulties:
  medium: 10
characteristics:
  intellect:
    name: Intellect
    min: 1
    max: 10
    default: 3
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
        - { at: 4, bonus: -1 }
        - { at: 7, bonus: 0 }
        - { at: 10, bonus: 1 }
  psyche:
    name: Psyche
    min: 1
    max: 10
    default: 4
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
        - { at: 7, bonus: 0 }
        - { at: 10, bonus: 1 }
abilities:
  logic:
    name: Logic
    characteristic: intellect
    tags: [scholar]
  empathy:
    name: Empathy
    characteristic: psyche
  arcana:
    name: Arcana
    characteristic: intellect
tags:
  scholar:
    name: Scholar
    grants:
      perks: [bookworm]
  learned:
    name: Learned
    grants:
      abilities: [arcana]
  artist:
    name: Artist
  dark:
    name: Dark
  afraid:
    name: Afraid
perks:
  artist:
    name: Artist
    modifiers:
      characteristics: { psyche: 1 }
      abilities: { logic: 1 }
      tags: { artist: 2 }
    grants_tags: [artist]
  night_vision:
    name: Night Vision
    modifiers:
      tags: { dark: 2 }
  bookworm:
    name: Bookworm
    modifiers:
      abilities: { logic: 1 }
    grants_tags: [learned]
conditions:
  shaken:
    name: Shaken
    modifiers:
      abilities: { logic: -2 }
    grants_tags: [afraid]
    duration: 2
environments:
  dark:
    name: Dark
    tags: [dark]
    modifiers:
      abilities: { logic: -2 }
items:
  focus_charm:
    name: Focus Charm
    modifiers:
      abilities: { logic: 1 }
resources:
  focus: { name: Focus, min: 0, max: 5 }
starting_character:
  characteristics: { intellect: 5, psyche: 6 }
  abilities: { logic: 2, empathy: 1 }
  perks: [artist]
"#;

    fn fixture() -> Rulebook {
        Rulebook::load(FIXTURE).expect("fixture loads").rulebook
    }

    #[test]
    fn sample_rulebook_loads_without_warnings() {
        let loaded = Rulebook::load(SAMPLE).expect("sample loads");
        assert!(
            loaded.warnings.is_empty(),
            "warnings: {:?}",
            loaded.warnings
        );
    }

    #[test]
    fn bonus_table_clamps_at_both_ends() {
        let rulebook = fixture();
        let intellect = &rulebook.characteristics["intellect"];
        assert_eq!(intellect.bonus_for(1), -2);
        assert_eq!(intellect.bonus_for(3), -2);
        assert_eq!(intellect.bonus_for(4), -1);
        assert_eq!(intellect.bonus_for(7), 0);
        assert_eq!(intellect.bonus_for(9), 0);
        assert_eq!(intellect.bonus_for(10), 1);
        assert_eq!(intellect.bonus_for(99), 1);
    }

    #[test]
    fn loader_sorts_thresholds() {
        let yaml = r#"
characteristics:
  luck:
    name: Luck
    bonus:
      thresholds:
        - { at: 10, bonus: 2 }
        - { at: 1, bonus: -1 }
        - { at: 5, bonus: 0 }
"#;
        let rulebook = Rulebook::load(yaml).expect("loads").rulebook;
        assert_eq!(rulebook.characteristics["luck"].bonus_for(7), 0);
        assert_eq!(rulebook.characteristics["luck"].bonus_for(12), 2);
    }

    #[test]
    fn active_check_math_and_criticals() {
        let rulebook = fixture();
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        let request = CheckRequest::new("logic", 10).with_tags(&["artist"]);

        // intellect 5 -> -1, logic 2, artist perk ability +1, artist tag +2.
        let breakdown = checks.breakdown("logic", &["artist"], 0).unwrap();
        assert_eq!(breakdown.total(), 4);

        let mut dice = ScriptedDice::new(&[3, 4]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.score, 11);
        assert_eq!(result.outcome, Outcome::Success);

        let mut dice = ScriptedDice::new(&[1, 2]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.score, 7);
        assert_eq!(result.outcome, Outcome::Failure);

        let mut dice = ScriptedDice::new(&[1, 1]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.outcome, Outcome::CriticalFailure);

        let mut dice = ScriptedDice::new(&[6, 6]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.outcome, Outcome::CriticalSuccess);
        assert_eq!(result.roll.dice, vec![6, 6]);
    }

    #[test]
    fn passive_check_adds_six_and_never_rolls() {
        let rulebook = fixture();
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);

        let request = CheckRequest::new("logic", 10).with_tags(&["artist"]);
        let result = checks.passive(&request).unwrap();
        assert_eq!(result.value, 10);
        assert!(result.passed);

        let request = CheckRequest::new("logic", 11).with_tags(&["artist"]);
        let result = checks.passive(&request).unwrap();
        assert_eq!(result.value, 10);
        assert!(!result.passed);

        let request = CheckRequest::new("logic", 10)
            .with_tags(&["artist"])
            .with_modifier(-1);
        let result = checks.passive(&request).unwrap();
        assert_eq!(result.value, 9);
        assert!(!result.passed);
    }

    #[test]
    fn environment_modifiers_and_tags_join_the_check() {
        let rulebook = fixture();
        let mut character = Character::from_starting(&rulebook);
        character.add_perk(&rulebook, "night_vision");
        character.enter_environment(&rulebook, "dark");

        let checks = Checks::new(&rulebook, &character);
        let breakdown = checks.breakdown("logic", &["artist"], 0).unwrap();

        // intellect bonus -1, logic 2, artist ability +1, artist tag +2,
        // night vision (dark) +2, dark environment (logic) -2.
        assert_eq!(breakdown.total(), 4);
        let sources: Vec<&str> = breakdown
            .entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect();
        assert!(sources.contains(&"perk night_vision (tag dark)"));
        assert!(sources.contains(&"environment dark (logic)"));

        character.clear_environment("dark");
        let checks = Checks::new(&rulebook, &character);
        let breakdown = checks.breakdown("logic", &["artist"], 0).unwrap();
        assert_eq!(breakdown.total(), 4);
    }

    #[test]
    fn one_off_modifier_shows_in_the_breakdown() {
        let rulebook = fixture();
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        let breakdown = checks.breakdown("logic", &[], -2).unwrap();
        assert!(breakdown
            .entries
            .iter()
            .any(|entry| entry.source == "one-off" && entry.value == -2));
    }

    #[test]
    fn extreme_modifiers_saturate_instead_of_overflowing() {
        let rulebook = fixture();
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);

        let request = CheckRequest::new("logic", 10).with_modifier(i32::MAX);
        let mut dice = ScriptedDice::new(&[2, 2]);
        let result = checks.active(&mut dice, &request).unwrap();
        assert_eq!(result.score, i32::MAX);
        assert_eq!(result.outcome, Outcome::CriticalSuccess);

        let result = checks.passive(&request).unwrap();
        assert_eq!(result.value, i32::MAX);
        assert!(result.passed);

        let mut breakdown = Breakdown::default();
        breakdown.push("high", i32::MAX);
        breakdown.push("higher", i32::MAX);
        assert_eq!(breakdown.total(), i32::MAX);

        let mut breakdown = Breakdown::default();
        breakdown.push("low", i32::MIN);
        breakdown.push("lower", i32::MIN);
        assert_eq!(breakdown.total(), i32::MIN);
    }

    #[test]
    fn conditions_modify_and_expire() {
        let rulebook = fixture();
        let mut character = Character::from_starting(&rulebook);
        let changes = character.add_condition(&rulebook, "shaken");
        assert_eq!(
            changes,
            vec![StateChange::ConditionAdded("shaken".to_string())]
        );
        assert!(character.has_condition("shaken"));

        let checks = Checks::new(&rulebook, &character);
        let breakdown = checks.breakdown("logic", &[], 0).unwrap();
        assert_eq!(breakdown.total(), 0);
        assert!(breakdown
            .entries
            .iter()
            .any(|entry| entry.source == "condition shaken (logic)"));

        assert!(character.on_scene_end().is_empty());
        let changes = character.on_scene_end();
        assert_eq!(
            changes,
            vec![StateChange::ConditionRemoved("shaken".to_string())]
        );
        assert!(!character.has_condition("shaken"));
    }

    #[test]
    fn reapplying_a_condition_emits_a_refresh_only_when_the_duration_changes() {
        let rulebook = fixture();
        let mut character = Character::from_starting(&rulebook);

        let changes = character.add_condition(&rulebook, "shaken");
        assert_eq!(
            changes,
            vec![StateChange::ConditionAdded("shaken".to_string())]
        );

        assert!(character.add_condition(&rulebook, "shaken").is_empty());

        let changes = character.add_timed_condition(&rulebook, "shaken", Some(5));
        assert_eq!(
            changes,
            vec![StateChange::ConditionRefreshed("shaken".to_string())]
        );

        let changes = character.add_timed_condition(&rulebook, "shaken", None);
        assert_eq!(
            changes,
            vec![StateChange::ConditionRefreshed("shaken".to_string())]
        );

        assert!(character
            .add_timed_condition(&rulebook, "shaken", None)
            .is_empty());
    }

    #[test]
    fn tag_grants_chain_until_stable() {
        let rulebook = fixture();
        let mut character = Character::from_starting(&rulebook);
        assert!(!character.has_ability("arcana"));

        let changes = character.add_tag(&rulebook, "scholar");
        assert!(changes.contains(&StateChange::TagAdded("scholar".to_string())));
        assert!(changes.contains(&StateChange::PerkAdded("bookworm".to_string())));
        assert!(character.has_perk("bookworm"));
        assert!(character.has_ability("arcana"));
    }

    #[test]
    fn long_grant_chains_apply_every_grant() {
        const DEPTH: usize = 40;
        let mut yaml = String::from("tags:\n");
        for i in 0..DEPTH {
            yaml.push_str(&format!(
                "  t{i}:\n    name: Tag {i}\n    grants:\n      perks: [p{i}]\n"
            ));
        }
        yaml.push_str("perks:\n");
        for i in 0..DEPTH {
            yaml.push_str(&format!("  p{i}:\n    name: Perk {i}\n"));
            if i + 1 < DEPTH {
                yaml.push_str(&format!("    grants_tags: [t{}]\n", i + 1));
            }
        }

        let rulebook = Rulebook::load(&yaml)
            .expect("chain rulebook loads")
            .rulebook;
        let mut character = Character::from_starting(&rulebook);
        character.add_tag(&rulebook, "t0");
        for i in 0..DEPTH {
            assert!(character.has_perk(&format!("p{i}")), "p{i} was not granted");
        }
    }

    #[test]
    fn resources_clamp_at_both_ends() {
        let rulebook = fixture();
        let mut character = Character::from_starting(&rulebook);
        assert_eq!(character.resource("focus"), Some(5));

        let changes = character.spend_resource(&rulebook, "focus", 2);
        assert_eq!(
            changes,
            vec![StateChange::ResourceChanged {
                id: "focus".to_string(),
                from: 5,
                to: 3
            }]
        );

        character.spend_resource(&rulebook, "focus", 99);
        assert_eq!(character.resource("focus"), Some(3));

        character.spend_resource(&rulebook, "focus", 3);
        assert_eq!(character.resource("focus"), Some(0));

        character.restore_resource(&rulebook, "focus", 99);
        assert_eq!(character.resource("focus"), Some(5));
        assert!(character.spend_resource(&rulebook, "unknown", 1).is_empty());
    }

    #[test]
    fn resource_payments_respect_the_floor_and_integer_limits() {
        let mut rulebook = fixture();
        rulebook.resources.get_mut("focus").unwrap().min = 2;
        let mut character = Character::from_starting(&rulebook);
        for amount in [-1, i32::MIN, i32::MAX, 4] {
            assert!(character
                .spend_resource(&rulebook, "focus", amount)
                .is_empty());
            assert_eq!(character.resource("focus"), Some(5));
        }
        character.spend_resource(&rulebook, "focus", 3);
        assert_eq!(character.resource("focus"), Some(2));
        assert!(character
            .restore_resource(&rulebook, "focus", -1)
            .is_empty());
        character.restore_resource(&rulebook, "focus", i32::MAX);
        assert_eq!(character.resource("focus"), Some(5));
        assert!(character.can_spend_resource(&rulebook, "focus", 0));
    }

    const DERIVED_FIXTURE: &str = r#"
characteristics:
  physique:
    name: Physique
    min: 1
    max: 14
    default: 5
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
  psyche:
    name: Psyche
    min: 1
    max: 14
    default: 5
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
resources:
  health:
    name: Health
    min: 0
    start_full: true
    max_from:
      - characteristic: physique
        mode: thresholds
        thresholds:
          - { at: 1, value: 10 }
          - { at: 5, value: 20 }
          - { at: 8, value: 30 }
          - { at: 11, value: 40 }
      - characteristic: psyche
        mode: per_point
        base: 0
        value_per_point: 2
  focus: { name: Focus, min: 0, max: 5 }
conditions:
  hangover:
    name: Hangover
    modifiers:
      characteristics: { psyche: -1 }
    duration: 4
starting_character:
  characteristics: { physique: 11, psyche: 4 }
"#;

    fn derived_fixture() -> Rulebook {
        Rulebook::load(DERIVED_FIXTURE)
            .expect("fixture loads")
            .rulebook
    }

    #[test]
    fn derived_max_sums_thresholds_and_per_point() {
        let rulebook = derived_fixture();
        let character = Character::from_starting(&rulebook);
        // physique 11 -> 40, psyche 4 * 2 -> 8.
        assert_eq!(character.resource_max(&rulebook, "health"), Some(48));
        assert_eq!(character.resource("health"), Some(48));
        // Plain max resources keep working.
        assert_eq!(character.resource_max(&rulebook, "focus"), Some(5));
    }

    #[test]
    fn starting_resources_clamp_to_the_derived_max() {
        let mut rulebook = derived_fixture();
        rulebook
            .starting_character
            .resources
            .insert("health".to_string(), 99);
        let character = Character::from_starting(&rulebook);
        assert_eq!(character.resource("health"), Some(48));
    }

    #[test]
    fn spend_and_restore_respect_the_derived_max() {
        let rulebook = derived_fixture();
        let mut character = Character::from_starting(&rulebook);
        character.spend_resource(&rulebook, "health", 8);
        assert_eq!(character.resource("health"), Some(40));
        character.restore_resource(&rulebook, "health", 99);
        assert_eq!(character.resource("health"), Some(48));
    }

    #[test]
    fn conditions_do_not_move_derived_maxima() {
        let rulebook = derived_fixture();
        let mut character = Character::from_starting(&rulebook);
        character.add_condition(&rulebook, "hangover");
        assert_eq!(character.resource_max(&rulebook, "health"), Some(48));
    }

    #[test]
    fn loader_rejects_unknown_derivation_characteristic() {
        let yaml = r#"
resources:
  health:
    name: Health
    max_from:
      - characteristic: missing
        mode: per_point
        value_per_point: 2
"#;
        let error = Rulebook::load(yaml).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown characteristic `missing`"),
            "{error}"
        );
    }

    #[test]
    fn starting_character_applies_grants_and_defaults() {
        let rulebook = Rulebook::load(SAMPLE).expect("sample loads").rulebook;
        let character = Character::from_starting(&rulebook);

        assert_eq!(character.characteristic("psyche"), Some(4));
        assert_eq!(character.ability_level("logic"), 1);
        assert_eq!(character.ability_level("empathy"), 2);
        assert!(character.has_ability("arcana"));
        assert!(character.has_perk("artist"));
        assert!(character.has_perk("bookworm"));
        assert!(character.has_item("rusty_cleaver"));
        assert_eq!(character.resource("focus"), Some(3));
    }

    #[test]
    fn unknown_ability_is_an_error() {
        let rulebook = fixture();
        let character = Character::from_starting(&rulebook);
        let checks = Checks::new(&rulebook, &character);
        let error = checks.breakdown("nope", &[], 0).unwrap_err();
        assert_eq!(error, CheckError::UnknownAbility("nope".to_string()));
    }

    #[test]
    fn loader_reports_validation_errors() {
        let yaml = r#"
abilities:
  logic:
    name: Logic
    characteristic: missing
"#;
        let error = Rulebook::load(yaml).unwrap_err();
        let message = error.to_string();
        assert!(
            message.contains("unknown characteristic `missing`"),
            "{message}"
        );
    }

    #[test]
    fn loader_warns_about_unknown_tags() {
        let yaml = r#"
perks:
  artist:
    name: Artist
    modifiers:
      tags: { artist: 2 }
"#;
        let loaded = Rulebook::load(yaml).expect("loads");
        assert_eq!(loaded.warnings.len(), 1);
        assert!(loaded.warnings[0].contains("tag `artist`"));
    }

    #[test]
    fn loader_rejects_bad_dice_notation() {
        let yaml = r#"
dice:
  default: standard
  profiles:
    standard:
      notation: "2d7"
      direction: over
      outcomes:
        - { outcome: success }
"#;
        let error = Rulebook::load(yaml).unwrap_err();
        assert!(error.to_string().contains("standard"), "{error}");
    }

    #[test]
    fn loader_rejects_unknown_default_profile() {
        let yaml = r#"
dice:
  default: missing
  profiles:
    standard:
      notation: "2d6"
      direction: over
      outcomes:
        - { outcome: success }
"#;
        let error = Rulebook::load(yaml).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");
    }

    #[test]
    fn loader_rejects_unknown_advantage_pool() {
        let yaml = r#"
dice:
  default: standard
  profiles:
    standard:
      notation: "1d20"
      direction: over
      advantage: nope
      outcomes:
        - { outcome: success }
"#;
        let error = Rulebook::load(yaml).unwrap_err();
        assert!(error.to_string().contains("nope"), "{error}");
    }

    #[test]
    fn loader_warns_without_catch_all() {
        let yaml = r#"
dice:
  default: standard
  profiles:
    standard:
      notation: "2d6"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
"#;
        let loaded = Rulebook::load(yaml).expect("loads");
        assert!(
            loaded.warnings.iter().any(|w| w.contains("catch-all")),
            "{:?}",
            loaded.warnings
        );
    }

    #[test]
    fn loader_rejects_reversed_outcome_bounds() {
        for (low, high) in [
            ("score_at_least: 10", "score_at_most: 5"),
            ("margin_at_least: 0", "margin_at_most: -1"),
            ("degrees_at_least: 3", "degrees_at_most: 2"),
            ("target_at_least: 60", "target_at_most: 50"),
            ("degrees_min: 2", "degrees_max: 1"),
        ] {
            let yaml = format!(
                "dice:\n  default: standard\n  profiles:\n    standard:\n      notation: \"2d6\"\n      outcomes:\n        - {{ {low}, {high}, outcome: failure }}\n"
            );
            let error = Rulebook::load(&yaml).unwrap_err();
            let message = error.to_string();
            assert!(
                message.contains("standard") && message.contains("outcome row 0"),
                "{message}"
            );
        }
    }

    #[test]
    fn loader_accepts_equal_outcome_bounds() {
        let yaml = r#"
dice:
  default: standard
  profiles:
    standard:
      notation: "2d6"
      outcomes:
        - { score_at_least: 7, score_at_most: 7, outcome: success }
        - { outcome: failure }
"#;
        Rulebook::load(yaml).expect("equal bounds load");
    }

    #[test]
    fn direct_bonus_passes_value_through() {
        let yaml = r#"
characteristics:
  ws:
    name: WS
    min: 1
    max: 100
    bonus: direct
"#;
        let rulebook = Rulebook::load(yaml).expect("loads").rulebook;
        assert_eq!(rulebook.characteristics["ws"].bonus_for(42), 42);
    }

    #[cfg(feature = "spells")]
    #[test]
    fn spells_spend_resources_and_roll() {
        let rulebook = Rulebook::load(SAMPLE).expect("sample loads").rulebook;
        let mut character = Character::from_starting(&rulebook);
        let mut dice = ScriptedDice::new(&[6, 6]);

        let result = spell::cast(&rulebook, &mut character, "telekinesis", &mut dice).unwrap();
        assert_eq!(result.outcome, Outcome::CriticalSuccess);
        assert_eq!(result.spent, 2);
        assert_eq!(character.resource("focus"), Some(1));

        let error = spell::cast(&rulebook, &mut character, "telekinesis", &mut dice).unwrap_err();
        assert!(matches!(error, spell::SpellError::NotEnough { .. }));
    }

    #[cfg(feature = "spells")]
    #[test]
    fn spells_ignore_advantage_from_state() {
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
resources:
  mana: { name: Mana, min: 0, max: 5 }
spells:
  bolt:
    name: Bolt
    ability: a
    cost: { resource: mana, amount: 1 }
    check: { difficulty: 10 }
dice:
  default: standard
  profiles:
    standard:
      notation: "1d20"
      direction: over
      advantage: advantage_pool
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
    advantage_pool:
      notation: "2d20kh1"
      direction: over
      outcomes:
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
starting_character:
  characteristics: { c: 1 }
  perks: [lucky]
  resources: { mana: 5 }
"#;
        let rulebook = Rulebook::load(yaml).expect("loads").rulebook;
        let mut character = Character::from_starting(&rulebook);
        assert!(character.has_perk("lucky"));
        let mut dice = ScriptedDice::new(&[10, 4]);
        let result = spell::cast(&rulebook, &mut character, "bolt", &mut dice).unwrap();
        assert_eq!(result.check.pool, "standard");
    }
}
