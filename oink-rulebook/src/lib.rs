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
    Ability, BonusTable, BonusThreshold, CharacterCreation, Characteristic, CharacteristicBonus,
    Condition, Cost, CreationBase, CreationCosts, CreationMode, CreationPools, CreationPreset,
    CreationValidation, Degrees, DiceConfig, DicePool, DiceProfile, Die, DifficultyRef,
    DirectMarker, Direction, Environment, Item, Keep, LevelInterval, LevelPoints, LevelRewards,
    LevelThreshold, Levelling, MaxFromEntry, MaxFromMode, MaxFromThreshold, Modifiers, OutcomeRule,
    Perk, Prerequisite, Requirement, Resource, Rulebook, Spell, SpellCheck, StartingCharacter, Tag,
    TagGrants,
};
pub use modifiers::{Breakdown, BreakdownEntry};
pub use names::{Names, Section};
#[cfg(feature = "spells")]
pub use spell::{CastResult, SpellError};
pub use state::{Character, CreationPointKind, CreationSpent, StateChange};

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

    const CREATION_FIXTURE: &str = r#"
characteristics:
  intellect:
    name: Intellect
    min: 1
    max: 14
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
  physique:
    name: Physique
    min: 1
    max: 14
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
abilities:
  logic:
    name: Logic
    characteristic: intellect
  endurance:
    name: Endurance
    characteristic: physique
perks:
  juggernaut:
    name: Juggernaut
tags:
  strong:
    name: Strong
character_creation:
  mode: pool
  base:
    characteristics: { intellect: 2, physique: 2 }
    abilities: { logic: 0, endurance: 0 }
  pools:
    characteristic_points: 10
    ability_points: 5
    perk_points: 1
  costs:
    characteristics: 1
    abilities: 1
  validate: all_points_spent
  presets:
    bruiser:
      name: Bruiser
      characteristics: { intellect: 2, physique: 8 }
      abilities: { endurance: 3 }
      perks: [juggernaut]
      tags: [strong]
