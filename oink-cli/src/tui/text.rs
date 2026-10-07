//! Styled lines for the view: prose, checks, notices, choices, and the
//! status bar. Everything here is a pure function of the data and the theme.

use oink_core::oink_rulebook::RollKind;
use oink_core::CheckRecord;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::markup::Run;
use crate::wrap;

use super::app::{Notice, NoticeKind, Page, Status};
use super::theme::Theme;

/// A prose paragraph, wrapped.
pub fn paragraph_lines(runs: &[Run], width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let pieces = pieces(runs, theme.prose);
    wrap_lines(&pieces, width)
}

/// The prose, checks, and notices of a page as one list of lines, with a
/// blank line between paragraphs and before the records block.
pub fn page_lines(page: &Page, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let mut blocks: Vec<Vec<Line<'static>>> = page
        .paragraphs
        .iter()
        .map(|paragraph| paragraph_lines(paragraph, width, theme))
        .collect();
    let mut records = Vec::new();
    for check in &page.checks {
        records.extend(check_lines(check, width, theme));
    }
    for notice in &page.notices {
        records.extend(notice_lines(notice, width, theme));
    }
    if !records.is_empty() {
        blocks.push(records);
    }
    let mut lines = Vec::new();
    for (index, block) in blocks.into_iter().enumerate() {
        if index > 0 {
            lines.push(Line::default());
        }
        lines.extend(block);
    }
    lines
}

/// One check, wrapped: dice, modifiers, score against target, and the
/// outcome in its color.
pub fn check_lines(check: &CheckRecord, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let modifiers = if check.breakdown.entries.is_empty() {
        "no modifiers".to_string()
    } else {
        check
            .breakdown
            .entries
            .iter()
            .map(|entry| format!("{} {:+}", entry.source, entry.value))
            .collect::<Vec<String>>()
            .join(", ")
    };
    let outcome = check.outcome.replace('_', " ");
    let outcome_style = outcome_style(&check.outcome, theme);
    let mut pieces: Vec<(String, Style)> = Vec::new();
    match &check.dice {
        Some(roll) => {
            let dice = match roll.kind {
                RollKind::Percentile => format!("d% {}", roll.total),
                RollKind::Sum => {
                    let faces: Vec<String> = roll
                        .dice
                        .iter()
                        .zip(roll.kept.iter())
                        .map(|(face, kept)| {
                            if *kept {
                                face.to_string()
                            } else {
                                format!("({face})")
                            }
                        })
                        .collect();
                    format!("{} = {}", faces.join(" + "), roll.total)
                }
            };
            pieces.push((
                format!(
                    "✦ {} check: {dice}; {modifiers}; score {} vs {}: ",
                    check.ability, check.score, check.target
                ),
                theme.check,
            ));
            pieces.push((outcome, outcome_style));
            pieces.push((
                format!(", margin {:+}, {} degrees", check.margin, check.degrees),
                theme.check,
            ));
        }
        None => {
            pieces.push((
                format!(
                    "✦ {} sense: {modifiers}; score {} vs {}: ",
                    check.ability, check.score, check.difficulty
                ),
                theme.check,
            ));
            pieces.push((outcome, outcome_style));
        }
    }
    let borrowed: Vec<(&str, Style)> = pieces
        .iter()
        .map(|(text, style)| (text.as_str(), *style))
        .collect();
    wrap_lines(&borrowed, width)
}

fn outcome_style(outcome: &str, theme: &Theme) -> Style {
    match outcome {
        "critical_success" => theme.critical_success,
        "success" | "pass" => theme.success,
        "critical_failure" => theme.critical_failure,
        "failure" | "fail" => theme.failure,
        _ => theme.check,
    }
}

/// One notice, wrapped, with its sign.
pub fn notice_lines(notice: &Notice, width: usize, theme: &Theme) -> Vec<Line<'static>> {
    let (sign, style) = match notice.kind {
        NoticeKind::Gain => ("+ ", theme.gain),
        NoticeKind::Loss => ("- ", theme.loss),
        NoticeKind::Condition => ("~ ", theme.condition),
        NoticeKind::Note => ("· ", theme.note),
    };
    wrap_lines(&[(sign, style), (notice.text.as_str(), style)], width)
}

