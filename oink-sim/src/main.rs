//! Desktop simulator for oink, mimicking the M5Paper (960x540, 16-level grayscale).
//!
//! Keys 1-9 select a choice. Esc quits.

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Gray4,
    prelude::*,
    text::Text,
};
use embedded_graphics_simulator::{
    sdl2::Keycode, OutputSettingsBuilder, SimulatorDisplay, SimulatorEvent, Window,
};
use oink_core::{data::GameData, CheckRecord, Engine, Event};

const WIDTH: u32 = 960;
const HEIGHT: u32 = 540;
const MARGIN: i32 = 24;
const LINE_HEIGHT: i32 = 24;
const MAX_COLS: usize = ((WIDTH as i32 - 2 * MARGIN) / 10) as usize; // FONT_10X20 is 10px wide

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ink = std::fs::read_to_string("assets/story/main.ink")?;
    let data = GameData::from_yaml(
        &std::fs::read_to_string("assets/data/config.yaml")?,
        &std::fs::read_to_string("assets/data/rulebook.yaml")?,
    )?;

    let title = data.config.title.clone();
    let mut engine = Engine::new(&ink, data)?;
    let mut event = engine.start()?;
    let mut checks = engine.take_checks();

    let mut display = SimulatorDisplay::<Gray4>::new(Size::new(WIDTH, HEIGHT));
    let settings = OutputSettingsBuilder::new().scale(1).build();
    let mut window = Window::new(&title, &settings);

    render(&mut display, &event, &checks)?;
    window.update(&display);

    'run: loop {
        let mut chosen = None;
        for ev in window.events() {
            match ev {
                SimulatorEvent::Quit => break 'run,
                SimulatorEvent::KeyDown { keycode, .. } => {
                    if keycode == Keycode::Escape {
                        break 'run;
                    }
                    if let Some(i) = digit(keycode) {
                        chosen = Some(i);
                    }
                }
                _ => {}
            }
        }
        if let Some(i) = chosen {
            if let Event::Scene { choices, .. } = &event {
                if i < choices.len() {
                    event = engine.choose(i)?;
                    checks = engine.take_checks();
                    render(&mut display, &event, &checks)?;
                    window.update(&display);
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    Ok(())
}

fn digit(key: Keycode) -> Option<usize> {
    match key {
        Keycode::Num1 => Some(0),
        Keycode::Num2 => Some(1),
        Keycode::Num3 => Some(2),
        Keycode::Num4 => Some(3),
        Keycode::Num5 => Some(4),
        Keycode::Num6 => Some(5),
        Keycode::Num7 => Some(6),
        Keycode::Num8 => Some(7),
        Keycode::Num9 => Some(8),
        _ => None,
    }
}

fn render(
    display: &mut SimulatorDisplay<Gray4>,
    event: &Event,
    checks: &[CheckRecord],
) -> Result<(), std::convert::Infallible> {
    display.clear(Gray4::WHITE)?;
    let style = MonoTextStyle::new(&FONT_10X20, Gray4::BLACK);
    let mut y = MARGIN + LINE_HEIGHT;

    let (text, choices) = match event {
        Event::Scene { text, choices } => (text, Some(choices)),
        Event::TheEnd { text } => (text, None),
    };

    for paragraph in text {
        for line in wrap(paragraph, MAX_COLS) {
            Text::new(&line, Point::new(MARGIN, y), style).draw(display)?;
            y += LINE_HEIGHT;
        }
        y += LINE_HEIGHT / 2;
    }

    for check in checks {
        for line in wrap(&check.describe(), MAX_COLS) {
            Text::new(&line, Point::new(MARGIN, y), style).draw(display)?;
            y += LINE_HEIGHT;
        }
    }
    if !checks.is_empty() {
        y += LINE_HEIGHT / 2;
    }

    y += LINE_HEIGHT;
    match choices {
        Some(choices) => {
            for choice in choices {
                let label = format!("{}. {}", choice.index + 1, choice.text);
                for line in wrap(&label, MAX_COLS) {
                    Text::new(&line, Point::new(MARGIN, y), style).draw(display)?;
                    y += LINE_HEIGHT;
                }
            }
        }
        None => {
            Text::new("~ La tua vita e la tua missione terminano qui ~", Point::new(MARGIN, y), style).draw(display)?;
        }
    }

    Ok(())
}

fn wrap(text: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if !current.is_empty() && current.len() + 1 + word.len() > cols {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
