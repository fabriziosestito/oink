//! Static game data: engine config and the loaded rulebook.

use oink_rulebook::{LoadError, Rulebook};
use serde::Deserialize;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub title: String,
}

/// The engine config plus the loaded rulebook, shared with bound functions.
#[derive(Debug, Clone)]
pub struct GameData {
    pub config: Config,
    pub rulebook: Rc<Rulebook>,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum DataError {
    Config(serde_yaml::Error),
    Rulebook(LoadError),
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::Config(error) => write!(f, "failed to parse config YAML: {error}"),
            DataError::Rulebook(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DataError {}

impl From<serde_yaml::Error> for DataError {
    fn from(error: serde_yaml::Error) -> Self {
        DataError::Config(error)
    }
}

impl From<LoadError> for DataError {
    fn from(error: LoadError) -> Self {
        DataError::Rulebook(error)
    }
}

impl GameData {
    pub fn from_yaml(config: &str, rulebook: &str) -> Result<Self, DataError> {
        let config: Config = serde_yaml::from_str(config)?;
        let loaded = Rulebook::load(rulebook)?;
        Ok(Self {
            config,
            rulebook: Rc::new(loaded.rulebook),
            warnings: loaded.warnings,
        })
    }
}
