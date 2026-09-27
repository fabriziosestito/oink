---
title: Character state and scenes
sidebar_position: 3
---

# Character state and scenes

The rulebook defines character behavior. The engine owns the live character
instance in memory. Calls such as `add_item()` change it immediately and return
structured records for the UI.

## Character sheet

The current character stores these values:

```rust
pub struct Character {
    characteristics: BTreeMap<String, i32>,
    abilities: BTreeMap<String, i32>,
    perks: BTreeSet<String>,
    conditions: BTreeMap<String, Option<u32>>,
    inventory: BTreeSet<String>,
    tags: BTreeSet<String>,
    environments: BTreeSet<String>,
    resources: BTreeMap<String, i32>,
}
```

The fields are private. Use the [Rust API](rust-api.md) or the [Ink API](ink-api.md)
to read and change them. Conditions store remaining scene counts; `None` means
the condition lasts until removed. Environments are stored here today even though
they describe the environment.

## Scene boundaries

The writer defines a scene boundary with `end_scene()`.
Each call advances all timed conditions by one scene. A remaining duration of
1 or 0 expires and emits `StateChange::ConditionRemoved`.

```ink
EXTERNAL add_condition(id)
EXTERNAL has_condition(id)
EXTERNAL end_scene()

~ add_condition("nicotine_rush")
The conversation takes most of the evening.
* [Leave]
    ~ end_scene()
    -> street

=== street ===
{ has_condition("nicotine_rush"):
    The effect has one scene left.
}
* [Walk home]
    ~ end_scene()
    -> home

=== home ===
{ not has_condition("nicotine_rush"):
    The effect is gone.
}
-> END
```

The sample condition lasts two scenes. Choices, knots, `Engine::choose()`, and
`END` do not advance durations by themselves. Call `end_scene()` once on each
route that leaves a scene. Two calls count as two boundaries.

Call it before applying conditions that belong to the next scene.
Permanent conditions remain. Environments also remain until `clear_environment(id)`
is called. `end_scene()` does not clear queues, move the story, or save a file.

## Resource payments

`spend_resource(id, amount)` succeeds only when the full payment leaves the
resource at or above its minimum. Failure leaves it unchanged and returns false.
A zero payment returns true and emits no record.

`restore_resource(id, amount)` adds up to the maximum and returns whether the
value changed. Both Ink functions reject negative amounts and unknown resources.
The Rust character methods return no changes for invalid or unaffordable operations.

```ink
EXTERNAL spend_resource(id, amount)
EXTERNAL resource(id)

{ spend_resource("focus", 2):
    You keep your thoughts together.
- else:
    You need more focus.
}
Focus remaining: {resource("focus")}.
-> END
```

## State-change records

`StateChange` describes a mutation that already happened. It does not apply
the mutation when read. The enum preserves IDs and values so the UI can choose
icons, labels, and layout without parsing a sentence.

```rust
pub enum StateChange {
    ItemAdded(String),
    ItemRemoved(String),
    PerkAdded(String),
    PerkRemoved(String),
    ConditionAdded(String),
    ConditionRemoved(String),
    EnvironmentEntered(String),
    EnvironmentCleared(String),
    TagAdded(String),
    TagRemoved(String),
    AbilityGranted(String),
    ResourceChanged { id: String, from: i32, to: i32 },
}
```

`Display` supplies default text through `change.to_string()`.
`Engine::take_changes()` returns and empties the queue. Records accumulate until
the host drains them; the queue does not reset at scene boundaries.
The simulator currently displays check records but does not drain state-change notices.

## Story facts and saving

Use Ink variables for facts such as `VAR spoke_with_guard = false`.
They can last across scenes and chapters. `StateChange` notices are not story
memory, and oink has no separate world-flags API today.

Bladeink owns the story variables, current choices, and execution position.
Oink owns the character sheet and dice source. Oink does not yet expose a
combined save/load API. Persistence is tracked separately from rulebook work.
