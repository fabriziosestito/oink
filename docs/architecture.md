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
  -> oink-ui (config only, no renderer)

oink-ui
  -> oink-core (rulebook ids in the ui block)
  -> embedded-graphics (optional, `render` feature)
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
The keys `s`, `i`, and `l` open the character sheet, the inventory, and the
session log as paged panels over the page; Backspace closes the open panel,
and the wheel or PgUp/PgDn turn its pages. In the inventory the arrow keys
select an item, and Enter or a click opens its detail page with the
description, weight, tags, consumable flag, modifiers, and picture.
A dialog scene draws the speaker name above the prose and the speaker portrait
in a left column, when the bundle configures a portrait file. A map scene
draws the map label above the location choices and marks each choice with a
location glyph.
The cover, the portraits, and the scene and item pictures show through
ratatui-image, as pictures in terminals with the Kitty, iTerm2, or Sixel
graphics protocol and as Unicode half blocks elsewhere.
When stdin or stdout is not a terminal, or with `--plain`, the player prints a
transcript and reads one pick per line.
The `--seed` and `--choices` flags make a run repeatable without input.

The emphasis markup in story and choice text, `**bold**`, `*italic*`, and
`_italic_`, is parsed in `oink-core/src/markup.rs`, as described in the
[Ink API reference](reference/ink-api.md#emphasis-in-prose). The engine passes
the text through unchanged, and every player applies the same parser.

`oink-ui` is the shared reader layout for device players. It owns the three
zones, the pager, and the sheet, inventory, and log views. It draws through
`embedded-graphics` `DrawTarget` and has no file, network, or terminal access.
Ports supply screen metrics, input events, and storage. A page without a bar
gives its space to the center, and `DirtyZones` reports the zones whose
geometry changed between two frames for a partial e-ink refresh.

The `oink-ui::config` module is always compiled. It parses the optional `ui`
block of `config.yaml` into page sizes, sheet order, the base layout with its
top and bottom bars, the per-mode page overrides, the character names and
portrait files, the map labels, status settings, and palettes.
`resolve_page(mode)` merges a mode's `pages` block into the base
layout, so a page can drop a bar or resize it, and an unknown mode falls back
to the base layout with a warning. It validates the rulebook ids and other
references. The drawing modules sit behind the default `render` feature, so
a player that needs only the presentation settings can use `oink-ui`
without `embedded-graphics`.
The terminal player does that: it reads the reader, status, and terminal
theme blocks and draws with ratatui. `oink-core` never reads the `ui` block.
See the [UI configuration reference](reference/ui.md).

The `oink-ui::content` module is always compiled too. It turns an engine event
into wrapped lines through the shared markup parser, keeps the scene mode,
the speaker, mood, portrait, map, and image values, puts check records before
the numbered choices, and paginates the result with `Pager`. It also reads the
top-bar values from the character sheet and builds the character-sheet panel,
one page per configured group, the inventory panel, one page per configured
item count, and the log panel from a host-side ring buffer. The renderer uses
ProFont and `Metrics` to fit the lines in the center zone, fills the top bar
with the resource and characteristic values, draws the page controls, and
fills the bottom bar with the sheet, inventory, and log buttons. The focused
button draws inverted. Every draw reports its touch targets: the choice rows,
the page arrows, the bar buttons, and the back label. The terminal player
keeps its ratatui drawing and reuses the panels, the pager, and the log from
`content`. A device player uses both modules.

Each player is a separate crate with its own binary. Device players will live
under `players/`, with the M5Paper first. Their shared layout is `oink-ui`.
The terminal player draws with ratatui and uses only the `config` module.

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

`Engine::snapshot()` writes one JSON save envelope. It holds the bladeink story
state, the `Character` (including environments and resources), the dice state,
and the last `Event`. `Engine::restore()` loads the envelope into the same
engine and returns the saved event, so the player can redraw the scene.

The envelope carries `SAVE_VERSION`, which is 1 today. A file with another
version is rejected. Restoring needs the same bundle and story source: the
engine keeps its compiled story, rulebook, and Ink bindings, and only replaces
state. The dice state keeps a seeded run repeatable across a save.

The host owns the files. The terminal player writes `saves/slot-<n>.json`
beside the bundle, with one file per numbered slot up to
`ui.reader.save_slots`. UI notification queues are not part of the save; the
session log and the notices restart empty on load.

Story facts that must survive a save belong in Ink variables. Pending changes
and checks are drained before a save in the normal player loop.

## Design records

This page documents current architecture. Proposed systems and unresolved
decisions belong in [GitHub issues](https://github.com/fabriziosestito/oink/issues).
Update the reference when an exposed API or behavior changes.
