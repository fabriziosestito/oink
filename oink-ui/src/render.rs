//! Drawing for the reader.
//!
//! The empty page draws paper, one rule under the top bar, and one above the
//! bottom bar. The reader fills the top bar with resource and characteristic
//! values, the center with the current pager page and the page controls, and
//! the bottom bar with the sheet, inventory, log, and menu buttons.

use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::{Point, Size};
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Gray4;
use embedded_graphics::prelude::Drawable;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::text::{Baseline, Text};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::content::{Header, Line, Pager, Panel, Tone};
use crate::input::{Action, BarButton, Focus, UiState};
use crate::layout::Zones;
use crate::metrics::Metrics;
use crate::theme::Theme;

/// Pixels kept free around the center text.
pub const MARGIN: u32 = 4;
/// Rows reserved at the bottom of the center for the pager controls.
pub const CONTROL_ROWS: u32 = 1;

/// The text columns and rows that fit in the center zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSize {
    pub columns: usize,
    pub rows: usize,
}

/// How much text fits on one center page for `metrics`.
pub fn page_size(metrics: Metrics) -> PageSize {
    let center = Zones::from_metrics(metrics).center;
    let columns = center.size.width.saturating_sub(2 * MARGIN) as usize
        / metrics.font.character_width().max(1) as usize;
    let reserved = 2 * MARGIN + CONTROL_ROWS * metrics.font.line_height();
    let rows = center.size.height.saturating_sub(reserved) as usize
        / metrics.font.line_height().max(1) as usize;
    PageSize { columns, rows }
}

/// Where the reader drew its touch targets.
///
/// A host maps a touch point to a control with [`HitAreas::choice_at`],
/// [`HitAreas::previous_page_at`], [`HitAreas::next_page_at`],
/// [`HitAreas::button_at`], and [`HitAreas::back_at`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HitAreas {
    choices: Vec<(usize, Rectangle)>,
    previous_page: Option<Rectangle>,
    next_page: Option<Rectangle>,
    buttons: Vec<(BarButton, Rectangle)>,
    back: Option<Rectangle>,
}

impl HitAreas {
    /// The choice under `point`, if any.
    pub fn choice_at(&self, point: Point) -> Option<usize> {
        self.choices
            .iter()
            .find(|(_, area)| area.contains(point))
            .map(|(choice, _)| *choice)
    }

    /// True when `point` lands on the previous-page arrow.
    pub fn previous_page_at(&self, point: Point) -> bool {
        self.previous_page.is_some_and(|area| area.contains(point))
    }

    /// True when `point` lands on the next-page arrow.
    pub fn next_page_at(&self, point: Point) -> bool {
        self.next_page.is_some_and(|area| area.contains(point))
    }

    /// The bottom-bar button under `point`, if any.
    pub fn button_at(&self, point: Point) -> Option<BarButton> {
        self.buttons
            .iter()
            .find(|(_, area)| area.contains(point))
            .map(|(button, _)| *button)
    }

    /// True when `point` lands on the back label of a panel.
    pub fn back_at(&self, point: Point) -> bool {
        self.back.is_some_and(|area| area.contains(point))
    }

    /// The action for a touch at `point`, if any.
    ///
    /// Choices pick directly. The page arrows send [`Action::PageUp`] and
    /// [`Action::PageDown`], the bar buttons open their view, and the back
    /// label sends [`Action::Back`]. A point outside every control returns
    /// `None`.
    pub fn action_at(&self, point: Point) -> Option<Action> {
        if let Some(choice) = self.choice_at(point) {
            return Some(Action::Pick(choice));
        }
        if self.previous_page_at(point) {
            return Some(Action::PageUp);
        }
        if self.next_page_at(point) {
            return Some(Action::PageDown);
        }
        if let Some(button) = self.button_at(point) {
            return Some(Action::Open(button.view()));
        }
        if self.back_at(point) {
            return Some(Action::Back);
        }
        None
    }

    pub fn choices(&self) -> &[(usize, Rectangle)] {
        &self.choices
    }

    pub fn buttons(&self) -> &[(BarButton, Rectangle)] {
        &self.buttons
    }

    pub fn previous_page(&self) -> Option<Rectangle> {
        self.previous_page
    }

