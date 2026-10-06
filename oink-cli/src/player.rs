//! The transcript player. Scenes print one after another like a log, and
//! picks come from keys and mouse clicks, from lines on stdin, or from a
//! fixed list.

use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, IsTerminal, Write};
use std::ops::Range;

use crossterm::cursor;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEventKind,
    KeyModifiers, MouseButton, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode};
use oink_core::data::GameData;
use oink_core::oink_rulebook::StateChange;
use oink_core::{CheckRecord, Choice, Engine, Event};

use crate::bundle::{Bundle, Story};
use crate::image::Images;

/// The house line under every ending.
pub const THE_END: &str = "~ La tua vita e la tua missione terminano qui ~";

const PROMPT: &str = "> ";
const MIN_WIDTH: usize = 20;
const MAX_WIDTH: usize = 100;
const DEFAULT_WIDTH: usize = 80;
const COVER_COLUMNS: u32 = 24;

pub struct Options {
    /// Seed the dice so every run rolls the same.
    pub seed: Option<u64>,
    /// Picks to play without waiting, as 0-based indexes.
    pub choices: Option<Vec<usize>>,
}

/// Where picks come from.
pub enum Input {
    /// A fixed list, no waiting. The run stops when the list runs out.
    Scripted(std::vec::IntoIter<usize>),
    /// One number per line on stdin, for pipes and files.
    Lines,
    /// Keys 1 to 9 and left clicks on choice lines, in raw mode while waiting.
    Terminal,
}

impl Input {
    pub fn detect(choices: Option<Vec<usize>>) -> Self {
        match choices {
            Some(list) => Input::Scripted(list.into_iter()),
            None if io::stdin().is_terminal() && io::stdout().is_terminal() => Input::Terminal,
            None => Input::Lines,
        }
    }

    /// The next pick, or `None` when the player quits or the input ends.
    fn pick(
        &mut self,
        count: usize,
        rows: &[usize],
        out: &mut dyn Write,
    ) -> Result<Option<usize>, Box<dyn Error>> {
        match self {
            Input::Scripted(list) => {
                let Some(pick) = list.next() else {
                    return Ok(None);
                };
                if pick >= count {
                    return Err(PlayError::OutOfRange {
                        pick: pick + 1,
                        count,
                    }
                    .into());
                }
                writeln!(out, "{PROMPT}{}", pick + 1)?;
                writeln!(out)?;
                Ok(Some(pick))
            }
            Input::Lines => pick_from_lines(count, out),
            Input::Terminal => pick_from_terminal(count, rows, out),
        }
    }
}

#[derive(Debug)]
pub enum PlayError {
    OutOfRange { pick: usize, count: usize },
}

impl fmt::Display for PlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayError::OutOfRange { pick, count } => {
                let noun = if *count == 1 { "choice" } else { "choices" };
                write!(
                    f,
                    "choice {pick} is out of range: this scene has {count} {noun}"
                )
            }
        }
    }
}

impl Error for PlayError {}

/// Play a bundle on `out`, which is stdout outside tests.
pub fn play(bundle: &Bundle, options: Options, out: &mut dyn Write) -> Result<(), Box<dyn Error>> {
    let input = Input::detect(options.choices);
    let images = if matches!(input, Input::Terminal) {
        Images::detect()
    } else {
        Images::disabled()
    };
    let width = text_width(&input);
    run(bundle, options.seed, input, &images, width, out)
}

pub fn run(
    bundle: &Bundle,
    seed: Option<u64>,
    mut input: Input,
    images: &Images,
    width: usize,
    out: &mut dyn Write,
) -> Result<(), Box<dyn Error>> {
    let data = GameData::from_yaml(&bundle.config, &bundle.rulebook)?;
    for warning in &data.warnings {
        eprintln!("warning: {warning}");
    }
    let title = data.config.title.clone();
    let mut engine = match &bundle.story {
        Story::Ink(source) => Engine::new(source, data)?,
        Story::Json(json) => Engine::from_json(json, data)?,
    };
    if let Some(seed) = seed {
        engine.set_seed(seed);
    }

    if let Some(cover) = bundle.cover() {
        out.flush()?;
        images.show(&cover, COVER_COLUMNS)?;
    }
    writeln!(out, "{title}")?;
    writeln!(out, "{}", "=".repeat(title.chars().count()))?;
    writeln!(out)?;

    let mut event = engine.start()?;
    loop {
        let checks = engine.take_checks();
        let changes = engine.take_changes();
        match event {
            Event::TheEnd { text } => {
                print_scene(out, &text, &checks, &changes, width)?;
                writeln!(out, "{THE_END}")?;
                return Ok(());
            }
            Event::Scene { text, choices } => {
                print_scene(out, &text, &checks, &changes, width)?;
                let rows = print_choices(out, &choices, width)?;
                let Some(pick) = input.pick(choices.len(), &rows, out)? else {
                    return Ok(());
                };
                event = engine.choose(pick)?;
            }
        }
    }
}

