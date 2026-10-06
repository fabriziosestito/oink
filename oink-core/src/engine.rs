//! Engine: wraps the Ink runtime, the rulebook, and player state.

use crate::data::GameData;
use crate::external::bind_external_functions;
use bladeink::story::Story;
use bladeink::story_error::StoryError;
use bladeink_compiler::Compiler;
use oink_rulebook::{
    Breakdown, Character, CheckResult, Dice, DiceRoll, RollKind, Rulebook, SeededDice, StateChange,
    SystemDice,
};
use std::cell::{Ref, RefCell};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Choice {
    pub index: usize,
    pub text: String,
}

/// What the UI should present next.
#[derive(Debug, Clone)]
pub enum Event {
    /// Prose paragraphs followed by choices to pick from.
    Scene {
        text: Vec<String>,
        choices: Vec<Choice>,
    },
    /// The story reached an end. "La tua vita e la tua missione terminano qui."
    TheEnd { text: Vec<String> },
}

/// Errors from compiling the ink source or running the story.
#[derive(Debug)]
pub enum EngineError {
    Compile(String),
    Story(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Compile(msg) => write!(f, "failed to compile story: {msg}"),
            EngineError::Story(msg) => write!(f, "story error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<StoryError> for EngineError {
    fn from(err: StoryError) -> Self {
        EngineError::Story(err.to_string())
    }
}

/// A check the story made, so the UI can show the roll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRecord {
    pub ability: String,
    pub difficulty: i32,
    pub outcome: String,
    pub pool: String,
    pub score: i32,
    pub target: i32,
    pub margin: i32,
    pub degrees: i32,
    pub dice: Option<DiceRoll>,
    pub breakdown: Breakdown,
}

impl CheckRecord {
    /// A one-line summary for the UI: every die with dropped dice marked,
    /// every modifier source, and the score against the target with degrees.
    pub fn describe(&self) -> String {
        let modifiers = self
            .breakdown
            .entries
            .iter()
            .map(|entry| format!("{} {:+}", entry.source, entry.value))
            .collect::<Vec<String>>()
            .join(", ");
        let modifiers = if modifiers.is_empty() {
            "no modifiers".to_string()
        } else {
            modifiers
        };
        match &self.dice {
            Some(roll) => {
                let dice_desc = match roll.kind {
                    RollKind::Percentile => format!("d% {}", roll.total),
                    RollKind::Sum => {
                        let parts: Vec<String> = roll
                            .dice
                            .iter()
                            .zip(roll.kept.iter())
                            .map(|(face, kept)| {
                                if *kept {
                                    format!("die {face}")
                                } else {
                                    format!("die {face} (dropped)")
                                }
                            })
                            .collect();
                        format!("{} = {}", parts.join(" + "), roll.total)
                    }
                };
                format!(
                    "* {} check: {}; {}; score {}, target {}, margin {:+}, {}, {} degrees",
                    self.ability,
                    dice_desc,
                    modifiers,
                    self.score,
                    self.target,
                    self.margin,
                    self.outcome,
                    self.degrees
                )
            }
            None => format!(
                "* {} sense: {}; score {}, {} vs {}",
                self.ability, modifiers, self.score, self.outcome, self.difficulty
            ),
        }
    }

