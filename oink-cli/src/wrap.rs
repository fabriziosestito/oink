//! Word wrap for styled text. One algorithm serves the plain transcript and
//! the full-screen player.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Wrap `pieces`, text fragments each with a style, into lines of at most
/// `width` display cells. Words split on whitespace, including whitespace at
/// a fragment boundary, and join with one space. A word wider than `width`
/// starts on a new line and splits into chunks of at most `width` cells.
/// Adjacent fragments with the same style merge.
pub fn wrap<S: Copy + PartialEq>(pieces: &[(&str, S)], width: usize) -> Vec<Vec<(String, S)>> {
    let mut words: Vec<Vec<(String, S)>> = Vec::new();
    let mut word: Vec<(String, S)> = Vec::new();
    for &(text, style) in pieces {
        for ch in text.chars() {
            if ch.is_whitespace() {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            } else {
                push(&mut word, ch.encode_utf8(&mut [0; 4]), style);
            }
        }
    }
    if !word.is_empty() {
        words.push(word);
    }

    let mut lines = Vec::new();
    let mut line: Vec<(String, S)> = Vec::new();
    let mut length = 0;
    for word in words {
        let word_length = word_width(&word);
        if word_length > width {
            if length > 0 {
                lines.push(std::mem::take(&mut line));
                length = 0;
            }
            for chunk in split_word(&word, width) {
                if length > 0 {
                    lines.push(std::mem::take(&mut line));
                }
                length = word_width(&chunk);
                line = chunk;
            }
            continue;
        }
        if length > 0 && length + 1 + word_length > width {
            lines.push(std::mem::take(&mut line));
            length = 0;
        }
        if let Some(style) = line.last().map(|(_, style)| *style) {
            push(&mut line, " ", style);
            length += 1;
        }
        for (text, style) in word {
            push(&mut line, &text, style);
        }
        length += word_length;
    }
    if length > 0 {
        lines.push(line);
    }
    lines
}

/// Wrap plain text.
pub fn wrap_plain(text: &str, width: usize) -> Vec<String> {
    wrap(&[(text, ())], width)
        .into_iter()
        .map(|line| line.into_iter().map(|(text, ())| text).collect())
        .collect()
}

/// The display width of a word, the sum of its fragments.
fn word_width<S>(word: &[(String, S)]) -> usize {
    word.iter().map(|(text, _)| text.width()).sum()
}

/// Split a word wider than `width` into chunks of at most `width` display
/// cells. The cut falls between characters, and each fragment keeps its
/// style. A character wider than `width` keeps its own chunk.
fn split_word<S: Copy + PartialEq>(word: &[(String, S)], width: usize) -> Vec<Vec<(String, S)>> {
    let mut chunks = Vec::new();
    let mut chunk: Vec<(String, S)> = Vec::new();
    let mut chunk_width = 0;
    for (text, style) in word {
        for ch in text.chars() {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if !chunk.is_empty() && chunk_width + char_width > width {
                chunks.push(std::mem::take(&mut chunk));
                chunk_width = 0;
            }
            push(&mut chunk, ch.encode_utf8(&mut [0; 4]), *style);
            chunk_width += char_width;
        }
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}

fn push<S: Copy + PartialEq>(line: &mut Vec<(String, S)>, text: &str, style: S) {
    match line.last_mut() {
        Some((last, last_style)) if *last_style == style => last.push_str(text),
        _ => line.push((text.to_string(), style)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_on_words_and_counts_display_cells() {
        assert_eq!(
            wrap_plain("one two three four", 9),
            vec!["one two", "three", "four"]
        );
        assert_eq!(wrap_plain("è è è", 3), vec!["è è", "è"]);
        assert_eq!(
            wrap_plain("supercalifragilistic is long", 5),
            vec!["super", "calif", "ragil", "istic", "is", "long"]
        );
        assert!(wrap_plain("   ", 10).is_empty());
    }

    #[test]
    fn oversized_words_split_into_rows() {
        let ninety = "a".repeat(90);
        assert_eq!(
            wrap_plain(&ninety, 80),
            vec!["a".repeat(80), "a".repeat(10)]
        );
        assert_eq!(wrap_plain("abcabcdef", 5), vec!["abcab", "cdef"]);
        assert_eq!(wrap_plain("a abcdef", 5), vec!["a", "abcde", "f"]);
    }

    #[test]
    fn splitting_keeps_fragment_styles() {
        let lines = wrap(&[("un", 'a'), ("believ", 'b'), ("able", 'a')], 5);
        assert_eq!(
            lines,
            vec![
                vec![("un".to_string(), 'a'), ("bel".to_string(), 'b')],
                vec![("iev".to_string(), 'b'), ("ab".to_string(), 'a')],
                vec![("le".to_string(), 'a')],
            ]
        );
    }

    #[test]
    fn wide_characters_take_two_cells() {
        assert_eq!(wrap_plain("界 界", 4), vec!["界", "界"]);
        assert_eq!(wrap_plain("界 界", 5), vec!["界 界"]);
        assert_eq!(wrap_plain("界界", 3), vec!["界", "界"]);
        assert_eq!(wrap_plain("界界界", 5), vec!["界界", "界"]);
        assert_eq!(wrap_plain("a 界", 4), vec!["a 界"]);
        assert_eq!(wrap_plain("a 界", 3), vec!["a", "界"]);
    }

    #[test]
    fn styled_fragments_keep_their_style_and_merge() {
        let lines = wrap(&[("one ", 'a'), ("two", 'b'), (" three", 'a')], 7);
        assert_eq!(
            lines,
            vec![
                vec![("one ".to_string(), 'a'), ("two".to_string(), 'b')],
                vec![("three".to_string(), 'a')],
            ]
        );
    }

    #[test]
    fn a_style_change_inside_a_word_does_not_split_it() {
        let lines = wrap(&[("un", 'a'), ("believ", 'b'), ("able ok", 'a')], 20);
        assert_eq!(
            lines,
            vec![vec![
                ("un".to_string(), 'a'),
                ("believ".to_string(), 'b'),
                ("able ok".to_string(), 'a'),
            ]]
        );
    }

    #[test]
    fn whitespace_at_a_fragment_boundary_separates_words() {
        let lines = wrap(&[("one", 'a'), (" ", 'a'), ("two", 'b')], 20);
        assert_eq!(
            lines,
            vec![vec![("one ".to_string(), 'a'), ("two".to_string(), 'b')]]
        );
    }
}