fn text_width(input: &Input) -> usize {
    let columns = match input {
        Input::Terminal => match terminal::size() {
            Ok((columns, _)) if columns > 0 => columns as usize,
            _ => DEFAULT_WIDTH,
        },
        _ => DEFAULT_WIDTH,
    };
    columns.clamp(MIN_WIDTH, MAX_WIDTH)
}

fn print_scene(
    out: &mut dyn Write,
    text: &[String],
    checks: &[CheckRecord],
    changes: &[StateChange],
    width: usize,
) -> io::Result<()> {
    for paragraph in text {
        for line in wrap(paragraph, width) {
            writeln!(out, "{line}")?;
        }
        writeln!(out)?;
    }
    for check in checks {
        for line in wrap(&check.describe(), width) {
            writeln!(out, "{line}")?;
        }
    }
    for change in changes {
        for line in wrap(&format!("+ {change}"), width) {
            writeln!(out, "{line}")?;
        }
    }
    if !checks.is_empty() || !changes.is_empty() {
        writeln!(out)?;
    }
    Ok(())
}

/// Print the numbered choices and return how many lines each one took.
fn print_choices(out: &mut dyn Write, choices: &[Choice], width: usize) -> io::Result<Vec<usize>> {
    let mut rows = Vec::with_capacity(choices.len());
    for (number, choice) in choices.iter().enumerate() {
        let lines = wrap(&format!("{}. {}", number + 1, choice.text), width);
        rows.push(lines.len());
        for line in lines {
            writeln!(out, "{line}")?;
        }
    }
    writeln!(out)?;
    Ok(rows)
}

fn pick_from_lines(count: usize, out: &mut dyn Write) -> Result<Option<usize>, Box<dyn Error>> {
    let stdin = io::stdin();
    let mut line = String::new();
    loop {
        write!(out, "{PROMPT}")?;
        out.flush()?;
        line.clear();
        if stdin.lock().read_line(&mut line)? == 0 {
            writeln!(out)?;
            return Ok(None);
        }
        let answer = line.trim();
        if answer.eq_ignore_ascii_case("q") {
            writeln!(out)?;
            return Ok(None);
        }
        match answer.parse::<usize>() {
            Ok(number) if (1..=count).contains(&number) => {
                writeln!(out, "{number}")?;
                writeln!(out)?;
                return Ok(Some(number - 1));
            }
            _ => writeln!(out, "Type a number from 1 to {count}, or q to quit.")?,
        }
    }
}

fn pick_from_terminal(
    count: usize,
    rows: &[usize],
    out: &mut dyn Write,
) -> Result<Option<usize>, Box<dyn Error>> {
    write!(out, "{PROMPT}")?;
    out.flush()?;
    let pick = {
        let _raw = RawMode::enter()?;
        wait_for_pick(count, rows)?
    };
    match pick {
        Some(pick) => {
            writeln!(out, "{}", pick + 1)?;
            writeln!(out)?;
        }
        None => writeln!(out)?,
    }
    Ok(pick)
}

/// Block until a key or a click picks a choice. Esc, q, and Ctrl-C quit.
/// When the terminal does not report the cursor row, clicks are ignored
/// and keys still work.
fn wait_for_pick(count: usize, rows: &[usize]) -> io::Result<Option<usize>> {
    let mut zones = click_zones(rows);
    loop {
        match event::read()? {
            TermEvent::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None)
                }
                KeyCode::Char(digit) => {
                    if let Some(number) = digit.to_digit(10) {
                        let number = number as usize;
                        if (1..=count).contains(&number) {
                            return Ok(Some(number - 1));
                        }
                    }
                }
                _ => {}
            },
            TermEvent::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                if let Some(index) = zones.iter().position(|zone| zone.contains(&mouse.row)) {
                    return Ok(Some(index));
                }
            }
            TermEvent::Resize(..) => zones = click_zones(rows),
            _ => {}
        }
    }
}

/// Ask the terminal where the prompt is and map the choice lines above it.
fn click_zones(rows: &[usize]) -> Vec<Range<u16>> {
    cursor::position()
        .map(|(_, prompt_row)| choice_zones(prompt_row, rows))
        .unwrap_or_default()
}

