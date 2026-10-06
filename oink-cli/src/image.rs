//! Pictures in the terminal through the Kitty, iTerm2, or Sixel graphics
//! protocol. Terminals without one of them get no pictures.

use std::error::Error;
use std::path::Path;

use viuer::{Config, KittySupport};

pub struct Images {
    enabled: bool,
}

impl Images {
    /// Ask the terminal once which protocol it speaks. Call this only when
    /// stdout is a terminal: the probe writes escape codes and reads the reply.
    pub fn detect() -> Self {
        let enabled = !matches!(viuer::get_kitty_support(), KittySupport::None)
            || viuer::is_iterm_supported()
            || viuer::is_sixel_supported();
        Self { enabled }
    }

    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Show a PNG `columns` cells wide at the cursor and leave the cursor
    /// below it. Does nothing when the terminal has no graphics protocol.
    pub fn show(&self, path: &Path, columns: u32) -> Result<(), Box<dyn Error>> {
        if !self.enabled {
            return Ok(());
        }
        let picture = image::open(path)?;
        let config = Config {
            absolute_offset: false,
            width: Some(columns),
            ..Default::default()
        };
        viuer::print(&picture, &config)?;
        Ok(())
    }
}
