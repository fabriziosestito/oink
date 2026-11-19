# AGENTS.md

Guidance for AI agents (and humans) working on **oink**.

## What is oink

**oink — gamebooks for e-ink.** A librogame (gamebook) engine for e-ink
devices, written in Rust. The first target is the M5Paper (ESP32 + 960x540
16-level grayscale e-ink + touch); more e-ink hardware, iOS, and Android come
later. The name is a pun: **oink** (pig, the mascot) / **Ink** (the narrative
language) / **e-ink** (the display). Stylized forms like "o·ink" may appear in
branding only — the canonical name for repo, crates, and binaries is `oink`.

## Core design decisions

These were deliberate choices. Do not revisit them without a strong reason.

1. **Ink for narrative, YAML for data.**
   - Story, scenes, branching, choices, skill-check flow: **Ink**
     (`story.ink` in a game bundle).
   - Rule definitions and starting character: **YAML** (`rulebook.yaml`).
     The game title lives in `config.yaml`.
   - A game bundle is one directory with those three fixed file names.
     An optional `ui:` block in `config.yaml` sets presentation: reader
     page sizes, the status bar, and the e-ink and terminal palettes. The
     engine never reads it. The demo game is `examples/high-pass/`.
   - Checks use configurable dice profiles from YAML. The default profile rolls
     2d6 over with margin degrees and passive 6. Profiles can use d2 through
     d20 and d%, roll over or under, and define their own outcome table.
   - Ink references data by ID; rich item/perk structure never lives in Ink
     (Ink has no structs). Runtime state (inventory, HP) lives in the engine
     and is exposed to Ink via external functions.

2. **Rust, not Go, not Arduino/C++.**
   - Go/TinyGo has immature Xtensa/ESP32 support — rejected.
   - Target the `std` route via **esp-idf** (`esp-idf-hal`/`esp-idf-svc`) so
     `serde_yaml`, heap, threads, and FS work on device. Toolchain: `espup`.

3. **No emulator; players instead.**
   - There is no M5Paper emulator. The desktop test bed is the terminal
     player, `oink run` in `oink-cli`. In a terminal it draws a full-screen
     page with `ratatui`: prose column, check records and notices, choices,
     and a status bar. It takes keys and mouse clicks and shows the cover
     through `ratatui-image` (Kitty, iTerm2, Sixel, or Unicode half blocks).
     Pipes, `--choices`, and `--plain` get a plain transcript instead.
     No SDL, no system dependencies.
   - Each player is its own crate with its own binary. Device players go
     under `players/` when they exist (M5Paper first), because they need
     their own toolchains and cannot share one crate with the CLI.
   - Device rendering goes through `embedded-graphics`'s `DrawTarget` trait.
     The shared device layout lives in `oink-ui`; the terminal player uses
     only its `config` module and draws with ratatui.
   - On device the target is the IT8951 e-ink driver (`it8951` crate).
   - The engine core is a plain Rust crate, unit-testable with `cargo test` —
     faster iteration than any emulator.

4. **Ink runtime: `bladeink` (2.x).**
   - Pure-Rust port of inkle's reference runtime; full language support
     (threads, flows, external functions, choice tags, save/load state).
   - Compiled with `bladeink-compiler`: when a bundle is loaded from
     `story.ink` (`Engine::new`), at build time for firmware
     (`Engine::from_json` loads a precompiled `story.ink.json`).
   - Replaced `inkling` (0.12, unmaintained since 2020, no external
     functions/tags). Keep engine code runtime-agnostic where cheap.

5. **Not published to crates.io.** This is an application (players +
   firmware), not a library. The crate name `oink` is taken on crates.io
   (pig latin crate) — irrelevant since we distribute via git/releases/flash.
   The CLI crate is `oink-cli` and its binary is `oink`. If the core is ever
   extracted as a library, publish as `oink-engine` or similar, not `oink`.

## Workspace layout