"#;

    #[test]
    fn creation_config_loads_with_pools_costs_and_presets() {
        let rulebook = Rulebook::load(CREATION_FIXTURE)
            .expect("fixture loads")
            .rulebook;
        let creation = rulebook.creation.expect("creation is set");
        assert_eq!(creation.mode, CreationMode::Pool);
        assert_eq!(creation.pools.characteristic_points, 10);
        assert_eq!(creation.pools.ability_points, 5);
        assert_eq!(creation.pools.perk_points, 1);
        assert_eq!(creation.costs.characteristics, 1);
        assert_eq!(creation.validate, CreationValidation::AllPointsSpent);
        let bruiser = &creation.presets["bruiser"];
        assert_eq!(bruiser.characteristics["physique"], 8);
        assert_eq!(bruiser.abilities["endurance"], 3);
        assert_eq!(bruiser.perks, vec!["juggernaut".to_string()]);
    }

    #[test]
    fn missing_creation_section_means_no_creation() {
        let rulebook = fixture();
        assert!(rulebook.creation.is_none());
    }

    #[test]
    fn loader_rejects_zero_creation_costs() {
        for (costs, message) in [
            (
                "costs:\n    characteristics: 0",
                "character creation cost for characteristics must be positive",
            ),
            (
                "costs:\n    abilities: 0",
                "character creation cost for abilities must be positive",
            ),
        ] {
            let yaml = format!("character_creation:\n  {costs}\n");
            let error = Rulebook::load(&yaml).unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn creation_costs_default_to_one_flat_point() {
        let yaml = "character_creation:\n  pools:\n    characteristic_points: 3\n";
        let rulebook = Rulebook::load(yaml).expect("loads").rulebook;
        let creation = rulebook.creation.expect("creation is set");
        assert_eq!(creation.costs.characteristics, 1);
        assert_eq!(creation.costs.abilities, 1);
    }

    #[test]
    fn loader_rejects_unknown_creation_references() {
        for (section, message) in [
            (
                "base:\n    characteristics: { missing: 2 }",
                "character creation base has unknown characteristic `missing`",
            ),
            (
                "presets:\n    p:\n      name: P\n      abilities: { missing: 1 }",
                "character creation preset `p` has unknown ability `missing`",
            ),
            (
                "presets:\n    p:\n      name: P\n      perks: [missing]",
                "character creation preset `p` has unknown perk `missing`",
            ),
        ] {
            let yaml = format!("character_creation:\n  {section}\n");
            let error = Rulebook::load(&yaml).unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    fn creation_character() -> (Rulebook, Character) {
        let rulebook = Rulebook::load(CREATION_FIXTURE)
            .expect("fixture loads")
            .rulebook;
        let character = Character::begin_creation(&rulebook);
        (rulebook, character)
    }

    #[test]
    fn begin_creation_uses_base_and_zeroes_spent() {
        let (rulebook, character) = creation_character();
        assert_eq!(character.characteristic("intellect"), Some(2));
        assert_eq!(character.characteristic("physique"), Some(2));
        assert_eq!(character.ability_level("logic"), 0);
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            10
        );
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Ability),
            5
        );
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Perk),
            1
        );
        assert_eq!(character.validate_creation(&rulebook).len(), 3);
    }

    #[test]
    fn spend_point_raises_and_consumes_pool() {
        let (rulebook, mut character) = creation_character();
        assert!(character.spend_point(&rulebook, CreationPointKind::Characteristic, "physique"));
        assert_eq!(character.characteristic("physique"), Some(3));
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            9
        );
        assert!(character.spend_point(&rulebook, CreationPointKind::Ability, "logic"));
        assert_eq!(character.ability_level("logic"), 1);
        assert!(character.spend_point(&rulebook, CreationPointKind::Perk, "juggernaut"));
        assert!(character.has_perk("juggernaut"));
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Perk),
            0
        );
        // Already owned perks and unknown ids spend nothing.
        assert!(!character.spend_point(&rulebook, CreationPointKind::Perk, "juggernaut"));
        assert!(!character.spend_point(&rulebook, CreationPointKind::Perk, "missing"));
        assert!(!character.spend_point(&rulebook, CreationPointKind::Characteristic, "missing"));
        assert!(!character.spend_point(&rulebook, CreationPointKind::Ability, "missing"));
    }

    #[test]
    fn spend_point_without_creation_section_fails() {
        let rulebook = fixture();
        let mut character = Character::begin_creation(&rulebook);
        assert!(!character.spend_point(&rulebook, CreationPointKind::Characteristic, "intellect"));
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            0
        );
        assert!(character.validate_creation(&rulebook).is_empty());
    }

    #[test]
    fn spend_point_respects_costs_and_maxima() {
        let yaml = r#"
characteristics:
  c:
    name: C
    min: 1
    max: 10
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
character_creation:
  base:
    characteristics: { c: 2 }
  pools:
    characteristic_points: 3
  costs:
    characteristics: 2
"#;
        let rulebook = Rulebook::load(yaml).expect("loads").rulebook;
        let mut character = Character::begin_creation(&rulebook);
        assert!(character.spend_point(&rulebook, CreationPointKind::Characteristic, "c"));
        assert_eq!(character.characteristic("c"), Some(3));
        // One point left, cost two: refused without changes.
        assert!(!character.spend_point(&rulebook, CreationPointKind::Characteristic, "c"));
        assert_eq!(character.characteristic("c"), Some(3));
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            1
        );

        let yaml = r#"
characteristics:
  c:
    name: C
    min: 1
    max: 3
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
character_creation:
  base:
    characteristics: { c: 2 }
  pools:
    characteristic_points: 10
