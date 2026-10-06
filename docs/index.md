---
title: oink documentation
sidebar_position: 1
---

# oink documentation

oink is a gamebook engine for e-ink devices. Authors define rules in YAML and
write story flow in Ink. The terminal player runs the same engine used by
future device applications.

This reference describes the current code. The project is in development,
so APIs can change. Future designs live in GitHub issues.

## For story authors

- Read the [rulebook overview](rulebook.md) for the files a game needs.
- Use the [YAML reference](reference/yaml.md) to define characters and rules.
- Read [checks and modifiers](reference/checks.md) for the 2d6 calculation.
- Read [character state](reference/state.md) for inventory, conditions, and scene boundaries.
- Use the [Ink API](reference/ink-api.md) to connect a story to the rules.

## For application developers

- Use the [Rust API](reference/rust-api.md) to load and run a story.
- Read [architecture](architecture.md) for ownership and crate boundaries.
- Read [publishing](publishing.md) to use these pages in a documentation site.

For local setup and the player controls, see the
[contributing guide](https://github.com/fabriziosestito/oink/blob/main/CONTRIBUTING.md).
