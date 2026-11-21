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
| `snapshot(&self)` | `Result<String, EngineError>` | Writes the story, the character, the dice, and the last event as one JSON save |
| `restore(&mut self, snapshot)` | `Result<Event, EngineError>` | Loads a save into this engine and returns the scene it left |
| `last_event(&self)` | `Option<&Event>` | The last event the story produced |

Drop a character borrow before advancing or mutating the engine.
`set_seed()` affects rulebook rolls, not Ink's own `RANDOM()` calls.
Queues accumulate until drained. `snapshot()` does not drain the queues;
`restore()` clears them.

The save envelope has the format version `SAVE_VERSION`, which is 1. The file
holds the story state, the `Character`, the dice state, the last `Event`, and
a checksum of that payload. A file with another version, a missing checksum,
or a checksum that does not match the payload fails with `EngineError::Save`
before the story runtime parses it. Restoring needs the same story source and
rulebook: the engine keeps its compiled story and bindings and replaces only
state. The dice state continues a seeded run.

The active check is not part of the save. After `restore()`, `check_roll()`,
`check_score()`, and the other check queries report no active check until the
story rolls again.

```rust
pub struct Choice {
    pub index: usize,
    pub text: String,
}

pub enum Event {
    Scene {
        mode: String,
        speaker: Option<String>,
        mood: Option<String>,
        portrait: Option<String>,
        map: Option<String>,
        image: Option<String>,
        text: Vec<String>,
        choices: Vec<Choice>,
    },
    TheEnd { text: Vec<String> },
}

pub enum EngineError {
    Compile(String),
    Story(String),
    Save(String),
}
```

Text contains trimmed, nonempty output chunks. `Scene.mode` is the scene mode
from the `# mode: <id>` knot tag, and `prose` without one. `Scene.speaker`,
`mood`, `portrait`, and `map` carry the ids of the other scene tags, or `None`
without the tag. `Scene.image` carries the `# image: <path>` value, a file path
relative to the bundle directory, or `None` without the tag. The host chooses its own display and input handling for each
mode. See the [scene tags](ink-api.md#scene-tags).

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
    pub pool: Option<&'a str>,
}

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

pub struct DiceRoll {
    pub kind: RollKind,
    pub dice: Vec<u16>,
    pub kept: Vec<bool>,
    pub total: i32,
}

pub enum RollKind {
    Sum,
    Percentile,
}

pub struct PassiveResult {
    pub passed: bool,
    pub value: i32,
    pub difficulty: i32,
    pub breakdown: Breakdown,
}
```

Build a request with `CheckRequest::new(ability, difficulty)`, then use 
`with_tags(tags)`, `with_modifier(value)`, and `with_pool(Some(name))` when needed.
A missing pool means the rulebook default. `CheckResult.pool` names the profile
that ran, which can differ from the request when advantage or disadvantage
applies. See [checks](checks.md#which-profile-runs) for the order.
`Checks::new(&rulebook, &character)` creates a borrowed resolver.

| Method | Result |
| --- | --- |
| `active(&self, &mut dice, &request)` | `Result<CheckResult, CheckError>` |
| `passive(&self, &request)` | `Result<PassiveResult, CheckError>` |
| `breakdown(&self, ability, tags, modifier)` | `Result<Breakdown, CheckError>` |

An unknown ability returns `CheckError::UnknownAbility`.
An unknown profile returns `CheckError::UnknownPool`.
`Outcome` has `CriticalFailure`, `Failure`, `Success`, and `CriticalSuccess`.
Use `as_str()` for the Ink-compatible result or `is_success()` for a boolean.

```rust
pub trait Dice {
    fn roll(&mut self, sides: u16) -> u16;
}

pub struct DiceConfig {
    pub default: String,
    pub profiles: BTreeMap<String, DiceProfile>,
}

pub struct BreakdownEntry {
    pub source: String,
    pub value: i32,
}