    pub fn next_page(&self) -> Option<Rectangle> {
        self.next_page
    }

    pub fn back(&self) -> Option<Rectangle> {
        self.back
    }
}

/// Draw the empty reader on `target`.
pub fn draw_empty<D>(target: &mut D, metrics: Metrics, theme: Theme) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    draw_chrome(target, metrics, theme)
}

/// Draw the current page of `pager` on `target`.
///
/// The caller builds the pager from a [`crate::content::Scene`] with
/// [`page_size`], keeps it across draws, and advances it on page actions.
/// The returned [`HitAreas`] holds the choice rows and the page arrows.
pub fn draw_page<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    pager: &Pager,
) -> Result<HitAreas, D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    draw_chrome(target, metrics, theme)?;
    draw_pager(target, metrics, theme, pager)
}

/// Draw the whole reader: the top values, the current page, and the buttons.
///
/// The caller keeps the pager and the UI state across draws and advances them
/// on page and focus actions. The returned [`HitAreas`] covers the choices,
/// the page arrows, and the bottom buttons.
pub fn draw_reader<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    pager: &Pager,
    header: &Header,
    state: &UiState,
) -> Result<HitAreas, D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let mut areas = draw_page(target, metrics, theme, pager)?;
    draw_header(target, metrics, theme, header)?;
    areas.buttons = draw_buttons(target, metrics, theme, state)?;
    Ok(areas)
}

/// Draw one reader view over the whole screen.
///
/// The panel title takes the top bar, the paged body fills the center, and
/// the bottom bar shows how to leave the view. The caller opens a panel on
/// `View::Sheet`, `View::Inventory`, or `View::Log` and closes it on `Back`.
/// The returned [`HitAreas`] holds the page arrows and the back label.
pub fn draw_panel<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    panel: &Panel,
    pager: &Pager,
) -> Result<HitAreas, D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    draw_chrome(target, metrics, theme)?;
    draw_title(target, metrics, theme, &panel.title)?;
    let mut areas = draw_pager(target, metrics, theme, pager)?;
    areas.back = draw_back(target, metrics, theme)?;
    Ok(areas)
}

fn draw_pager<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    pager: &Pager,
) -> Result<HitAreas, D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let zones = Zones::from_metrics(metrics);
    let size = page_size(metrics);
    if size.columns == 0 || size.rows == 0 {
        return Ok(HitAreas::default());
    }

    let left = zones.center.top_left.x + MARGIN as i32;
    let line_height = metrics.font.line_height() as i32;
    let row_width = zones.center.size.width.saturating_sub(2 * MARGIN);
    let mut choices: Vec<(usize, Rectangle)> = Vec::new();
    let mut y = zones.center.top_left.y + MARGIN as i32;
    for line in pager.current().iter().take(size.rows) {
        draw_line(target, line, left, y, metrics, theme)?;
        if let Some(choice) = line.choice {
            let row = Rectangle::new(
                Point::new(left, y),
                Size::new(row_width, line_height as u32),
            );
            match choices.last_mut() {
                Some((last, area)) if *last == choice => {
                    area.size.height += line_height as u32;
                }
                _ => choices.push((choice, row)),
            }
        }
        y += line_height;
    }

    let (previous_page, next_page) = draw_controls(target, metrics, theme, pager)?;
    Ok(HitAreas {
        choices,
        previous_page,
        next_page,
        ..HitAreas::default()
    })
}

fn draw_title<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    title: &str,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let top = Zones::from_metrics(metrics).top;
    let line_height = metrics.font.line_height();
    if top.size.height < line_height || title.is_empty() {
        return Ok(());
    }
    let columns = top.size.width.saturating_sub(2 * MARGIN) as usize
        / metrics.font.character_width().max(1) as usize;
    let title = fitted(title, columns);
    let width = title.width() * metrics.font.character_width() as usize;
    let x = top.top_left.x + (top.size.width as usize).saturating_sub(width) as i32 / 2;
    let y = top.top_left.y + top.size.height.saturating_sub(line_height) as i32 / 2;
    let style = MonoTextStyle::new(metrics.font.font(), theme.ink);
    Text::with_baseline(&title, Point::new(x, y), style, Baseline::Top).draw(target)?;
    Ok(())
}

