//! The numbered save slots of one bundle.
//!
//! A save is one JSON file per slot in a `saves` directory beside the bundle.
//! The engine writes the file contents; this module owns the paths, the slot
//! range, and the file access.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The directory that holds the slot files.
pub const DIR: &str = "saves";

/// The numbered save slots of a bundle.
#[derive(Debug, Clone)]
pub struct Saves {
    dir: PathBuf,
    slots: usize,
}

impl Saves {
    /// Slots in `dir`. A count below one becomes one.
    pub fn new(dir: PathBuf, slots: usize) -> Self {
        Self {
            dir,
            slots: slots.max(1),
        }
    }

    /// The slots of `bundle_dir`, stored in its `saves` directory.
    pub fn beside(bundle_dir: &Path, slots: usize) -> Self {
        Self::new(bundle_dir.join(DIR), slots)
    }

    /// How many slots the player offers.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// The file of one slot. The first slot is 1, not 0.
    pub fn path(&self, slot: usize) -> Result<PathBuf, String> {
        if slot == 0 || slot > self.slots {
            return Err(format!("slot {slot} is outside 1 to {}", self.slots));
        }
        Ok(self.dir.join(format!("slot-{slot}.json")))
    }

    /// True when the slot holds a save file.
    pub fn exists(&self, slot: usize) -> bool {
        self.path(slot).map(|path| path.is_file()).unwrap_or(false)
    }

    /// Read a slot. `None` means the slot is empty.
    pub fn read(&self, slot: usize) -> Result<Option<String>, String> {
        let path = self.path(slot)?;
        match fs::read_to_string(&path) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    /// Write a slot, creating the saves directory when it is missing.
    pub fn write(&self, slot: usize, text: &str) -> Result<(), String> {
        let path = self.path(slot)?;
        fs::create_dir_all(&self.dir)
            .map_err(|error| format!("cannot create {}: {error}", self.dir.display()))?;
        fs::write(&path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        std::env::temp_dir().join(format!(
            "oink-saves-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ))
    }

    #[test]
    fn a_slot_round_trips_and_an_empty_slot_reads_none() {
        let saves = Saves::new(temp_dir(), 2);
        assert_eq!(saves.slots(), 2);
        assert!(!saves.exists(1));
        assert_eq!(saves.read(1).unwrap(), None);

        saves.write(1, "{\"version\":1}").unwrap();
        assert!(saves.exists(1));
        assert_eq!(saves.read(1).unwrap().as_deref(), Some("{\"version\":1}"));
        assert!(!saves.exists(2));

        let _ = fs::remove_dir_all(saves.dir);
    }

    #[test]
    fn slots_outside_the_range_are_errors() {
        let saves = Saves::new(temp_dir(), 2);
        assert!(saves.path(0).unwrap_err().contains("outside"));
        assert!(saves.path(3).unwrap_err().contains("outside"));
        assert!(saves.write(3, "x").unwrap_err().contains("outside"));
        let _ = fs::remove_dir_all(saves.dir);
    }

    #[test]
    fn a_slot_file_is_named_after_the_slot() {
        let saves = Saves::new(PathBuf::from("/tmp/game/saves"), 3);
        assert_eq!(
            saves.path(2).unwrap(),
            PathBuf::from("/tmp/game/saves/slot-2.json")
        );
    }
}
