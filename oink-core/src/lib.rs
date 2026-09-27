//! oink-core: gamebook engine core.
//!
//! Story flow is driven by Ink (via `bladeink`), while the rulebook
//! (`oink-rulebook`) owns checks, modifiers, resources, and character state.

pub mod data;
pub mod engine;
mod external;

pub use engine::{CheckRecord, Choice, Engine, EngineError, Event};
pub use oink_rulebook;
