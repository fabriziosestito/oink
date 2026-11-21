//! The full-screen player: a book page in the terminal, drawn with ratatui.
//! The prose sits in a centered column with the checks and notices under
//! it, the choices stay pinned below, and a status bar shows the character.

mod app;
mod text;
mod theme;
mod view;

use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::path::Path;

use image::DynamicImage;
use oink_core::Engine;
use oink_ui::{UiConfig, View};
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEvent,
    KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::layout::Position;
use ratatui::DefaultTerminal;

use crate::bundle::Bundle;
use crate::game::{self, THE_END};
use crate::picture::{Painter, Picture};
use crate::saves::Saves;
use app::{Action, App, Presentation, Screen};
use theme::Theme;

/// Rows one notch of the mouse wheel scrolls.
const WHEEL_ROWS: u16 = 3;

/// Play `bundle` full screen and leave the terminal as it was. After an
/// ending, the house line stays on the terminal.
pub fn run(bundle: &Bundle, seed: Option<u64>) -> Result<(), Box<dyn Error>> {
    let engine = game::load(bundle, seed)?;
    let ui = UiConfig::from_config(&bundle.config)?;
    ui.validate(&engine.data.rulebook)?;
    let picture = bundle.cover().and_then(|path| match image::open(&path) {
        Ok(picture) => Some(picture),
        Err(error) => {
            eprintln!("warning: cannot read {}: {error}", path.display());
            None
        }
    });
    let theme = Theme::from_config(&ui.theme.terminal);

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        hook(info);
    }));
    let mut terminal = ratatui::init();
    let result = play(&mut terminal, bundle, engine, ui, picture, &theme);
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
    bundle: &Bundle,
    engine: Engine,
    ui: UiConfig,
    picture: Option<DynamicImage>,
    theme: &Theme,
) -> Result<bool, Box<dyn Error>> {
    let _mouse = MouseCapture::enable()?;
    let painter = Painter::probe();
    let mut cover = picture.map(|picture| painter.view(picture));
    let saves = Saves::beside(&bundle.dir, ui.reader.save_slots);
    let presentation = Presentation {
        reader: ui.reader,
        status: ui.status,
        characters: ui.characters,
        maps: ui.maps,
    };
    let mut app = App::new(engine, cover.is_some(), presentation, saves)?;
    // The dialog portraits, loaded on first use and kept for the session.
    let mut portraits: HashMap<String, Option<Picture>> = HashMap::new();
    // The scene and item pictures, keyed by their path in the bundle.
    let mut images: HashMap<String, Option<Picture>> = HashMap::new();
    while !app.quit {
        let portrait = match app.portrait().map(str::to_string) {
            Some(id) => {
                if !portraits.contains_key(&id) {
                    let (picture, warning) = load_portrait(bundle, &app, &painter, &id);
                    if let Some(warning) = warning {
                        app.notice = Some(warning);
                    }
                    portraits.insert(id.clone(), picture);
                }
                portraits.get_mut(&id).and_then(|picture| picture.as_mut())
            }
            None => None,
        };
        let image = match app.image_path().map(str::to_string) {
            Some(path) => {
                if !images.contains_key(&path) {
                    let (picture, warning) = load_image(&bundle.dir, &painter, &path);
                    if let Some(warning) = warning {
                        app.notice = Some(warning);
                    }
                    images.insert(path.clone(), picture);
                }
                images.get_mut(&path).and_then(|picture| picture.as_mut())
            }
            None => None,
        };
        terminal
            .draw(|frame| view::draw(frame, &mut app, theme, cover.as_mut(), portrait, image))?;
        app.notice = None;
        if let Some(action) = action(event::read()?) {
            app.update(action)?;
        }
    }
    Ok(app.screen == Screen::End)
}

/// The portrait picture of a character, when its file reads, plus a warning
/// for the status bar when it does not.
fn load_portrait(
    bundle: &Bundle,
    app: &App,
    painter: &Painter,
    id: &str,
) -> (Option<Picture>, Option<String>) {
    let Some(path) = app.portrait_path(id).map(|path| bundle.dir.join(path)) else {
        return (None, None);
    };
    match image::open(&path) {
        Ok(picture) => (Some(painter.view(picture)), None),
        Err(error) => (
            None,
            Some(format!(
                "warning: cannot read portrait {}: {error}",
                path.display()
            )),
        ),
    }
}

/// A scene or item picture, when its file reads, plus a warning for the
/// status bar when it does not.
fn load_image(dir: &Path, painter: &Painter, path: &str) -> (Option<Picture>, Option<String>) {
    let path = dir.join(path);
    match image::open(&path) {
        Ok(picture) => (Some(painter.view(picture)), None),
        Err(error) => (
            None,
            Some(format!(
                "warning: cannot read image {}: {error}",
                path.display()
            )),
        ),
    }
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
        KeyCode::Backspace => Some(Action::Close),
        KeyCode::Char('s') => Some(Action::Open(View::Sheet)),
        KeyCode::Char('i') => Some(Action::Open(View::Inventory)),
        KeyCode::Char('l') => Some(Action::Open(View::Log)),
        KeyCode::Char('m') => Some(Action::Open(View::Menu)),
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
        assert_eq!(action(press(KeyCode::Backspace)), Some(Action::Close));
        assert_eq!(
            action(press(KeyCode::Char('s'))),
            Some(Action::Open(View::Sheet))
        );
        assert_eq!(
            action(press(KeyCode::Char('i'))),
            Some(Action::Open(View::Inventory))
        );
        assert_eq!(
            action(press(KeyCode::Char('l'))),
            Some(Action::Open(View::Log))
        );
        assert_eq!(
            action(press(KeyCode::Char('m'))),
            Some(Action::Open(View::Menu))
        );
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

    #[test]
    fn a_pnm_scene_image_loads_and_draws() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let dir = std::env::temp_dir().join(format!(
            "oink-image-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(dir.join("art")).unwrap();
        std::fs::write(
            dir.join("art/test.pgm"),
            "P2\n# a tiny gradient\n3 2\n255\n0 127 255\n255 127 0\n",
        )
        .unwrap();

        let painter = Painter::halfblocks();
        let (picture, warning) = load_image(&dir, &painter, "art/test.pgm");
        assert!(warning.is_none(), "{warning:?}");
        let mut picture = picture.expect("the ASCII PGM loads");
        let mut terminal = Terminal::new(TestBackend::new(8, 4)).unwrap();
        terminal
            .draw(|frame| picture.render(frame, frame.area()))
            .unwrap();

        // The Unicode fallback paints half blocks, so the picture is visible
        // even when the terminal has no graphics protocol.
        let drawn: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(
            drawn.contains('▀') || drawn.contains('▄'),
            "the picture draws half blocks: {drawn:?}"
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
