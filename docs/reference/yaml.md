---
title: YAML reference
sidebar_position: 1
---

# YAML reference

The engine loads `config.yaml` and `rulebook.yaml` as two separate documents.
`config.yaml` requires a string `title`. All sections below belong to
`rulebook.yaml` and are optional unless a referenced definition needs them.

## IDs and display names

Definitions are maps keyed by stable string IDs, such as `lockpicking` or
`night_vision`. Each definition requires a string `name`.
Descriptions are optional strings. Lists and modifier maps default to empty.

```yaml
names:
  characteristics: Attributes
  abilities: Skills
  perks: Talents
  conditions: Conditions
  items: Inventory
  resources: Resources
  spells: Powers
  tags: Tags
  environments: Environments
```

Omitted labels use the section names in title case. Labels change presentation
only. YAML keys and Ink function names stay the same.

## Characteristics

| Field | Type | Default | Meaning |
| --- | --- | --- | --- |
| `name` | string | Required | Display name |
| `description` | string | Absent | Description |
| `min` | integer | `1` | Lowest starting value |
| `max` | integer | `10` | Highest starting value |
| `default` | integer | `min` when building a character | Starting value unless overridden |
| `bonus.thresholds` | list | Empty | Entries with integer `at` and `bonus` |
| `bonus` | string | Threshold table | `direct` passes the stored value untouched |

The loader sorts thresholds from highest to lowest. The highest threshold
reached by the value supplies the bonus. Values below the lowest threshold use
its bonus. An empty table returns 0 and produces a loader warning.
Use `bonus: direct` for percentile systems where the characteristic forms the
target base.

```yaml
characteristics:
  strength:
    name: Strength
    min: 1
    max: 14
    default: 10
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
        - { at: 10, bonus: 1 }
        - { at: 13, bonus: 2 }
  weapon_skill:
    name: Weapon Skill
    min: 1
    max: 100
    bonus: direct
```

## Abilities and difficulties

An ability requires `name` and `characteristic`, which references a characteristic ID.
It also accepts `description` and a list of `tags`.
The starting character assigns ability levels. A missing ability contributes level 0.

```yaml
abilities:
  climbing:
    name: Climbing
    characteristic: strength
    tags: [physical]
difficulties:
  easy: 8
  medium: 10
  hard: 12
```

Difficulty names map to integers. There is no built-in difficulty table when
the section is omitted. The sample rulebook provides one.
Ability tags are metadata today; pass check tags explicitly to the check API.

## Modifiers

Perks, conditions, items, and environments accept the same modifier fields:

```yaml
modifiers:
  characteristics: { strength: 1 }
  abilities: { climbing: -2 }
  tags: { physical: 2 }
```

These are signed integer adjustments to the check total.
Characteristic modifiers apply to checks based on that characteristic.
They do not rewrite the stored characteristic or recalculate its threshold bonus.
Tag modifiers apply when the check has the named tag.
See [checks and modifiers](checks.md) for stacking rules.

## Perks and conditions

