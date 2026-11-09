---
title: UI configuration
sidebar_position: 6
---

# UI configuration

A game bundle can ship an optional `ui.yaml` file. The file changes how a
player presents the game. It does not change the story or the rules. The
terminal player reads the `status` and `theme.terminal` blocks. An e-ink
player reads the `reader` and `theme.eink` blocks.

A bundle without `ui.yaml` uses the player defaults. The file is not
required. The engine never reads it. The `--plain` transcript ignores it.

## Example

```yaml
version: 1

reader:
  items_per_page: 8
  log_entries: 50
  save_slots: 3
  sheet_order: [characteristics, abilities, perks, conditions, spells]
  top_bar:
    resources: [focus]
    characteristics: [intellect, psyche, physique, motorics]

status:
  resources: [focus]
  show_level: true
  show_conditions: true

theme:
  eink:
    paper: 15
    ink: 0
    dim: 7
    accent: 3
  terminal:
    text: default
    background: default
    accent: cyan
```

## Fields

`version` is the file format. This player reads version `1`. Another value
fails with an error that names the supported version.

`reader.items_per_page` sets the number of inventory items on one page.
`reader.log_entries` caps the session log. `reader.save_slots` sets the
number of numbered save slots. All three values must be greater than zero.

`reader.sheet_order` sets the page order of the character sheet. The values
are `characteristics`, `abilities`, `perks`, `conditions`, and `spells`. A
group can appear once. The default order is this full list.

`reader.top_bar.resources` and `reader.top_bar.characteristics` name the
values in the top summary bar of an e-ink reader. An empty list hides the
group. Every id must exist in `rulebook.yaml`.

`status.resources` sets the resources in the terminal status bar. When the
field is absent, the player shows every resource. An empty list hides the
resources. `status.show_level` and `status.show_conditions` show or hide the
level and the conditions. Both default to `true`.

`theme.eink` holds the Gray4 tones for an e-ink reader. A Gray4 value is a
4-bit grayscale level from `0` (black) to `15` (white). The fields are
`paper`, `ink`, `dim`, and `accent`. The default is `15`, `0`, `7`, and `4`.

`theme.terminal` holds three terminal colors. `text` sets the prose, the
choices, and the status text, `background` sets the page background, and
`accent` sets the title, the choice numbers, and the notes. The value is an
ANSI color name such as `cyan`, or `default` to follow the terminal theme.
The semantic styles for success, failure, conditions, and resources keep
their standard colors. The `NO_COLOR` environment variable wins over this
block.

## Defaults

Every field is optional. A missing field keeps its default. An empty file
keeps every default too. Unknown fields are errors, so a typo fails instead
of passing in silence. A resource or characteristic id that is not in the
rulebook is an error.

## Errors

The player stops with an error when the file has bad YAML, an unknown field,
an unsupported version, a zero page size, a duplicate sheet group, a Gray4
value outside `0` to `15`, or an unknown resource or characteristic id.

## Demo

The demo bundle has a full example in `examples/high-pass/ui.yaml`.
