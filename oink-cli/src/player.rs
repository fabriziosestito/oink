//! The transcript player. Scenes print one after another like a log, and
//! picks come from lines on stdin or from a fixed list. In a terminal the
//! full-screen player in `tui` takes over unless `--plain` asks for this one.

use std::error::Error;
use std::fmt;
use std::io::{self, BufRead, IsTerminal, Write};

use oink_core::oink_rulebook::StateChange;
use oink_core::{CheckRecord, Choice, Event};
use ratatui::crossterm::terminal;

use crate::bundle::Bundle;
use crate::game::{self, THE_END};
use crate::markup;
use crate::tui;
use crate::wrap::wrap_plain;

const PROMPT: &str = "> ";
const MIN_WIDTH: usize = 20;
const MAX_WIDTH: usize = 100;
const DEFAULT_WIDTH: usize = 80;

pub struct Options {
    /// Seed the dice so every run rolls the same.
    pub seed: Option<u64>,
    /// Picks to play without waiting, as 0-based indexes.
    pub choices: Option<Vec<usize>>,
    /// Print the transcript even in a terminal.
    pub plain: bool,
}

/// Where picks come from.
pub enum Input {
    /// A fixed list, no waiting. The run stops when the list runs out.
    Scripted(std::vec::IntoIter<usize>),
    /// One number per line on stdin, for pipes, files, and `--plain`.
    Lines,
}

impl Input {
    /// The next pick, or `None` when the player quits or the input ends.
    fn pick(&mut self, count: usize, out: &mut dyn Write) -> Result<Option<usize>, Box<dyn Error>> {
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

/// Play a bundle. Interactive terminals get the full-screen player; pipes,
/// scripted picks, and `--plain` get the transcript on `out`.
pub fn play(bundle: &Bundle, options: Options, out: &mut dyn Write) -> Result<(), Box<dyn Error>> {
    let interactive = io::stdin().is_terminal() && io::stdout().is_terminal();
    let input = match options.choices {
        Some(list) => Input::Scripted(list.into_iter()),
        None if interactive && !options.plain => return tui::run(bundle, options.seed),
        None => Input::Lines,
    };
    let width = text_width(io::stdout().is_terminal());
    run(bundle, options.seed, input, width, out)
}

pub fn run(
    bundle: &Bundle,
    seed: Option<u64>,
    mut input: Input,
    width: usize,
    out: &mut dyn Write,
) -> Result<(), Box<dyn Error>> {
    let mut engine = game::load(bundle, seed)?;
    let title = engine.data.config.title.clone();
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
                print_choices(out, &choices, width)?;
                let Some(pick) = input.pick(choices.len(), out)? else {
                    return Ok(());
                };
                event = engine.choose(pick)?;
            }
        }
    }
}

fn text_width(terminal: bool) -> usize {
    let columns = match terminal::size() {
        Ok((columns, _)) if terminal && columns > 0 => columns as usize,
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
        for line in wrap_plain(&markup::plain(paragraph), width) {
            writeln!(out, "{line}")?;
        }
        writeln!(out)?;
    }
    for check in checks {
        for line in wrap_plain(&check.describe(), width) {
            writeln!(out, "{line}")?;
        }
    }
    for change in changes {
        for line in wrap_plain(&format!("+ {change}"), width) {
            writeln!(out, "{line}")?;
        }
    }
    if !checks.is_empty() || !changes.is_empty() {
        writeln!(out)?;
    }
    Ok(())
}

fn print_choices(out: &mut dyn Write, choices: &[Choice], width: usize) -> io::Result<()> {
    for (number, choice) in choices.iter().enumerate() {
        let text = format!("{}. {}", number + 1, markup::plain(&choice.text));
        for line in wrap_plain(&text, width) {
            writeln!(out, "{line}")?;
        }
    }
    writeln!(out)?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Story;
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
A door, and a *long* corridor behind it that goes on and on and on for a while.
+ [Open the **door**]
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
            width,
            &mut out,
        )?;
        Ok(String::from_utf8(out).unwrap())
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
