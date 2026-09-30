![A pig knight reading a large red book](assets/logo.png)

# oink 🐽

[![CI](https://github.com/fabriziosestito/oink/actions/workflows/ci.yml/badge.svg)](https://github.com/fabriziosestito/oink/actions/workflows/ci.yml)
[![Docs](https://github.com/fabriziosestito/oink/actions/workflows/docs.yml/badge.svg)](https://github.com/fabriziosestito/oink/actions/workflows/docs.yml)

oink is a *librogame*
([gamebook](https://en.wikipedia.org/wiki/Gamebook)) engine for e-ink devices.
It brings branching stories, 2d6 skill checks, and data-driven rules to screens
that stay readable in sunlight. Think choose-your-own-adventure, with dice.
Desktop builds run today, and phones and more e-ink hardware are on the roadmap.

oink is in early development.

## 🐽 Features

- 📖 **Stories that branch.** Choices, consequences, and several endings. A
  decision can come back chapters later.
- 🎲 **2d6 skill checks.** Roll two dice, add an ability and any modifiers, and
  beat the difficulty, the way
  [Disco Elysium](https://en.wikipedia.org/wiki/Disco_Elysium) does it. A double
  six always wins, a double one always loses. White checks can be retried, red
  checks are one shot.
- 🧾 **Your world, your rules.** Define abilities, perks, conditions, and items
  in plain text. The engine tracks them and uses them in checks. See the
  [documentation](documentation.md).
- 🐽 **Built for e-ink.** E-ink devices are the main event, and the same engine
  runs on Linux, Windows, macOS, iOS, and Android.
## 🚧 Coming soon

- 📟 [M5Paper](https://docs.m5stack.com/en/core/m5paper) firmware first, then
  more e-ink devices.
- 🧩 Presentation modes: prose, dialog, and map layouts chosen by story tags.
- 💾 Bookmarks: save and load your place in the story.
- 📱 iOS and Android builds.

## 🚀 Quickstart

Play the demo story on your laptop, no hardware needed. You need Rust stable.
On Linux, first install the SDL2 development package (`sudo apt install
libsdl2-dev` on Debian and Ubuntu). On macOS, install CMake instead: the
simulator builds its own SDL2 from source there, so the first build takes a
few minutes.

```sh
make sim    # play the demo story
make test   # the engine walks the demo story in milliseconds
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full command list and the
simulator controls.

## 📖 Documentation

The documentation covers the rulebook, the YAML format, checks, and the engine
API. Read it at <https://fabriziosestito.github.io/oink/>, or run `make docs`
to serve the site on your machine with hot reload.

## 📜 License

[Apache-2.0](LICENSE).
