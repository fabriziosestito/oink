//! The full-screen player: a book page in the terminal, drawn with ratatui.
//! The prose sits in a centered column with the checks and notices under
//! it, the choices stay pinned below, and a status bar shows the character.

mod app;
mod text;
mod theme;
mod view;

use std::error::Error;
use std::io;

use image::DynamicImage;
use oink_core::Engine;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEvent,
    KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Position;
use ratatui::DefaultTerminal;

use crate::bundle::Bundle;
use crate::cover::Cover;
use crate::game::{self, THE_END};
use app::{Action, App, Screen};
use theme::Theme;

/// Rows one notch of the mouse wheel scrolls.
const WHEEL_ROWS: u16 = 3;

/// Play `bundle` full screen and leave the terminal as it was. After an
/// ending, the house line stays on the terminal.
pub fn run(bundle: &Bundle, seed: Option<u64>) -> Result<(), Box<dyn Error>> {
    let engine = game::load(bundle, seed)?;
    let picture = bundle.cover().and_then(|path| match image::open(&path) {
        Ok(picture) => Some(picture),
        Err(error) => {
            eprintln!("warning: cannot read {}: {error}", path.display());
            None
        }
    });
    let theme = Theme::detect();

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        hook(info);
    }));
    let mut terminal = ratatui::init();
    let result = play(&mut terminal, engine, picture, &theme);
    ratatui::restore();
    drop(terminal);

    if result? {
        println!("{THE_END}");
    }
    Ok(())
}

/// The event loop. Returns whether the story reached an ending.
fn play(
    terminal: &mut DefaultTerminal,
    engine: Engine,
    picture: Option<DynamicImage>,
    theme: &Theme,
) -> Result<bool, Box<dyn Error>> {
    let _mouse = MouseCapture::enable()?;
    let mut cover = picture.map(Cover::new);
    let mut app = App::new(engine, cover.is_some())?;
    while !app.quit {
        terminal.draw(|frame| view::draw(frame, &mut app, theme, cover.as_mut()))?;
        if let Some(action) = action(event::read()?) {
            app.update(action)?;
        }
    }
    Ok(app.screen == Screen::End)
}

fn action(event: TermEvent) -> Option<Action> {
    match event {
        TermEvent::Key(key) if key.kind == KeyEventKind::Press => key_action(key),
        TermEvent::Mouse(mouse) => mouse_action(mouse),
        _ => None,
    }
}

fn key_action(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => Some(Action::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Action::Quit),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Confirm),
        KeyCode::Up | KeyCode::Char('k') => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::Down),
        KeyCode::PageUp => Some(Action::PageUp),
        KeyCode::PageDown => Some(Action::PageDown),
        KeyCode::Home => Some(Action::ScrollTop),
        KeyCode::End => Some(Action::ScrollBottom),
        KeyCode::Char(digit) => digit
            .to_digit(10)
            .filter(|number| *number > 0)
            .map(|number| Action::Pick(number as usize - 1)),
        _ => None,
    }
}

fn mouse_action(mouse: MouseEvent) -> Option<Action> {
    let position = Position::new(mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => Some(Action::Click(position)),
        MouseEventKind::Moved => Some(Action::Hover(position)),
        MouseEventKind::ScrollUp => Some(Action::ScrollUp(WHEEL_ROWS)),
        MouseEventKind::ScrollDown => Some(Action::ScrollDown(WHEEL_ROWS)),
        _ => None,
    }
}

/// Mouse capture, released on drop. `ratatui::restore` does not release it.
struct MouseCapture;

impl MouseCapture {
    fn enable() -> io::Result<Self> {
        execute!(io::stdout(), EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for MouseCapture {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(code: KeyCode) -> TermEvent {
        TermEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn keys_map_to_actions() {
        assert_eq!(action(press(KeyCode::Char('1'))), Some(Action::Pick(0)));
        assert_eq!(action(press(KeyCode::Char('9'))), Some(Action::Pick(8)));
        assert_eq!(action(press(KeyCode::Char('0'))), None);
        assert_eq!(action(press(KeyCode::Char('x'))), None);
        assert_eq!(action(press(KeyCode::Enter)), Some(Action::Confirm));
        assert_eq!(action(press(KeyCode::Char('j'))), Some(Action::Down));
        assert_eq!(action(press(KeyCode::Up)), Some(Action::Up));
        assert_eq!(action(press(KeyCode::Esc)), Some(Action::Quit));
        assert_eq!(
            action(TermEvent::Key(KeyEvent::new(
                KeyCode::Char('c'),
                KeyModifiers::CONTROL
            ))),
            Some(Action::Quit)
        );
        let mut release = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(action(TermEvent::Key(release)), None);
    }

    #[test]
    fn mouse_events_map_to_actions() {
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 4,
            row: 7,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            action(TermEvent::Mouse(click)),
            Some(Action::Click(Position::new(4, 7)))
        );
        let wheel = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            ..click
        };
        assert_eq!(
            action(TermEvent::Mouse(wheel)),
            Some(Action::ScrollDown(WHEEL_ROWS))
        );
        assert_eq!(action(TermEvent::Resize(80, 24)), None);
    }
}
