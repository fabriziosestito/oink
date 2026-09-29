//! Dice sources and notation parsing for checks.
//!
//! Notation selects the pool, such as `2d6`, `1d20`, `2d20kh1`, or `d%`.
//! The engine supports d2, d4, d6, d8, d10, d12, d20, and d%.

use crate::model::{DicePool, Die, Keep};
use std::fmt;

/// A source of dice rolls. Returns values in `1..=sides`.
pub trait Dice {
    fn roll(&mut self, sides: u16) -> u16;
}

/// Error from parsing a dice notation string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DicePoolError(pub String);

impl fmt::Display for DicePoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DicePoolError {}

/// Parse a notation string into a [`DicePool`].
///
/// Accepts `2d6`, `1d20`, `1d4`, `1d2`, `2d10`, `2d20kh1`, `2d20kl1`,
/// `4d6kh3`, `d%`, `d100`, and `1d100`. The canonical percentile form is
/// `d%`. A plain `2d10` stays a sum. Keep notation (`kh`/`kl`) does not
/// apply to d%.
pub fn parse_notation(notation: &str) -> Result<DicePool, DicePoolError> {
    let input = notation.trim();
    if input.is_empty() {
        return Err(DicePoolError("empty dice notation".to_string()));
    }
    let lower = input.to_ascii_lowercase();

    let d_pos = lower
        .find('d')
        .ok_or_else(|| DicePoolError(format!("invalid dice notation `{notation}`: missing `d`")))?;
    let (count_str, rest) = (&lower[..d_pos], &lower[d_pos + 1..]);

    let count_opt: Option<u16> = if count_str.is_empty() {
        None
    } else {
        let count: u16 = count_str.parse().map_err(|_| {
            DicePoolError(format!(
                "invalid dice notation `{notation}`: bad count `{count_str}`"
            ))
        })?;
        if !(1..=10).contains(&count) {
            return Err(DicePoolError(format!(
                "invalid dice notation `{notation}`: count must be 1..=10"
            )));
        }
        Some(count)
    };

    if rest.is_empty() {
        return Err(DicePoolError(format!(
            "invalid dice notation `{notation}`: missing die size"
        )));
    }

    let (die, after_face) = if let Some(after) = rest.strip_prefix('%') {
        (Die::Percentile, after)
    } else {
        let digit_len = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digit_len == 0 {
            return Err(DicePoolError(format!(
                "invalid dice notation `{notation}`: missing die size"
            )));
        }
        let (face_str, after) = rest.split_at(digit_len);
        let faces: u16 = face_str.parse().map_err(|_| {
            DicePoolError(format!(
                "invalid dice notation `{notation}`: bad face `{face_str}`"
            ))
        })?;
        if faces == 100 {
            (Die::Percentile, after)
        } else if let Some(die) = Die::from_faces(faces) {
            (die, after)
        } else {
            return Err(DicePoolError(format!(
                "invalid dice notation `{notation}`: unsupported die d{faces}"
            )));
        }
    };

    let keep = if after_face.is_empty() {
        Keep::All
    } else if let Some(num) = after_face.strip_prefix("kh") {
        let n: u16 = num.parse().map_err(|_| {
            DicePoolError(format!(
                "invalid dice notation `{notation}`: bad keep count"
            ))
        })?;
        Keep::Highest(n)
    } else if let Some(num) = after_face.strip_prefix("kl") {
        let n: u16 = num.parse().map_err(|_| {
            DicePoolError(format!(
                "invalid dice notation `{notation}`: bad keep count"
            ))
        })?;
        Keep::Lowest(n)
    } else {
        return Err(DicePoolError(format!(
            "invalid dice notation `{notation}`: bad keep clause"
        )));
    };

    if die == Die::Percentile {
        if let Some(count) = count_opt {
            if count != 1 {
                return Err(DicePoolError(format!(
                    "invalid dice notation `{notation}`: d% holds one die"
                )));
            }
        }
        if keep != Keep::All {
            return Err(DicePoolError(format!(
                "invalid dice notation `{notation}`: keep does not apply to d%"
            )));
        }
        return Ok(DicePool {
            count: 1,
            die,
            keep: Keep::All,
        });
    }

    let count = count_opt.ok_or_else(|| {
        DicePoolError(format!("invalid dice notation `{notation}`: missing count"))
    })?;

    match keep {
        Keep::All => {}
        Keep::Highest(n) | Keep::Lowest(n) => {
            if n == 0 || n > count {
                return Err(DicePoolError(format!(
                    "invalid dice notation `{notation}`: keep must be 1..=count"
                )));
            }
        }
    }

    Ok(DicePool { count, die, keep })
}