pub struct Breakdown {
    pub entries: Vec<BreakdownEntry>,
}
```

A custom dice source must return values from 1 through the requested sides.
`SeededDice::new(seed)` uses SplitMix64 with rejection sampling for repeatable rolls.
`SystemDice::new()` seeds that generator from the host clock and process ID.
`DiceConfig::default()` returns the 2d6 standard profile. `parse_notation()`
parses pool strings such as `2d6`, `2d20kh1`, and `d%`.
`Breakdown::total()` sums contributions. Its `Display` implementation formats
the list and subtotal. Zero contributions are omitted.

## Character operations

`Character::from_starting(&rulebook)` builds a sheet from definitions and applies grants.
The rulebook has no persistence or display code. Its methods mutate only the
in-memory character and return notices.

| Methods | Behavior |
| --- | --- |
| `has_item`, `has_perk`, `has_condition`, `has_ability`, `has_environment` | Test presence by ID |
| `item_count(id)` | Returns the stack size, or 0 when the item is absent |
| `has_tag(&rulebook, tag)` / `effective_tags(&rulebook)` | Read the combined character tags |
| `characteristic(id)` / `resource(id)` | Return `Option<i32>` |
| `ability_level(id)` | Returns the level, or 0 if absent |
| `resource_max(&rulebook, id)` | Returns the effective maximum, derived when `max_from` is set |
| `perk_ids`, `item_ids`, `condition_ids`, `environment_ids`, `ability_ids`, `tag_ids` | Iterate stored IDs |
| `item_stacks()` | Iterates `(&String, u32)` pairs, one per carried item |
| `add_item`, `add_perk`, `add_condition`, `enter_environment`, `add_tag` | Take `&rulebook` and an ID; return `Vec<StateChange>` |
| `remove_item`, `remove_perk`, `remove_condition`, `clear_environment`, `remove_tag` | Take an ID; return `Vec<StateChange>` |
| `consume_item(id, amount)` | Removes up to `amount` units and clears the stack at zero; returns a notice |
| `add_timed_condition(&rulebook, id, Option<u32>)` | Overrides the configured duration; a changed duration returns a refresh notice |
| `apply_grants(&rulebook)` | Adds missing grants until stable |
| `can_spend_resource(&rulebook, id, amount)` | Tests whether a full nonnegative payment fits |
| `spend_resource` / `restore_resource` | Take `&rulebook`, ID, and amount; return notices |
| `on_scene_end()` | Decrements durations and returns expiry notices |
| `begin_creation(&rulebook)` | Builds a fresh sheet from the creation base |
| `reset_character(&rulebook)` | Rebuilds the sheet from the creation base |
| `points_available(&rulebook, kind)` | Returns unspent pool and level-bonus points as `u32` |
| `spend_point(&rulebook, kind, id)` | Spends the flat cost (1 without creation) and applies one pick; false without changes |
| `apply_preset(&rulebook, id)` | Applies a preset, refreshes derived maxima, marks pools spent |
| `set_characteristic` / `set_ability` | Write values directly without spending; false for unknown IDs |
| `validate_creation(&rulebook)` | Returns unspent-pool messages, level rewards included; empty means complete |
| `meets_prerequisite(&rulebook, id)` | True when every requirement holds; ids without an entry pass |
| `xp()` / `level()` | Return banked experience and current level |
| `add_xp(amount)` | Banks experience without levelling |
| `level_up_ready(&rulebook)` | True when banked XP reaches the next curve level below the cap |
| `level_up(&mut self, rulebook)` | Rises one level with rewards; false when not ready |

The Rust mutators currently accept unknown item, perk, condition, tag, and
environment IDs. Callers must validate them. The Ink bindings validate additions
of items, perks, conditions, and environments before calling those mutators.
There is no standalone Rust `Character::use_item()` method today; the Ink
binding composes condition application and `consume_item()`.

## Spell API

Call `oink_rulebook::spell::cast()` directly, or bind `cast_spell()` in
stories through `oink-core`.

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
`MissingPrerequisites`, `InvalidCost`, `NotEnough`, and `Check`. Rejected casts leave the balance and
dice sequence unchanged. Resolved casts pay even on failure.
Direct Rust callers receive notices in `changes`; the engine binding queues them.

## UI config

`oink-ui` reads the `ui` block of `config.yaml` and owns the shared reader
layout. The `config` module has no renderer dependency and is always
compiled. The drawing modules need the default `render` feature.

```rust
use oink_ui::UiConfig;

let ui = UiConfig::from_config(config_yaml)?;
ui.validate(&engine.data.rulebook)?;
```

`UiConfig::from_config` parses the `ui` block of `config.yaml` and checks its
structure.
`validate` checks the resource and characteristic ids against the rulebook.
`UiConfig::default()` is the same as a missing or empty block.
A device player reads `reader` and `theme.eink`; the terminal player reads
`reader`, `status`, and `theme.terminal`, and resolves the `characters` and
`maps` blocks for dialog and map scenes. See the
[UI configuration reference](ui.md) for the format.

The `content` module is always compiled too. A player builds the panels and
the pager from it:

```rust
use oink_ui::{Log, Pager, Panel, View};

// The sheet at 40 columns; the pager holds 10 rows per page.
let panel = Panel::sheet(&rulebook, &character, &reader, 40);
let mut pager = Pager::new(panel.lines, 10);
pager.next_page();

// The session log, filled by the host from the state-change notices.
let mut log = Log::from_config(&reader);
log.push("item added: Brass Key");
let panel = log.panel(40);
```

`Panel::sheet`, `Panel::inventory`, and `Panel::log` wrap to a column count;
`Pager` splits the lines by rows and honors the panel's page breaks. `View`
names the screens a reader shows: the story page and the three panels.

## Generated API reference

Run `cargo doc --workspace --all-features --no-deps` to generate rustdoc for all
public types. The manual pages describe story-author behavior and stable concepts.
Rustdoc exposes exact signatures for application developers.
