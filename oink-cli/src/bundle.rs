//! A game bundle: one directory with `config.yaml`, `rulebook.yaml`, and
//! `story.ink` (or a precompiled `story.ink.json`). A `cover.png` is optional.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const CONFIG: &str = "config.yaml";
pub const RULEBOOK: &str = "rulebook.yaml";
pub const STORY_INK: &str = "story.ink";
pub const STORY_JSON: &str = "story.ink.json";
pub const COVER: &str = "cover.png";

/// The story source, as found in the bundle.
#[derive(Debug)]
pub enum Story {
    /// Ink source, compiled when the engine loads it.
    Ink(String),
    /// Precompiled ink JSON, loaded as is.
    Json(String),
}

#[derive(Debug)]
pub struct Bundle {
    pub dir: PathBuf,
    pub config: String,
    pub rulebook: String,
    pub story: Story,
}

#[derive(Debug)]
pub enum BundleError {
    NotADirectory(PathBuf),
    Read { path: PathBuf, source: io::Error },
    MissingStory(PathBuf),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleError::NotADirectory(path) => {
                write!(f, "{} is not a directory", path.display())
            }
            BundleError::Read { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())
            }
            BundleError::MissingStory(dir) => write!(
                f,
                "{} has no {STORY_INK} and no {STORY_JSON}",
                dir.display()
            ),
        }
    }
}

impl std::error::Error for BundleError {}

impl Bundle {
    /// Read the three bundle files from `dir`. The Ink source wins when
    /// both the source and the precompiled JSON exist.
    pub fn open(dir: &Path) -> Result<Self, BundleError> {
        if !dir.is_dir() {
            return Err(BundleError::NotADirectory(dir.to_path_buf()));
        }
        let config = read(dir, CONFIG)?;
        let rulebook = read(dir, RULEBOOK)?;
        let story = if dir.join(STORY_INK).is_file() {
            Story::Ink(read(dir, STORY_INK)?)
        } else if dir.join(STORY_JSON).is_file() {
            Story::Json(read(dir, STORY_JSON)?)
        } else {
            return Err(BundleError::MissingStory(dir.to_path_buf()));
        };
        Ok(Self {
            dir: dir.to_path_buf(),
            config,
            rulebook,
            story,
        })
    }

    /// The cover picture, when the bundle ships one.
    pub fn cover(&self) -> Option<PathBuf> {
        let path = self.dir.join(COVER);
        path.is_file().then_some(path)
    }
}

fn read(dir: &Path, name: &str) -> Result<String, BundleError> {
    let path = dir.join(name);
    fs::read_to_string(&path).map_err(|source| BundleError::Read { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("oink-bundle-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn opens_a_directory_with_ink_source() {
        let dir = temp_dir("ink");
        fs::write(dir.join(CONFIG), "title: T\n").unwrap();
        fs::write(dir.join(RULEBOOK), "resources: {}\n").unwrap();
        fs::write(dir.join(STORY_INK), "Hi.\n-> END\n").unwrap();
        fs::write(dir.join(STORY_JSON), "{}").unwrap();

        let bundle = Bundle::open(&dir).unwrap();
        assert_eq!(bundle.config, "title: T\n");
        assert!(matches!(bundle.story, Story::Ink(ref source) if source.starts_with("Hi.")));
        assert!(bundle.cover().is_none());

        fs::write(dir.join(COVER), b"not really a png").unwrap();
        assert_eq!(bundle.cover(), Some(dir.join(COVER)));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn falls_back_to_precompiled_json() {
        let dir = temp_dir("json");
        fs::write(dir.join(CONFIG), "title: T\n").unwrap();
        fs::write(dir.join(RULEBOOK), "resources: {}\n").unwrap();
        fs::write(dir.join(STORY_JSON), "{}").unwrap();

        let bundle = Bundle::open(&dir).unwrap();
        assert!(matches!(bundle.story, Story::Json(ref json) if json == "{}"));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reports_what_is_missing() {
        let dir = temp_dir("missing");
        let error = Bundle::open(&dir.join("nope")).unwrap_err();
        assert!(error.to_string().ends_with("is not a directory"), "{error}");

        let error = Bundle::open(&dir).unwrap_err();
        assert!(error.to_string().contains(CONFIG), "{error}");

        fs::write(dir.join(CONFIG), "title: T\n").unwrap();
        fs::write(dir.join(RULEBOOK), "resources: {}\n").unwrap();
        let error = Bundle::open(&dir).unwrap_err();
        assert!(matches!(error, BundleError::MissingStory(_)), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }
}
