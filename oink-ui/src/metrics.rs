//! Fixed screen measurements for one device.
//!
//! A port defines its own values for the panel in use. The layout code reads
//! the values and never hard-codes a screen size.

/// The pixel measurements of one screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub width: u32,
    pub height: u32,
    pub top_bar_height: u32,
    pub bottom_bar_height: u32,
}

impl Metrics {
    pub const fn new(width: u32, height: u32, top_bar_height: u32, bottom_bar_height: u32) -> Self {
        Self {
            width,
            height,
            top_bar_height,
            bottom_bar_height,
        }
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
}
