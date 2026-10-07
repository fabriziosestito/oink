//! Engine setup shared by the transcript and the full-screen player.

use std::error::Error;

use oink_core::data::GameData;
use oink_core::Engine;

use crate::bundle::{Bundle, Story};

/// The house line under every ending.
pub const THE_END: &str = "~ La tua vita e la tua missione terminano qui ~";

/// Parse the bundle, print its warnings to stderr, and build the engine.
pub fn load(bundle: &Bundle, seed: Option<u64>) -> Result<Engine, Box<dyn Error>> {
    let data = GameData::from_yaml(&bundle.config, &bundle.rulebook)?;
    for warning in &data.warnings {
        eprintln!("warning: {warning}");
    }
    let mut engine = match &bundle.story {
        Story::Ink(source) => Engine::new(source, data)?,
        Story::Json(json) => Engine::from_json(json, data)?,
    };
    if let Some(seed) = seed {
        engine.set_seed(seed);
    }
    Ok(engine)
}