```
oink/
├── Cargo.toml           # workspace (resolver 2); shared deps in [workspace.dependencies]
├── Makefile             # build / run / test / check / fmt / lint / docs / clean / m5paper
├── documentation.md    # entry point to user documentation
├── docs/index.md       # documentation navigation
├── docs/reference/     # YAML, checks, state, Ink API, Rust API, and UI config
├── docs/architecture.md # current crate boundaries and runtime ownership
├── website/             # Docusaurus site: theme and build config
├── oink-core/           # engine core: Ink runtime wrapper + YAML data model
│   ├── src/
│   │   ├── lib.rs
│   │   ├── data.rs      # Config, GameData (rulebook-backed)
│   │   ├── engine.rs    # Engine, Event, Choice + tests
│   │   ├── external.rs  # external function bindings
│   │   ├── tags.rs      # scene tag parsing: mode id from `# mode: <id>`
│   │   └── markup.rs    # emphasis markup shared by players
│   └── tests/fixtures/  # synthetic config, rulebook, and story for tests
├── oink-ui/             # shared reader layout and UI config for device players
│   ├── src/             # `config` and `content` always build; drawing needs `render`
│   │   ├── lib.rs
│   │   ├── config.rs    # the ui block of config.yaml: schema, defaults, validation, page overrides
│   │   ├── content.rs   # scene lines, panels, markup, wrapping, pager
│   │   ├── metrics.rs   # screen measurements and font sizes
│   │   ├── layout.rs    # top, center, and bottom zones, page bar heights, dirty zones
│   │   ├── theme.rs     # Gray4 palette
│   │   ├── input.rs     # actions, views, and focus
│   │   └── render.rs    # zone chrome, top values, center page, buttons
│   └── tests/           # Gray4 framebuffer and PGM snapshots
├── oink-rulebook/       # rulebook crate: checks, modifiers, resources, character state
│   └── src/
│       ├── lib.rs
│       ├── model.rs     # resource definitions from rulebook.yaml
│       ├── loader.rs    # YAML loading and validation
│       ├── modifiers.rs # modifier stacking and breakdowns
│       ├── check.rs     # active and passive checks with dice profiles
│       ├── dice.rs      # Dice trait, notation parser, seeded dice for tests
│       ├── names.rs     # renameable display labels
│       ├── spell.rs     # cost and cast resolution
│       └── state.rs     # Character state and change events
├── oink-cli/            # the `oink` command: `run` plays a bundle in the terminal
│   ├── src/
│   │   ├── main.rs      # clap: oink run <BUNDLE_DIR> [--seed N] [--choices 1,3,2] [--plain]
│   │   ├── bundle.rs    # reads config.yaml, rulebook.yaml, story.ink[.json], cover.png
│   │   ├── game.rs      # engine setup shared by both players, THE_END line
│   │   ├── saves.rs     # numbered save slots in a `saves` directory beside the bundle
│   │   ├── markup.rs    # emphasis markup in prose: **bold**, *italic*, _italic_
│   │   ├── wrap.rs      # word wrap over styled fragments
│   │   ├── player.rs    # the transcript: stdin lines and scripted picks, --plain
│   │   ├── picture.rs   # cover, portrait, and scene pictures through ratatui-image
│   │   └── tui/         # the full-screen player (ratatui)
│   │       ├── mod.rs   # event loop, key and mouse mapping, terminal setup
│   │       ├── app.rs   # state: page, highlight, scroll, status, panels, actions
│   │       ├── view.rs  # layout: header, prose column, choices, status bar
│   │       ├── text.rs  # styled lines for prose, checks, notices, choices
│   │       └── theme.rs # semantic styles from the 16-color palette, NO_COLOR
│   └── tests/
│       ├── run.rs       # plays examples/high-pass through the binary
│       └── docs.rs      # runs the Ink examples in docs/ against the demo rulebook
├── players/             # (planned) device players, one crate each: m5paper first
├── examples/
│   └── high-pass/       # demo game bundle: The High Pass
│       ├── config.yaml  # game title
│       ├── rulebook.yaml # rulebook: characteristics, abilities, perks, checks
│       └── story.ink    # the story
└── assets/
    └── logo.png         # mascot (hi-res in logo-hires.png), branding only
```

## Engine API (oink-core)

- `GameData::from_yaml(config, rulebook)` — parse the two YAML docs.
- `Engine::new(ink_source, data)` — compile ink, load the story, build the
  starting character from `rulebook.starting_character`, and bind the
  external functions.
- `Engine::from_json(story_json, data)` — load precompiled `.ink.json`
  (build-time compile path for firmware).
- `Engine::character()` — read the character sheet. `Engine::take_changes()`
  drains the `StateChange` queue. `Engine::take_checks()` returns the checks
  since the last call, dice and breakdown included, for the UI.
  `Engine::set_seed(seed)` makes rolls deterministic.
- `Engine::start() -> Result<Event, EngineError>` /
  `Engine::choose(index) -> Result<Event, EngineError>`.
- `Event::Scene { mode, text, choices }` — prose paragraphs + choices to
  render. `mode` is the `# mode: <id>` scene tag, `prose` by default.