/// Deterministic dice built on SplitMix64. Use them for tests, replays, and
/// saves that must resume the same sequence.
#[derive(Debug, Clone)]
pub struct SeededDice {
    state: u64,
}

impl SeededDice {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_bounded(&mut self, sides: u16) -> u16 {
        debug_assert!(sides > 0);
        if sides == 0 {
            return 0;
        }
        // Rejection sampling avoids modulo bias for most die sizes.
        let sides_u64 = u64::from(sides);
        let bound = u64::MAX - (u64::MAX % sides_u64);
        loop {
            let value = self.next_u64();
            if value < bound {
                return (value % sides_u64 + 1) as u16;
            }
        }
    }
}

impl Dice for SeededDice {
    fn roll(&mut self, sides: u16) -> u16 {
        self.next_bounded(sides)
    }
}

/// Dice seeded from the system clock and the process id.
#[derive(Debug)]
pub struct SystemDice {
    inner: SeededDice,
}

impl SystemDice {
    pub fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(0x5EED_5EED);
        let pid = u64::from(std::process::id());
        Self {
            inner: SeededDice::new(nanos ^ (pid << 32)),
        }
    }
}

impl Default for SystemDice {
    fn default() -> Self {
        Self::new()
    }
}

impl Dice for SystemDice {
    fn roll(&mut self, sides: u16) -> u16 {
        self.inner.roll(sides)
    }
}

/// A fixed sequence of rolls for tests.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ScriptedDice {
    rolls: Vec<u16>,
    index: usize,
}

#[cfg(test)]
impl ScriptedDice {
    pub(crate) fn new(rolls: &[u16]) -> Self {
        Self {
            rolls: rolls.to_vec(),
            index: 0,
        }
    }
}

#[cfg(test)]
impl Dice for ScriptedDice {
    fn roll(&mut self, sides: u16) -> u16 {
        let roll = self.rolls.get(self.index).copied().unwrap_or(1);
        self.index += 1;
        if sides == 0 {
            return 0;
        }
        if roll >= 1 && roll <= sides {
            roll
        } else {
            (roll.saturating_sub(1) % sides) + 1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Die, Keep};

    #[test]
    fn parses_supported_notations() {
        let pool = parse_notation("2d6").unwrap();
        assert_eq!(pool.count, 2);
        assert_eq!(pool.die, Die::D6);
        assert_eq!(pool.keep, Keep::All);

        let pool = parse_notation("1d20").unwrap();
        assert_eq!(pool.count, 1);
        assert_eq!(pool.die, Die::D20);

        let pool = parse_notation("2d20kh1").unwrap();
        assert_eq!(pool.keep, Keep::Highest(1));

        let pool = parse_notation("2d20kl1").unwrap();
        assert_eq!(pool.keep, Keep::Lowest(1));

        let pool = parse_notation("4d6kh3").unwrap();
        assert_eq!(pool.count, 4);
        assert_eq!(pool.keep, Keep::Highest(3));

        for notation in ["d%", "d100", "1d100", "1d%"] {
            let pool = parse_notation(notation).unwrap();
            assert_eq!(pool.die, Die::Percentile, "{notation}");
            assert_eq!(pool.count, 1, "{notation}");
            assert_eq!(pool.keep, Keep::All, "{notation}");
        }

        for notation in ["1d4", "1d2", "2d10", "2d8", "3d12"] {
            assert!(parse_notation(notation).is_ok(), "{notation}");
        }
    }

    #[test]
    fn rejects_bad_notations() {
        for notation in [
            "0d6",
            "2d0",
            "2d6kh",
            "2d6kh3",
            "2d6kl1kh1",
            "d6",
            "2d7",
            "d3",
            "2d%",
            "d%kh1",
            "",
            "2d",
            "d",
            "2d6kh0",
            "11d6",
            "2d1000",
        ] {
            assert!(parse_notation(notation).is_err(), "{notation}");
        }
    }

    #[test]
    fn seeded_runs_are_deterministic() {
        let mut first = SeededDice::new(42);
        let mut second = SeededDice::new(42);
        for sides in [6, 20, 10, 4] {
            for _ in 0..20 {
                assert_eq!(first.roll(sides), second.roll(sides));
            }
        }
        // Rolls stay in range for every supported size.
        let mut dice = SeededDice::new(7);
        for sides in [2, 4, 6, 8, 10, 12, 20] {
            for _ in 0..100 {
                let roll = dice.roll(sides);
                assert!((1..=sides).contains(&roll));
            }
        }
    }
}