    pub(crate) fn from_active(ability: String, difficulty: i32, result: &CheckResult) -> Self {
        Self {
            ability,
            difficulty,
            outcome: result.outcome.as_str().to_string(),
            pool: result.pool.clone(),
            score: result.score,
            target: result.target,
            margin: result.margin,
            degrees: result.degrees,
            dice: Some(result.roll.clone()),
            breakdown: result.breakdown.clone(),
        }
    }
}

/// State shared with the bound external functions.
pub(crate) struct RuleState {
    pub(crate) rulebook: Rc<Rulebook>,
    pub(crate) character: Character,
    pub(crate) dice: Box<dyn Dice>,
    pub(crate) changes: Vec<StateChange>,
    pub(crate) checks: Vec<CheckRecord>,
    pub(crate) last_check: Option<CheckResult>,
}

pub(crate) type Shared = Rc<RefCell<RuleState>>;

pub struct Engine {
    story: Story,
    pub data: GameData,
    state: Shared,
}

impl Engine {
    /// Compile ink source and load it into a new engine.
    pub fn new(ink_source: &str, data: GameData) -> Result<Self, EngineError> {
        let json = Compiler::new()
            .compile(ink_source)
            .map_err(|err| EngineError::Compile(err.to_string()))?;
        Self::from_json(&json, data)
    }

    /// Load a story from ink JSON, e.g. a precompiled `main.ink.json`.
    ///
    /// On device this is the entry point: compile at build time, flash the
    /// JSON, skip the compiler.
    pub fn from_json(story_json: &str, data: GameData) -> Result<Self, EngineError> {
        let mut story = Story::new(story_json)?;
        let rulebook = Rc::clone(&data.rulebook);
        let character = Character::from_starting(&rulebook);
        let state: Shared = Rc::new(RefCell::new(RuleState {
            rulebook,
            character,
            dice: Box::new(SystemDice::new()),
            changes: Vec::new(),
            checks: Vec::new(),
            last_check: None,
        }));
        bind_external_functions(&mut story, &state)?;
        Ok(Self { story, data, state })
    }

    /// Replace the dice with a seeded source. Use it in tests and replays.
    pub fn set_seed(&mut self, seed: u64) {
        self.state.borrow_mut().dice = Box::new(SeededDice::new(seed));
    }

    /// Read the character sheet.
    pub fn character(&self) -> Ref<'_, Character> {
        Ref::map(self.state.borrow(), |state| &state.character)
    }

    /// Take the changes made since the last call.
    pub fn take_changes(&mut self) -> Vec<StateChange> {
        std::mem::take(&mut self.state.borrow_mut().changes)
    }

    /// Take the checks made since the last call.
    pub fn take_checks(&mut self) -> Vec<CheckRecord> {
        std::mem::take(&mut self.state.borrow_mut().checks)
    }

    /// Start the story and return the first event.
    pub fn start(&mut self) -> Result<Event, EngineError> {
        self.resume()
    }

    /// Pick a choice by index and continue.
    pub fn choose(&mut self, index: usize) -> Result<Event, EngineError> {
        self.story.choose_choice_index(index)?;
        self.resume()
    }

