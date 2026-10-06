// Mountain pass: body-or-mind creation, a sheet summary, then trials,
// a heroic climb, a spell, a level-up milestone and a verdict.
// Note: multi-branch "- condition:" blocks do not work with bladeink,
// so outcomes use nested conditionals.

EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
EXTERNAL passive_check(ability, dc, tags, modifier)
EXTERNAL check_roll()
EXTERNAL points_available(kind)
EXTERNAL spend_point(kind, id)
EXTERNAL apply_preset(id)
EXTERNAL characteristic(id)
EXTERNAL ability_level(id)
EXTERNAL add_perk(id)
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

Snow on the high pass. A courier run, alone.
+ [Take the veteran preset]
    ~ apply_preset("veteran")
    No questions asked. -> sheet
+ [Train yourself]
    -> spend

=== spend ===
Body or mind? Points: {points_available("characteristic")}.
{ points_available("characteristic") > 0:
    * [Build Physique]
        ~ spend_point("characteristic", "physique")
        -> spend
    * [Sharpen Intellect]
        ~ spend_point("characteristic", "intellect")
        -> spend
    * [Sharpen Psyche]
        ~ spend_point("characteristic", "psyche")
        -> spend
- else:
    -> spend_ability
}

=== spend_ability ===
Hands to train? Skill points: {points_available("ability")}.
{ points_available("ability") > 0:
    * [Drill Logic]
        ~ spend_point("ability", "logic")
        -> spend_ability
    * [Drill Endurance]
        ~ spend_point("ability", "endurance")
        -> spend_ability
- else:
    -> spend_perk
}

=== spend_perk ===
One talent to claim.
{ points_available("perk") > 0:
    * [Learn Night Vision]
        ~ spend_point("perk", "night_vision")
        -> spend_perk
- else:
    -> sheet
}

=== sheet ===
INT {characteristic("intellect")} PSY {characteristic("psyche")} PHY {characteristic("physique")} MOT {characteristic("motorics")}.
Logic {ability_level("logic")} Empathy {ability_level("empathy")} Endurance {ability_level("endurance")}.
Focus {resource("focus")}/{resource_max("focus")}. Level {level()}, XP {xp()}.
+ [Shoulder the pack]
    -> pass1

=== pass1 ===
~ enter_environment("dark")
The ravine is dark. Loose stones tick above.
{ passive_check("logic", 5, "", 0):
    You spot the warning cairn in time.
}
* [Climb the rockfall]
    ~ outcome = roll_check("endurance", 8, "", 0, "standard")
    Roll {check_roll()}: {outcome}. -> shrine
* [Light a lantern first]
    ~ add_item("lantern")
    { has_item("lantern"):
        Warm light. -> shrine
    }

=== shrine ===
A wayside shrine, older than the guild.
~ outcome = roll_check("logic", 8, "", 0, "standard")
Roll {check_roll()}: {outcome}.
{ outcome == "success" or outcome == "critical_success":
    The runes align. -> keeper
- else:
    The runes stay silent. -> keeper
}

=== keeper ===
~ clear_environment("dark")
The shrine-keeper offers night-sight training, for sharp minds only.
+ { characteristic("intellect") >= 5 } [Accept eagle-eyes training]
    ~ add_perk("eagle_eyes")
    -> eyrie
+ [Leave an offering]
    -> eyrie

=== eyrie ===
The storm breaks over the ridge. One rope, one chance.
~ outcome = roll_check("endurance", 12, "", 0, "heroic")
Roll {check_roll()}: {outcome}.
{ outcome == "critical_success":
    You dance up the ice. -> nightcamp
- else:
    { outcome == "success":
        Slow and steady. -> nightcamp
    - else:
        { outcome == "critical_failure":
            The rope sings, then holds. Barely. -> nightcamp
        - else:
            You haul yourself up, shaking. -> nightcamp
        }
    }
}

=== nightcamp ===
~ use_item("cigarette")
{ has_condition("nicotine_rush"):
    Steady hands for the spell.
}
Focus {resource("focus")}/{resource_max("focus")}, the working costs 2.
* [Cast light into the dark]
    ~ outcome = cast_spell("telekinesis")
    Roll {check_roll()}: {outcome}. -> milestone
* [Save your strength]
    -> milestone

=== milestone ===
~ add_xp(150)
~ ready = level_up_ready()
{ ready:
    ~ level_up()
    Something settles into place. Level {level()}. -> levelup
- else:
    -> verdict
}

=== levelup ===
* [Keen senses: empathy +1]
    ~ spend_point("ability", "empathy")
    -> verdict
* [Move on]
    -> verdict

=== verdict ===
~ end_scene()
The guild seal waits at the pass head.
* [Ask for judgment]
    ~ outcome = roll_check("empathy", 10, "artist", 0, "standard")
    { outcome == "success" or outcome == "critical_success":
        "The pass is open," Oka says. -> END
    - else:
        "The mountain waits," Oka says. -> END
    }
