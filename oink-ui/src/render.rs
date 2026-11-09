//! Drawing for the reader.
//!
//! Phase 1 draws the empty page: paper, then one rule under the top bar and
//! one above the bottom bar. Later phases fill the zones with text.

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::primitives::Rectangle;

use crate::layout::Zones;
use crate::metrics::Metrics;
use crate::theme::Theme;

/// Draw the empty reader on `target`.
pub fn draw_empty<D>(target: &mut D, metrics: Metrics, theme: Theme) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let zones = Zones::from_metrics(metrics);
    target.clear(theme.paper)?;

    if metrics.width > 0 && zones.top.size.height > 0 {
        let y = zones.top.size.height as i32 - 1;
        let rule = Rectangle::new(Point::new(0, y), Size::new(metrics.width, 1));
        target.fill_solid(&rule, theme.dim)?;
    }

    if metrics.width > 0 && zones.bottom.size.height > 0 {
        let rule = Rectangle::new(zones.bottom.top_left, Size::new(metrics.width, 1));
        target.fill_solid(&rule, theme.dim)?;
    }

    Ok(())
}