Perks accept `name`, `description`, `modifiers`, `grants_tags`, `advantage`,
and `disadvantage`. Conditions accept those fields plus an optional nonnegative
integer `duration`. The duration counts explicit `end_scene()` calls, as
described in [scene boundaries](state.md#scene-boundaries).
A perk or condition with `advantage: true` selects the profile advantage pool.
One with `disadvantage: true` selects the disadvantage pool. Both together
cancel to the base profile. Several sources on one side do not stack.

```yaml
perks:
  steady_hands:
    name: Steady Hands
    modifiers:
      abilities: { climbing: 1 }
  lucky:
    name: Lucky
    advantage: true
conditions:
  shaken:
    name: Shaken
    duration: 2
    modifiers:
      abilities: { climbing: -2 }
```

Omit `duration` to keep a condition until the story removes it.
Adding an existing condition resets its duration without stacking its modifiers.
A duration of 0 expires at the next `end_scene()` call.

## Prerequisites

The optional `prerequisites` map gates perks, abilities, items, and spells by
id behind minimum values. Characteristics read stored values, abilities read
owned levels with absent meaning 0, resources read current balances.

```yaml
prerequisites:
  juggernaut:
    requires:
      characteristics: { physique: 6 }
  telekinesis:
    requires:
      abilities: { arcana: 2 }
      resources: { focus: 2 }
```

Ink `add_perk` and `add_item` refuse gated ids with an error. Creation
`spend_point` returns false and presets skip gated perks. Spell casting does
not enforce prerequisites yet.

## Environments

Environments accept `name`, `description`, `tags`, and `modifiers`.
They describe the current environment. Their tags join the explicit check tags
while active, and their modifiers apply to both active and passive checks.

```yaml
environments:
  rain:
    name: Heavy Rain
    tags: [wet]
    modifiers:
      abilities: { climbing: -2 }
```

The story enters and clears environments by ID. They do not expire when a
condition expires or when a scene ends.

## Items

Items accept `name`, `description`, `tags`, `modifiers`, a nonnegative integer
`weight` (default 0), `consumable` (default false), and `applies_conditions`.
Each condition in `applies_conditions` must exist in the rulebook.

```yaml
items:
  climbing_rope:
    name: Climbing Rope
    weight: 2
    tags: [tool]
    modifiers:
      abilities: { climbing: 1 }
  calming_tea:
    name: Calming Tea
    consumable: true
    applies_conditions: [calm]
conditions:
  calm:
    name: Calm
    duration: 2
```

Inventory stores unique item IDs. It has no quantities or equipment slots.
Carried items supply modifiers and tags. Weight is metadata; the engine does
not enforce a carrying limit. `use_item()` applies conditions and removes an
owned consumable. An absent item or a non-consumable is left unchanged.

## Tags and grants

Tags accept `name`, `description`, and optional `grants.abilities` and
`grants.perks` lists. Granted abilities start at level 0.
Existing ability levels are preserved.

```yaml
tags:
  climber:
    name: Climber
    description: Trained for steep ground.
    grants:
      abilities: [climbing]
      perks: [steady_hands]
```

Character tags, perk tags, condition tags, and carried item tags contribute to
`has_tag()`. Environment tags belong to checks and do not enter that character-tag set.
Grants remain after their source tag is removed. Grant expansion continues until
the sheet stops changing.

## Resources

Resources require `name` and either integer `max` or a `max_from` derivation
list. They accept integer `min` (default 0) and boolean `start_full`
(default true). `starting_character.resources` overrides the initial value.

```yaml
resources:
  health:
    name: Health
    min: 0
    max_from:
      - characteristic: physique
        mode: thresholds
        thresholds:
          - { at: 1, value: 10 }
          - { at: 11, value: 40 }
      - characteristic: psyche
        mode: per_point
        base: 0
        value_per_point: 2
  focus: { name: Focus, min: 0, max: 5, start_full: false }
```

A `thresholds` entry reads the highest `at` the characteristic reaches, like a
bonus table. A `per_point` entry computes `base + value_per_point *
characteristic`. Entries sum together, and derivation reads stored
characteristic values, so temporary conditions never move resource maxima.
Character creation pools live in `character_creation`: each pool grants points
that `spend_point` consumes at the flat `costs` rate, presets mark every pool
spent, and `validate: all_points_spent` requires each pool to reach zero. See
[Character creation](#character-creation).

Starting values are clamped to the bounds. Payments are all-or-nothing above
the minimum. Restoration stops at the maximum.

## Spells

Spell definitions require `name`, `ability`, `cost`, and `check`.
`description` is optional. Cost requires a resource ID and a nonnegative
integer amount. Difficulty accepts an integer or a name from `difficulties`.

```yaml
spells:
  sure_footing:
    name: Sure Footing
    ability: climbing
    cost: { resource: focus, amount: 2 }
    check: { difficulty: medium, tags: [physical] }
```

## Character creation

The optional `character_creation` section describes point-buy creation. When
absent, stories use `starting_character` directly. Creation runs only when the
story calls its functions, so a story can also apply a preset silently and
skip spending entirely.

```yaml
character_creation:
  mode: pool
  base:
    characteristics: { intellect: 2, physique: 2 }
    abilities: { logic: 0 }
  pools:
    characteristic_points: 10
    ability_points: 5
    perk_points: 1
  costs:
    characteristics: 1
    abilities: 1
  validate: all_points_spent
  presets:
    bruiser:
      name: Bruiser
      characteristics: { physique: 8 }
      abilities: { endurance: 3 }
      perks: [juggernaut]
      tags: [strong]
```

Costs are flat points per pick, default to 1, and must be positive: a zero
cost would grant unlimited picks without ever completing validation. The
loader rejects preset and
base entries that reference unknown characteristics, abilities, or perks.

## Levelling

The optional `levelling` section sets the XP curve, the level cap, and the
rewards. Sheets start at level 1 with 0 XP. Experience comes only from the
story through `add_xp`; checks never grant XP.

```yaml
levelling:
  max_level: 10
  xp_curve:
    - { level: 2, xp: 100 }
    - { level: 3, xp: 250 }
  rewards:
    per_level:
      characteristic_points: 1
      ability_points: 2
    interval:
      every: 3
      perk_points: 1
```

Each entry names the XP needed to reach that level. `level_up()` rises one
level when banked XP reaches the next curve entry, granting the per-level
points plus the interval perk points when the new level hits the interval.
Curve levels must sit within 2 and the cap with no duplicates, and the
interval must be positive when present.

Definitions load in all builds, and casting works in every build.
See the [Ink API](ink-api.md#spells) for cost and failure behavior.

## Dice pools

The `dice` section names the default profile and the named profiles. Each
profile sets a notation string, a direction (`over` or `under`), a degrees
formula (`margin`, `tens`, or `none`), an optional passive constant, optional
advantage and disadvantage pool names, and an ordered outcome table. The first
matching outcome row wins. A row without tests always matches. An unmatched
table returns `failure` with a loader warning.

```yaml
dice:
  default: standard
  profiles:
    standard:
      notation: "2d6"
      direction: over
      degrees: margin
      passive: 6
      outcomes:
        - { all_max: true, outcome: critical_success }
        - { all_min: true, outcome: critical_failure }
        - { margin_at_least: 5, outcome: critical_success }
        - { margin_at_least: 0, outcome: success }
        - { margin_at_least: -4, outcome: failure }
        - { outcome: critical_failure }
    wfrp:
      notation: "d%"
      direction: under
      degrees: tens
      outcomes:
        - { score_at_most: 5, outcome: critical_success, degrees_min: 1 }
        - { score_at_least: 96, outcome: critical_failure, degrees_max: -1 }
        - { doubles: true, margin_at_least: 0, outcome: critical_success }
        - { doubles: true, margin_at_most: -1, outcome: critical_failure }
        - { margin_at_least: 0, outcome: success }
        - { outcome: failure }
```
A pool can contain 1 to 10 dice. Supported dice are d2, d4, d6, d8, d10, 
d12,
d20, and d%. Use keep notation
for advantage pools, such as `2d20kh1` and `2d20kl1`. `d100` and `1d100`
are aliases for `d%`. A plain `2d10` stays a sum. When the section is absent,
the loader inserts the 2d6 standard profile above. That profile keeps the old
double-based criticals and adds margin-based ones: margin 5 or more is a
critical success, margin -5 or less is a critical failure.

## Starting character

```yaml
starting_character:
  characteristics: { strength: 10 }
  abilities: { climbing: 2 }
  tags: [climber]
  perks: [steady_hands]
  inventory: [climbing_rope]
  resources: { focus: 3 }
```

All fields are optional. Characteristics and resources use their definition
defaults when omitted. The remaining collections start empty.
The engine applies tag grants when it builds the character.
Conditions and environments start empty.

## Loading errors and warnings

The loader rejects malformed YAML, reversed bounds on characteristics,
resources, and outcome row pairs, invalid characteristic
defaults, unknown ability parents, unknown grant targets, unknown starting IDs,
bad dice notation, unknown die sizes, unknown dice defaults, and unknown
advantage pools. It also rejects unknown conditions applied by items and
invalid spell references or negative spell costs. Prerequisite entries must
gate a known perk, ability, item, or spell and reference known characteristics,
abilities, and resources. Levelling curve levels must sit within 2 and the cap
with no duplicates, and a present interval must be positive.

Unknown tags used by abilities, grants, environments, spell checks, or tag
modifiers produce warnings. Descriptive item tags need no registry entry.
Missing bonus tables produce warnings. `GameData.warnings` exposes these to the host.

Validation is not exhaustive today. Unknown YAML fields are ignored, and
characteristic or ability IDs inside modifier maps are not checked.
