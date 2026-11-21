---
title: UI configuration
sidebar_position: 6
---

# UI configuration

A game bundle sets its presentation in the optional `ui` block of
`config.yaml`. The block changes how a player presents the game. It does not
change the story or the rules. The terminal player reads the `reader`,
`status`, and `theme.terminal` blocks. An e-ink player reads the `reader` and
`theme.eink` blocks.

A bundle without the block uses the player defaults. The block is not
required. The engine never reads it. The `--plain` transcript ignores it.

## Example

```yaml
title: "The High Pass"

ui:
  reader:
    items_per_page: 8
    log_entries: 50
    save_slots: 3
    sheet_order: [characteristics, abilities, perks, conditions, spells]

  layout:
    top_bar:
      resources: [focus]
      characteristics: [intellect, psyche, physique, motorics]
    bottom_bar:
      buttons: [sheet, inventory, log, menu]

  pages:
    dialog:
      top_bar:
        height: 96
      bottom_bar: false
    map:
      top_bar: false
      bottom_bar: false

  characters:
    keeper:
      name: The Shrine-Keeper
    oka:
      name: Oka
      portrait: portraits/oka.png

  maps:
    ravine:
      label: The Ravine

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

`reader.items_per_page` sets the number of inventory items on one page. An
item that needs more rows continues on the following page. A stack shows its
count as `Rope x2` and its unit weight in parentheses as `Rope x2 (2)`. An
empty inventory shows a notice. The arrow keys select an item, and Enter or a
click opens its detail page: the description, weight, tags, consumable flag,
modifiers, and the item `image` from the rulebook. Backspace returns to the
list. `reader.log_entries` caps the
session log; the oldest entry drops when the buffer is full. `reader.save_slots`
sets the number of numbered save slots and the menu rows. All three values
must be greater than zero.

The menu button opens the save and load slots. Each slot has a save row and a
load row. The terminal player keeps the slots in a `saves` directory beside
the bundle, one JSON file per slot. The engine writes the file contents and
the player owns the paths.
See [character state and scenes](state.md#story-facts-and-saving).

`reader.sheet_order` sets the page order of the character sheet. Each group
starts a new page. A group lists characteristic values, ability levels, perk
names, condition names, or spells with the ability and the resource cost. An
empty group shows `None`, so the configured order stays visible. The values
are `characteristics`, `abilities`, `perks`, `conditions`, and `spells`. A
group can appear once. The default order is this full list.

`layout` is the base layout of the reader: a top bar, a center page, and a
bottom bar. Every scene uses it unless a `pages` block overrides it.

`layout.top_bar.resources` and `layout.top_bar.characteristics` name the
values in the top summary bar of an e-ink reader. The resource values draw at
the left edge and the characteristic values at the right edge, in the listed
order. A value reads `Health 8/10` for a resource and `Physique 7` for a
characteristic. Text that does not fit is cut, and the characteristic group
loses space first. An empty list hides the group. Every id must exist in
`rulebook.yaml`. `layout.top_bar.height` sets the bar height in pixels; the
device metrics give the default.

`layout.bottom_bar.buttons` lists the buttons of the bottom bar in display
order. The values are `sheet`, `inventory`, `log`, and `menu`. A name can
appear once, and the default list is `sheet`, `inventory`, `log`, and `menu`.
An empty list shows an empty bar. `layout.bottom_bar.height` sets the bar
height in pixels.

`pages` overrides the base layout for one scene mode. The key is the mode
id from the `# mode: <id>` tag in the story; see the
[scene tags](ink-api.md#scene-tags). The rules for one block:

1. A zone key the block omits keeps the base value.
2. The value `false` removes the zone on that page, and `true` keeps it.
3. A mapping replaces the fields it lists, and the other fields inherit.

A page without the center key always keeps the center, because the center
zone always exists. The removed bars give their space to the center. A mode
id without a page block uses the base layout, and the player writes a
warning. The `prose` mode uses the base layout too, unless a `pages.prose`
block overrides it, and it never warns.

`characters` names the characters a scene can speak with. The key is the id
from the `# speaker: <id>` and `# portrait: <id>` tags; see the
[scene tags](ink-api.md#scene-tags). `name` is the display name and is
required. `portrait` is the portrait file, relative to the bundle directory,
and is optional. A player that draws pictures loads the file when a scene
shows the character. A character without a portrait file, or a file that does
not read, draws with the name only. An id that is not in the block falls back
to the raw id as the display name.

`maps` names the map layouts of the map pages. The key is the id from the
`# map: <id>` tag. `label` is the display name, drawn above the location
choices, and is required. An id that is not in the block falls back to the raw
id as the label.

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

Every field is optional. A missing field keeps its default. A missing or
empty block keeps every default too. Unknown fields in the block are errors,
so a typo fails instead of passing in silence. A resource or characteristic
id that is not in the rulebook is an error.

## Errors

The player stops with an error when the block has bad YAML, an unknown
field, a zero page size, a duplicate sheet group, a bottom bar with an
unknown or repeated button, a repeated resource or characteristic id, a
Gray4 value outside `0` to `15`, a character without a name, a map without a
label, or an unknown resource or characteristic id in the base layout or in
a page block.

## Demo

The demo bundle sets every block in `examples/high-pass/config.yaml`.
