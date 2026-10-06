// Test fixture, not a game. A short walk through every binding with a
// known shape: creation (preset or spend), an environment and a passive
// check, two active rolls on two dice profiles, an item branch, a
// consumable that applies a condition, a spell, a level-up, one scene
// boundary, and two endings decided by a roll.
//
// Multi-branch "- condition:" blocks do not work with bladeink, so
// outcomes use nested conditionals.

EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
EXTERNAL passive_check(ability, dc, tags, modifier)
EXTERNAL check_roll()
EXTERNAL points_available(kind)
EXTERNAL spend_point(kind, id)
EXTERNAL apply_preset(id)
EXTERNAL characteristic(id)
EXTERNAL ability_level(id)
EXTERNAL has_item(id)
EXTERNAL add_item(id)
EXTERNAL use_item(id)
EXTERNAL has_condition(id)
EXTERNAL enter_environment(id)
EXTERNAL clear_environment(id)
EXTERNAL end_scene()
EXTERNAL resource(id)
EXTERNAL resource_max(id)
EXTERNAL level()
EXTERNAL xp()
EXTERNAL add_xp(amount)
EXTERNAL level_up_ready()
EXTERNAL level_up()
EXTERNAL cast_spell(id)

VAR outcome = ""
VAR ready = false

Start.
+ [Preset]
    ~ apply_preset("ready")
    -> dark
+ [Spend]
    -> spend_body

=== spend_body ===
Spend body: {points_available("characteristic")}.
{ points_available("characteristic") > 0:
    * [Body]
        ~ spend_point("characteristic", "body")
        -> spend_body
- else:
    -> spend_ability
}

=== spend_ability ===
Spend ability: {points_available("ability")}.
{ points_available("ability") > 0:
    * [Might]
        ~ spend_point("ability", "might")
        -> spend_ability
- else:
    -> spend_perk
}

=== spend_perk ===
Spend perk: {points_available("perk")}.
{ points_available("perk") > 0:
    * [Brave]
        ~ spend_point("perk", "brave")
        -> spend_perk
- else:
    -> dark
}

=== dark ===
~ enter_environment("dark")
Sheet: body {characteristic("body")} might {ability_level("might")} stamina {resource("stamina")}/{resource_max("stamina")} level {level()} xp {xp()}.
{ passive_check("wits", 5, "", 0):
    Passive passed.
}
* [Roll]
    ~ outcome = roll_check("might", 7, "", 0, "pair")
    Roll {check_roll()}: {outcome}.
    -> key
* [Key]
    ~ add_item("key")
    { has_item("key"):
        Key taken.
    }
    -> key

=== key ===
~ clear_environment("dark")
~ outcome = roll_check("wits", 10, "", 0, "single")
Roll {check_roll()}: {outcome}.
~ use_item("potion")
{ has_condition("buzzed"):
    Buzzed.
}
* [Zap]
    ~ outcome = cast_spell("zap")
    Zap {check_roll()}: {outcome}.
    -> milestone
* [Rest]
    -> milestone

=== milestone ===
~ add_xp(10)
~ ready = level_up_ready()
{ ready:
    ~ level_up()
    Level {level()}.
}
~ end_scene()
Verdict.
* [Judge]
    ~ outcome = roll_check("wits", 16, "tested", 0, "pair")
    { outcome == "success" or outcome == "critical_success":
        Open. -> END
    - else:
        Closed. -> END
    }
