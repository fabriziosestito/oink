---
title: Rust API
sidebar_position: 5
---

# Rust API

Use `oink-core` to run stories. Use `oink-rulebook` for rules and character
operations without a story runtime. Both crates run on the host without a display.

## Loading

```rust
use oink_core::{data::GameData, Engine, Event};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = "title: Example";
    let rulebook = "{}";
    let source = "The road is quiet.\n-> END";
    let data = GameData::from_yaml(config, rulebook)?;
    let mut engine = Engine::new(source, data)?;

    if let Event::TheEnd { text } = engine.start()? {
        assert_eq!(text, vec!["The road is quiet."]);
    }
    Ok(())
}
```

`GameData::from_yaml(config, rulebook)` parses both documents.
It returns `DataError::Config` for configuration YAML errors or
`DataError::Rulebook` for rulebook parse and validation errors.
Read `warnings` before moving the value into the engine.

```rust
pub struct Config {
    pub title: String,
}

pub struct GameData {
    pub config: Config,
    pub rulebook: Rc<Rulebook>,
    pub warnings: Vec<String>,
}
```

These are the current public names. `GameData` contains fixed definitions,
not the live character. `Rc` shares one rulebook allocation within a thread.

## Engine

| Method | Result | Behavior |
| --- | --- | --- |
| `Engine::new(source, data)` | `Result<Engine, EngineError>` | Compiles source, loads the story, builds the character, and binds functions |
| `Engine::from_json(json, data)` | `Result<Engine, EngineError>` | Loads precompiled story JSON with the same character and bindings |
| `start(&mut self)` | `Result<Event, EngineError>` | Advances to the first choice or ending; it does not reset a used engine |
| `choose(&mut self, index)` | `Result<Event, EngineError>` | Selects a zero-based choice and advances the story |
| `character(&self)` | `Ref<'_, Character>` | Borrows the live character for reading |
| `set_seed(&mut self, seed: u64)` | `()` | Replaces the rulebook dice source with deterministic dice |
| `take_changes(&mut self)` | `Vec<StateChange>` | Returns and empties mutation notices |
| `take_checks(&mut self)` | `Vec<CheckRecord>` | Returns and empties check records |

Drop a character borrow before advancing or mutating the engine.
`set_seed()` affects rulebook rolls, not Ink's own `RANDOM()` calls.
Queues accumulate until drained. Save/load is not exposed yet.

```rust
pub struct Choice {
    pub index: usize,
    pub text: String,
}

pub enum Event {
    Scene { text: Vec<String>, choices: Vec<Choice> },
    TheEnd { text: Vec<String> },
}

pub enum EngineError {
    Compile(String),
    Story(String),
}
```

Text contains trimmed, nonempty output chunks. The engine does not interpret
presentation tags yet. The host chooses its own display and input handling.

## Rulebook loader

`Rulebook::load(yaml)` returns `Result<Loaded, LoadError>`.
`Loaded` contains `rulebook: Rulebook` and `warnings: Vec<String>`.
`LoadError::Yaml` carries a parse error; `LoadError::Validation` carries a list
of validation messages. See the [YAML reference](yaml.md) for validation limits.

Definitions are public model structs under `oink_rulebook::model`, re-exported
at the crate root. `Names::label(Section)` reads a configurable display label.
`Characteristic::bonus_for(value)` and `BonusTable::bonus_for(value)` read the
threshold table. Load through `Rulebook::load()` so thresholds are sorted.
`DifficultyRef::resolve(&rulebook)` resolves a numeric or named difficulty.

## Checks and dice

```rust
pub struct CheckRequest<'a> {
    pub ability: &'a str,
    pub difficulty: i32,
    pub tags: &'a [&'a str],
    pub modifier: i32,
}

pub struct CheckResult {
    pub outcome: Outcome,
    pub total: i32,
    pub dice: [u8; 2],
    pub breakdown: Breakdown,
}

pub struct PassiveResult {
    pub passed: bool,
    pub value: i32,
    pub difficulty: i32,
    pub breakdown: Breakdown,
}
```

Build a request with `CheckRequest::new(ability, difficulty)`, then use
`with_tags(tags)` and `with_modifier(value)` when needed.
`Checks::new(&rulebook, &character)` creates a borrowed resolver.