"#;
        let rulebook = Rulebook::load(yaml).expect("loads").rulebook;
        let mut character = Character::begin_creation(&rulebook);
        assert!(character.spend_point(&rulebook, CreationPointKind::Characteristic, "c"));
        assert_eq!(character.characteristic("c"), Some(3));
        // At max: refused even with points left.
        assert!(!character.spend_point(&rulebook, CreationPointKind::Characteristic, "c"));
    }

    #[test]
    fn apply_preset_sets_sheet_and_marks_pools_spent() {
        let (rulebook, mut character) = creation_character();
        assert!(character.apply_preset(&rulebook, "bruiser"));
        assert_eq!(character.characteristic("physique"), Some(8));
        assert_eq!(character.ability_level("endurance"), 3);
        assert!(character.has_perk("juggernaut"));
        assert!(character.has_tag(&rulebook, "strong"));
        assert!(character.validate_creation(&rulebook).is_empty());
        assert!(!character.apply_preset(&rulebook, "missing"));
    }

    #[test]
    fn preset_matches_manual_spending() {
        let (rulebook, mut manual) = creation_character();
        for _ in 0..6 {
            assert!(manual.spend_point(&rulebook, CreationPointKind::Characteristic, "physique"));
        }
        for _ in 0..3 {
            assert!(manual.spend_point(&rulebook, CreationPointKind::Ability, "endurance"));
        }
        assert!(manual.spend_point(&rulebook, CreationPointKind::Perk, "juggernaut"));

        let (_, mut preset) = creation_character();
        assert!(preset.apply_preset(&rulebook, "bruiser"));

        assert_eq!(
            manual.characteristic("physique"),
            preset.characteristic("physique")
        );
        assert_eq!(
            manual.characteristic("intellect"),
            preset.characteristic("intellect")
        );
        assert_eq!(
            manual.ability_level("endurance"),
            preset.ability_level("endurance")
        );
        assert_eq!(manual.has_perk("juggernaut"), preset.has_perk("juggernaut"));
    }

    #[test]
    fn setters_write_directly_and_reset_restores_base() {
        let (rulebook, mut character) = creation_character();
        assert!(character.set_characteristic(&rulebook, "physique", 99));
        assert_eq!(character.characteristic("physique"), Some(14));
        assert!(!character.set_characteristic(&rulebook, "missing", 5));
        assert!(character.set_ability(&rulebook, "logic", -5));
        assert_eq!(character.ability_level("logic"), 0);
        assert!(!character.set_ability(&rulebook, "missing", 1));
        // Setters spend nothing.
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            10
        );

        character.reset_character(&rulebook);
        assert!(character.spend_point(&rulebook, CreationPointKind::Characteristic, "physique"));
        assert_eq!(character.characteristic("physique"), Some(3));
        character.reset_character(&rulebook);
        assert_eq!(character.characteristic("physique"), Some(2));
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            10
        );
        assert!(!character.has_perk("juggernaut"));
    }

    #[test]
    fn validate_creation_catches_unspent_points() {
        let (rulebook, mut character) = creation_character();
        let problems = character.validate_creation(&rulebook);
        assert_eq!(problems.len(), 3);
        assert!(problems
            .iter()
            .any(|problem| problem.contains("characteristic")));

        for _ in 0..10 {
            assert!(character.spend_point(
                &rulebook,
                CreationPointKind::Characteristic,
                "physique"
            ));
        }
        for _ in 0..5 {
            assert!(character.spend_point(&rulebook, CreationPointKind::Ability, "logic"));
        }
        assert!(character.spend_point(&rulebook, CreationPointKind::Perk, "juggernaut"));
        assert!(character.validate_creation(&rulebook).is_empty());
    }

    const PREREQ_FIXTURE: &str = r#"
characteristics:
  physique:
    name: Physique
    min: 1
    max: 14
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
  motorics:
    name: Motorics
    min: 1
    max: 14
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
abilities:
  endurance:
    name: Endurance
    characteristic: physique
resources:
  focus: { name: Focus, min: 0, max: 5 }
perks:
  juggernaut:
    name: Juggernaut
  titan:
    name: Titan
items:
  rope:
    name: Rope
prerequisites:
  juggernaut:
    requires:
      characteristics: { physique: 6 }
  titan:
    requires:
      characteristics: { physique: 99 }
  endurance:
    requires:
      characteristics: { motorics: 4 }
  rope:
    requires:
      abilities: { endurance: 1 }
      resources: { focus: 2 }