- `Event::TheEnd { text }` — story over. House flavor: the death/ending screen
  references the classic librogame line *"La tua vita e la tua missione
  terminano qui"* (Lupo Solitario). Fighting Fantasy trivia: those books had
  400 paragraphs, paragraph 400 = victory.
- `EngineError::{Compile, Story}` — runtime-agnostic error type (no `bladeink`
  types leak into the public API).
- Bound external functions: `roll_check`, `passive_check`, `passive_value`,
  `check_breakdown`, `difficulty`, `enter_environment`, `clear_environment`,
  `has_item`, `add_item`, `remove_item`, `use_item`, `has_perk`, `add_perk`,
  `remove_perk`, `has_condition`, `add_condition`, `remove_condition`,
  `has_tag`, `ability_level`, `characteristic`, `characteristic_bonus`,
  `resource`, `spend_resource`, `restore_resource`, `end_scene`, `cast_spell`.

## Presentation modes: the tag contract

Ink is UI-agnostic; it emits text + tags and the engine interprets them.
oink defines its own tag vocabulary (this is OUR scene format — keep this
section updated as the single source of truth):

| Tag | Where | Meaning |
|---|---|---|
| `# mode: prose` | knot | default full-text gamebook rendering |
| `# mode: dialog` | knot | portrait/name dialog layout |
| `# mode: map` | knot | choices render as tappable map locations |
| `# speaker: <id>` | knot | character speaking (id defined in `ui.characters`) |
| `# mood: <id>` | knot | portrait variant (carried, no player draws variants yet) |
| `# portrait: <id>` | knot | explicit portrait override |
| `# map: <id>` | knot | which map layout (id defined in `ui.maps`) |
| `# image: <path>` | knot | scene picture, relative to the bundle directory |

The tags configure the whole scene: read them at the top of a knot, and the
value of the last knot wins when a scene crosses several knots. `oink-core/src/tags.rs`
parses all six keys and `Event::Scene` returns `mode`, `speaker`, `mood`,
`portrait`, `map`, and `image`; `mode` defaults to `prose`. The terminal player
draws the scene picture above the prose, the dialog name, the portrait column,
and the map label with location markers.
Per-line tags stay open: `bladeink` exposes tags per line
(`Story::get_current_tags`, `Choice::tags`) and per choice, so no runtime
change is needed, but the engine reads them per scene for now.

