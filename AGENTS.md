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
     (`assets/story/*.ink`).
   - Rule definitions and starting character: **YAML** (`assets/data/rulebook.yaml`).
     The game title lives in `assets/data/config.yaml`.
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

3. **No emulator; display abstraction instead.**
   - There is no M5Paper emulator. All rendering goes through
     `embedded-graphics`'s `DrawTarget` trait.
   - Desktop dev uses `embedded-graphics-simulator` (SDL2). On device the
     target is the IT8951 e-ink driver (`it8951` crate). Same engine and UI
     code, two mains behind crates.
   - The engine core is a plain Rust crate, unit-testable with `cargo test` —
     faster iteration than any emulator.

4. **Ink runtime: `bladeink` (2.x).**
   - Pure-Rust port of inkle's reference runtime; full language support
     (threads, flows, external functions, choice tags, save/load state).
   - Compiled with `bladeink-compiler`: at startup in the simulator
     (`Engine::new`), at build time for firmware (`Engine::from_json` loads
     precompiled `.ink.json`).
   - Replaced `inkling` (0.12, unmaintained since 2020, no external
     functions/tags). Keep engine code runtime-agnostic where cheap.

5. **Not published to crates.io.** This is an application (firmware +
   simulator), not a library. The crate name `oink` is taken on crates.io
   (pig latin crate) — irrelevant since we distribute via git/releases/flash.
   If the core is ever extracted as a library, publish as `oink-engine` or
   similar, not `oink`.

## Workspace layout

```
oink/
├── Cargo.toml           # workspace (resolver 2); shared deps in [workspace.dependencies]
├── Makefile             # build / sim / test / check / fmt / lint / docs / clean / m5paper
├── .cargo/config.toml   # SDL2 link path + CMake policy (macOS/aarch64)
├── documentation.md    # entry point to user documentation
├── docs/index.md       # documentation navigation
├── docs/reference/     # YAML, checks, state, Ink API, and Rust API
├── docs/architecture.md # current crate boundaries and runtime ownership
├── website/             # Docusaurus site: theme and build config
├── oink-core/           # engine core: Ink runtime wrapper + YAML data model
│   └── src/
│       ├── lib.rs
│       ├── data.rs      # Config, GameData (rulebook-backed)
│       └── engine.rs    # Engine, Event, Choice, external bindings + tests
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
├── oink-sim/            # desktop simulator: 960x540 Gray4, keys 1-9 choose, Esc quits
│   └── src/main.rs
├── oink-m5paper/        # (planned) ESP32 firmware: esp-idf-hal + it8951 + GT911 touch
└── assets/
    ├── logo.png         # mascot (hi-res in logo-hires.png)
    ├── story/main.ink   # demo story (troll on a bridge)
    └── data/
        ├── config.yaml  # game title
        └── rulebook.yaml # rulebook: characteristics, abilities, perks, checks
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
- `Event::Scene { text, choices }` — prose paragraphs + choices to render.
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
| `# speaker: <id>` | line | character speaking (id defined in YAML) |
| `# mood: <id>` | line | portrait variant |
| `# portrait: <id>` | line | explicit portrait override |
| `# map: <id>` | knot | which map layout (defined in YAML) |

⚠️ This table is the intended contract, not yet implemented: no tag parsing
happens in `oink-core` today (see roadmap). `bladeink` exposes tags
(`Story::get_current_tags`, `Choice::tags`) so no runtime change is needed.

External functions are bound by `oink-core` (inventory, perks, conditions,
tags, checks, environments, resources; see the Engine API list). The writer calls
`end_scene()` once per scene boundary to advance timed conditions. Choices,
knots, and story endings do not advance durations automatically.

Character definitions (name, portrait assets), map layouts, and mode config
go in YAML, referenced by ID from tags.

## Commands

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup, the full `make` list, and the
simulator controls.

Always run `make test` (and `make lint` before committing) after engine
changes. The simulator is the manual test bed.

`make docs` starts the documentation site with hot reload on both `docs/` and
`website/`. See [docs/publishing.md](docs/publishing.md) for the site layout,
theme, and deployment.

## Conventions

- Rust 2021 edition, workspace-level shared dependencies.
- Keep `oink-core` free of any display/IO concerns — it must stay
  `cargo test`-able on the host and eventually embedded-friendly.
- All rendering against `embedded_graphics::DrawTarget`; never code against
  the simulator or IT8951 directly in shared code.
- Sample/demo content lives in `assets/`; the simulator loads it from the
  workspace root (run via `make sim` or from repo root).
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

- [ ] `oink-m5paper` firmware crate (esp-idf, it8951, GT911 touch, SD/flash
      asset loading; espup toolchain).
- [ ] More e-ink hardware targets beyond the M5Paper.
- [ ] Mobile builds (iOS, Android).
- [ ] Tag parsing in `oink-core` (`mode`, `speaker`, ...) — extend
      `Event::Scene` with mode + per-line speaker.
- [x] External functions (inventory/perks/conditions/tags/checks/environments/
      resources) bound into Ink via `bladeink`'s `bind_external_function`.
- [ ] Dialog and map renderers in the UI layer.
- [ ] Characters/maps YAML schemas.
- [x] Spells module with cost and cast resolution.
- [ ] Story and world state (flags, counters) and save/load (issue #2).
- [x] Pig mascot + logo (`assets/logo.png`, hi-res in `assets/logo-hires.png`).