/// The screen rows of each choice, counted back from the prompt row. The
/// choices sit above one blank line and the prompt.
fn choice_zones(prompt_row: u16, rows: &[usize]) -> Vec<Range<u16>> {
    let mut end = prompt_row.saturating_sub(1);
    let mut zones = vec![0..0; rows.len()];
    for (index, &count) in rows.iter().enumerate().rev() {
        let start = end.saturating_sub(count as u16);
        zones[index] = start..end;
        end = start;
    }
    zones
}

/// Raw mode plus mouse capture, restored on drop.
struct RawMode;

impl RawMode {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(io::stdout(), EnableMouseCapture) {
            let _ = disable_raw_mode();
            return Err(error);
        }
        Ok(Self)
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        let _ = disable_raw_mode();
    }
}

/// Word wrap to `width` characters. A word longer than the width keeps its
/// own line.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut length = 0;
    for word in text.split_whitespace() {
        let word_length = word.chars().count();
        if length > 0 && length + 1 + word_length > width {
            lines.push(std::mem::take(&mut current));
            length = 0;
        }
        if length > 0 {
            current.push(' ');
            length += 1;
        }
        current.push_str(word);
        length += word_length;
    }
    if length > 0 {
        lines.push(current);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const RULEBOOK: &str = "
characteristics:
  body:
    name: Body
    min: 1
    max: 9
    default: 5
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
abilities:
  might: { name: Might, characteristic: body }
items:
  key: { name: Key }
starting_character:
  characteristics: { body: 5 }
  abilities: { might: 1 }
";

    const STORY: &str = r#"
EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL add_item(id)
A door, and a long corridor behind it that goes on and on and on for a while.
+ [Open the door]
    ~ add_item("key")
    ~ roll_check("might", 7, "", 0)
    -> inside
+ [Leave]
    You leave.
    -> END
=== inside ===
Inside.
+ [Sit]
    -> END
"#;

    fn bundle() -> Bundle {
        Bundle {
            dir: PathBuf::new(),
            config: "title: Têst".to_string(),
            rulebook: RULEBOOK.to_string(),
            story: Story::Ink(STORY.to_string()),
        }
    }

    fn transcript(picks: Vec<usize>, width: usize) -> Result<String, Box<dyn Error>> {
        let mut out = Vec::new();
        run(
            &bundle(),
            Some(7),
            Input::Scripted(picks.into_iter()),
            &Images::disabled(),
            width,
            &mut out,
        )?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn wrap_breaks_on_words_and_counts_characters() {
        assert_eq!(
            wrap("one two three four", 9),
            vec!["one two", "three", "four"]
        );
        assert_eq!(wrap("è è è", 3), vec!["è è", "è"]);
        assert_eq!(
            wrap("supercalifragilistic is long", 5),
            vec!["supercalifragilistic", "is", "long"]
        );
        assert!(wrap("   ", 10).is_empty());
    }

    #[test]
    fn zones_count_back_from_the_prompt() {
        // Rows 6 to 8 hold the choices, 9 is blank, 10 is the prompt.
        assert_eq!(choice_zones(10, &[1, 2]), vec![6..7, 7..9]);
        // Choices that scrolled off the top clamp at row 0.
        assert_eq!(choice_zones(2, &[1, 3]), vec![0..0, 0..1]);
    }

    #[test]
    fn scripted_run_prints_the_transcript_to_the_end() {
        let text = transcript(vec![0, 0], 40).unwrap();
        assert!(text.starts_with("Têst\n====\n\n"), "{text}");
        assert!(
            text.contains("A door, and a long corridor behind it\n"),
            "{text}"
        );
        assert!(
            text.contains("1. Open the door\n2. Leave\n\n> 1\n\n"),
            "{text}"
        );
        assert!(text.contains("* might check: die "), "{text}");
        assert!(text.contains("+ item added: key\n"), "{text}");
        assert!(
            text.contains("+ item added: key\n\n1. Sit\n\n> 1\n\n"),
            "{text}"
        );
        assert!(text.ends_with(&format!("{THE_END}\n")), "{text}");
    }

    #[test]
    fn scripted_run_stops_when_the_picks_run_out() {
        let text = transcript(vec![], 80).unwrap();
        assert!(text.ends_with("1. Open the door\n2. Leave\n\n"), "{text}");
    }

    #[test]
    fn scripted_pick_out_of_range_is_an_error() {
        let error = transcript(vec![2], 80).unwrap_err();
        assert_eq!(
            error.to_string(),
            "choice 3 is out of range: this scene has 2 choices"
        );
    }
}
