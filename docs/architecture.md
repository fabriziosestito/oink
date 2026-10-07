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
oink-cli
  -> oink-core
       -> bladeink / bladeink-compiler
       -> oink-rulebook
```

`oink-rulebook` defines YAML models, validation, character operations,
modifiers, checks, and spell resolution. It contains no database,
file-storage, display, or Ink runtime integration. `SystemDice` is the current
host default and reads the clock and process ID; tests can inject `SeededDice`.

`oink-core` loads the engine inputs, creates the runtime, and binds Ink external
functions. It owns the live character, dice source, and notification queues.
Rulebook operations perform the character mutations.

`oink-cli` is the first player. It builds the `oink` command, whose `run`
subcommand reads a game bundle from disk and plays it in the terminal.
A bundle is one directory with `config.yaml`, `rulebook.yaml`, and `story.ink`
or a precompiled `story.ink.json`, plus an optional `cover.png`.
In a terminal the player draws a full-screen page with ratatui: the prose in a
centered column, the check records and state notices under it, the choices
below, and a status bar with the character resources, conditions, and level.
Picks come from keys 1 to 9, the arrow keys with Enter, or mouse clicks.
The cover shows through ratatui-image, as a picture in terminals with the
Kitty, iTerm2, or Sixel graphics protocol and as Unicode half blocks elsewhere.
When stdin or stdout is not a terminal, or with `--plain`, the player prints a
transcript and reads one pick per line.
The `--seed` and `--choices` flags make a run repeatable without input.

The player parses the emphasis markup in story and choice text, `**bold**`,
`*italic*`, and `_italic_`, as described in the
[Ink API reference](reference/ink-api.md#emphasis-in-prose). The engine passes
the text through unchanged, so the markup is a player concern until a second
player needs it.

Each player is a separate crate with its own binary. Device players will live
under `players/`, with the M5Paper first. Device rendering goes through the
`embedded-graphics` `DrawTarget` trait. The terminal player draws with ratatui
and does not use it.

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

The terminal player drains both queues after every step and prints check
records and state changes under the scene text. Other hosts can do the same
to show item, perk, resource, or condition notices.
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
