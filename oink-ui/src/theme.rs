//! The Gray4 palette used by the reader.

use embedded_graphics::pixelcolor::Gray4;

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

impl Default for Theme {
    fn default() -> Self {
        Self {
            paper: Gray4::new(15),
            ink: Gray4::new(0),
            dim: Gray4::new(7),
            accent: Gray4::new(4),
        }
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
}
