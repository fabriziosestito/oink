//! Dice sources for checks.

/// A source of d6 rolls.
pub trait Dice {
    fn roll_d6(&mut self) -> u8;
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
}

impl Dice for SeededDice {
    fn roll_d6(&mut self) -> u8 {
        (self.next_u64() % 6 + 1) as u8
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
    fn roll_d6(&mut self) -> u8 {
        self.inner.roll_d6()
    }
}

/// A fixed sequence of rolls for tests.
#[cfg(test)]
#[derive(Debug)]
pub(crate) struct ScriptedDice {
    rolls: Vec<u8>,
    index: usize,
}

#[cfg(test)]
impl ScriptedDice {
    pub(crate) fn new(rolls: &[u8]) -> Self {
        Self {
            rolls: rolls.to_vec(),
            index: 0,
        }
    }
}

#[cfg(test)]
impl Dice for ScriptedDice {
    fn roll_d6(&mut self) -> u8 {
        let roll = self.rolls.get(self.index).copied().unwrap_or(1);
        self.index += 1;
        roll
    }
}
