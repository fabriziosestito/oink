//! Emphasis markup in story text: `**bold**`, `*italic*`, and `_italic_`.
//!
//! Ink has no inline formatting, so oink defines this small convention. A
//! marker opens only when a non-space character follows it and closes only
//! when a non-space character precedes it, so `5 * 3` stays as written.
//! Underscores inside a word, as in `snake_case`, are never markers. An
//! unmatched marker prints as a literal character.

/// Bold and italic flags for one run of text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Emphasis {
    pub bold: bool,
    pub italic: bool,
}

/// A stretch of text with one emphasis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub text: String,
    pub emphasis: Emphasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bold,
    Italic,
}

impl Kind {
    fn marker(self, underscore: bool) -> &'static str {
        match (self, underscore) {
            (Kind::Bold, _) => "**",
            (Kind::Italic, false) => "*",
            (Kind::Italic, true) => "_",
        }
    }
}

#[derive(Debug)]
enum Token {
    Text(String),
    Marker {
        kind: Kind,
        underscore: bool,
        can_open: bool,
        can_close: bool,
        /// `Some(true)` opens a matched pair, `Some(false)` closes one.
        matched: Option<bool>,
    },
}

/// Split `text` into runs, markers removed.
pub fn parse(text: &str) -> Vec<Run> {
    let mut tokens = tokenize(text);
    match_markers(&mut tokens);
    build_runs(tokens)
}

/// The text without markers.
pub fn plain(text: &str) -> String {
    parse(text).into_iter().map(|run| run.text).collect()
}

fn tokenize(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch != '*' && ch != '_' {
            current.push(ch);
            index += 1;
            continue;
        }
        let mut end = index;
        while end < chars.len() && chars[end] == ch {
            end += 1;
        }
        let length = end - index;
        let before = index.checked_sub(1).map(|i| chars[i]);
        let after = chars.get(end).copied();
        let underscore = ch == '_';
        let kinds: &[Kind] = match (underscore, length) {
            (false, 1) | (true, 1) => &[Kind::Italic],
            (false, 2) => &[Kind::Bold],
            (false, 3)
                if can_open(before, after, underscore) && !can_close(before, after, underscore) =>
            {
                &[Kind::Bold, Kind::Italic]
            }
            (false, 3) => &[Kind::Italic, Kind::Bold],
            _ => &[],
        };
        if kinds.is_empty() {
            current.extend(std::iter::repeat_n(ch, length));
        } else {
            if !current.is_empty() {
                tokens.push(Token::Text(std::mem::take(&mut current)));
            }
            for &kind in kinds {
                tokens.push(Token::Marker {
                    kind,
                    underscore,
                    can_open: can_open(before, after, underscore),
                    can_close: can_close(before, after, underscore),
                    matched: None,
                });
            }
        }
        index = end;
    }
    if !current.is_empty() {
        tokens.push(Token::Text(current));
    }
    tokens
}

fn can_open(before: Option<char>, after: Option<char>, underscore: bool) -> bool {
    let follows_text = after.is_some_and(|c| !c.is_whitespace());
    let word_boundary = !underscore || !before.is_some_and(|c| c.is_alphanumeric());
    follows_text && word_boundary
}

fn can_close(before: Option<char>, after: Option<char>, underscore: bool) -> bool {
    let ends_text = before.is_some_and(|c| !c.is_whitespace());
    let word_boundary = !underscore || !after.is_some_and(|c| c.is_alphanumeric());
    ends_text && word_boundary
}

/// Pair each closer with the nearest open marker of its kind. Markers left
/// open in between become literal text.
fn match_markers(tokens: &mut [Token]) {
    let mut open: Vec<(usize, Kind)> = Vec::new();
    for index in 0..tokens.len() {
        let Token::Marker {
            kind,
            can_open,
            can_close,
            ..
        } = tokens[index]
        else {
            continue;
        };
        let opener = if can_close {
            open.iter().rposition(|&(_, open_kind)| open_kind == kind)
        } else {
            None
        };
        match opener {
            Some(position) => {
                let (opener_index, _) = open[position];
                open.truncate(position);
                set_matched(&mut tokens[opener_index], true);
                set_matched(&mut tokens[index], false);
            }
            None if can_open => open.push((index, kind)),
            None => {}
        }
    }
}

