// Debug story: spend points, print the sheet, then checks, items and one
// spell with small roll readouts. Run it with:
//   cp assets/story/demo_debug.ink assets/story/main.ink && make sim
//
// Note: multi-branch "- condition:" blocks do not work with bladeink,
// so outcomes use nested conditionals.

EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
EXTERNAL check_roll()
EXTERNAL points_available(kind)
EXTERNAL spend_point(kind, id)
EXTERNAL apply_preset(id)
EXTERNAL characteristic(id)
EXTERNAL ability_level(id)
EXTERNAL add_perk(id)
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
EXTERNAL cast_spell(id)

VAR outcome = ""

New recruit. Spend first.
+ [Veteran preset]
    ~ apply_preset("veteran")
    -> sheet
+ [Spend yourself]
    -> spend

=== spend ===
Body points: {points_available("characteristic")}.
{ points_available("characteristic") > 0:
    * [Intellect +1]
        ~ spend_point("characteristic", "intellect")
        -> spend
    * [Physique +1]
        ~ spend_point("characteristic", "physique")
        -> spend
    * [Done]
        -> sheet
- else:
    -> sheet
}

=== sheet ===
INT {characteristic("intellect")} PSY {characteristic("psyche")} PHY {characteristic("physique")} MOT {characteristic("motorics")}.
Logic {ability_level("logic")} Empathy {ability_level("empathy")} Endurance {ability_level("endurance")}.
Focus {resource("focus")}/{resource_max("focus")}. Level {level()}, XP {xp()}.
+ [Continue]
    -> trial

=== trial ===
~ enter_environment("dark")
A locked door in the dark.
* [Pick it]
    ~ outcome = roll_check("lockpicking", 6, "thief", 0, "standard")
    Roll {check_roll()}: {outcome}. -> vault
* [Break it]
    ~ outcome = roll_check("endurance", 8, "", 0, "standard")
    Roll {check_roll()}: {outcome}. -> vault

=== vault ===
Supplies and a lantern.
~ add_item("lantern")
~ clear_environment("dark")
The quartermaster nods at sharp minds.
+ { characteristic("intellect") >= 5 } [Take eagle-eyes]
    ~ add_perk("eagle_eyes")
    -> magic
+ [Move on]
    -> magic

=== magic ===
Focus {resource("focus")}/{resource_max("focus")}, spell costs 2.
* [Cast telekinesis]
    ~ outcome = cast_spell("telekinesis")
    Roll {check_roll()}: {outcome}. -> finale
* [Smoke instead]
    ~ use_item("cigarette")
    { has_condition("nicotine_rush"):
        Steady.
    }
    -> finale

=== finale ===
~ end_scene()
Done. -> END
