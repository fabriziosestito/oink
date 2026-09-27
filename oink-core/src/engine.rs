//! Engine: wraps the Ink runtime, the rulebook, and player state.

use crate::data::GameData;
use crate::external::bind_external_functions;
use bladeink::story::Story;
use bladeink::story_error::StoryError;
use bladeink_compiler::Compiler;
use oink_rulebook::{Breakdown, Character, Dice, Rulebook, SeededDice, StateChange, SystemDice};
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
    pub total: i32,
    pub dice: Option<[u8; 2]>,
    pub breakdown: Breakdown,
}

impl CheckRecord {
    /// A one-line summary for the UI: each die, every modifier source, the
    /// total, and the target.
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
        match self.dice {
            Some(dice) => {
                let dice_total = i32::from(dice[0]) + i32::from(dice[1]);
                format!(
                    "* {} check: die {} + die {} = {}; {}; total {}, {} vs {}",
                    self.ability,
                    dice[0],
                    dice[1],
                    dice_total,
                    modifiers,
                    self.total,
                    self.outcome,
                    self.difficulty
                )
            }
            None => format!(
                "* {} sense: {}; total {}, {} vs {}",
                self.ability, modifiers, self.total, self.outcome, self.difficulty
            ),
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

    fn demo_engine() -> Engine {
        let ink = std::fs::read_to_string("../assets/story/main.ink").unwrap();
        let data = GameData::from_yaml(
            &std::fs::read_to_string("../assets/data/config.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/rulebook.yaml").unwrap(),
        )
        .unwrap();
        assert!(data.warnings.is_empty(), "warnings: {:?}", data.warnings);
        Engine::new(&ink, data).unwrap()
    }

    #[test]
    fn story_runs_and_ends() {
        let mut engine = demo_engine();
        assert!(engine.character().has_item("rusty_cleaver"));

        let event = engine.start().unwrap();
        let Event::Scene { text, choices } = event else {
            panic!("expected choices")
        };
        assert_eq!(choices.len(), 3);
        assert!(text
            .join(" ")
            .contains("Something in the troll's grip is not as steady"));

        // Talk to the troll -> victory -> END
        let event = engine.choose(1).unwrap();
        assert!(matches!(event, Event::TheEnd { .. }));
    }

    #[test]
    fn precompiled_json_matches_source() {
        let ink = std::fs::read_to_string("../assets/story/main.ink").unwrap();
        let json = Compiler::new().compile(&ink).unwrap();
        let data = GameData::from_yaml(
            &std::fs::read_to_string("../assets/data/config.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/rulebook.yaml").unwrap(),
        )
        .unwrap();
        let mut engine = Engine::from_json(&json, data).unwrap();
        assert!(matches!(engine.start().unwrap(), Event::Scene { .. }));
    }

    #[test]
    fn fight_path_calls_external_functions() {
        let mut engine = demo_engine();
        engine.set_seed(7);
        engine.start().unwrap();

        // Charge the troll. Every outcome of the check reaches the end.
        let event = engine.choose(0).unwrap();
        assert!(matches!(event, Event::TheEnd { .. }));
    }

    #[test]
    fn victory_applies_condition_and_clears_environment() {
        let mut engine = demo_engine();
        engine.start().unwrap();

        let event = engine.choose(1).unwrap();
        assert!(matches!(event, Event::TheEnd { .. }));

        {
            let character = engine.character();
            assert!(character.has_condition("shaken"));
            assert!(!character.has_environment("dark"));
        }

        let changes = engine.take_changes();
        let described: Vec<String> = changes.iter().map(StateChange::to_string).collect();
        assert!(described.iter().any(|line| line.contains("shaken")));
        assert!(described.iter().any(|line| line.contains("dark")));
    }

    #[test]
    fn consumables_and_resources_work_end_to_end() {
        let ink = r#"
EXTERNAL use_item(item)
EXTERNAL has_item(item)
EXTERNAL has_condition(condition)
EXTERNAL resource(id)
EXTERNAL spend_resource(id, amount)
EXTERNAL restore_resource(id, amount)

VAR spent = false

~ use_item("cigarette")
{ has_item("cigarette"):
    The cigarette is still in your pocket.
- else:
    The cigarette is gone.
}
~ spent = spend_resource("focus", 2)
Focus is {resource("focus")}.
~ spent = restore_resource("focus", 1)
Focus is {resource("focus")}.
{ has_condition("nicotine_rush"):
    The edges of the world go soft.
}
-> END
"#;
        let data = GameData::from_yaml(
            &std::fs::read_to_string("../assets/data/config.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/rulebook.yaml").unwrap(),
        )
        .unwrap();
        let mut engine = Engine::new(ink, data).unwrap();

        let event = engine.start().unwrap();
        let Event::TheEnd { text } = event else {
            panic!("expected the end")
        };
        let joined = text.join(" ");
        assert!(joined.contains("The cigarette is gone."), "{joined}");
        assert!(joined.contains("Focus is 1."), "{joined}");
        assert!(joined.contains("Focus is 2."), "{joined}");
        assert!(
            joined.contains("The edges of the world go soft."),
            "{joined}"
        );
    }

    #[test]
    fn checks_are_recorded_for_the_ui() {
        let mut engine = demo_engine();
        engine.start().unwrap();

        let checks = engine.take_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].ability, "empathy");
        assert_eq!(checks[0].outcome, "pass");
        assert!(checks[0].dice.is_none());

