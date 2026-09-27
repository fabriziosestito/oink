---
title: Checks and modifiers
sidebar_position: 2
---

# Checks and modifiers

An active check rolls two six-sided dice. The rulebook adds a characteristic
bonus, an ability level, and matching modifiers. A passive check uses the same
calculation with a fixed 6 in place of the dice.

## Active checks

```text
total = die 1 + die 2 + characteristic bonus + ability level + modifiers
```

A total equal to the difficulty succeeds. Two ones always return
`critical_failure`, even if the total meets the difficulty.
Two sixes always return `critical_success`, even if the total falls short.
Other rolls return `success` or `failure`.

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
adds a penalty of -1 to this check only.

## Passive checks and queries

```text
value = 6 + characteristic bonus + ability level + modifiers
```

`passive_check(ability, dc, tags, modifier)` returns 1 when the value meets the
difficulty, otherwise 0. It records a check for the UI.
`passive_value(ability, tags, modifier)` returns the value without recording a check.
`check_breakdown(ability, tags, modifier)` returns the contributions without the
passive constant or dice. It does not record a check either.

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
It converts the stored characteristic value through its bonus table, then adds
the owned ability level. If the character lacks that ability, its level is 0.

It then adds matching modifiers from carried items, owned perks, active
conditions, and active environments. A characteristic modifier targets checks
based on that characteristic. An ability modifier targets only that ability.
A tag modifier targets a tag present on the check.

The check tag set combines explicit tags with active environment tags.
Pass explicit tags as a comma-separated string, such as `"thief,dark"`.
Spaces are trimmed and empty entries are ignored. Repeated tags do not stack.
Ability metadata tags and character tags are not added automatically.

Each owned source is visited once. A source with several matching modifier
entries contributes all of them. There is no total modifier cap.

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

## Check records

The engine exposes each active check, passive check, and spell check through
`Engine::take_checks()`. Each record contains the target, result, and a typed
modifier breakdown. Active and spell records include both individual dice.

```rust
pub struct CheckRecord {
    pub ability: String,
    pub difficulty: i32,
    pub outcome: String,
    pub total: i32,
    pub dice: Option<[u8; 2]>,
    pub breakdown: Breakdown,
}
```

Passive outcomes are `pass` and `fail`; their dice field is `None`.
`CheckRecord::describe()` returns a text summary. A graphical UI can instead
draw both dice and format each breakdown entry separately.