| Method | Result |
| --- | --- |
| `active(&self, &mut dice, &request)` | `Result<CheckResult, CheckError>` |
| `passive(&self, &request)` | `Result<PassiveResult, CheckError>` |
| `breakdown(&self, ability, tags, modifier)` | `Result<Breakdown, CheckError>` |

An unknown ability returns `CheckError::UnknownAbility`.
`Outcome` has `CriticalFailure`, `Failure`, `Success`, and `CriticalSuccess`.
Use `as_str()` for the Ink-compatible result or `is_success()` for a boolean.

```rust
pub trait Dice {
    fn roll_d6(&mut self) -> u8;
}

pub struct BreakdownEntry {
    pub source: String,
    pub value: i32,
}

pub struct Breakdown {
    pub entries: Vec<BreakdownEntry>,
}
```

A custom dice source must return values from 1 through 6.
`SeededDice::new(seed)` uses SplitMix64 for repeatable rolls.
`SystemDice::new()` seeds that generator from the host clock and process ID.
`Breakdown::total()` sums contributions. Its `Display` implementation formats
the list and subtotal. Zero contributions are omitted.

## Character operations

`Character::from_starting(&rulebook)` builds a sheet from definitions and applies grants.
The rulebook has no persistence or display code. Its methods mutate only the
in-memory character and return notices.

| Methods | Behavior |
| --- | --- |
| `has_item`, `has_perk`, `has_condition`, `has_ability`, `has_environment` | Test presence by ID |
| `has_tag(&rulebook, tag)` / `effective_tags(&rulebook)` | Read the combined character tags |
| `characteristic(id)` / `resource(id)` | Return `Option<i32>` |
| `ability_level(id)` | Returns the level, or 0 if absent |
| `resource_max(&rulebook, id)` | Returns the configured maximum |
| `perk_ids`, `item_ids`, `condition_ids`, `environment_ids`, `ability_ids`, `tag_ids` | Iterate stored IDs |
| `add_item`, `add_perk`, `add_condition`, `enter_environment`, `add_tag` | Take `&rulebook` and an ID; return `Vec<StateChange>` |
| `remove_item`, `remove_perk`, `remove_condition`, `clear_environment`, `remove_tag` | Take an ID; return `Vec<StateChange>` |
| `add_timed_condition(&rulebook, id, Option<u32>)` | Overrides the configured duration; a changed duration returns a refresh notice |
| `apply_grants(&rulebook)` | Adds missing grants until stable |
| `can_spend_resource(&rulebook, id, amount)` | Tests whether a full nonnegative payment fits |
| `spend_resource` / `restore_resource` | Take `&rulebook`, ID, and amount; return notices |
| `on_scene_end()` | Decrements durations and returns expiry notices |

The Rust mutators currently accept unknown item, perk, condition, tag, and
environment IDs. Callers must validate them. The Ink bindings validate additions
of items, perks, conditions, and environments before calling those mutators.
There is no standalone Rust `Character::use_item()` method today; the Ink
binding composes condition application and item removal.

## Optional spell API

Enable `oink-rulebook/spells` to call `oink_rulebook::spell::cast()` directly.
Enable `oink-core/spells` to also bind `cast_spell()` in stories.
The core feature enables the rulebook feature automatically.

```rust
pub fn cast<D: Dice + ?Sized>(
    rulebook: &Rulebook,
    character: &mut Character,
    spell_id: &str,
    dice: &mut D,
) -> Result<CastResult, SpellError>;

pub struct CastResult {
    pub outcome: Outcome,
    pub check: CheckResult,
    pub spent: i32,
    pub changes: Vec<StateChange>,
}
```

Errors include `UnknownSpell`, `UnknownDifficulty`, `UnknownResource`,
`InvalidCost`, `NotEnough`, and `Check`. Rejected casts leave the balance and
dice sequence unchanged. Resolved casts pay even on failure.
Direct Rust callers receive notices in `changes`; the engine binding queues them.

## Generated API reference

Run `cargo doc --workspace --all-features --no-deps` to generate rustdoc for all
public types. The manual pages describe story-author behavior and stable concepts.
Rustdoc exposes exact signatures for application developers.