        engine.set_seed(7);
        engine.choose(0).unwrap();
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].ability, "endurance");
        assert!(checks[0].dice.is_some());
        assert!(!checks[0].breakdown.entries.is_empty());
    }

    #[test]
    fn check_records_show_each_die_and_the_modifiers() {
        let mut engine = demo_engine();
        engine.set_seed(7);
        engine.start().unwrap();

        let checks = engine.take_checks();
        assert!(checks[0].describe().contains("sense:"));

        engine.choose(0).unwrap();
        let checks = engine.take_checks();
        let dice = checks[0].dice.expect("active check rolls dice");
        let description = checks[0].describe();
        assert!(
            description.contains(&format!("die {} + die {}", dice[0], dice[1])),
            "{description}"
        );
        assert!(description.contains("total"), "{description}");
        assert!(description.contains("vs 10"), "{description}");
    }

    #[test]
    fn items_gate_later_scenes() {
        let mut engine = demo_engine();
        engine.start().unwrap();

        // Search the toll booth. The wizard tag lights the way.
        let event = engine.choose(2).unwrap();
        let Event::Scene { text, .. } = event else {
            panic!("expected the waiting scene")
        };
        assert!(text
            .join(" ")
            .contains("The lantern throws the troll's shadow"));
        assert!(engine.character().has_item("lantern"));

        let event = engine.choose(0).unwrap();
        assert!(matches!(event, Event::TheEnd { .. }));
    }

    fn example_engine(ink: &str) -> Engine {
        let data = GameData::from_yaml(
            include_str!("../../assets/data/config.yaml"),
            include_str!("../../assets/data/rulebook.yaml"),
        )
        .unwrap();
        Engine::new(ink, data).unwrap()
    }

    #[test]
    fn three_argument_queries_preserve_tags_and_penalties() {
        let mut engine = example_engine(
            r#"
EXTERNAL passive_value(ability, tags, modifier)
EXTERNAL check_breakdown(ability, tags, modifier)
Value: {passive_value("empathy", "artist", -3)}.
Breakdown: {check_breakdown("empathy", "artist", -3)}.
-> END
"#,
        );
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        let text = text.join(" ");
        assert!(text.contains("Value: 8."), "{text}");
        assert!(text.contains("perk artist (tag artist) +2"), "{text}");
        assert!(text.contains("one-off -3"), "{text}");
        assert!(engine.take_checks().is_empty());
    }

    #[test]
    fn explicit_scene_boundaries_expire_conditions_once() {
        let mut engine = example_engine(
            r#"
EXTERNAL add_condition(id)
EXTERNAL end_scene()
~ add_condition("nicotine_rush")
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
        assert!(engine.character().has_condition("nicotine_rush"));
        engine.choose(0).unwrap();
        assert!(engine.character().has_condition("nicotine_rush"));
        engine.choose(0).unwrap();
        assert!(!engine.character().has_condition("nicotine_rush"));
        let expired = engine.take_changes().into_iter().filter(|change| {
            matches!(change, StateChange::ConditionRemoved(id) if id == "nicotine_rush")
        }).count();
        assert_eq!(expired, 1);
    }

    #[test]
    fn passive_records_are_not_duplicated_by_lookahead() {
        let mut engine = example_engine(
            r#"
EXTERNAL passive_check(ability, dc, tags, modifier)
First line.
{passive_check("empathy", 8, "", 0): You notice the clue.}
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
{spend_resource("focus", 4): Paid.|Not paid.}
Focus: {resource("focus")}.
-> END
"#,
        );
        let Event::TheEnd { text } = engine.start().unwrap() else {
            panic!("expected end")
        };
        assert!(text.join(" ").contains("Not paid."));
        assert_eq!(engine.character().resource("focus"), Some(3));
        assert!(engine.take_changes().is_empty());
    }

    #[test]
    fn wrong_argument_types_fail_instead_of_using_defaults() {
        let mut engine = example_engine(
            r#"
EXTERNAL passive_value(ability, tags, modifier)
{passive_value("empathy", "", "wrong")}
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
                if source.contains("EXTERNAL cast_spell") && !cfg!(feature = "spells") {
                    continue;
                }
                let yaml = if page == "overview" {
                    minimal_yaml
                } else {
                    include_str!("../../assets/data/rulebook.yaml")
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

    #[cfg(feature = "spells")]
    #[test]
    fn spells_are_bound_and_spend_resources() {
        let ink = r#"
EXTERNAL cast_spell(id)
EXTERNAL resource(id)

VAR outcome = ""

~ outcome = cast_spell("telekinesis")
The spell: {outcome}.
Focus is {resource("focus")}.
-> END
"#;
        let data = GameData::from_yaml(
            &std::fs::read_to_string("../assets/data/config.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/rulebook.yaml").unwrap(),
        )
        .unwrap();
        let mut engine = Engine::new(ink, data).unwrap();
        engine.set_seed(3);

        let event = engine.start().unwrap();
        let Event::TheEnd { text } = event else {
            panic!("expected the end")
        };
        assert!(text.join(" ").contains("Focus is 1."), "{text:?}");
        let checks = engine.take_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].ability, "arcana");
        assert_eq!(checks[0].difficulty, 12);
        assert!(checks[0].dice.is_some());
        assert_eq!(
            engine.take_changes(),
            vec![StateChange::ResourceChanged {
                id: "focus".to_string(),
                from: 3,
                to: 1,
            }]
        );
    }
}
