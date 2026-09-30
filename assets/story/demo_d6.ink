// d6 demo: uses the default "standard" profile (2d6, roll-over).
// To try it: cp assets/story/demo_d6.ink assets/story/main.ink && make sim
//
// Note: multi-branch "- condition:" blocks do not work with bladeink,
// so the four outcomes are written as nested conditionals.

EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL passive_check(ability, dc, tags, modifier)

VAR outcome = ""

A wooden door blocks the cellar. You hear footsteps above you.
{ passive_check("empathy", 8, "", 0):
    Someone is holding their breath on the other side.
}

*   [Barge it with your shoulder]
    ~ outcome = roll_check("endurance", 8, "", 0)
    { outcome == "critical_success":
        The door flies off its hinges. -> cellar
    - else:
        { outcome == "success":
            The door gives way with a creak. -> cellar
        - else:
            { outcome == "critical_failure":
                You bounce back and hit the floor. -> corridor
            - else:
                Your shoulder aches, the door stays shut. -> corridor
            }
        }
    }
*   [Pick the lock]
    ~ outcome = roll_check("lockpicking", 6, "thief", 0)
    { outcome == "success" or outcome == "critical_success":
        The bolt clicks open. -> cellar
    - else:
        The pick bends out of shape. -> corridor
    }

=== cellar ===
You found the supplies. Mission accomplished.
-> END

=== corridor ===
You turn back. You will try again tomorrow.
-> END