    fn resume(&mut self) -> Result<Event, EngineError> {
        let mut text = Vec::new();
        while self.story.can_continue() {
            let line = self.story.cont()?;
            let line = line.trim();
            if !line.is_empty() {
                text.push(line.to_string());
            }
        }
        let choices: Vec<Choice> = self
            .story
            .get_current_choices()
            .iter()
            .map(|choice| Choice {
                index: *choice.index.borrow(),
                text: choice.text.clone(),
            })
            .collect();
        if choices.is_empty() {
            Ok(Event::TheEnd { text })
        } else {
            Ok(Event::Scene { text, choices })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = include_str!("../tests/fixtures/config.yaml");
    const RULEBOOK: &str = include_str!("../tests/fixtures/rulebook.yaml");
    const STORY: &str = include_str!("../tests/fixtures/story.ink");

    fn fixture_data() -> GameData {
        let data = GameData::from_yaml(CONFIG, RULEBOOK).unwrap();
        assert!(data.warnings.is_empty(), "warnings: {:?}", data.warnings);
        data
    }

    /// The fixture story on the fixture rulebook.
    fn fixture_engine() -> Engine {
        Engine::new(STORY, fixture_data()).unwrap()
    }

    /// An inline story on the fixture rulebook.
    fn example_engine(ink: &str) -> Engine {
        Engine::new(ink, fixture_data()).unwrap()
    }

    #[test]
    fn story_runs_and_ends() {
        let mut engine = fixture_engine();
        assert!(engine.character().has_item("stick"));

        let event = engine.start().unwrap();
        let Event::Scene { text, choices } = event else {
            panic!("expected choices")
        };
        assert_eq!(choices.len(), 2);
        assert!(text.join(" ").contains("Start."));

        // Spend path reaches the first pool with its points shown.
        let event = engine.choose(1).unwrap();
        let Event::Scene { text, .. } = event else {
            panic!("expected the spend scene")
        };
        assert!(text.join(" ").contains("Spend body: 1."));
    }

    #[test]
    fn precompiled_json_matches_source() {
        let json = Compiler::new().compile(STORY).unwrap();
        let mut engine = Engine::from_json(&json, fixture_data()).unwrap();
        assert!(matches!(engine.start().unwrap(), Event::Scene { .. }));
    }

    #[test]
    fn trial_checks_reach_the_end() {
        for trial_pick in [0, 1] {
            let mut engine = fixture_engine();
            engine.set_seed(7);
            engine.start().unwrap();

            // Preset, then either dark branch. Every outcome of either
            // check reaches the end.
            engine.choose(0).unwrap();
            let mut event = engine.choose(trial_pick).unwrap();
            let mut steps = 0;
            loop {
                match event {
                    Event::TheEnd { .. } => break,
                    Event::Scene { choices, .. } => {
                        steps += 1;
                        assert!(steps < 20, "trial did not end");
                        event = engine.choose(0).unwrap();
                        let _ = choices;
                    }
                }
            }
        }
    }

    #[test]
    fn potion_applies_condition_and_the_walk_clears_dark() {
        let mut engine = fixture_engine();
        engine.start().unwrap();

        // Spend path: exhaust every pool with the first option, take the
        // key branch, rest, then ask for judgment.
        let mut event = None;
        for pick in [1, 0, 0, 0, 1, 1, 0] {
            event = Some(engine.choose(pick).unwrap());
        }
        assert!(matches!(event, Some(Event::TheEnd { .. })));

        {
            let character = engine.character();
            assert!(character.has_condition("buzzed"));
            assert!(!character.has_environment("dark"));
            assert!(character.has_perk("brave"));
            assert_eq!(character.characteristic("body"), Some(3));
            assert_eq!(character.ability_level("might"), 2);
        }

        let changes = engine.take_changes();
        let described: Vec<String> = changes.iter().map(StateChange::to_string).collect();
        assert!(described.iter().any(|line| line.contains("buzzed")));
        assert!(described.iter().any(|line| line.contains("dark")));
    }

    #[test]
    fn consumables_and_resources_work_end_to_end() {
        let mut engine = example_engine(
            r#"
EXTERNAL use_item(item)
EXTERNAL has_item(item)
EXTERNAL has_condition(condition)
EXTERNAL resource(id)
EXTERNAL spend_resource(id, amount)
EXTERNAL restore_resource(id, amount)

VAR spent = false

~ use_item("potion")
{ has_item("potion"):
    The potion is still there.
- else:
    The potion is gone.
}
~ spent = spend_resource("stamina", 2)
Stamina is {resource("stamina")}.
~ spent = restore_resource("stamina", 1)
Stamina is {resource("stamina")}.
{ has_condition("buzzed"):
    Buzzed.
}
-> END
"#,
        );

        let event = engine.start().unwrap();
        let Event::TheEnd { text } = event else {
            panic!("expected the end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("The potion is gone."), "{joined}");
        assert!(joined.contains("Stamina is 1."), "{joined}");
        assert!(joined.contains("Stamina is 2."), "{joined}");
        assert!(joined.contains("Buzzed."), "{joined}");
    }

    #[test]
    fn checks_are_recorded_for_the_ui() {
        let mut engine = fixture_engine();
        engine.start().unwrap();
        assert!(engine.take_checks().is_empty());

        // Preset: the dark passive records once.
        engine.set_seed(7);
        engine.choose(0).unwrap();
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].ability, "wits");
        assert_eq!(checks[0].outcome, "pass");
        assert!(checks[0].dice.is_none());

        // Roll, then the key scene rolls too: two active records on two
        // profiles.
        engine.choose(0).unwrap();
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0].ability, "might");
        assert_eq!(checks[0].pool, "pair");
        assert_eq!(checks[1].ability, "wits");
        assert_eq!(checks[1].pool, "single");
        assert!(checks.iter().all(|c| c.dice.is_some()));
        assert!(!checks[0].breakdown.entries.is_empty());
    }