Inline emphasis is part of the scene format too: `**bold**`, `*italic*`, and
`_italic_` in prose and choice text, with the opening and closing rules in
[docs/reference/ink-api.md](docs/reference/ink-api.md#emphasis-in-prose).
The parser lives in `oink-core/src/markup.rs` and every player shares it. The
engine passes text through unchanged.

External functions are bound by `oink-core` (inventory, perks, conditions,
tags, checks, environments, resources; see the Engine API list). The writer calls
`end_scene()` once per scene boundary to advance timed conditions. Choices,
knots, and story endings do not advance durations automatically.

Character definitions (name, portrait assets), map layouts, and mode config
go in YAML, referenced by ID from tags.

## Commands

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, the full `make` list, and the
player controls.

Always run `make test` (and `make lint` before committing) after engine
changes. The terminal player is the manual test bed: `make run`, or
`cargo run -p oink-cli -- run examples/high-pass`. For a repeatable walk, add
`--seed N --choices 1,1,...`; CI plays the demo that way.

`make docs` starts the documentation site with hot reload on both `docs/` and
`website/`. See [docs/publishing.md](docs/publishing.md) for the site layout,
theme, and deployment.

## Conventions

- Rust 2021 edition, workspace-level shared dependencies.
- Keep `oink-core` free of any display/IO concerns — it must stay
  `cargo test`-able on the host and eventually embedded-friendly.
- Presentation settings live in `oink-ui::config` and the optional `ui:`
  block of `config.yaml`. Players read them; `oink-core` never does.
- Device rendering goes through `embedded_graphics::DrawTarget`; never code
  against the IT8951 directly in shared code. The terminal player draws with
  `ratatui` and does not use `embedded-graphics`.
- Players are binaries, not libraries. Shared host code (bundle loading, a
  layout for e-ink) gets extracted only when a second player needs it.
- The demo game lives in `examples/high-pass/`; `assets/` holds branding
  only. Tests never read the demo: `oink-core/tests/fixtures/` holds a
  synthetic rulebook and story built for tests, and `oink-rulebook` uses
  inline YAML.
- Prose (docs, READMEs, error messages, chat replies): load the
  `simple-english` skill first and follow its plain-English rules.
- User documentation starts at [documentation.md](documentation.md).
  Keep reference pages in `docs/reference/` and current architecture in
  `docs/architecture.md`. Use plain Markdown and relative links, with
  Docusaurus-compatible front matter. Document exposed API changes, defaults,
  errors, and effects in the matching page. Future designs belong in issues.
  The Docusaurus site in `website/` renders the same pages; `make docs` serves
  them locally.

## Commits

- **"Commit" means Conventional Commits.** e.g. `fix: clamp inventory size`,
  `feat: parse mode tags`, `docs: update tag contract`.
- One sentence, no body, no bullet lists: the subject line is the whole
  message (commitlint enforces `body-empty` and a 100-char header). A
  `Signed-off-by:` trailer is allowed.
- commitlint enforces the format (`commitlint.config.mjs`, CI job).
- **No agent attribution.** Never add `Co-Authored-By:`, "Generated with",
  or any assistant/tool byline to a commit. The user is the sole author.

## Roadmap / open items

- [x] Terminal player: `oink run` with keys, mouse, the cover picture, and
      the `--seed` and `--choices` flags for CI.
- [x] Full-screen ratatui page: styled checks and notices, a status bar
      with resources, conditions, and level, emphasis markup in prose,
      `--plain` for the transcript.
- [x] Optional `ui:` block in `config.yaml`: reader page sizes, sheet order,
      top bar, status bar, and palettes. Parsed by `oink-ui::config`, never
      by `oink-core`.
- [x] Per-page layout resolution: `ui.layout` base layout, `ui.pages.<id>`
      overrides that merge field by field and can remove a bar, and zone
      dirty flags for partial e-ink refreshes.
- [x] Center page: wrapped prose through the markup parser, check records,
      numbered choices, and the pager.
- [x] Reader bars: resource and characteristic values on top, and the sheet,
      inventory, and log buttons below.
- [x] Paged character sheet: one page per configured group, with spell
      ability and cost.
- [x] Paged inventory: name and weight, grouped by the configured item count.
- [x] Session log: a host-side ring buffer with a newest-first paged view.
- [x] Touch targets: choice rows, page arrows, bar buttons, and the back
      label.
- [x] Inventory stacks: repeated items keep a count, and the inventory panel
      shows it.
- [x] Inventory detail pages: the arrow keys select an item, Enter or a click
      opens its description, weight, tags, consumable flag, modifiers, and
      picture, and Backspace returns to the list.
- [x] Scene and item pictures: `# image: <path>` on a knot and `image:` on an
      item in `rulebook.yaml`, both relative to the bundle directory.
- [ ] Terminal player extras: a help overlay and a journal of past scenes.
- [x] Terminal player panels: the sheet, the inventory, and the session log
      open with the `s`, `i`, and `l` keys, page with PgUp/PgDn and the wheel,
      and close with Backspace. The log feeds from the state-change notices.
- [x] Save and load slots: `Engine::snapshot()` and `Engine::restore()` write
      and read a versioned JSON envelope of story, character, and dice state.
      The `m` key opens the menu, which saves and loads numbered slots in
      `saves/` beside the bundle.
- [ ] `players/m5paper` firmware crate (esp-idf, it8951, GT911 touch, SD/flash
      bundle loading; espup toolchain).
- [ ] A cover picture for `examples/high-pass` (`cover.png`).
- [ ] More e-ink hardware targets beyond the M5Paper.
- [ ] Mobile builds (iOS, Android).
- [x] Mode tag parsing in `oink-core`: `Event::Scene` returns the mode id.
      Per-line speaker tags (`speaker`, `mood`, `portrait`) still open.
- [x] External functions (inventory/perks/conditions/tags/checks/environments/
      resources) bound into Ink via `bladeink`'s `bind_external_function`.
- [x] Dialog and map renderers in the terminal player: speaker name, portrait
      column, map label, location markers. Device renderers wait for the
      M5Paper crate.
- [x] Character names and portraits and map labels in the `ui` block of
      `config.yaml`.
- [ ] Full characters/maps YAML schemas (portrait variants, map geometry).
- [x] Spells module with cost and cast resolution.
- [ ] Story and world state (flags, counters).
- [x] Pig mascot + logo (`assets/logo.png`, hi-res in `assets/logo-hires.png`).
