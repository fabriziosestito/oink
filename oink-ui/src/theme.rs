//! The Gray4 palette used by the reader.

use embedded_graphics::pixelcolor::Gray4;

use crate::config::EinkTheme;

/// The four tones the reader draws.
///
/// The default theme is ink on paper. A 1-bit panel maps `dim` and `accent`
/// to one of the two available levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub paper: Gray4,
    pub ink: Gray4,
    pub dim: Gray4,
    pub accent: Gray4,
}

impl Theme {
    /// Build the palette from the `theme.eink` block of `ui.yaml`.
    pub fn from_config(config: &EinkTheme) -> Self {
        Self {
            paper: Gray4::new(config.paper),
            ink: Gray4::new(config.ink),
            dim: Gray4::new(config.dim),
            accent: Gray4::new(config.accent),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&EinkTheme::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_graphics::pixelcolor::GrayColor;

    #[test]
    fn the_default_theme_runs_from_paper_to_ink() {
        let theme = Theme::default();
        assert!(theme.paper.luma() > theme.dim.luma());
        assert!(theme.dim.luma() > theme.accent.luma());
        assert!(theme.accent.luma() > theme.ink.luma());
    }

    #[test]
    fn the_theme_follows_the_config() {
        let config = EinkTheme {
            paper: 14,
            ink: 1,
            dim: 6,
            accent: 3,
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.paper.luma(), 14);
        assert_eq!(theme.ink.luma(), 1);
        assert_eq!(theme.dim.luma(), 6);
        assert_eq!(theme.accent.luma(), 3);
    }
}