fn draw_back<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
) -> Result<Option<Rectangle>, D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let bottom = Zones::from_metrics(metrics).bottom;
    let line_height = metrics.font.line_height();
    if bottom.size.height < line_height {
        return Ok(None);
    }
    let x = bottom.top_left.x + MARGIN as i32;
    let y = bottom.top_left.y + bottom.size.height.saturating_sub(line_height) as i32 / 2;
    let style = MonoTextStyle::new(metrics.font.font(), theme.dim);
    Text::with_baseline("Back", Point::new(x, y), style, Baseline::Top).draw(target)?;
    let width = "Back".width() as u32 * metrics.font.character_width();
    Ok(Some(Rectangle::new(
        Point::new(x, y),
        Size::new(width, line_height),
    )))
}

fn draw_header<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    header: &Header,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let zones = Zones::from_metrics(metrics);
    let top = zones.top;
    let line_height = metrics.font.line_height();
    if top.size.height < line_height || header.is_empty() {
        return Ok(());
    }
    let columns = top.size.width.saturating_sub(2 * MARGIN) as usize
        / metrics.font.character_width().max(1) as usize;
    let resources = header.resources_text();
    let characteristics = header.characteristics_text();
    let left = if characteristics.is_empty() {
        fitted(&resources, columns)
    } else {
        fitted(&resources, columns.saturating_sub(2) / 2)
    };
    let right = fitted(
        &characteristics,
        columns.saturating_sub(left.width()).saturating_sub(2),
    );

    let y = top.top_left.y + top.size.height.saturating_sub(line_height) as i32 / 2;
    let style = MonoTextStyle::new(metrics.font.font(), theme.dim);
    let cell = metrics.font.character_width() as i32;
    if !left.is_empty() {
        let x = top.top_left.x + MARGIN as i32;
        Text::with_baseline(&left, Point::new(x, y), style, Baseline::Top).draw(target)?;
    }
    if !right.is_empty() {
        let x =
            top.top_left.x + top.size.width as i32 - MARGIN as i32 - right.width() as i32 * cell;
        Text::with_baseline(&right, Point::new(x, y), style, Baseline::Top).draw(target)?;
    }
    Ok(())
}

fn draw_buttons<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    state: &UiState,
) -> Result<Vec<(BarButton, Rectangle)>, D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let bottom = Zones::from_metrics(metrics).bottom;
    let line_height = metrics.font.line_height();
    if bottom.size.height < line_height {
        return Ok(Vec::new());
    }
    let cell = metrics.font.character_width() as i32;
    let labels: Vec<(BarButton, String)> = BarButton::ALL
        .iter()
        .map(|button| (*button, format!(" {} ", button.label())))
        .collect();
    let columns: usize = labels.iter().map(|(_, label)| label.width() + 1).sum();
    let width = bottom.size.width.saturating_sub(2 * MARGIN);
    let fits = (columns as u32) <= width / metrics.font.character_width().max(1);
    let mut x = if fits {
        bottom.top_left.x + bottom.size.width as i32 - MARGIN as i32 - columns as i32 * cell
    } else {
        bottom.top_left.x + MARGIN as i32
    };
    let y = bottom.top_left.y + bottom.size.height.saturating_sub(line_height) as i32 / 2;
    let mut areas = Vec::with_capacity(labels.len());
    for (button, label) in labels {
        let selected = state.focus() == Focus::Bar && state.highlighted_button() == button;
        let label_width = (label.width() * metrics.font.character_width() as usize) as u32;
        if selected {
            let fill = Rectangle::new(Point::new(x, y), Size::new(label_width, line_height));
            target.fill_solid(&fill, theme.ink)?;
        }
        let color = if selected { theme.paper } else { theme.dim };
        let style = MonoTextStyle::new(metrics.font.font(), color);
        Text::with_baseline(&label, Point::new(x, y), style, Baseline::Top).draw(target)?;
        areas.push((
            button,
            Rectangle::new(Point::new(x, y), Size::new(label_width, line_height)),
        ));
        x += (label.width() + 1) as i32 * cell;
    }
    Ok(areas)
}

