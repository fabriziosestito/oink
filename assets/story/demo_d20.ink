// d20 demo: uses the "heroic" profile (1d20, roll-over, natural 20 crits).
// To try it: cp assets/story/demo_d20.ink assets/story/main.ink && make sim
//
// Note: multi-branch "- condition:" blocks do not work with bladeink,
// so the four outcomes are written as nested conditionals.

EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
EXTERNAL check_margin()

VAR outcome = ""

A dragon sleeps on a mound of gold. One scale glints off to the side.
*   [Sneak toward the gold]
    ~ outcome = roll_check("lockpicking", 12, "thief", 0, "heroic")
    { outcome == "critical_success":
        Not even a clink. The gold is yours. -> gold
    - else:
        { outcome == "success":
            One soft step after another. The gold is yours. -> gold
        - else:
            { outcome == "critical_failure":
                You kick a goblet. The dragon opens one eye. -> flight
            - else:
                A metallic echo. The dragon stirs. -> flight
            }
        }
    }
*   [Challenge the dragon to a duel]
    ~ outcome = roll_check("endurance", 12, "", 0, "heroic")
    Margin: {check_margin()}.
    { outcome == "success" or outcome == "critical_success":
        The dragon accepts and loses. Legendary. -> gold
    - else:
        The dragon snorts, amused. Better run. -> flight
    }

=== gold ===
Rich and alive. The taverns will sing of you.
-> END

=== flight ===
Alive and poor. Your legs are still shaking.
-> END
