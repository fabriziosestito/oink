---
title: Architecture
sidebar_position: 4
---

# Architecture

oink separates game rules, story execution, and presentation.
Rules operate on in-memory values. The engine connects them to the story.
The host application owns display, input, and file access.

## Crates

```text
oink-sim
  -> oink-core
       -> bladeink / bladeink-compiler
       -> oink-rulebook
```

`oink-rulebook` defines YAML models, validation, character operations,
modifiers, checks, and optional spell resolution. It contains no database,
file-storage, display, or Ink runtime integration. `SystemDice` is the current
host default and reads the clock and process ID; tests can inject `SeededDice`.

`oink-core` loads the engine inputs, creates the runtime, and binds Ink external
functions. It owns the live character, dice source, and notification queues.
Rulebook operations perform the character mutations.

`oink-sim` reads assets from disk, handles keyboard input, and renders story
output with `embedded-graphics`. Its renderer currently takes a simulator display
directly. Extracting a shared renderer for hardware remains future work.

## Fixed definitions and live state

The current implementation uses these names and fields:

```rust
pub struct Config {
    pub title: String,
}

pub struct GameData {
    pub config: Config,
    pub rulebook: Rc<Rulebook>,
    pub warnings: Vec<String>,
}

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
```

`Config` and `Rulebook` are fixed inputs during play.
The two `Rc<Rulebook>` handles point to the same allocation, not two copies.
`Character`, the dice position, and both queues change during play.
These structs show the current code; the proposed naming cleanup is not implemented.

`Rc` shares ownership within one thread. `RefCell` permits checked mutable
borrows at runtime. Bound closures need these handles because the story invokes
them while it advances. Do not retain a character borrow while advancing the engine.

## One story step

1. The host calls `Engine::start()` or `Engine::choose(index)`.
2. Bladeink advances story execution and invokes declared external functions.
3. A binding validates its arguments and calls rulebook operations.
4. Mutations update the live character and append `StateChange` records.
5. Active, passive, and spell checks append `CheckRecord` values.
6. The engine returns scene text and choices, or an ending.
7. The host drains queues and renders the output it needs.

The simulator drains check records today. Other hosts can also drain state
changes to show item, perk, resource, or condition notices.
The queues are transient output, not an ordered history for later story queries.

## Story memory

`Story` is `bladeink::story::Story`. It owns Ink variables, visit counts,
execution position, and current choices. A variable such as
`VAR spoke_with_guard = false` can persist across the whole story.

The rulebook owns character-sheet semantics. Inventory and Health belong on the
character, while narrative facts can stay in Ink variables. A separate world
flags store is not implemented and is not needed merely to remember a conversation.

## Scenes and persistence

The story calls `end_scene()` to advance condition durations. The engine does
not infer scene boundaries from choices, knots, or output chunks.
This keeps story structure independent of condition lifetime.

There is no combined save/load API today. Future core persistence must account
for bladeink state, character state, active environments, and the rulebook dice
position. UI notification queues are not a substitute for that snapshot.

## Design records

This page documents current architecture. Proposed systems and unresolved
decisions belong in [GitHub issues](https://github.com/fabriziosestito/oink/issues).
Update the reference when an exposed API or behavior changes.
