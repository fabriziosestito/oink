---
title: Checks and modifiers
sidebar_position: 2
---

# Checks and modifiers

An active check rolls the profile dice pool. The rulebook adds a characteristic
contribution, an ability level, and matching modifiers. Over adds dice plus
contribution against difficulty. Under rolls dice alone against contribution
plus difficulty. A passive check uses the same contribution with the profile
passive constant in place of the dice.

## Active checks

```text
over: score = dice + contribution, target = difficulty, margin = score - target
under: score = dice, target = contribution + difficulty, margin = target - score
```

Success needs a margin of zero or more. The ordered outcome table decides
criticals and normal results. The first matching row wins. Face tests look at
kept dice only. Dropped dice never trigger a face test.

```ink
EXTERNAL roll_check(ability, difficulty, tags, modifier)

VAR result = ""

~ result = roll_check("lockpicking", 12, "thief", -1)
{ result == "success" or result == "critical_success":
    The door opens.
- else:
    The door stays shut.
}
-> END
```

This example uses `lockpicking` from the sample rulebook. The final argument
adds a penalty of -1 to this check only. A fifth argument names another dice
profile, such as `roll_check("melee_basic", 20, "", 0, "wfrp")`. Four-argument
calls use the default profile and keep working.

Degrees come from the profile formula: `margin` uses the margin, `tens` uses
the tens digits, and `none` stays 0. WFRP Success Levels use `tens`. The
matched outcome row can bound degrees with `degrees_min` and `degrees_max`.

## Passive checks and queries

```text
value = contribution + passive constant
```

`passive_check(ability, dc, tags, modifier)` returns 1 when the value meets the
difficulty, otherwise 0. It records a check for the UI. The constant comes
from the default profile. A missing constant means 0. Direction does not
change the comparison.
`passive_value(ability, tags, modifier)` returns the value without recording a check.
`check_breakdown(ability, tags, modifier)` returns the contributions without the
passive constant or dice. It does not record a check either.
`check_roll()`, `check_score()`, `check_target()`, `check_margin()`,
`check_degrees()`, and `check_die(index)` read the last active check. They
return story errors when no check ran yet or the index is out of range.

```ink
EXTERNAL passive_check(ability, dc, tags, modifier)
EXTERNAL passive_value(ability, tags, modifier)

{ passive_check("empathy", 8, "", 0):
    His smile does not reach his eyes.
}
Your passive value is {passive_value("empathy", "artist", -1)}.
-> END
```

The simulator currently shows recorded passive failures as well as successes.
A host UI can hide failed passive checks if the story needs hidden thresholds.

## How modifiers combine

For an ability check, the engine reads the ability's parent characteristic.
It converts the stored value through its bonus table, or passes it untouched
with `bonus: direct`. It then adds the owned ability level. If the character
lacks that ability, its level is 0.

It then adds matching modifiers from carried items, owned perks, active
conditions, and active environments. A characteristic modifier targets checks
based on that characteristic. An ability modifier targets only that ability.
A tag modifier targets a tag present on the check.

The check tag set combines explicit tags with active environment tags.
Pass explicit tags as a comma-separated string, such as `"thief,dark"`.
Spaces are trimmed and empty entries are ignored. Repeated tags do not stack.
Ability metadata tags and character tags are not added automatically.

Each owned source is visited once. A source with several matching modifier
entries contributes all of them. There is no total modifier cap. Totals
saturate at the `i32` bounds, so extreme values clamp instead of overflowing.

```yaml
perks:
  night_vision:
    name: Night Vision
    modifiers:
      tags: { dark: 2 }
environments:
  dark:
    name: Darkness
    tags: [dark]
    modifiers:
      abilities: { lockpicking: -2 }
```

Here the environment subtracts 2 from Lockpicking. Night Vision adds 2 to any
check tagged `dark`, including other abilities while this environment is active.
Use narrower check tags when a bonus must apply only to visual tasks.

A perk or condition with `advantage: true` selects the advantage pool. One
with `disadvantage: true` selects the disadvantage pool. Both together cancel.
A story pool argument beats the state flags.

## Check records

The engine exposes each active check, passive check, and spell check through
`Engine::take_checks()`. Each record contains the pool, score, target, margin,
degrees, result, and a typed modifier breakdown. Active and spell records
include the dice log with kept flags.

```rust
pub struct CheckRecord {
    pub ability: String,
    pub difficulty: i32,
    pub outcome: String,
    pub pool: String,
    pub score: i32,
    pub target: i32,
    pub margin: i32,
    pub degrees: i32,
    pub dice: Option<DiceRoll>,
    pub breakdown: Breakdown,
}
```

Passive outcomes are `pass` and `fail`; their dice field is `None`.
`CheckRecord::describe()` returns a text summary with dropped dice marked and
the score against the target with degrees. A graphical UI can instead draw
each die and format each breakdown entry separately.
