---
title: Rulebook overview
sidebar_position: 2
---

# Rulebook overview

A game uses an engine configuration, a rulebook, and a story.
The configuration holds the title. The rulebook defines the character sheet
and the rules. The story controls choices, consequences, and endings.

## Engine configuration

`assets/data/config.yaml` contains:

```yaml
title: "Oink Demo"
```

The system uses two six-sided dice for active checks. There is no dice
expression setting. Passive checks use a fixed value of 6 instead of dice.

## Rulebook

`assets/data/rulebook.yaml` contains named definitions and a starting character.
IDs connect the definitions. Display names can change without changing those IDs.
The [YAML reference](reference/yaml.md) lists every section and its fields.

```yaml
characteristics:
  dexterity:
    name: Dexterity
    min: 1
    max: 14
    default: 10
    bonus:
      thresholds:
        - { at: 1, bonus: -2 }
        - { at: 4, bonus: -1 }
        - { at: 7, bonus: 0 }
        - { at: 10, bonus: 1 }
        - { at: 13, bonus: 2 }
abilities:
  lockpicking:
    name: Lockpicking
    characteristic: dexterity
starting_character:
  abilities: { lockpicking: 2 }
```

Here the character has Dexterity 10, a characteristic bonus of +1, and
Lockpicking level 2. An active Lockpicking check starts at `2d6 + 3`.

## Story

Declare each engine function used by the story with `EXTERNAL`.
The engine binds the function when it loads the story. A check returns a result
that the story can use to choose the next passage.

```ink
EXTERNAL roll_check(ability, difficulty, tags, modifier)

VAR outcome = ""

The lock is old, but the door is solid.
* [Pick the lock]
    ~ outcome = roll_check("lockpicking", 10, "", 0)
    { outcome == "success" or outcome == "critical_success":
        The lock opens.
    - else:
        The lock holds.
    }
    -> END
```

Use this story with the YAML example above. The [Ink API](reference/ink-api.md)
documents the arguments, return values, and failures of each function.