character_creation:
  base:
    characteristics: { physique: 2, motorics: 2 }
  pools:
    characteristic_points: 10
    perk_points: 1
  presets:
    bruiser:
      name: Bruiser
      characteristics: { physique: 8 }
      perks: [juggernaut]
    dreamer:
      name: Dreamer
      characteristics: { physique: 8 }
      perks: [titan]
starting_character:
  resources: { focus: 3 }
"#;

    fn prereq_fixture() -> (Rulebook, Character) {
        let rulebook = Rulebook::load(PREREQ_FIXTURE)
            .expect("fixture loads")
            .rulebook;
        let character = Character::from_starting(&rulebook);
        (rulebook, character)
    }

    #[test]
    fn prerequisites_gate_on_values() {
        let (rulebook, mut character) = prereq_fixture();
        assert!(!character.meets_prerequisite(&rulebook, "juggernaut"));
        assert!(character.set_characteristic(&rulebook, "physique", 6));
        assert!(character.meets_prerequisite(&rulebook, "juggernaut"));

        assert!(!character.meets_prerequisite(&rulebook, "endurance"));
        assert!(character.set_characteristic(&rulebook, "motorics", 4));
        assert!(character.meets_prerequisite(&rulebook, "endurance"));

        // Rope needs endurance 1 and focus 2: balance starts at 3.
        assert!(!character.meets_prerequisite(&rulebook, "rope"));
        assert!(character.set_ability(&rulebook, "endurance", 1));
        assert!(character.meets_prerequisite(&rulebook, "rope"));
        character.spend_resource(&rulebook, "focus", 2);
        assert_eq!(character.resource("focus"), Some(1));
        assert!(!character.meets_prerequisite(&rulebook, "rope"));

        // Ids without an entry always pass.
        assert!(character.meets_prerequisite(&rulebook, "whatever"));
    }

    #[test]
    fn loader_rejects_unknown_prerequisite_targets() {
        for (section, message) in [
            (
                "missing:\n    requires:\n      characteristics: { physique: 1 }",
                "prerequisite `missing` gates unknown perk, ability, item, or spell",
            ),
            (
                "juggernaut:\n    requires:\n      characteristics: { missing: 1 }",
                "prerequisite `juggernaut` requires unknown characteristic `missing`",
            ),
            (
                "juggernaut:\n    requires:\n      abilities: { missing: 1 }",
                "prerequisite `juggernaut` requires unknown ability `missing`",
            ),
            (
                "juggernaut:\n    requires:\n      resources: { missing: 1 }",
                "prerequisite `juggernaut` requires unknown resource `missing`",
            ),
        ] {
            let yaml = format!(
                "perks:\n  juggernaut:\n    name: Juggernaut\nprerequisites:\n  {section}\n"
            );
            let error = Rulebook::load(&yaml).unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
        }
    }

    #[test]
    fn spend_point_enforces_prerequisites() {
        let (rulebook, _) = prereq_fixture();
        let mut created = Character::begin_creation(&rulebook);
        assert!(!created.spend_point(&rulebook, CreationPointKind::Perk, "juggernaut"));
        assert!(created.set_characteristic(&rulebook, "physique", 6));
        assert!(created.spend_point(&rulebook, CreationPointKind::Perk, "juggernaut"));
        assert!(created.has_perk("juggernaut"));
    }

    #[test]
    fn apply_preset_skips_unmet_prerequisites() {
        let (rulebook, _) = prereq_fixture();
        let mut created = Character::begin_creation(&rulebook);
        assert!(created.apply_preset(&rulebook, "bruiser"));
        assert!(created.has_perk("juggernaut"));

        assert!(created.apply_preset(&rulebook, "dreamer"));
        assert_eq!(created.characteristic("physique"), Some(8));
        assert!(!created.has_perk("titan"));
    }

    const LEVELLING_FIXTURE: &str = r#"
characteristics:
  c:
    name: C
    min: 1
    max: 14
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  a:
    name: A
    characteristic: c
character_creation:
  pools:
    characteristic_points: 0
    ability_points: 0
    perk_points: 0