/// Cut `text` to at most `columns` character cells.
fn fitted(text: &str, columns: usize) -> String {
    if text.width() <= columns {
        return text.to_string();
    }
    let mut fitted = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + width > columns {
            break;
        }
        fitted.push(ch);
        used += width;
    }
    fitted
}

fn draw_chrome<D>(target: &mut D, metrics: Metrics, theme: Theme) -> Result<(), D::Error>
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

fn draw_line<D>(
    target: &mut D,
    line: &Line,
    x: i32,
    y: i32,
    metrics: Metrics,
    theme: Theme,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    let mut cursor = x;
    for span in &line.spans {
        if span.text.is_empty() {
            continue;
        }
        let color = match span.tone {
            Tone::Ink => theme.ink,
            Tone::Dim => theme.dim,
            Tone::Accent => theme.accent,
        };
        let style = MonoTextStyle::new(metrics.font.font(), color);
        Text::with_baseline(&span.text, Point::new(cursor, y), style, Baseline::Top)
            .draw(target)?;
        cursor += (span.text.width() * metrics.font.character_width() as usize) as i32;
    }
    Ok(())
}

fn draw_controls<D>(
    target: &mut D,
    metrics: Metrics,
    theme: Theme,
    pager: &Pager,
) -> Result<(Option<Rectangle>, Option<Rectangle>), D::Error>
where
    D: DrawTarget<Color = Gray4>,
{
    if pager.pages() <= 1 {
        return Ok((None, None));
    }
    let center = Zones::from_metrics(metrics).center;
    let line_height = metrics.font.line_height();
    let cell = metrics.font.character_width();
    let left = if pager.has_previous() { "<" } else { " " };
    let right = if pager.has_next() { ">" } else { " " };
    let label = format!("{left} {}/{} {right}", pager.page() + 1, pager.pages());
    let width = label.width() * cell as usize;
    let x = center.top_left.x + center.size.width as i32 - MARGIN as i32 - width as i32;
    let y = center.top_left.y + center.size.height as i32 - MARGIN as i32 - line_height as i32;
    let style = MonoTextStyle::new(metrics.font.font(), theme.dim);
    Text::with_baseline(&label, Point::new(x, y), style, Baseline::Top).draw(target)?;

    // The arrow glyphs are too small for a fingertip. Pad each hit box by up
    // to two cells sideways and half a line up and down, then clamp it to the
    // center zone. The pad never exceeds half the gap between the glyphs, so
    // the two boxes cannot overlap.
    let gap = (width as i32 - 2 * cell as i32).max(0);
    let pad_x = (2 * cell as i32).min(gap / 2);
    let pad_y = line_height as i32 / 2;
    let left_edge = center.top_left.x;
    let right_edge = center.top_left.x + center.size.width as i32;
    let top_edge = center.top_left.y;
    let bottom_edge = center.top_left.y + center.size.height as i32;
    let hit = |left: i32| {
        let x0 = (left - pad_x).max(left_edge);
        let x1 = (left + cell as i32 + pad_x).min(right_edge);
        let y0 = (y - pad_y).max(top_edge);
        let y1 = (y + line_height as i32 + pad_y).min(bottom_edge);
        Some(Rectangle::new(
            Point::new(x0, y0),
            Size::new((x1 - x0).max(0) as u32, (y1 - y0).max(0) as u32),
        ))
    };
    let previous = if pager.has_previous() { hit(x) } else { None };
    let next = if pager.has_next() {
        hit(x + width as i32 - cell as i32)
    } else {
        None
    };
    Ok((previous, next))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::FontSize;

    #[test]
    fn page_size_leaves_room_for_margins_and_controls() {
        let size = page_size(Metrics::new(320, 240, 28, 36));
        assert_eq!(size.columns, 52);
        assert_eq!(size.rows, 10);

        let large = page_size(Metrics::new(320, 240, 28, 36).with_font(FontSize::Large));
        assert_eq!(large.columns, 44);
        assert_eq!(large.rows, 8);
    }

    #[test]
    fn a_screen_with_no_center_has_no_room() {
        let size = page_size(Metrics::new(320, 200, 120, 120));
        assert_eq!(size.columns, 52);
        assert_eq!(size.rows, 0);
    }
}