    #[test]
    fn check_records_show_each_die_and_the_modifiers() {
        let mut engine = fixture_engine();
        engine.set_seed(7);
        engine.start().unwrap();
        engine.choose(0).unwrap();
        engine.take_checks();
        engine.choose(0).unwrap();
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 2);
        let roll = checks[0].dice.clone().expect("active check rolls dice");
        let description = checks[0].describe();
        assert!(
            description.contains(&format!("die {}", roll.dice[0])),
            "{description}"
        );
        assert!(
            description.contains(&format!("die {}", roll.dice[1])),
            "{description}"
        );
        assert!(
            description.contains("perk brave (tag dark) +2"),
            "{description}"
        );
        assert!(description.contains("score"), "{description}");
        assert!(description.contains("target 7"), "{description}");
    }

    #[test]
    fn key_branch_grants_the_item() {
        let mut engine = fixture_engine();
        engine.start().unwrap();

        // Preset, take the key: the dark branch grants the item and shows it.
        engine.choose(0).unwrap();
        let event = engine.choose(1).unwrap();
        let Event::Scene { text, .. } = event else {
            panic!("expected the key scene")
        };
        assert!(text.join(" ").contains("Key taken."));
        assert!(engine.character().has_item("key"));
    }

    #[test]
    fn three_argument_queries_preserve_tags_and_penalties() {
        let mut engine = example_engine(
            r#"
EXTERNAL passive_value(ability, tags, modifier)
EXTERNAL check_breakdown(ability, tags, modifier)
Value: {passive_value("wits", "tested", -3)}.
Breakdown: {check_breakdown("wits", "tested", -3)}.
-> END
"#,
        );
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let text = text.join(" ");
        assert!(text.contains("Value: 9."), "{text}");
        assert!(text.contains("perk keen (tag tested) +2"), "{text}");
        assert!(text.contains("one-off -3"), "{text}");
        assert!(engine.take_checks().is_empty());
    }

    #[test]
    fn roll_check_with_profile_and_queries() {
        let mut engine = example_engine(
            r#"
EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
EXTERNAL check_roll()
EXTERNAL check_score()
EXTERNAL check_target()
EXTERNAL check_margin()
EXTERNAL check_degrees()
EXTERNAL check_die(index)
VAR outcome = ""
~ outcome = roll_check("might", 10, "", 0, "pair")
Outcome {outcome} roll {check_roll()} score {check_score()} target {check_target()} margin {check_margin()} degrees {check_degrees()} die0 {check_die(0)} die1 {check_die(1)}.
-> END
"#,
        );
        engine.set_seed(7);
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("Outcome "), "{joined}");
        assert!(joined.contains("roll "), "{joined}");
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].pool, "pair");
    }

    #[test]
    fn last_check_survives_take_checks() {
        let mut engine = example_engine(
            r#"
EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL check_score()
~ roll_check("might", 10, "", 0)
First.
+ [Continue]
    Score {check_score()}.
    -> END
"#,
        );
        engine.set_seed(7);
        engine.start().unwrap();
        // Drain the UI queue between scenes.
        assert_eq!(engine.take_checks().len(), 1);
        let Event::TheEnd { text } = engine.choose(0).unwrap() else {
            panic!("expected end")
        };
        assert!(text.join(" ").contains("Score "), "{text:?}");
    }

    #[test]
    fn fixture_story_runs_both_openings_to_both_endings() {
        let data = fixture_data();
        let mut open = false;
        let mut closed = false;
        for seed in 0..40 {
            for first in [0, 1] {
                let mut engine = Engine::new(STORY, data.clone()).unwrap();
                engine.set_seed(seed);
                let mut event = engine.start().unwrap();
                let mut transcript = String::new();
                let mut steps = 0;
                let end = loop {
                    match event {
                        Event::TheEnd { text } => break text.join(" "),
                        Event::Scene { text, choices } => {
                            transcript.push_str(&text.join(" "));
                            transcript.push(' ');
                            steps += 1;
                            assert!(steps < 60, "fixture story did not end");
                            assert!(!choices.is_empty());
                            let at = if steps == 1 {
                                first.min(choices.len() - 1)
                            } else {
                                0
                            };
                            event = engine.choose(at).unwrap();
                        }
                    }
                };
                assert!(!transcript.contains("outcome =="), "leak: {transcript}");
                assert!(transcript.contains("Sheet: body "), "{transcript}");
                assert!(transcript.contains("Passive passed."), "{transcript}");
                assert!(transcript.contains("Level 2."), "{transcript}");
                if end.contains("Open.") {
                    open = true;
                }
                if end.contains("Closed.") {
                    closed = true;
                }
            }
        }
        assert!(open && closed);
    }

    #[test]
    fn unknown_profile_is_a_story_error() {
        let mut engine = example_engine(
            r#"
EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
~ roll_check("might", 10, "", 0, "nope")
-> END
"#,
        );
        let error = engine.start().unwrap_err();
        assert!(error.to_string().contains("nope"), "{error}");
    }

    #[test]
    fn check_die_out_of_range_is_a_story_error() {
        let mut engine = example_engine(
            r#"
EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL check_die(index)
~ roll_check("might", 10, "", 0)
Die {check_die(5)}.
-> END
"#,
        );
        let error = engine.start().unwrap_err();
        assert!(error.to_string().contains("out of range"), "{error}");
    }

    #[test]
    fn check_queries_without_a_roll_are_story_errors() {
        let mut engine = example_engine(
            r#"
EXTERNAL check_score()
Score {check_score()}.
-> END
"#,
        );
        let error = engine.start().unwrap_err();
        assert!(error.to_string().contains("no active check"), "{error}");
    }

    #[test]
    fn explicit_scene_boundaries_expire_conditions_once() {
        let mut engine = example_engine(
            r#"
EXTERNAL add_condition(id)
EXTERNAL end_scene()
~ add_condition("buzzed")
First scene.
+ [Stay here]
    -> same_scene
=== same_scene ===
Still the first scene.
+ [Leave]
    ~ end_scene()
    -> second_scene
=== second_scene ===
Second scene.
+ [Leave again]
    ~ end_scene()
    -> END
"#,
        );
        engine.start().unwrap();
        engine.choose(0).unwrap();
        assert!(engine.character().has_condition("buzzed"));
        engine.choose(0).unwrap();
        assert!(engine.character().has_condition("buzzed"));
        engine.choose(0).unwrap();
        assert!(!engine.character().has_condition("buzzed"));
        let expired = engine
            .take_changes()
            .into_iter()
            .filter(|change| matches!(change, StateChange::ConditionRemoved(id) if id == "buzzed"))
            .count();
        assert_eq!(expired, 1);
    }

    #[test]
    fn passive_records_are_not_duplicated_by_lookahead() {
        let mut engine = example_engine(
            r#"
EXTERNAL passive_check(ability, dc, tags, modifier)
First line.
{passive_check("wits", 8, "", 0): You notice the clue.}
+ [Continue]
    -> END
"#,
        );
        engine.start().unwrap();
        assert_eq!(engine.take_checks().len(), 1);
        engine.choose(0).unwrap();
        assert!(engine.take_checks().is_empty());
    }

    #[test]
    fn resource_payments_are_all_or_nothing() {
        let mut engine = example_engine(
            r#"
EXTERNAL spend_resource(id, amount)
EXTERNAL resource(id)
{spend_resource("stamina", 4): Paid.|Not paid.}
Stamina: {resource("stamina")}.
-> END
"#,
        );
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        assert!(text.join(" ").contains("Not paid."));
        assert_eq!(engine.character().resource("stamina"), Some(3));
        assert!(engine.take_changes().is_empty());
    }

    #[test]
    fn resource_max_binding_reports_the_maximum() {
        let data = GameData::from_yaml(
            "title: Test",
            "resources:\n  focus: { name: Focus, min: 0, max: 5 }\n",
        )
        .unwrap();
        let mut engine = Engine::new(
            "EXTERNAL resource_max(id)\nMax {resource_max(\"focus\")}.\n-> END\n",
            data,
        )
        .unwrap();
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        assert!(text.join(" ").contains("Max 5."), "{text:?}");

        let data = GameData::from_yaml(
            "title: Test",
            "resources:\n  focus: { name: Focus, min: 0, max: 5 }\n",
        )
        .unwrap();
        let mut engine = Engine::new(
            "EXTERNAL resource_max(id)\nMax {resource_max(\"nope\")}.\n-> END\n",
            data,
        )
        .unwrap();
        let error = engine.start().unwrap_err();
        assert!(
            error.to_string().contains("unknown resource `nope`"),
            "{error}"
        );
    }

    #[test]
    fn creation_bindings_spend_and_apply_presets() {
        let yaml = "characteristics:\n  physique: { name: Physique, min: 1, max: 14 }\ncharacter_creation:\n  pools: { characteristic_points: 10 }\n  presets:\n    bruiser: { name: Bruiser, characteristics: { physique: 8 } }\n";
        let data = GameData::from_yaml("title: Test", yaml).unwrap();
        let mut engine = Engine::new(
            "EXTERNAL points_available(kind)\nEXTERNAL spend_point(kind, id)\nEXTERNAL apply_preset(id)\nPoints {points_available(\"characteristic\")}.\n~ spend_point(\"characteristic\", \"physique\")\nLeft {points_available(\"characteristic\")}.\n~ apply_preset(\"bruiser\")\n-> END\n",
            data,
        )
        .unwrap();
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("Points 10."), "{joined}");
        assert!(joined.contains("Left 9."), "{joined}");
        assert_eq!(engine.character().characteristic("physique"), Some(8));

        let data = GameData::from_yaml(
            "title: Test",
            "characteristics:\n  physique: { name: Physique, min: 1, max: 14 }\ncharacter_creation:\n  pools: { characteristic_points: 10 }\n",
        )
        .unwrap();
        let mut engine = Engine::new(
            "EXTERNAL points_available(kind)\nPoints {points_available(\"nope\")}.\n-> END\n",
            data,
        )
        .unwrap();
        let error = engine.start().unwrap_err();
        assert!(
            error.to_string().contains("unknown point kind `nope`"),
            "{error}"
        );
    }

    #[test]
    fn levelling_bindings_bank_xp_and_level_up() {
        let yaml = "characteristics:\n  c: { name: C, min: 1, max: 14 }\ncharacter_creation:\n  pools: { characteristic_points: 0 }\nlevelling:\n  max_level: 3\n  xp_curve:\n    - { level: 2, xp: 100 }\n  rewards:\n    per_level: { characteristic_points: 1 }\n";
        let data = GameData::from_yaml("title: Test", yaml).unwrap();
        let mut engine = Engine::new(
            "EXTERNAL xp()\nEXTERNAL add_xp(amount)\nEXTERNAL level()\nEXTERNAL level_up_ready()\nEXTERNAL level_up()\nEXTERNAL points_available(kind)\nVAR up = false\nLevel {level()}, XP {xp()}.\n~ add_xp(120)\nReady {level_up_ready()}.\n~ up = level_up()\nLevel {level()}, bonus {points_available(\"characteristic\")}.\n-> END\n",
            data,
        )
        .unwrap();
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("Level 1, XP 0."), "{joined}");
        assert!(joined.contains("Ready true."), "{joined}");
        assert!(joined.contains("Level 2, bonus 1."), "{joined}");

        let data = GameData::from_yaml(
            "title: Test",
            "characteristics:\n  c: { name: C, min: 1, max: 14 }\n",
        )
        .unwrap();
        let mut engine =
            Engine::new("EXTERNAL add_xp(amount)\n~ add_xp(-5)\n-> END\n", data).unwrap();
        let error = engine.start().unwrap_err();
        assert!(
            error.to_string().contains("xp amount must be nonnegative"),
            "{error}"
        );
    }

    #[test]
    fn add_perk_and_add_item_enforce_prerequisites() {
        let yaml = "characteristics:\n  physique: { name: Physique, min: 1, max: 14 }\nperks:\n  juggernaut: { name: Juggernaut }\nitems:\n  rope: { name: Rope }\nprerequisites:\n  juggernaut:\n    requires: { characteristics: { physique: 6 } }\n  rope:\n    requires: { characteristics: { physique: 6 } }\n";
        let data = GameData::from_yaml("title: Test", yaml).unwrap();
        let mut engine = Engine::new(
            "EXTERNAL set_characteristic(id, value)\nEXTERNAL add_perk(id)\nEXTERNAL add_item(id)\nEXTERNAL has_perk(id)\nEXTERNAL has_item(id)\n~ set_characteristic(\"physique\", 8)\n~ add_perk(\"juggernaut\")\n~ add_item(\"rope\")\n{ has_perk(\"juggernaut\"): Strong.|Weak.}\n{ has_item(\"rope\"): Rope.|No rope.}\n-> END\n",
            data,
        )
        .unwrap();
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("Strong."), "{joined}");
        assert!(joined.contains("Rope."), "{joined}");

        let data = GameData::from_yaml(
            "title: Test",
            "characteristics:\n  physique: { name: Physique, min: 1, max: 14 }\nperks:\n  juggernaut: { name: Juggernaut }\nprerequisites:\n  juggernaut:\n    requires: { characteristics: { physique: 6 } }\n",
        )
        .unwrap();
        let mut engine = Engine::new(
            "EXTERNAL add_perk(id)\n~ add_perk(\"juggernaut\")\n-> END\n",
            data,
        )
        .unwrap();
        let error = engine.start().unwrap_err();
        assert!(
            error
                .to_string()
                .contains("missing prerequisites for `juggernaut`"),
            "{error}"
        );
    }

    #[test]
    fn wrong_argument_types_fail_instead_of_using_defaults() {
        let mut engine = example_engine(
            r#"
EXTERNAL passive_value(ability, tags, modifier)
{passive_value("wits", "", "wrong")}
-> END
"#,
        );
        assert!(engine
            .start()
            .unwrap_err()
            .to_string()
            .contains("argument 3 must be integer"));
    }

    #[test]
    fn perk_condition_and_value_bindings_work_end_to_end() {
        let mut engine = example_engine(
            r#"
EXTERNAL ability_level(id)
EXTERNAL characteristic(id)
EXTERNAL characteristic_bonus(id)
EXTERNAL add_perk(id)
EXTERNAL remove_perk(id)
EXTERNAL has_perk(id)
EXTERNAL add_condition(id)
EXTERNAL remove_condition(id)

Might {ability_level("might")}, absent {ability_level("sneak")}.
Body {characteristic("body")}, bonus {characteristic_bonus("body")}.
~ add_perk("brave")
{ has_perk("brave"): Brave.|Not brave.}
~ remove_perk("brave")
{ has_perk("brave"): Still brave.|Brave gone.}
~ add_condition("tired")
~ remove_condition("tired")
-> END
"#,
        );
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("Might 1, absent 0."), "{joined}");
        assert!(joined.contains("Body 2, bonus -1."), "{joined}");
        assert!(joined.contains("Brave."), "{joined}");
        assert!(joined.contains("Brave gone."), "{joined}");
        assert_eq!(
            engine.take_changes(),
            vec![
                StateChange::PerkAdded("brave".to_string()),
                StateChange::PerkRemoved("brave".to_string()),
                StateChange::ConditionAdded("tired".to_string()),
                StateChange::ConditionRemoved("tired".to_string()),
            ]
        );
    }

    #[test]
    fn unknown_perk_and_characteristic_produce_story_errors() {
        let mut engine = example_engine(
            r#"
EXTERNAL add_perk(id)
~ add_perk("nope")
-> END
"#,
        );
        let error = engine.start().unwrap_err();
        assert!(error.to_string().contains("unknown perk `nope`"), "{error}");

        let mut engine = example_engine(
            r#"
EXTERNAL characteristic(id)
{characteristic("nope")}
-> END
"#,
        );
        let error = engine.start().unwrap_err();
        assert!(
            error.to_string().contains("unknown characteristic `nope`"),
            "{error}"
        );
    }

    #[test]
    fn reference_ink_examples_compile_and_run() {
        let overview = include_str!("../../docs/rulebook.md");
        let minimal_yaml = overview
            .split("```yaml\n")
            .nth(2)
            .unwrap()
            .split("```")
            .next()
            .unwrap();
        for (page, markdown) in [
            ("overview", overview),
            ("checks", include_str!("../../docs/reference/checks.md")),
            ("state", include_str!("../../docs/reference/state.md")),
            ("ink-api", include_str!("../../docs/reference/ink-api.md")),
        ] {
            for block in markdown.split("```ink\n").skip(1) {
                let source = block.split("```").next().unwrap();
                let yaml = if page == "overview" {
                    minimal_yaml
                } else {
                    include_str!("../../examples/high-pass/rulebook.yaml")
                };
                let data = GameData::from_yaml("title: Documentation", yaml).unwrap();
                let mut engine =
                    Engine::new(source, data).unwrap_or_else(|error| panic!("{page}: {error}"));
                engine.set_seed(7);
                let mut event = engine
                    .start()
                    .unwrap_or_else(|error| panic!("{page}: {error}"));
                let mut steps = 0;
                while let Event::Scene { choices, .. } = event {
                    assert!(!choices.is_empty(), "{page}");
                    steps += 1;
                    assert!(steps < 10, "example did not end: {page}");
                    event = engine
                        .choose(0)
                        .unwrap_or_else(|error| panic!("{page}: {error}"));
                }
            }
        }
    }

    #[test]
    fn spells_are_bound_and_spend_resources() {
        let mut engine = example_engine(
            r#"
EXTERNAL cast_spell(id)
EXTERNAL resource(id)

VAR outcome = ""

~ outcome = cast_spell("zap")
The spell: {outcome}.
Stamina is {resource("stamina")}.
-> END
"#,
        );
        engine.set_seed(3);

        let event = engine.start().unwrap();
        let Event::TheEnd { text } = event else {
            panic!("expected the end")
        };
        assert!(text.join(" ").contains("Stamina is 1."), "{text:?}");
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].ability, "spark");
        assert_eq!(checks[0].difficulty, 7);
        assert!(checks[0].dice.is_some());
        assert_eq!(
            engine.take_changes(),
            vec![StateChange::ResourceChanged {
                id: "stamina".to_string(),
                from: 3,
                to: 1,
            }]
        );
    }
}
