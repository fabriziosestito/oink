//! Drawing: a header with the title, the page column, and a footer with the
//! status bar. The view also records where the choices landed and how far
//! the prose can scroll, so the state can answer clicks and keys.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::text::{Line, Text};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

use crate::cover::Cover;
use crate::game::THE_END;

use super::app::{App, Screen};
use super::text;
use super::theme::Theme;

/// The widest prose column, in cells.
pub const MEASURE: u16 = 80;
/// Cells kept free on each side of the column.
const MARGIN: u16 = 2;
/// Rows kept for the prose when the choices are tall.
const MIN_PROSE_ROWS: u16 = 3;

pub fn draw(frame: &mut Frame, app: &mut App, theme: &Theme, cover: Option<&mut Cover>) {
    let area = frame.area();
    if area.height < 6 || area.width < 2 * MARGIN + 4 {
        frame.render_widget(Paragraph::new("Window too small").style(theme.hint), area);
        return;
    }
    let [header, body, footer] = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(2),
    ])
    .areas(area);
    let title = match app.screen {
        Screen::Cover => "",
        Screen::Page | Screen::End => app.title.as_str(),
    };
    draw_header(frame, header, title, theme);
    let column = column(body);
    match app.screen {
        Screen::Cover => draw_cover(frame, column, app, theme, cover),
        Screen::Page | Screen::End => draw_page(frame, column, body, app, theme),
    }
    draw_footer(frame, footer, app, theme);
}

/// The prose column: at most `MEASURE` wide, centered.
fn column(body: Rect) -> Rect {
    let width = body.width.saturating_sub(2 * MARGIN).clamp(1, MEASURE);
    Rect {
        x: body.x + (body.width - width) / 2,
        y: body.y,
        width,
        height: body.height,
    }
}

fn draw_header(frame: &mut Frame, area: Rect, title: &str, theme: &Theme) {
    let [row, rule] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    frame.render_widget(
        Paragraph::new(Line::styled(title.to_string(), theme.title)).alignment(Alignment::Center),
        row,
    );
    frame.render_widget(
        Block::new().borders(Borders::TOP).border_style(theme.rule),
        rule,
    );
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let [rule, bar] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
    frame.render_widget(
        Block::new().borders(Borders::TOP).border_style(theme.rule),
        rule,
    );
    let hints: &[&str] = match app.screen {
        Screen::Cover => &["Enter or click to begin · q quit", "Enter · q"],
        Screen::Page => &[
            "1-9 pick · ↑↓ move · Enter choose · q quit",
            "1-9 · ↑↓ · Enter · q",
            "q",
        ],
        Screen::End => &["Enter close", "Enter"],
    };
    let line = text::status_line(&app.status, hints, bar.width as usize, theme);
    frame.render_widget(Paragraph::new(line).style(theme.status), bar);
}

fn draw_cover(
    frame: &mut Frame,
    column: Rect,
    app: &App,
    theme: &Theme,
    cover: Option<&mut Cover>,
) {
    let [picture, caption] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(column);
    if let Some(cover) = cover {
        cover.render(frame, picture);
    }
    let lines = vec![
        Line::styled(app.title.clone(), theme.title).centered(),
        Line::default(),
        Line::styled("Press Enter to begin", theme.hint).centered(),
    ];
    frame.render_widget(Paragraph::new(Text::from(lines)), caption);
}

/// The scene: prose with checks and notices on top, then the choices after
/// one blank row. When the prose does not fit, it scrolls and the choices
/// stay pinned at the bottom. The ending has no choices and closes with the
/// house line.
fn draw_page(frame: &mut Frame, column: Rect, body: Rect, app: &mut App, theme: &Theme) {
    let width = column.width as usize;
    let mut lines = text::page_lines(&app.page, width, theme);
    if app.screen == Screen::End {
        lines.push(Line::default());
        lines.push(Line::styled(THE_END.to_string(), theme.the_end).centered());
    }
    let choices: Vec<Vec<Line<'static>>> = app
        .page
        .choices
        .iter()
        .enumerate()
        .map(|(index, runs)| text::choice_lines(index, runs, index == app.selected, width, theme))
        .collect();
    let choice_rows: u16 = choices.iter().map(|lines| row_count(lines.len())).sum();
    let choices_height = if choice_rows == 0 {
        0
    } else {
        (choice_rows + 1).min(column.height.saturating_sub(MIN_PROSE_ROWS))
    };
    let total = row_count(lines.len());
    let prose_height = total.clamp(1, column.height.saturating_sub(choices_height).max(1));
    let [prose_area, choices_area, _] = Layout::vertical([
        Constraint::Length(prose_height),
        Constraint::Length(choices_height),
        Constraint::Min(0),
    ])
    .areas(column);

    app.viewport = prose_area.height;
    app.max_scroll = total.saturating_sub(prose_area.height);
    app.scroll = app.scroll.min(app.max_scroll);
    frame.render_widget(
        Paragraph::new(Text::from(lines)).scroll((app.scroll, 0)),
        prose_area,
    );
    if app.max_scroll > 0 {
        let mut state = ScrollbarState::new(total as usize)
            .position(app.scroll as usize)
            .viewport_content_length(prose_area.height as usize);
        let bar = Rect {
            x: body.right().saturating_sub(1),
            y: prose_area.y,
            width: 1,
            height: prose_area.height,
        };
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight).style(theme.rule),
            bar,
            &mut state,
        );
    }

    app.choice_areas.clear();
    if choices_height == 0 {
        return;
    }
    let bottom = choices_area.bottom();
    let mut y = choices_area.y + 1;
    for lines in choices {
        if y >= bottom {
            break;
        }
        let height = row_count(lines.len()).min(bottom - y);
        let area = Rect {
            x: choices_area.x,
            y,
            width: choices_area.width,
            height,
        };
        frame.render_widget(Paragraph::new(Text::from(lines)), area);
        app.choice_areas.push(area);
        y += height;
    }
}