levelling:
  max_level: 4
  xp_curve:
    - { level: 2, xp: 100 }
    - { level: 3, xp: 250 }
    - { level: 4, xp: 450 }
  rewards:
    per_level:
      characteristic_points: 1
      ability_points: 2
    interval:
      every: 3
      perk_points: 1
"#;

    fn levelling_fixture() -> (Rulebook, Character) {
        let rulebook = Rulebook::load(LEVELLING_FIXTURE)
            .expect("fixture loads")
            .rulebook;
        let character = Character::from_starting(&rulebook);
        (rulebook, character)
    }

    #[test]
    fn level_starts_at_one_with_zero_xp() {
        let (rulebook, character) = levelling_fixture();
        assert_eq!(character.level(), 1);
        assert_eq!(character.xp(), 0);
        assert!(!character.level_up_ready(&rulebook));
        let created = Character::begin_creation(&rulebook);
        assert_eq!(created.level(), 1);
    }

    #[test]
    fn add_xp_banks_without_levelling() {
        let (rulebook, mut character) = levelling_fixture();
        character.add_xp(120);
        assert_eq!(character.xp(), 120);
        assert_eq!(character.level(), 1);
        assert!(character.level_up_ready(&rulebook));
    }

    #[test]
    fn level_up_grants_rewards_one_step() {
        let (rulebook, mut character) = levelling_fixture();
        assert!(!character.level_up(&rulebook));
        character.add_xp(120);
        assert!(character.level_up(&rulebook));
        assert_eq!(character.level(), 2);
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Characteristic),
            1
        );
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Ability),
            2
        );
        // Next threshold needs 250: not ready, second call fails shut.
        assert!(!character.level_up_ready(&rulebook));
        assert!(!character.level_up(&rulebook));
        assert_eq!(character.level(), 2);
    }

    #[test]
    fn level_up_climbs_one_step_with_interval_rewards() {
        let (rulebook, mut character) = levelling_fixture();
        character.add_xp(1000);
        assert!(character.level_up(&rulebook));
        assert!(character.level_up(&rulebook));
        assert!(character.level_up(&rulebook));
        assert_eq!(character.level(), 4);
        // Level 3 hits the interval: one perk point granted.
        assert_eq!(
            character.points_available(&rulebook, CreationPointKind::Perk),
            1
        );
        // At the cap: not ready, level_up refuses.
        assert!(!character.level_up_ready(&rulebook));
        assert!(!character.level_up(&rulebook));
    }

    #[test]
    fn levelling_absent_means_no_levels() {
        let rulebook = fixture();
        let mut character = Character::from_starting(&rulebook);
        character.add_xp(500);
        assert_eq!(character.xp(), 500);
        assert!(!character.level_up_ready(&rulebook));
        assert!(!character.level_up(&rulebook));
        assert_eq!(character.level(), 1);
    }

    #[test]
    fn checks_grant_no_xp() {
        let (rulebook, character) = levelling_fixture();
        let checks = Checks::new(&rulebook, &character);
        let mut dice = SeededDice::new(7);
        let request = CheckRequest::new("a", 5);
        checks.active(&mut dice, &request).unwrap();
        assert_eq!(character.xp(), 0);
    }

    #[test]
    fn loader_rejects_bad_levelling() {
        for (curve, message) in [
            (
                "- { level: 1, xp: 0 }",
                "levelling curve level 1 is outside 2..=4",
            ),
            (
                "- { level: 5, xp: 100 }",
                "levelling curve level 5 is outside 2..=4",
            ),
            (
                "- { level: 2, xp: 100 }\n    - { level: 2, xp: 200 }",
                "levelling curve level 2 appears twice",
            ),
        ] {
            let yaml = format!("levelling:\n  max_level: 4\n  xp_curve:\n    {curve}\n");
            let error = Rulebook::load(&yaml).unwrap_err();
            assert!(error.to_string().contains(message), "{error}");
        }
        let yaml = "levelling:\n  max_level: 4\n  rewards:\n    interval:\n      every: 0\n      perk_points: 1\n";
        let error = Rulebook::load(yaml).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("levelling interval every must be positive"),
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
