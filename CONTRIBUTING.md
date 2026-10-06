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
optional `cover.png` shows above the title in terminals with a graphics
protocol.

In a terminal, `oink run` prints each scene and waits at a `>` prompt. Press a
key from 1 to 9 to pick a choice, or click the choice line. Press Esc, q, or
Ctrl-C to quit. While the player waits, it captures the mouse, so plain drag
selection does not work. Hold Shift while you drag to select text.

When stdin or stdout is not a terminal, the player reads one number per line
from stdin and shows no pictures:

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
