# Contributing to oink

Thanks for the interest. This page holds the setup steps, the commands, and the
commit rules. Agent guidance lives in [AGENTS.md](AGENTS.md).
The [architecture guide](docs/architecture.md) describes current ownership.
The [documentation index](documentation.md) links the public API references.

Update the relevant reference page with each API change. Keep future designs
in GitHub issues. Use plain Markdown so the pages can move into a documentation site.

## Requirements

You need Rust stable. Nothing else: the terminal player has no system
dependencies.

## Commands

```sh
make build   # build the workspace
make run     # play the demo in the terminal
make test    # run the tests
make check   # type-check the workspace
make fmt     # format the code
make lint    # clippy, with warnings as errors
```

Run `make test` after engine changes. Run `make fmt` and `make lint` before you
commit.

## Run the player

`make run` builds the `oink` command and runs `oink run examples/high-pass`.
Pass flags with `ARGS`, for example `make run ARGS="--seed 7"`.

A game bundle is one directory with `config.yaml`, `rulebook.yaml`, and
`story.ink`. A precompiled `story.ink.json` works in place of the source. An
optional `cover.png` opens the game on a cover screen.

In a terminal, `oink run` takes the whole window. The scene text sits in a
centered column, the dice checks and the character notices follow it, and the
choices come after one blank line. When the text does not fit, it scrolls and
the choices stay at the bottom. The bottom bar shows the resources, the active
conditions, the level, and the keys.

Press a key from 1 to 9 to pick a choice at once. Move the highlight with the
arrow keys, or with j and k, and press Enter to take it. Click a choice to pick
it. Scroll long text with PageUp, PageDown, Home, End, or the mouse wheel.
Press Esc, q, or Ctrl-C to quit. The player captures the mouse, so plain drag
selection does not work. Hold Shift while you drag to select text.

The player uses the 16 colors of the terminal palette, so it follows your
theme. If `NO_COLOR` is set to a non-empty value, the player keeps bold,
italic, and dim text and drops the colors. Terminals that speak the Kitty,
iTerm2, or Sixel graphics protocol show the cover as a picture. Other
terminals draw it with Unicode half blocks.

Story text can carry emphasis markup: `**bold**`, `*italic*`, and `_italic_`.
The [Ink API reference](docs/reference/ink-api.md#emphasis-in-prose) has the
rules.

When stdin or stdout is not a terminal, or when you pass `--plain`, the player
prints each scene as a transcript without markup and waits at a `>` prompt.
Type a number and press Enter to pick a choice, or type q to quit:

```sh
printf '1\n1\n' | oink run examples/high-pass
```

Two flags make a run repeatable, which CI uses:

```sh
oink run examples/high-pass --seed 7 --choices 1,1,1,1,1,1,1
```

`--seed N` makes every dice roll the same on every run. `--choices` plays the
listed picks, numbered from 1, without waiting for input. The run stops when
the picks run out, and a pick outside the current scene is an error.

## Commits

- Use Conventional Commits: `feat: add map renderer`, `fix: clamp inventory
  size`, `docs: update tag contract`.
- Keep the subject line under 100 characters. Do not add a body, a bullet list,
  or a signature. commitlint enforces this in CI.
- Do not add co-author trailers or tool bylines. The author is the author.

## Code

- Keep `oink-core` free of display and IO code. It must build and test on the
  host.
- Device rendering goes through the `embedded-graphics` `DrawTarget` trait.
  Never call the e-ink driver from shared code.
- Tests never read the demo game. `oink-core/tests/fixtures/` holds a
  synthetic rulebook and story for tests, and `oink-rulebook` uses inline
  YAML. The CLI integration tests are the only tests that run
  `examples/high-pass`.
