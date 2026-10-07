//! Word wrap for styled text. One algorithm serves the plain transcript and
//! the full-screen player.

/// Wrap `pieces`, text fragments each with a style, into lines of at most
/// `width` characters. Words split on whitespace, including whitespace at a
/// fragment boundary, and join with one space. A word longer than the width
/// keeps its own line. Adjacent fragments with the same style merge.
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
        let word_length: usize = word.iter().map(|(text, _)| text.chars().count()).sum();
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
    fn breaks_on_words_and_counts_characters() {
        assert_eq!(
            wrap_plain("one two three four", 9),
            vec!["one two", "three", "four"]
        );
        assert_eq!(wrap_plain("è è è", 3), vec!["è è", "è"]);
        assert_eq!(
            wrap_plain("supercalifragilistic is long", 5),
            vec!["supercalifragilistic", "is", "long"]
        );
        assert!(wrap_plain("   ", 10).is_empty());
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