fn set_matched(token: &mut Token, opens: bool) {
    if let Token::Marker { matched, .. } = token {
        *matched = Some(opens);
    }
}

fn build_runs(tokens: Vec<Token>) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let mut emphasis = Emphasis::default();
    for token in tokens {
        match token {
            Token::Text(text) => push(&mut runs, &text, emphasis),
            Token::Marker {
                kind,
                matched: Some(_),
                ..
            } => match kind {
                Kind::Bold => emphasis.bold = !emphasis.bold,
                Kind::Italic => emphasis.italic = !emphasis.italic,
            },
            Token::Marker {
                kind, underscore, ..
            } => push(&mut runs, kind.marker(underscore), emphasis),
        }
    }
    runs
}

fn push(runs: &mut Vec<Run>, text: &str, emphasis: Emphasis) {
    match runs.last_mut() {
        Some(last) if last.emphasis == emphasis => last.text.push_str(text),
        _ => runs.push(Run {
            text: text.to_string(),
            emphasis,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Emphasis = Emphasis {
        bold: false,
        italic: false,
    };
    const BOLD: Emphasis = Emphasis {
        bold: true,
        italic: false,
    };
    const ITALIC: Emphasis = Emphasis {
        bold: false,
        italic: true,
    };
    const BOTH: Emphasis = Emphasis {
        bold: true,
        italic: true,
    };

    fn run(text: &str, emphasis: Emphasis) -> Run {
        Run {
            text: text.to_string(),
            emphasis,
        }
    }

    #[test]
    fn stars_and_underscores_mark_emphasis() {
        assert_eq!(
            parse("a *b* **c** _d_ e"),
            vec![
                run("a ", PLAIN),
                run("b", ITALIC),
                run(" ", PLAIN),
                run("c", BOLD),
                run(" ", PLAIN),
                run("d", ITALIC),
                run(" e", PLAIN),
            ]
        );
        assert_eq!(parse("***x***"), vec![run("x", BOTH)]);
        assert_eq!(
            parse("**bold *and italic* bold**"),
            vec![
                run("bold ", BOLD),
                run("and italic", BOTH),
                run(" bold", BOLD),
            ]
        );
    }

    #[test]
    fn markers_work_inside_words_and_next_to_punctuation() {
        assert_eq!(
            parse("un*believ*able"),
            vec![run("un", PLAIN), run("believ", ITALIC), run("able", PLAIN)]
        );
        assert_eq!(
            parse("Run, *now*!"),
            vec![run("Run, ", PLAIN), run("now", ITALIC), run("!", PLAIN)]
        );
    }

    #[test]
    fn stray_markers_stay_literal() {
        assert_eq!(parse("5 * 3 = 15"), vec![run("5 * 3 = 15", PLAIN)]);
        assert_eq!(
            parse("snake_case_name"),
            vec![run("snake_case_name", PLAIN)]
        );
        assert_eq!(parse("*open"), vec![run("*open", PLAIN)]);
        assert_eq!(parse("close*"), vec![run("close*", PLAIN)]);
        assert_eq!(parse("a ** b"), vec![run("a ** b", PLAIN)]);
        assert_eq!(parse("****"), vec![run("****", PLAIN)]);
        assert_eq!(parse("__init__"), vec![run("__init__", PLAIN)]);
    }

    #[test]
    fn a_closer_drops_the_openers_it_skips() {
        assert_eq!(
            parse("*a **b* c"),
            vec![run("a **b", ITALIC), run(" c", PLAIN)]
        );
    }

    #[test]
    fn plain_strips_matched_markers_only() {
        assert_eq!(plain("a *b* **c** _d_"), "a b c d");
        assert_eq!(plain("5 * 3 *x"), "5 * 3 *x");
        assert_eq!(plain(""), "");
    }
}
