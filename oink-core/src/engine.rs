//! Engine: wraps the Ink runtime and player state.

use crate::data::GameData;
use bladeink::story::Story;
use bladeink::story_error::StoryError;
use bladeink_compiler::Compiler;
use std::collections::HashSet;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Choice {
    pub index: usize,
    pub text: String,
}

/// What the UI should present next.
#[derive(Debug, Clone)]
pub enum Event {
    /// Prose paragraphs followed by choices to pick from.
    Scene {
        text: Vec<String>,
        choices: Vec<Choice>,
    },
    /// The story reached an end. "La tua vita e la tua missione terminano qui."
    TheEnd { text: Vec<String> },
}

/// Errors from compiling the ink source or running the story.
#[derive(Debug)]
pub enum EngineError {
    Compile(String),
    Story(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::Compile(msg) => write!(f, "failed to compile story: {msg}"),
            EngineError::Story(msg) => write!(f, "story error: {msg}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<StoryError> for EngineError {
    fn from(err: StoryError) -> Self {
        EngineError::Story(err.to_string())
    }
}

pub struct Engine {
    story: Story,
    pub data: GameData,
    pub inventory: HashSet<String>,
    pub perks: HashSet<String>,
}

impl Engine {
    /// Compile ink source and load it into a new engine.
    pub fn new(ink_source: &str, data: GameData) -> Result<Self, EngineError> {
        let json = Compiler::new()
            .compile(ink_source)
            .map_err(|err| EngineError::Compile(err.to_string()))?;
        Self::from_json(&json, data)
    }

    /// Load a story from ink JSON, e.g. a precompiled `main.ink.json`.
    ///
    /// On device this is the entry point: compile at build time, flash the
    /// JSON, skip the compiler.
    pub fn from_json(story_json: &str, data: GameData) -> Result<Self, EngineError> {
        let story = Story::new(story_json)?;
        let inventory = data.config.starting_inventory.iter().cloned().collect();
        Ok(Self {
            story,
            data,
            inventory,
            perks: HashSet::new(),
        })
    }

    /// Start the story and return the first event.
    pub fn start(&mut self) -> Result<Event, EngineError> {
        self.resume()
    }

    /// Pick a choice by index and continue.
    pub fn choose(&mut self, index: usize) -> Result<Event, EngineError> {
        self.story.choose_choice_index(index)?;
        self.resume()
    }

    fn resume(&mut self) -> Result<Event, EngineError> {
        let mut text = Vec::new();
        while self.story.can_continue() {
            let line = self.story.cont()?;
            let line = line.trim();
            if !line.is_empty() {
                text.push(line.to_string());
            }
        }
        let choices: Vec<Choice> = self
            .story
            .get_current_choices()
            .iter()
            .map(|choice| Choice {
                index: *choice.index.borrow(),
                text: choice.text.clone(),
            })
            .collect();
        if choices.is_empty() {
            Ok(Event::TheEnd { text })
        } else {
            Ok(Event::Scene { text, choices })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demo_engine() -> Engine {
        let ink = std::fs::read_to_string("../assets/story/main.ink").unwrap();
        let data = GameData::from_yaml(
            &std::fs::read_to_string("../assets/data/config.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/items.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/perks.yaml").unwrap(),
        )
        .unwrap();
        Engine::new(&ink, data).unwrap()
    }

    #[test]
    fn story_runs_and_ends() {
        let mut engine = demo_engine();
        assert!(engine.inventory.contains("sword"));

        let event = engine.start().unwrap();
        let Event::Scene { choices, .. } = event else {
            panic!("expected choices")
        };
        assert_eq!(choices.len(), 3);

        // Talk to the troll -> victory -> END
        let event = engine.choose(1).unwrap();
        assert!(matches!(event, Event::TheEnd { .. }));
    }

    #[test]
    fn precompiled_json_matches_source() {
        let ink = std::fs::read_to_string("../assets/story/main.ink").unwrap();
        let json = Compiler::new().compile(&ink).unwrap();
        let data = GameData::from_yaml(
            &std::fs::read_to_string("../assets/data/config.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/items.yaml").unwrap(),
            &std::fs::read_to_string("../assets/data/perks.yaml").unwrap(),
        )
        .unwrap();
        let mut engine = Engine::from_json(&json, data).unwrap();
        assert!(matches!(engine.start().unwrap(), Event::Scene { .. }));
    }
}
