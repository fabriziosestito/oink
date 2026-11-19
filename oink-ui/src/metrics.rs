//! Fixed screen measurements for one device.
//!
//! A port defines its own values for the panel in use. The layout code reads
//! the values and never hard-codes a screen size.

use embedded_graphics::mono_font::MonoFont;
use profont::{PROFONT_10_POINT, PROFONT_12_POINT, PROFONT_9_POINT};

/// The body text size, mapped to one ProFont bitmap face.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FontSize {
    /// The 9 point face, 6x11 pixels.
    Small,
    /// The 10 point face, 6x12 pixels.
    #[default]
    Regular,
    /// The 12 point face, 7x15 pixels.
    Large,
}

impl FontSize {
    /// The ProFont face for this size.
    pub const fn font(self) -> &'static MonoFont<'static> {
        match self {
            Self::Small => &PROFONT_9_POINT,
            Self::Regular => &PROFONT_10_POINT,
            Self::Large => &PROFONT_12_POINT,
        }
    }

    /// The width of one character cell, in pixels.
    pub const fn character_width(self) -> u32 {
        self.font().character_size.width
    }

    /// The height of one character cell, in pixels.
    pub const fn character_height(self) -> u32 {
        self.font().character_size.height
    }

    /// The distance between two text rows, in pixels.
    pub const fn line_height(self) -> u32 {
        self.font().character_size.height + 3
    }
}

/// The pixel measurements of one screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub width: u32,
    pub height: u32,
    pub top_bar_height: u32,
    pub bottom_bar_height: u32,
    pub font: FontSize,
}

impl Metrics {
    pub const fn new(width: u32, height: u32, top_bar_height: u32, bottom_bar_height: u32) -> Self {
        Self {
            width,
            height,
            top_bar_height,
            bottom_bar_height,
            font: FontSize::Regular,
        }
    }

    /// The same screen with another body text size.
    pub const fn with_font(mut self, font: FontSize) -> Self {
        self.font = font;
        self
    }

    /// The same screen with other bar heights, in pixels.
    pub const fn with_bars(mut self, top_bar_height: u32, bottom_bar_height: u32) -> Self {
        self.top_bar_height = top_bar_height;
        self.bottom_bar_height = bottom_bar_height;
        self
    }

    /// The height left between the two bars.
    pub const fn center_height(self) -> u32 {
        self.height
            .saturating_sub(self.top_bar_height)
            .saturating_sub(self.bottom_bar_height)
    }

    /// True when the screen has a positive size and the bars fit.
    pub const fn is_valid(self) -> bool {
        self.width > 0
            && self.height > 0
            && self.top_bar_height.saturating_add(self.bottom_bar_height) < self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_center_takes_the_height_between_the_bars() {
        let metrics = Metrics::new(960, 540, 64, 72);
        assert_eq!(metrics.center_height(), 404);
    }

    #[test]
    fn bars_taller_than_the_screen_leave_no_center() {
        let metrics = Metrics::new(320, 200, 120, 120);
        assert_eq!(metrics.center_height(), 0);
        assert!(!metrics.is_valid());
    }

    #[test]
    fn a_screen_with_room_for_the_bars_is_valid() {
        let metrics = Metrics::new(960, 540, 64, 72);
        assert!(metrics.is_valid());
    }

    #[test]
    fn the_font_size_sets_the_cell_and_row_size() {
        assert_eq!(FontSize::Small.character_width(), 6);
        assert_eq!(FontSize::Small.character_height(), 11);
        assert_eq!(FontSize::Small.line_height(), 14);
        assert_eq!(FontSize::Regular.character_width(), 6);
        assert_eq!(FontSize::Regular.character_height(), 12);
        assert_eq!(FontSize::Regular.line_height(), 15);
        assert_eq!(FontSize::Large.character_width(), 7);
        assert_eq!(FontSize::Large.character_height(), 15);
        assert_eq!(FontSize::Large.line_height(), 18);
    }

    #[test]
    fn the_default_font_is_regular() {
        let metrics = Metrics::new(320, 240, 28, 36).with_font(FontSize::Large);
        assert_eq!(metrics.font, FontSize::Large);
        assert_eq!(Metrics::new(320, 240, 28, 36).font, FontSize::Regular);
    }

    #[test]
    fn a_page_can_remove_or_resize_the_bars() {
        let base = Metrics::new(960, 540, 64, 72);

        // A page without bars: the center takes the whole screen.
        let full = base.with_bars(0, 0);
        assert_eq!(full.center_height(), 540);
        assert!(full.is_valid());

        // A page with a taller top bar.
        let taller = base.with_bars(96, 72);
        assert_eq!(taller.center_height(), 372);
        assert!(taller.is_valid());
    }
}
