// Prova d20: usa il profilo "heroic" (1d20, roll-over, 20 naturale critico).
// Per provarla: cp assets/story/demo_d20.ink assets/story/main.ink && make sim
//
// Nota: i rami "- condizione:" multipli non funzionano con bladeink,
// quindi i quattro esiti sono scritti come condizionali annidati.

EXTERNAL roll_check(ability, difficulty, tags, modifier, dice)
EXTERNAL check_margin()

VAR outcome = ""

Un drago dorme sopra un mucchio d'oro. Una scaglia luccica in disparte.
*   [Sguscia verso l'oro]
    ~ outcome = roll_check("lockpicking", 12, "thief", 0, "heroic")
    { outcome == "critical_success":
        Nemmeno un tintinnio. L'oro è tuo. -> oro
    - else:
        { outcome == "success":
            Un passo felpato dopo l'altro. L'oro è tuo. -> oro
        - else:
            { outcome == "critical_failure":
                Pesti una coppa. Il drago apre un occhio. -> fuga
            - else:
                Un'eco metallica. Il drago si agita. -> fuga
            }
        }
    }
*   [Sfida il drago a duello]
    ~ outcome = roll_check("endurance", 12, "", 0, "heroic")
    Margine: {check_margin()}.
    { outcome == "success" or outcome == "critical_success":
        Il drago accetta e perde. Leggendario. -> oro
    - else:
        Il drago sbuffa divertito. Meglio correre. -> fuga
    }

=== oro ===
Ricco e vivo. Le taverne canteranno di te.
-> END

=== fuga ===
Vivo e povero. Le gambe tremano ancora.
-> END
