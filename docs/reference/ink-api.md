---
title: Ink API
sidebar_position: 4
---

# Ink API

Declare every engine function that your story calls with `EXTERNAL`.
Function names and argument counts are fixed. Renaming a display label in YAML
does not rename an external function.

String arguments must be Ink strings. Numeric arguments must be integers.
Wrong types or argument counts produce a story error instead of a default value.
All IDs below refer to the loaded rulebook.

## Checks

| Function | Return value | Behavior |
| --- | --- | --- |
| `roll_check(ability, difficulty, tags, modifier)` | string | Rolls the default profile, records the check, and returns `critical_failure`, `failure`, `success`, or `critical_success` |
| `roll_check(ability, difficulty, tags, modifier, dice)` | string | Rolls the named profile and records the check |
| `check_roll()` | integer | Returns the kept dice total, or the 1..100 value for d% |
| `check_score()` | integer | Returns the score: dice plus contribution for over, dice alone for under |
| `check_target()` | integer | Returns the target number |
| `check_margin()` | integer | Returns the margin |
| `check_degrees()` | integer | Returns the degrees, or 0 when the profile uses `none` |
| `check_die(index)` | integer | Returns one face in roll order; for d% index 0 is tens and 1 is units with faces 0..9 |
| `passive_check(ability, dc, tags, modifier)` | integer | Records the passive check and returns 1 on a pass or 0 on a failure |
| `passive_value(ability, tags, modifier)` | integer | Returns the passive value without recording a check |
| `check_breakdown(ability, tags, modifier)` | string | Returns the contribution list and subtotal without dice or the passive constant |
| `difficulty(name)` | integer | Looks up a configured difficulty |

`ability`, `tags`, `name`, and `dice` are strings. `difficulty`, `dc`, and `modifier`
are integers. `modifier` is a one-off bonus or penalty for this check.
Use 0 when there is no one-off modifier.
`tags` is a comma-separated list, or `""` for none.
`dice` names a profile from the rulebook dice section.

Unknown abilities, difficulty names, and dice profiles produce errors. Tags need not be
registered to match at runtime. See [checks](checks.md) for the calculation.
Query functions read the last active check and produce errors when no check ran
yet. `check_die` also errors when the index is out of range.

```ink
EXTERNAL passive_value(ability, tags, modifier)
EXTERNAL check_breakdown(ability, tags, modifier)
EXTERNAL difficulty(name)

Passive value: {passive_value("empathy", "artist", -1)}.
Contributions: {check_breakdown("empathy", "artist", -1)}.
Target: {difficulty("medium")}.
-> END
```

## Inventory

| Function | Return value | Behavior |
| --- | --- | --- |
| `has_item(id)` | boolean | Tests ownership; false for an absent ID |
| `add_item(id)` | void | Adds a known item and applies tag grants; adding it twice does not duplicate it |
| `remove_item(id)` | void | Removes an owned item; absent IDs do nothing |
| `use_item(id)` | void | Applies an owned consumable's conditions and removes it |

All arguments are strings. `add_item()` and `use_item()` reject unknown definitions.
`add_item()` also rejects gated items whose prerequisites fail.
Using a known but absent item does nothing. Using a non-consumable does nothing.
There are no item quantities or equipment slots.

```ink
EXTERNAL add_item(id)
EXTERNAL has_item(id)
EXTERNAL remove_item(id)

~ add_item("lantern")
{ has_item("lantern"):
    You can see the steps below.
}
~ remove_item("lantern")
-> END
```

## Perks, conditions, and tags

| Function | Return value | Behavior |
| --- | --- | --- |
| `has_perk(id)` | boolean | Tests perk ownership |
| `add_perk(id)` | void | Adds a known perk and applies grants |
| `remove_perk(id)` | void | Removes it if present |
| `has_condition(id)` | boolean | Tests whether a condition is active |
| `add_condition(id)` | void | Adds a known condition with its configured duration, or resets it and emits a refresh notice |
| `remove_condition(id)` | void | Removes it if present |
| `has_tag(tag)` | boolean | Tests character, perk, condition, and carried-item tags |

Arguments are strings. Adding an unknown perk or condition produces an error.
Adding a perk or item whose prerequisites fail produces an error too.
Queries return false for absent IDs. Removal of absent IDs does nothing.
Removing a source does not remove abilities or perks it previously granted.
There are no `add_tag()` or `remove_tag()` Ink bindings today.

## Environments and scenes

