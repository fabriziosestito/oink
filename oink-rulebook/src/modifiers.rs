//! Modifier stacking and breakdowns.
//!
//! Every contribution keeps its source, so the story can show why a check
//! changed. Sources stack additively and each source counts once.

use crate::model::Modifiers;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreakdownEntry {
    pub source: String,
    pub value: i32,
}

/// An ordered list of modifier contributions with a running total.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Breakdown {
    pub entries: Vec<BreakdownEntry>,
}

impl Breakdown {
    pub fn push(&mut self, source: impl Into<String>, value: i32) {
        if value != 0 {
            self.entries.push(BreakdownEntry {
                source: source.into(),
                value,
            });
        }
    }

    pub fn total(&self) -> i32 {
        self.entries.iter().map(|entry| entry.value).sum()
    }
}

impl fmt::Display for Breakdown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .entries
            .iter()
            .map(|entry| format!("{} {:+}", entry.source, entry.value))
            .collect();
        write!(f, "{} = {:+}", parts.join(", "), self.total())
    }
}

pub(crate) fn apply_entity(
    breakdown: &mut Breakdown,
    kind: &str,
    id: &str,
    modifiers: &Modifiers,
    characteristic: Option<&str>,
    ability: &str,
    check_tags: &BTreeSet<String>,
) {
    if let Some(characteristic) = characteristic {
        if let Some(value) = modifiers.characteristics.get(characteristic) {
            breakdown.push(format!("{kind} {id} ({characteristic})"), *value);
        }
    }
    if let Some(value) = modifiers.abilities.get(ability) {
        breakdown.push(format!("{kind} {id} ({ability})"), *value);
    }
    for (tag, value) in &modifiers.tags {
        if check_tags.contains(tag) {
            breakdown.push(format!("{kind} {id} (tag {tag})"), *value);
        }
    }
}