/// One numbered choice, wrapped, continuation lines indented under the
/// text. The selected choice is highlighted across the full width.
pub fn choice_lines(
    number: usize,
    runs: &[Run],
    selected: bool,
    width: usize,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let label = format!("{}. ", number + 1);
    let indent = label.chars().count();
    let text_width = width.saturating_sub(indent).max(1);
    let pieces = pieces(runs, theme.choice);
    let mut lines = Vec::new();
    for (index, line) in wrap::wrap(&pieces, text_width).into_iter().enumerate() {
        let mut spans = Vec::with_capacity(line.len() + 1);
        let (prefix, prefix_style) = if index == 0 {
            (label.clone(), theme.choice_number)
        } else {
            (" ".repeat(indent), theme.choice)
        };
        spans.push(Span::styled(prefix, prefix_style));
        spans.extend(
            line.into_iter()
                .map(|(text, style)| Span::styled(text, style)),
        );
        let mut line = Line::from(spans);
        if selected {
            let used = line.width();
            if used < width {
                line.push_span(Span::raw(" ".repeat(width - used)));
            }
            for span in &mut line.spans {
                span.style = span.style.patch(theme.choice_selected);
            }
        }
        lines.push(line);
    }
    lines
}

/// The status bar: resources, conditions, and the level on the left, and
/// on the right the first of `hints` that fits.
pub fn status_line(status: &Status, hints: &[&str], width: usize, theme: &Theme) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    let mut first = true;
    let mut separate = |spans: &mut Vec<Span<'static>>| {
        if !first {
            spans.push(Span::styled("  ", theme.status));
        }
        first = false;
    };
    for resource in &status.resources {
        separate(&mut spans);
        spans.push(Span::styled(format!("{} ", resource.name), theme.status));
        let value = match resource.max {
            Some(max) => format!("{}/{max}", resource.value),
            None => resource.value.to_string(),
        };
        spans.push(Span::styled(
            value,
            theme.resource(resource.value, resource.max),
        ));
    }
    for condition in &status.conditions {
        separate(&mut spans);
        spans.push(Span::styled(format!("~ {condition}"), theme.condition));
    }
    if let Some(level) = status.level {
        separate(&mut spans);
        spans.push(Span::styled(format!("Lv {level}"), theme.status));
    }
    let left = Line::from(spans);
    let used = left.width();
    let mut line = left;
    let fitting = hints
        .iter()
        .map(|hint| (hint, hint.chars().count()))
        .find(|(_, hint_width)| used + 2 + hint_width <= width);
    if let Some((hint, hint_width)) = fitting {
        line.push_span(Span::raw(" ".repeat(width - used - hint_width)));
        line.push_span(Span::styled(hint.to_string(), theme.hint));
    }
    line
}

/// Runs as styled fragments on top of `base`.
fn pieces(runs: &[Run], base: Style) -> Vec<(&str, Style)> {
    runs.iter()
        .map(|run| {
            let mut style = base;
            if run.emphasis.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if run.emphasis.italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            (run.text.as_str(), style)
        })
        .collect()
}

