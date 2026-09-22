//! oink-core: gamebook engine core.
//!
//! Story flow is driven by Ink (via `bladeink`), while items, perks and
//! engine configuration are defined in YAML and loaded at startup.

pub mod data;
pub mod engine;

pub use engine::{Choice, Engine, EngineError, Event};