fn row_count(count: usize) -> u16 {
    u16::try_from(count).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup;
    use crate::tui::app::tests::app;
    use crate::tui::app::Action;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::style::Modifier;
    use ratatui::Terminal;

    fn render(app: &mut App, width: u16, height: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let theme = Theme::colored();
        terminal
            .draw(|frame| draw(frame, app, &theme, None))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn row(buffer: &Buffer, y: u16) -> String {
        (0..buffer.area.width)
            .map(|x| buffer.cell((x, y)).unwrap().symbol())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    fn lines_of(buffer: &Buffer) -> Vec<String> {
        (0..buffer.area.height).map(|y| row(buffer, y)).collect()
    }

    #[test]
    fn a_page_shows_title_prose_choices_and_status() {
        let mut app = app();
        let buffer = render(&mut app, 60, 16);
        let rows = lines_of(&buffer);
        assert_eq!(rows[0].trim(), "Têst");
        assert!(rows[1].starts_with("────"), "{rows:?}");
        assert_eq!(rows[2].trim(), "A door, and a long corridor behind it.");
        assert_eq!(rows[3].trim(), "", "one blank row before the choices");
        let first = rows
            .iter()
            .position(|r| r.trim() == "1. Open the door")
            .unwrap();
        assert_eq!(first, 4, "short scenes flow the choices under the prose");
        assert_eq!(rows[first + 1].trim(), "2. Leave");
        assert_eq!(app.choice_areas.len(), 2);
        assert_eq!(app.choice_areas[0].y, first as u16);
        assert_eq!(app.choice_areas[1].y, first as u16 + 1);
        assert!(rows[14].starts_with("────"), "{rows:?}");
        assert!(rows[15].contains("Health 10/10"), "{rows:?}");
        assert!(rows[15].ends_with("q quit"), "{rows:?}");

        let x = app.choice_areas[0].x;
        let selected = buffer.cell((x, first as u16)).unwrap();
        assert!(selected.modifier.contains(Modifier::REVERSED));
        let other = buffer.cell((x, first as u16 + 1)).unwrap();
        assert!(!other.modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn the_prose_column_is_capped_and_centered() {
        let mut app = app();
        let buffer = render(&mut app, 120, 16);
        let rows = lines_of(&buffer);
        let leading = rows[2].len() - rows[2].trim_start().len();
        assert_eq!(leading, (120 - MEASURE as usize) / 2);
        assert_eq!(app.choice_areas[0].width, MEASURE);
    }

    #[test]
    fn long_prose_scrolls_and_keeps_the_choices_visible() {
        let mut app = app();
        app.page.paragraphs = (0..12)
            .map(|n| markup::parse(&format!("Paragraph {n}.")))
            .collect();
        let buffer = render(&mut app, 40, 12);
        assert!(app.max_scroll > 0, "{}", app.max_scroll);
        let rows = lines_of(&buffer);
        assert!(
            rows.iter().any(|r| r.trim() == "1. Open the door"),
            "{rows:?}"
        );
        assert!(rows[2].contains("Paragraph 0."), "{rows:?}");

        app.update(Action::ScrollBottom).unwrap();
        let buffer = render(&mut app, 40, 12);
        let rows = lines_of(&buffer);
        assert!(!rows[2].contains("Paragraph 0."), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains("Paragraph 11.")), "{rows:?}");
    }

    #[test]
    fn the_ending_shows_the_house_line() {
        let mut app = app();
        app.update(Action::Pick(1)).unwrap();
        assert_eq!(app.screen, Screen::End);
        let buffer = render(&mut app, 60, 12);
        let rows = lines_of(&buffer);
        assert!(rows.iter().any(|r| r.trim() == "You leave."), "{rows:?}");
        assert!(rows.iter().any(|r| r.trim() == THE_END), "{rows:?}");
        assert!(app.choice_areas.is_empty());
        assert!(rows[11].ends_with("Enter close"), "{rows:?}");
    }

    #[test]
    fn a_tiny_window_shows_a_hint_instead_of_panicking() {
        let mut app = app();
        let buffer = render(&mut app, 20, 4);
        assert!(row(&buffer, 0).contains("Window too small"));
    }
}
