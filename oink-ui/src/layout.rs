//! The three fixed zones of the reader: top bar, center, bottom bar.

use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::primitives::Rectangle;

use crate::config::PageLayout;
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

/// Which zones changed between two frames, for e-ink partial refreshes.
///
/// A full refresh repaints every zone; a partial refresh repaints only the
/// dirty ones. [`DirtyZones::between`] reports the zones whose geometry
/// changed, which covers a page switch. The host adds the zones whose
/// content changed since the last draw: the center when the page turned,
/// a bar when its values or focus moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirtyZones {
    pub top: bool,
    pub center: bool,
    pub bottom: bool,
}

impl DirtyZones {
    pub const NONE: Self = Self {
        top: false,
        center: false,
        bottom: false,
    };
    pub const ALL: Self = Self {
        top: true,
        center: true,
        bottom: true,
    };

    /// True when no zone changed.
    pub const fn is_empty(self) -> bool {
        !self.top && !self.center && !self.bottom
    }

    /// The union of two change sets.
    pub const fn or(self, other: Self) -> Self {
        Self {
            top: self.top || other.top,
            center: self.center || other.center,
            bottom: self.bottom || other.bottom,
        }
    }

    /// The zones whose geometry changed between the two layouts.
    pub fn between(previous: Zones, current: Zones) -> Self {
        Self {
            top: previous.top != current.top,
            center: previous.center != current.center,
            bottom: previous.bottom != current.bottom,
        }
    }
}

impl PageLayout {
    /// The bar heights for this page on `base`, in pixels.
    ///
    /// A configured height wins, an unconfigured bar keeps the device
    /// default, and a bar the page removes gets height 0.
    pub fn bar_heights(&self, base: Metrics) -> (u32, u32) {
        let top = match &self.top_bar {
            Some(bar) => bar.height.unwrap_or(base.top_bar_height),
            None => 0,
        };
        let bottom = match &self.bottom_bar {
            Some(bar) => bar.height.unwrap_or(base.bottom_bar_height),
            None => 0,
        };
        (top, bottom)
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

    #[test]
    fn a_page_without_bars_gives_the_center_the_whole_screen() {
        let zones = Metrics::new(960, 540, 0, 0).zones();
        assert_eq!(zones.top.size.height, 0);
        assert_eq!(zones.center.top_left.y, 0);
        assert_eq!(zones.center.size, Size::new(960, 540));
        assert_eq!(zones.bottom.size.height, 0);
    }

    #[test]
    fn page_bar_heights_follow_the_resolved_layout() {
        let base = Metrics::new(960, 540, 64, 72);

        // The base layout keeps the device defaults.
        let base_page = PageLayout {
            top_bar: Some(crate::config::TopBar::default()),
            bottom_bar: Some(crate::config::BottomBar::default()),
            warnings: Vec::new(),
        };
        assert_eq!(base_page.bar_heights(base), (64, 72));

        // A configured height wins over the device default.
        let configured = PageLayout {
            top_bar: Some(crate::config::TopBar {
                height: Some(96),
                ..crate::config::TopBar::default()
            }),
            bottom_bar: None,
            warnings: Vec::new(),
        };
        assert_eq!(configured.bar_heights(base), (96, 0));
    }

    #[test]
    fn the_zone_diff_reports_only_the_changed_zones() {
        let before = Metrics::new(960, 540, 64, 72).zones();
        assert!(DirtyZones::between(before, before).is_empty());

        // A dialog page: no bottom bar, so the center grows downward.
        let after = Metrics::new(960, 540, 64, 0).zones();
        let dirty = DirtyZones::between(before, after);
        assert!(!dirty.top);
        assert!(dirty.center);
        assert!(dirty.bottom);

        assert_eq!(DirtyZones::NONE.or(DirtyZones::ALL), DirtyZones::ALL);
        assert!(!DirtyZones::ALL.is_empty());
    }
}
