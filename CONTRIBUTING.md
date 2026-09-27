# Contributing to oink

Thanks for the interest. This page holds the setup steps, the commands, and the
commit rules. Agent guidance lives in [AGENTS.md](AGENTS.md).
The [architecture guide](docs/architecture.md) describes current ownership.
The [documentation index](documentation.md) links the public API references.

Update the relevant reference page with each API change. Keep future designs
in GitHub issues. Use plain Markdown so the pages can move into a documentation site.

## Requirements

You need Rust stable and CMake. The simulator builds SDL2 from source on the
first run.

## Commands

```sh
make build   # build the workspace
make sim     # run the desktop simulator
make test    # run the tests (the engine walks the demo story)
make check   # type-check the workspace
make fmt     # format the code
make lint    # clippy, with warnings as errors
```

Run `make test` after engine changes. Run `make fmt` and `make lint` before you
commit.

Spells are optional and sit behind the `spells` feature. Test them with
`cargo test -p oink-core --features spells`.

## Run the simulator

`make sim` opens a 960x540 window. Press a key from 1 to 9 to select a choice.
Press Esc to quit. The simulator loads the story and the YAML data from
`assets/`.

## Commits

- Use Conventional Commits: `feat: add map renderer`, `fix: clamp inventory
  size`, `docs: update tag contract`.
- Keep the subject line under 100 characters. Do not add a body, a bullet list,
  or a signature. commitlint enforces this in CI.
- Do not add co-author trailers or tool bylines. The author is the author.

## Code

- Keep `oink-core` free of display and IO code. It must build and test on the
  host.
- Render through the `embedded-graphics` `DrawTarget` trait. Never call the
  simulator or the e-ink driver from shared code.