fn wrap_lines(pieces: &[(&str, Style)], width: usize) -> Vec<Line<'static>> {
    wrap::wrap(pieces, width)
        .into_iter()
        .map(|line| {
            Line::from(
                line.into_iter()
                    .map(|(text, style)| Span::styled(text, style))
                    .collect::<Vec<Span<'static>>>(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup;
    use crate::tui::app::ResourceStatus;
    use oink_core::oink_rulebook::{Breakdown, DiceRoll};

    fn text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn check(outcome: &str, dice: Option<DiceRoll>) -> CheckRecord {
        let mut breakdown = Breakdown::default();
        breakdown.push("might", 1);
        CheckRecord {
            ability: "might".to_string(),
            difficulty: 7,
            outcome: outcome.to_string(),
            pool: "2d6".to_string(),
            score: 9,
            target: 7,
            margin: 2,
            degrees: 1,
            dice,
            breakdown,
        }
    }

    #[test]
    fn prose_keeps_emphasis_through_wrapping() {
        let theme = Theme::colored();
        let runs = markup::parse("A door, and a *long* corridor behind it.");
        let lines = paragraph_lines(&runs, 20, &theme);
        assert_eq!(
            lines.iter().map(text).collect::<Vec<String>>(),
            vec!["A door, and a long", "corridor behind it."]
        );
        let italic = lines[0]
            .spans
            .iter()
            .find(|span| span.content == "long")
            .expect("the emphasized word is its own span");
        assert!(italic.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn active_checks_show_dice_and_color_the_outcome() {
        let theme = Theme::colored();
        let roll = DiceRoll {
            kind: RollKind::Sum,
            dice: vec![3, 5, 2],
            kept: vec![true, true, false],
            total: 8,
        };
        let lines = check_lines(&check("critical_success", Some(roll)), 200, &theme);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            text(&lines[0]),
            "✦ might check: 3 + 5 + (2) = 8; might +1; score 9 vs 7: critical success, margin +2, 1 degrees"
        );
        let outcome = lines[0]
            .spans
            .iter()
            .find(|span| span.content == "critical success")
            .unwrap();
        assert_eq!(outcome.style, theme.critical_success);
    }

    #[test]
    fn passive_checks_show_pass_or_fail() {
        let theme = Theme::colored();
        let lines = check_lines(&check("fail", None), 200, &theme);
        assert_eq!(
            text(&lines[0]),
            "✦ might sense: might +1; score 9 vs 7: fail"
        );
        let outcome = lines[0].spans.last().unwrap();
        assert_eq!(outcome.style, theme.failure);
    }

    #[test]
    fn notices_carry_their_sign() {
        let theme = Theme::colored();
        let notice = Notice {
            kind: NoticeKind::Loss,
            text: "Health 10 → 3".to_string(),
        };
        let lines = notice_lines(&notice, 80, &theme);
        assert_eq!(text(&lines[0]), "- Health 10 → 3");
        assert_eq!(lines[0].spans[0].style, theme.loss);
    }

    #[test]
    fn choices_number_the_first_line_and_indent_the_rest() {
        let theme = Theme::colored();
        let runs = markup::parse("Open the heavy door and step through");
        let lines = choice_lines(0, &runs, false, 20, &theme);
        assert_eq!(
            lines.iter().map(text).collect::<Vec<String>>(),
            vec!["1. Open the heavy", "   door and step", "   through"]
        );
        assert_eq!(lines[0].spans[0].style, theme.choice_number);

        let selected = choice_lines(1, &runs, true, 20, &theme);
        assert!(selected.iter().all(|line| line.width() == 20));
        assert!(selected[0]
            .spans
            .iter()
            .all(|span| span.style.add_modifier.contains(Modifier::REVERSED)));
    }

    #[test]
    fn the_status_bar_lists_the_sheet_and_right_aligns_the_hints() {
        let theme = Theme::colored();
        let status = Status {
            resources: vec![ResourceStatus {
                name: "Health".to_string(),
                value: 3,
                max: Some(10),
            }],
            conditions: vec!["Bruised".to_string()],
            level: Some(2),
        };
        let line = status_line(&status, &["press q to quit", "q quit"], 40, &theme);
        assert_eq!(text(&line), " Health 3/10  ~ Bruised  Lv 2     q quit");
        assert_eq!(line.width(), 40);
        let value = line
            .spans
            .iter()
            .find(|span| span.content == "3/10")
            .unwrap();
        assert_eq!(value.style, theme.resource_half);

        let wide = status_line(&status, &["press q to quit", "q quit"], 50, &theme);
        assert!(text(&wide).ends_with("press q to quit"), "{}", text(&wide));

        let narrow = status_line(&status, &["press q to quit", "q quit"], 30, &theme);
        assert_eq!(text(&narrow), " Health 3/10  ~ Bruised  Lv 2");
    }
}
