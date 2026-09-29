// oink demo story

EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL passive_check(ability, dc, tags, modifier)
EXTERNAL has_item(item)
EXTERNAL has_tag(tag)
EXTERNAL has_perk(perk)
EXTERNAL add_item(item)
EXTERNAL add_condition(condition)
EXTERNAL enter_environment(id)
EXTERNAL clear_environment(id)
EXTERNAL end_scene()

VAR outcome = ""

The old bridge creaks under your boots. On the far side, a troll
sharpens a rusty cleaver, eyeing you with mild professional interest.
~ enter_environment("dark")
{ passive_check("empathy", 8, "", 0):
    Something in the troll's grip is not as steady as it looks.
}

*   [Draw the sword and charge]
    ~ outcome = roll_check("endurance", 10, "", 0)
    { outcome == "critical_success":
        One clean blow. The troll looks at its cleaver, then at you, then leaves. -> victory
    - outcome == "success":
        Steel rings against steel. The troll is strong, but slow. -> victory
    - outcome == "critical_failure":
        The cleaver finds your shoulder before you find your footing. -> death
    - else:
        You back off the bridge, bruised but breathing. -> retreat
    }
*   [Try to talk]
    "Nice cleaver," you offer. The troll blinks. Nobody has complimented its cleaver in two hundred years. -> victory
*   [Search the toll booth]
    { has_tag("wizard"):
        You whisper a word, and a small light gathers in your palm. A lantern waits behind the counter. -> lantern
    - has_perk("night_vision"):
        Your eyes are enough. A lantern waits behind the counter. -> lantern
    - else:
        It is too dark to search. -> waiting
    }

=== lantern ===
~ add_item("lantern")
A brass lantern, still warm from another hand. -> waiting

=== waiting ===
{ has_item("lantern"):
    The lantern throws the troll's shadow long across the bridge.
}
The troll taps its cleaver against the bridge. It is waiting.
*   [Try to talk] -> victory
*   [Draw the sword and charge] -> fight

=== fight ===
~ outcome = roll_check("endurance", 10, "", 0)
{ outcome == "success" or outcome == "critical_success":
    The troll stumbles. You press the opening. -> victory
- outcome == "critical_failure":
    The cleaver finds you. -> death
- else:
    You back off, bruised but breathing. -> retreat
}

=== victory ===
~ end_scene()
~ clear_environment("dark")
~ add_condition("shaken")
The troll is gone. The road to adventure lies open, and your hands will not stop shaking.
-> END

=== retreat ===
~ end_scene()
~ clear_environment("dark")
You walk away, mission unaccomplished.
-> END

=== death ===
~ end_scene()
~ clear_environment("dark")
The last thing you see is the bridge, and the sky above it.
-> END
