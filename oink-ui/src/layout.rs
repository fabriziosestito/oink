//! The three fixed zones of the reader: top bar, center, bottom bar.

use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::primitives::Rectangle;

use crate::metrics::Metrics;

/// The top bar, the center page, and the bottom bar.
///
/// The rectangles tile the screen from top to bottom. The center rectangle
/// has zero height when the bars do not fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zones {
    pub top: Rectangle,
    pub center: Rectangle,
    pub bottom: Rectangle,
}

impl Zones {
    /// The three zones for `metrics`.
    ///
    /// The zones never overlap. When the configured bars are taller than the
    /// screen, the bottom bar shrinks and the center becomes zero height.
    /// [`Metrics::is_valid`] reports the invalid configuration to the host.
    pub fn from_metrics(metrics: Metrics) -> Self {
        let top_height = metrics.top_bar_height.min(metrics.height);
        let bottom_height = metrics
            .bottom_bar_height
            .min(metrics.height.saturating_sub(top_height));
        let bottom_y = metrics.height.saturating_sub(bottom_height);
        Self {
            top: Rectangle::new(Point::zero(), Size::new(metrics.width, top_height)),
            center: Rectangle::new(
                Point::new(0, top_height as i32),
                Size::new(metrics.width, bottom_y.saturating_sub(top_height)),
            ),
            bottom: Rectangle::new(
                Point::new(0, bottom_y as i32),
                Size::new(metrics.width, bottom_height),
            ),
        }
    }

    pub fn iter(self) -> impl Iterator<Item = Rectangle> {
        [self.top, self.center, self.bottom].into_iter()
    }
}

impl Metrics {
    /// The three zones for this screen.
    pub fn zones(self) -> Zones {
        Zones::from_metrics(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zones_tile_the_screen() {
        let metrics = Metrics::new(960, 540, 64, 72);
        let zones = metrics.zones();
        assert_eq!(zones.top.top_left, Point::zero());
        assert_eq!(zones.top.size, Size::new(960, 64));
        assert_eq!(zones.center.top_left, Point::new(0, 64));
        assert_eq!(zones.center.size, Size::new(960, 404));
        assert_eq!(zones.bottom.top_left, Point::new(0, 468));
        assert_eq!(zones.bottom.size, Size::new(960, 72));
        assert_eq!(
            zones.top.size.height + zones.center.size.height + zones.bottom.size.height,
            540
        );
    }

    #[test]
    fn the_zones_do_not_overlap() {
        let zones = Metrics::new(320, 200, 20, 24).zones();
        assert!(zones.top.top_left.y + zones.top.size.height as i32 <= zones.center.top_left.y);
        assert!(
            zones.center.top_left.y + zones.center.size.height as i32 <= zones.bottom.top_left.y
        );
    }

    #[test]
    fn oversized_bars_never_overlap() {
        let zones = Metrics::new(320, 200, 120, 120).zones();
        assert_eq!(zones.top.size.height, 120);
        assert_eq!(zones.center.size.height, 0);
        assert_eq!(zones.bottom.top_left.y, 120);
        assert_eq!(zones.bottom.size.height, 80);
        assert_eq!(
            zones.top.size.height + zones.center.size.height + zones.bottom.size.height,
            200
        );
    }
}
