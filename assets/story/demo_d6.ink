// Prova d6: usa il profilo default "standard" (2d6, roll-over).
// Per provarla: cp assets/story/demo_d6.ink assets/story/main.ink && make sim
//
// Nota: i rami "- condizione:" multipli non funzionano con bladeink,
// quindi i quattro esiti sono scritti come condizionali annidati.

EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL passive_check(ability, dc, tags, modifier)

VAR outcome = ""

Una porta di legno blocca la cantina. Senti passi sopra di te.
{ passive_check("empathy", 8, "", 0):
    Qualcuno trattiene il fiato dall'altra parte.
}

*   [Sfondala di spalla]
    ~ outcome = roll_check("endurance", 8, "", 0)
    { outcome == "critical_success":
        La porta vola via dai cardini. -> cantina
    - else:
        { outcome == "success":
            La porta cede scricchiolando. -> cantina
        - else:
            { outcome == "critical_failure":
                Rimbalzi indietro e cadi a terra. -> corridoio
            - else:
                La spalla duole, la porta resta chiusa. -> corridoio
            }
        }
    }
*   [Forza la serratura]
    ~ outcome = roll_check("lockpicking", 6, "thief", 0)
    { outcome == "success" or outcome == "critical_success":
        Il chiavistello scatta. -> cantina
    - else:
        Il grimaldello si piega. -> corridoio
    }

=== cantina ===
Hai trovato le provviste. Missione compiuta.
-> END

=== corridoio ===
Torni sui tuoi passi. Ci riproverai domani.
-> END