| Function | Return value | Behavior |
| --- | --- | --- |
| `enter_environment(id)` | void | Activates a known environment; repeated entry does not stack it |
| `clear_environment(id)` | void | Removes it if active |
| `end_scene()` | void | Advances timed conditions once and reports expired conditions |

Environment IDs are strings. Entering an unknown environment produces an error.
Clearing an absent ID does nothing. `end_scene()` takes no arguments.
The writer chooses boundaries; see [scene boundaries](state.md#scene-boundaries).

```ink
EXTERNAL enter_environment(id)
EXTERNAL clear_environment(id)
EXTERNAL end_scene()

~ enter_environment("dark")
The corridor is dark.
* [Step outside]
    ~ end_scene()
    ~ clear_environment("dark")
    The daylight hurts your eyes.
    -> END
```

## Character values and resources

| Function | Return value | Behavior |
| --- | --- | --- |
| `ability_level(id)` | integer | Returns the owned level, or 0 if absent or unknown |
| `characteristic(id)` | integer | Returns the stored characteristic value |
| `characteristic_bonus(id)` | integer | Returns its threshold bonus, or the raw value with `bonus: direct` |
| `resource(id)` | integer | Returns the current resource balance |
| `resource_max(id)` | integer | Returns the effective maximum, derived when `max_from` is set |
| `spend_resource(id, amount)` | boolean | Pays the full amount or returns false without a change |
| `restore_resource(id, amount)` | boolean | Restores up to the maximum; true if the balance changed |

IDs are strings and amounts are nonnegative integers. Unknown characteristics
or resources produce errors. Value queries do not include temporary check modifiers.
A zero payment succeeds without emitting a change record. Negative amounts produce errors.

## Character creation

These functions run only with a `character_creation` section; without one,
`points_available` returns 0 and the rest fail closed. Creation runs only when
the story calls these functions, so a story can also apply a preset silently.

| Function | Return value | Behavior |
| --- | --- | --- |
| `points_available(kind)` | integer | Returns unspent points; kind is `characteristic`, `ability`, or `perk` |
| `spend_point(kind, id)` | boolean | Spends the flat cost and applies one pick, or returns false without a change |
| `apply_preset(id)` | boolean | Applies a preset as the complete sheet and marks every pool spent |
| `set_characteristic(id, value)` | boolean | Sets a characteristic clamped to its bounds, spending nothing |
| `set_ability(id, value)` | boolean | Sets an ability level floored at zero, spending nothing |
| `reset_character()` | void | Rebuilds the sheet from the creation base |

Unknown point kinds produce errors. Unknown targets produce false. A preset
marks every pool spent, so preset and manual spending cannot stack. Stories
gate progress on `points_available`, e.g. `{ points_available("characteristic") > 0: ... }`.

## Levelling

XP comes only from the story; `add_xp` banks it without levelling, so the
story controls when the sheet changes. Sheets start at level 1.

| Function | Return value | Behavior |
| --- | --- | --- |
| `xp()` | integer | Returns banked experience |
| `add_xp(amount)` | void | Banks nonnegative experience |
| `level()` | integer | Returns the current level |
| `level_up_ready()` | boolean | True when banked XP reaches the next curve level below the cap |
| `level_up()` | boolean | Rises one level with rewards, or returns false when not ready |

Negative amounts produce errors. Without a `levelling` section, `add_xp` still
banks, but readiness and level-ups stay false.

## Spells

`cast_spell(id)` returns an active-check outcome string. The string ID must name
a spell in the rulebook. The application must enable `oink-core`'s `spells`
feature; default builds do not bind this function.

Casting validates the spell, resource balance, and check before applying a cost.
A resolved cast pays its full cost on success or failure, including critical failure.
An invalid or unaffordable cast produces a story error without spending or rolling.
The engine records both the resource change and the individual dice.

```ink
EXTERNAL cast_spell(id)
EXTERNAL resource(id)

VAR result = ""

{ resource("focus") >= 2:
    ~ result = cast_spell("telekinesis")
    { result == "success" or result == "critical_success":
        The key rises from the floor.
    - else:
        The key stays where it is.
    }
- else:
    You need more focus.
}
-> END
```

This example uses the sample spell, whose cost is 2 Focus with a minimum of 0.
The story owns all spell consequences beyond the resource cost.

## Execution and errors

Queries without side effects can run during bladeink's lookahead, its evaluation
of upcoming content. Dice rolls, mutations, `end_scene()`, and recorded passive
checks are bound as not lookahead-safe.

An external-function error becomes `EngineError::Story`. The host decides how
to display it. These functions do not provide a transaction over an entire
passage: mutations completed before a later error remain applied.
