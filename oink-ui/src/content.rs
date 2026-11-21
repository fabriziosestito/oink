//! The center page: prose, checks, choices, and pagination.
//!
//! The model is pure data. It parses the shared emphasis markup and wraps
//! runs to a column count, so the renderer and the tests agree on the same
//! lines without touching pixels.

use std::collections::VecDeque;

use oink_core::markup::{self, Emphasis, Run};
use oink_core::oink_rulebook::{Character, Rulebook};
use oink_core::{CheckRecord, Event};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::config::{ReaderConfig, SheetGroup, TopBar};

/// A screen the reader can show.
///
/// The reader shows the story page. The other screens are the panels: the
/// character sheet, the inventory, and the session log. `Menu` holds the save
/// and load slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Reader,
    Sheet,
    Inventory,
    Log,
    Menu,
}

/// The tone of a span, mapped to a theme colour by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Body text.
    Ink,
    /// Checks and secondary text.
    Dim,
    /// Choice numbers and other accents.
    Accent,
}

/// A stretch of text with one emphasis and one tone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub emphasis: Emphasis,
    pub tone: Tone,
}

/// One wrapped line on the center page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Line {
    pub spans: Vec<Span>,
    /// The zero-based choice this line belongs to, when it is a choice line.
    pub choice: Option<usize>,
    /// The inventory item this line belongs to, when it is an item line.
    pub item: Option<String>,
    /// Start a new page before this line even when the page has room.
    pub page_break: bool,
}

impl Line {
    /// A blank line that acts as a paragraph break.
    pub fn blank() -> Self {
        Self::default()
    }

    /// The same line, starting a new page.
    pub fn start_page(mut self) -> Self {
        self.page_break = true;
        self
    }

    /// The display width of the line, in character cells.
    pub fn width(&self) -> usize {
        self.spans.iter().map(|span| span.text.width()).sum()
    }

    /// The line as plain text, for tests and accessibility hooks.
    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }

    /// True when the line has no visible characters.
    pub fn is_blank(&self) -> bool {
        self.spans.iter().all(|span| span.text.trim().is_empty())
    }
}

/// One scene ready for the center zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scene {
    /// The scene mode from the `# mode: <id>` tag, `prose` by default.
    pub mode: String,
    /// The character speaking, from the `# speaker: <id>` tag.
    pub speaker: Option<String>,
    /// The portrait variant, from the `# mood: <id>` tag.
    pub mood: Option<String>,
    /// The portrait override, from the `# portrait: <id>` tag.
    pub portrait: Option<String>,
    /// The map layout, from the `# map: <id>` tag.
    pub map: Option<String>,
    /// The scene picture, from the `# image: <file>` tag, relative to the
    /// bundle directory.
    pub image: Option<String>,
    /// Prose paragraphs, each parsed into emphasis runs.
    pub paragraphs: Vec<Vec<Run>>,
    /// Checks made since the previous step.
    pub checks: Vec<CheckRecord>,
    /// Choice text, each parsed into emphasis runs.
    pub choices: Vec<Vec<Run>>,
    /// True for the story ending.
    pub ending: bool,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            mode: oink_core::DEFAULT_MODE.to_string(),
            speaker: None,
            mood: None,
            portrait: None,
            map: None,
            image: None,
            paragraphs: Vec::new(),
            checks: Vec::new(),
            choices: Vec::new(),
            ending: false,
        }
    }
}

impl Scene {
    /// Parse an engine event and its checks into display data.
    pub fn from_event(event: &Event, checks: Vec<CheckRecord>) -> Self {
        match event {
            Event::Scene {
                mode,
                speaker,
                mood,
                portrait,
                map,
                image,
                text,
                choices,
            } => Self {
                mode: mode.clone(),
                speaker: speaker.clone(),
                mood: mood.clone(),
                portrait: portrait.clone(),
                map: map.clone(),
                image: image.clone(),
                paragraphs: text
                    .iter()
                    .map(|paragraph| markup::parse(paragraph))
                    .collect(),
                checks,
                choices: choices
                    .iter()
                    .map(|choice| markup::parse(&choice.text))
                    .collect(),
                ending: false,
            },
            Event::TheEnd { text } => Self {
                paragraphs: text
                    .iter()
                    .map(|paragraph| markup::parse(paragraph))
                    .collect(),
                checks,
                choices: Vec::new(),
                ending: true,
                ..Self::default()
            },
        }
    }

    /// The full page flow: prose, checks, then numbered choices.
    ///
    /// Blocks are separated by one blank line. A paragraph with no text adds
    /// nothing. The column count is clamped to at least one.
    pub fn lines(&self, columns: usize) -> Vec<Line> {
        let columns = columns.max(1);
        let mut lines = Vec::new();
        for paragraph in &self.paragraphs {
            // A whitespace-only paragraph wraps to no lines. Skip it before
            // adding the separator, or it leaves two blank lines behind.
            let wrapped = wrap_runs(paragraph, columns, Tone::Ink);
            if wrapped.is_empty() {
                continue;
            }
            if !lines.is_empty() {
                lines.push(Line::blank());
            }
            lines.extend(wrapped);
        }
        if !self.checks.is_empty() {
            if !lines.is_empty() {
                lines.push(Line::blank());
            }
            for check in &self.checks {
                // The record uses underscores for Ink outcome names.
                let text = check.describe().replace('_', " ");
                lines.extend(wrap_text(&text, columns, Tone::Dim));
            }
        }
        if !self.choices.is_empty() {
            if !lines.is_empty() {
                lines.push(Line::blank());
            }
            for (index, choice) in self.choices.iter().enumerate() {
                lines.extend(choice_lines(index + 1, choice, columns));
            }
        }
        lines
    }
}

/// One labelled value in the top bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarValue {
    pub label: String,
    pub value: String,
}

impl BarValue {
    /// The label and the value as one run of text.
    pub fn text(&self) -> String {
        format!("{} {}", self.label, self.value)
    }
}

/// The top summary bar: resources on the left, characteristics on the right.
///
/// The values come from the live character sheet and the ids listed in
/// `layout.top_bar`. The renderer places the two groups at the bar edges and
/// drops what does not fit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Header {
    pub resources: Vec<BarValue>,
    pub characteristics: Vec<BarValue>,
}

impl Header {
    /// Read the configured values from the rulebook and the character.
    ///
    /// Unknown ids are skipped: [`crate::config::UiConfig::validate`] reports
    /// them before the player starts.
    pub fn new(rulebook: &Rulebook, character: &Character, config: &TopBar) -> Self {
        let resources = config
            .resources
            .iter()
            .filter_map(|id| {
                let definition = rulebook.resources.get(id)?;
                let value = character.resource(id)?;
                let max = character.resource_max(rulebook, id)?;
                Some(BarValue {
                    label: definition.name.clone(),
                    value: format!("{value}/{max}"),
                })
            })
            .collect();
        let characteristics = config
            .characteristics
            .iter()
            .filter_map(|id| {
                let definition = rulebook.characteristics.get(id)?;
                let value = character.characteristic(id)?;
                Some(BarValue {
                    label: definition.name.clone(),
                    value: value.to_string(),
                })
            })
            .collect();
        Self {
            resources,
            characteristics,
        }
    }

    /// The resource values as one line, in configuration order.
    pub fn resources_text(&self) -> String {
        join(&self.resources)
    }

    /// The characteristic values as one line, in configuration order.
    pub fn characteristics_text(&self) -> String {
        join(&self.characteristics)
    }

    /// True when the bar has no values to show.
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty() && self.characteristics.is_empty()
    }
}

fn join(values: &[BarValue]) -> String {
    values
        .iter()
        .map(BarValue::text)
        .collect::<Vec<_>>()
        .join("  ")
}

/// A titled body for one reader view, wrapped to the center column.
///
/// The host keeps one panel per open view, feeds it to a [`Pager`], and draws
/// it with the panel renderer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub title: String,
    pub lines: Vec<Line>,
}

impl Panel {
    /// The character sheet: one heading per configured group and its entries.
    ///
    /// Characteristics and abilities show their value, perks and conditions
    /// their name, and spells their ability and cost. Entries are sorted by
    /// name. An empty group shows `None` so the configured order stays
    /// visible.
    pub fn sheet(
        rulebook: &Rulebook,
        character: &Character,
        config: &ReaderConfig,
        columns: usize,
    ) -> Self {
        let mut lines = Vec::new();
        for (index, group) in config.sheet_order.iter().enumerate() {
            let mut heading = wrap_text(group.label(), columns, Tone::Accent);
            if index > 0 {
                if let Some(first) = heading.first_mut() {
                    first.page_break = true;
                }
            }
            lines.extend(heading);
            let mut entries = sheet_entries(*group, rulebook, character);
            entries.sort();
            if entries.is_empty() {
                lines.extend(wrap_text("None", columns, Tone::Dim));
            } else {
                for entry in entries {
                    lines.extend(wrap_text(&entry, columns, Tone::Ink));
                }
            }
        }
        Self {
            title: "Sheet".to_string(),
            lines,
        }
    }

    /// The session log, with entries in display order, newest first.
    ///
    /// Each entry wraps on its own. An empty log shows a notice.
    pub fn log<'a>(entries: impl IntoIterator<Item = &'a str>, columns: usize) -> Self {
        let mut lines = Vec::new();
        for entry in entries {
            lines.extend(wrap_text(entry, columns, Tone::Ink));
        }
        if lines.is_empty() {
            lines.extend(wrap_text("No entries yet.", columns, Tone::Dim));
        }
        Self {
            title: "Log".to_string(),
            lines,
        }
    }

    /// The carried items, sorted by name and split into `items_per_page`
    /// groups.
    ///
    /// An item shows its name, its count when the stack holds more than one
    /// unit, and its unit weight when the rulebook gives one. An empty
    /// inventory shows a notice.
    pub fn inventory(
        rulebook: &Rulebook,
        character: &Character,
        config: &ReaderConfig,
        columns: usize,
    ) -> Self {
        let mut items: Vec<(String, String, u32, u32)> = character
            .item_stacks()
            .filter_map(|(id, count)| {
                let definition = rulebook.items.get(id)?;
                Some((
                    id.to_string(),
                    definition.name.clone(),
                    count,
                    definition.weight,
                ))
            })
            .collect();
        items.sort_by(|left, right| (&left.1, &left.0).cmp(&(&right.1, &right.0)));

        let per_page = config.items_per_page.max(1);
        let mut lines = Vec::new();
        for (index, (id, name, count, weight)) in items.into_iter().enumerate() {
            let mut entry = name;
            if count > 1 {
                entry.push_str(&format!(" x{count}"));
            }
            if weight > 0 {
                entry.push_str(&format!(" ({weight})"));
            }
            let mut wrapped = wrap_text(&entry, columns, Tone::Ink);
            for line in &mut wrapped {
                line.item = Some(id.clone());
            }
            if index > 0 && index % per_page == 0 {
                if let Some(first) = wrapped.first_mut() {
                    first.page_break = true;
                }
            }
            lines.extend(wrapped);
        }
        if lines.is_empty() {
            lines.extend(wrap_text("Empty inventory.", columns, Tone::Dim));
        }
        Self {
            title: "Inventory".to_string(),
            lines,
        }
    }
}

fn sheet_entries(group: SheetGroup, rulebook: &Rulebook, character: &Character) -> Vec<String> {
    match group {
        SheetGroup::Characteristics => rulebook
            .characteristics
            .iter()
            .filter_map(|(id, definition)| {
                let value = character.characteristic(id)?;
                Some(format!("{} {value}", definition.name))
            })
            .collect(),
        SheetGroup::Abilities => character
            .ability_ids()
            .map(|id| {
                let name = rulebook
                    .abilities
                    .get(id)
                    .map(|ability| ability.name.as_str())
                    .unwrap_or(id);
                format!("{name} {}", character.ability_level(id))
            })
            .collect(),
        SheetGroup::Perks => character
            .perk_ids()
            .map(|id| {
                rulebook
                    .perks
                    .get(id)
                    .map(|perk| perk.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect(),
        SheetGroup::Conditions => character
            .condition_ids()
            .map(|id| {
                rulebook
                    .conditions
                    .get(id)
                    .map(|condition| condition.name.clone())
                    .unwrap_or_else(|| id.clone())
            })
            .collect(),
        SheetGroup::Spells => rulebook
            .spells
            .values()
            .map(|spell| {
                let ability = rulebook
                    .abilities
                    .get(&spell.ability)
                    .map(|ability| ability.name.as_str())
                    .unwrap_or(&spell.ability);
                let resource = rulebook
                    .resources
                    .get(&spell.cost.resource)
                    .map(|resource| resource.name.as_str())
                    .unwrap_or(&spell.cost.resource);
                format!(
                    "{} ({ability}, {} {resource})",
                    spell.name, spell.cost.amount
                )
            })
            .collect(),
    }
}

/// A host-side ring buffer of log entries, oldest first.
///
/// The host pushes one entry per notice or state change. Once the buffer is
/// full, the oldest entry drops. [`Log::panel`] shows the entries newest
/// first, and the pager splits them by rows like any other panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log {
    entries: VecDeque<String>,
    capacity: usize,
}

impl Log {
    /// A log that keeps at most `capacity` entries. Zero drops every entry.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            capacity,
        }
    }

    /// A log sized by `reader.log_entries`.
    pub fn from_config(config: &ReaderConfig) -> Self {
        Self::new(config.log_entries)
    }

    /// Record one entry, dropping the oldest when the buffer is full.
    pub fn push(&mut self, entry: impl Into<String>) {
        if self.capacity == 0 {
            return;
        }
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(entry.into());
    }

    /// The entries, oldest first.
    pub fn entries(&self) -> impl DoubleEndedIterator<Item = &str> {
        self.entries.iter().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// The log as a panel, newest entry first.
    pub fn panel(&self, columns: usize) -> Panel {
        Panel::log(self.entries().rev(), columns)
    }
}

/// One page of wrapped lines.
///
/// The host owns the pager, feeds it to the renderer, and advances it when
/// the player pages up or down. A new scene replaces the lines and returns
/// to the first page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pager {
    lines: Vec<Line>,
    rows: usize,
    page: usize,
}

impl Pager {
    /// A pager over `lines`, with `rows` lines per page.
    pub fn new(lines: Vec<Line>, rows: usize) -> Self {
        Self {
            lines,
            rows,
            page: 0,
        }
    }

    /// A pager over a scene at the given text size.
    pub fn from_scene(scene: &Scene, columns: usize, rows: usize) -> Self {
        Self::new(scene.lines(columns), rows)
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub const fn rows(&self) -> usize {
        self.rows
    }

    /// Replace the content and return to the first page.
    pub fn set_lines(&mut self, lines: Vec<Line>) {
        self.lines = lines;
        self.page = 0;
        self.clamp();
    }

    /// Change the page height and keep the current page in range.
    pub fn set_rows(&mut self, rows: usize) {
        self.rows = rows;
        self.clamp();
    }

    /// The current zero-based page.
    pub const fn page(&self) -> usize {
        self.page
    }

    /// Show `page`, clamped to the available pages.
    pub fn set_page(&mut self, page: usize) {
        self.page = page;
        self.clamp();
    }

    /// The zero-based page that holds `line`.
    pub fn page_of_line(&self, line: usize) -> usize {
        self.page_starts()
            .iter()
            .rposition(|start| *start <= line)
            .unwrap_or(0)
    }

    /// The number of pages. Zero rows means no page. An explicit line break
    /// and the row limit both start a new page.
    pub fn pages(&self) -> usize {
        self.page_starts().len()
    }

    /// The first line index of every page.
    fn page_starts(&self) -> Vec<usize> {
        if self.rows == 0 {
            return Vec::new();
        }
        let mut starts = vec![0];
        let mut count = 1;
        for (index, line) in self.lines.iter().enumerate().skip(1) {
            if line.page_break || count == self.rows {
                starts.push(index);
                count = 0;
            }
            count += 1;
        }
        starts
    }

    pub fn has_previous(&self) -> bool {
        self.page > 0
    }

    pub fn has_next(&self) -> bool {
        self.page + 1 < self.pages()
    }

    /// Show the next page. Returns false at the last page.
    pub fn next_page(&mut self) -> bool {
        if self.has_next() {
            self.page += 1;
            true
        } else {
            false
        }
    }

    /// Show the previous page. Returns false at the first page.
    pub fn previous_page(&mut self) -> bool {
        if self.has_previous() {
            self.page -= 1;
            true
        } else {
            false
        }
    }

    /// The lines on the current page.
    pub fn current(&self) -> &[Line] {
        let starts = self.page_starts();
        let start = starts
            .get(self.page)
            .copied()
            .unwrap_or(self.lines.len())
            .min(self.lines.len());
        let end = starts
            .get(self.page + 1)
            .copied()
            .unwrap_or(self.lines.len())
            .min(self.lines.len());
        &self.lines[start..end]
    }

    fn clamp(&mut self) {
        self.page = self.page.min(self.pages().saturating_sub(1));
    }
}

fn wrap_runs(runs: &[Run], columns: usize, tone: Tone) -> Vec<Line> {
    let pieces: Vec<(String, Emphasis)> = runs
        .iter()
        .map(|run| (run.text.clone(), run.emphasis))
        .collect();
    wrap(&pieces, columns)
        .into_iter()
        .map(|line| Line {
            spans: line
                .into_iter()
                .map(|(text, emphasis)| Span {
                    text,
                    emphasis,
                    tone,
                })
                .collect(),
            choice: None,
            item: None,
            page_break: false,
        })
        .collect()
}

fn wrap_text(text: &str, columns: usize, tone: Tone) -> Vec<Line> {
    wrap(&[(text.to_string(), Emphasis::default())], columns)
        .into_iter()
        .map(|line| Line {
            spans: line
                .into_iter()
                .map(|(text, emphasis)| Span {
                    text,
                    emphasis,
                    tone,
                })
                .collect(),
            choice: None,
            item: None,
            page_break: false,
        })
        .collect()
}

fn choice_lines(number: usize, runs: &[Run], columns: usize) -> Vec<Line> {
    let label = format!("{number}. ");
    let indent = label.width();
    let text_width = columns.saturating_sub(indent).max(1);
    let pieces: Vec<(String, Emphasis)> = runs
        .iter()
        .map(|run| (run.text.clone(), run.emphasis))
        .collect();
    let wrapped = wrap(&pieces, text_width);
    if wrapped.is_empty() {
        return vec![Line {
            spans: vec![Span {
                text: format!("{number}."),
                emphasis: Emphasis::default(),
                tone: Tone::Accent,
            }],
            choice: Some(number.saturating_sub(1)),
            item: None,
            page_break: false,
        }];
    }
    wrapped
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let mut spans = Vec::with_capacity(line.len() + 1);
            if index == 0 {
                spans.push(Span {
                    text: label.clone(),
                    emphasis: Emphasis::default(),
                    tone: Tone::Accent,
                });
            } else {
                spans.push(Span {
                    text: " ".repeat(indent),
                    emphasis: Emphasis::default(),
                    tone: Tone::Ink,
                });
            }
            spans.extend(line.into_iter().map(|(text, emphasis)| Span {
                text,
                emphasis,
                tone: Tone::Ink,
            }));
            Line {
                spans,
                choice: Some(number.saturating_sub(1)),
                item: None,
                page_break: false,
            }
        })
        .collect()
}

/// Wrap styled pieces into lines of at most `columns` cells.
///
/// Words split on whitespace and join with one space. A word wider than the
/// line starts a new line and splits into chunks. Adjacent pieces with the
/// same emphasis merge.
fn wrap<S: Copy + PartialEq>(pieces: &[(String, S)], columns: usize) -> Vec<Vec<(String, S)>> {
    let columns = columns.max(1);
    let mut words: Vec<Vec<(String, S)>> = Vec::new();
    let mut word: Vec<(String, S)> = Vec::new();
    for (text, style) in pieces {
        for ch in text.chars() {
            if ch.is_whitespace() {
                if !word.is_empty() {
                    words.push(std::mem::take(&mut word));
                }
            } else {
                push(&mut word, ch.encode_utf8(&mut [0; 4]), *style);
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
        if word_length > columns {
            if length > 0 {
                lines.push(std::mem::take(&mut line));
                length = 0;
            }
            for chunk in split_word(&word, columns) {
                if length > 0 {
                    lines.push(std::mem::take(&mut line));
                }
                length = word_width(&chunk);
                line = chunk;
            }
            continue;
        }
        if length > 0 && length + 1 + word_length > columns {
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

fn word_width<S>(word: &[(String, S)]) -> usize {
    word.iter().map(|(text, _)| text.width()).sum()
}

fn split_word<S: Copy + PartialEq>(word: &[(String, S)], columns: usize) -> Vec<Vec<(String, S)>> {
    let mut chunks = Vec::new();
    let mut chunk: Vec<(String, S)> = Vec::new();
    let mut chunk_width = 0;
    for (text, style) in word {
        for ch in text.chars() {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
            if !chunk.is_empty() && chunk_width + char_width > columns {
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
    use oink_core::oink_rulebook::Breakdown;
    use oink_core::Choice;

    fn scene_with(paragraphs: &[&str], checks: Vec<CheckRecord>, choices: &[&str]) -> Scene {
        Scene::from_event(
            &Event::Scene {
                mode: oink_core::DEFAULT_MODE.to_string(),
                speaker: None,
                mood: None,
                portrait: None,
                map: None,
                image: None,
                text: paragraphs.iter().map(|text| text.to_string()).collect(),
                choices: choices
                    .iter()
                    .enumerate()
                    .map(|(index, text)| Choice {
                        index,
                        text: text.to_string(),
                    })
                    .collect(),
            },
            checks,
        )
    }

    #[test]
    fn the_scene_keeps_its_mode() {
        let event = Event::Scene {
            mode: "dialog".to_string(),
            speaker: Some("keeper".to_string()),
            mood: None,
            portrait: None,
            map: Some("valley".to_string()),
            image: Some("art/valley.png".to_string()),
            text: vec!["Hello.".to_string()],
            choices: Vec::new(),
        };
        let scene = Scene::from_event(&event, Vec::new());
        assert_eq!(scene.mode, "dialog");
        assert_eq!(scene.speaker.as_deref(), Some("keeper"));
        assert_eq!(scene.map.as_deref(), Some("valley"));
        assert_eq!(scene.image.as_deref(), Some("art/valley.png"));
        assert_eq!(scene.mood, None);
        assert_eq!(scene.portrait, None);
        assert_eq!(Scene::default().mode, "prose");
        assert_eq!(Scene::default().speaker, None);
    }

    fn check(outcome: &str) -> CheckRecord {
        CheckRecord {
            ability: "logic".to_string(),
            difficulty: 8,
            outcome: outcome.to_string(),
            pool: "standard".to_string(),
            score: 9,
            target: 7,
            margin: 2,
            degrees: 1,
            dice: None,
            breakdown: Breakdown::default(),
        }
    }

    fn text_line(text: &str) -> Line {
        Line {
            spans: vec![Span {
                text: text.to_string(),
                emphasis: Emphasis::default(),
                tone: Tone::Ink,
            }],
            choice: None,
            item: None,
            page_break: false,
        }
    }

    #[test]
    fn markup_is_parsed_in_prose_and_choices() {
        let scene = scene_with(&["A **bold** line."], Vec::new(), &["Take *it*"]);
        assert_eq!(scene.paragraphs[0][1].text, "bold");
        assert!(scene.paragraphs[0][1].emphasis.bold);
        assert_eq!(scene.choices[0][1].text, "it");
        assert!(scene.choices[0][1].emphasis.italic);
        let lines = scene.lines(40);
        assert_eq!(lines[0].text(), "A bold line.");
        assert_eq!(lines[2].text(), "1. Take it");
    }

    #[test]
    fn blocks_are_separated_by_blank_lines() {
        let scene = scene_with(&["First.", "Second."], vec![check("success")], &["Go"]);
        let lines = scene.lines(40);
        assert_eq!(lines[0].text(), "First.");
        assert!(lines[1].is_blank());
        assert_eq!(lines[2].text(), "Second.");
        assert!(lines[3].is_blank());
        assert!(lines[4].text().starts_with("* logic sense:"));
        let choice_line = lines
            .iter()
            .position(|line| line.text().starts_with("1. "))
            .expect("choice line");
        assert!(lines[choice_line - 1].is_blank());
        assert_eq!(lines[choice_line].spans[0].text, "1. ");
        assert_eq!(lines[choice_line].spans[0].tone, Tone::Accent);
    }

    #[test]
    fn checks_come_before_choices_and_use_plain_outcomes() {
        let scene = scene_with(&["A scene."], vec![check("critical_success")], &["Go"]);
        let lines = scene.lines(80);
        let check_line = lines
            .iter()
            .position(|line| line.text().contains("logic sense"))
            .expect("check line");
        let choice_line = lines
            .iter()
            .position(|line| line.text().starts_with("1. "))
            .expect("choice line");
        assert!(check_line < choice_line);
        assert!(lines[check_line].text().contains("critical success"));
    }

    #[test]
    fn choices_wrap_with_a_hanging_indent() {
        let scene = scene_with(
            &[],
            Vec::new(),
            &["A choice that needs more than one line to fit"],
        );
        let lines = scene.lines(20);
        assert_eq!(lines[0].text(), "1. A choice that");
        assert_eq!(lines[1].text(), "   needs more than");
        assert_eq!(lines[1].spans[0].text, "   ");
        assert!(lines.iter().map(|line| line.width()).max().unwrap() <= 20);
        assert!(lines.iter().all(|line| line.choice == Some(0)));
    }

    #[test]
    fn long_words_split_into_rows() {
        let scene = scene_with(&[&"a".repeat(12)], Vec::new(), &[]);
        let lines = scene.lines(5);
        assert_eq!(
            lines.iter().map(Line::text).collect::<Vec<_>>(),
            vec!["aaaaa", "aaaaa", "aa"]
        );
    }

    #[test]
    fn wide_characters_count_as_two_cells() {
        let scene = scene_with(&["界 界"], Vec::new(), &[]);
        let lines = scene.lines(4);
        assert_eq!(
            lines.iter().map(Line::text).collect::<Vec<_>>(),
            vec!["界", "界"]
        );
    }

    #[test]
    fn empty_lines_are_skipped_and_blank_scene_has_no_lines() {
        let scene = scene_with(&[], Vec::new(), &[]);
        assert!(scene.lines(40).is_empty());
        let scene = scene_with(&[""], Vec::new(), &[]);
        assert!(scene.lines(40).is_empty());
    }

    #[test]
    fn a_whitespace_only_paragraph_leaves_one_blank_line() {
        let scene = scene_with(&["First.", "   ", "Second."], Vec::new(), &[]);
        let lines = scene.lines(40);
        assert_eq!(
            lines.iter().map(Line::text).collect::<Vec<_>>(),
            ["First.", "", "Second."]
        );
    }

    #[test]
    fn pager_splits_lines_into_pages() {
        let lines: Vec<Line> = (0..5).map(|index| text_line(&index.to_string())).collect();
        let mut pager = Pager::new(lines, 2);
        assert_eq!(pager.pages(), 3);
        assert_eq!(pager.current().len(), 2);
        assert!(pager.has_next());
        assert!(!pager.has_previous());
        assert!(pager.next_page());
        assert_eq!(pager.page(), 1);
        assert_eq!(pager.current()[0].text(), "2");
        assert!(pager.next_page());
        assert_eq!(pager.current().len(), 1);
        assert!(!pager.next_page());
        assert!(pager.previous_page());
        assert_eq!(pager.page(), 1);
    }

    #[test]
    fn pager_clamps_the_page_when_rows_change() {
        let lines: Vec<Line> = (0..9).map(|index| text_line(&index.to_string())).collect();
        let mut pager = Pager::new(lines, 2);
        pager.next_page();
        pager.next_page();
        pager.next_page();
        assert_eq!(pager.page(), 3);
        pager.set_rows(5);
        assert_eq!(pager.pages(), 2);
        assert_eq!(pager.page(), 1);
        pager.set_rows(0);
        assert_eq!(pager.pages(), 0);
        assert_eq!(pager.page(), 0);
        assert!(pager.current().is_empty());
    }

    #[test]
    fn replacing_the_lines_returns_to_the_first_page() {
        let mut pager = Pager::new(vec![text_line("one"), text_line("two")], 1);
        assert!(pager.next_page());
        pager.set_lines(vec![text_line("three")]);
        assert_eq!(pager.page(), 0);
        assert_eq!(pager.current()[0].text(), "three");
    }

    #[test]
    fn an_empty_scene_has_one_empty_page() {
        let pager = Pager::from_scene(&Scene::default(), 40, 10);
        assert_eq!(pager.pages(), 1);
        assert!(pager.current().is_empty());
    }

    #[test]
    fn an_explicit_break_starts_a_page_early() {
        let lines = vec![
            text_line("one"),
            text_line("two").start_page(),
            text_line("three"),
        ];
        let mut pager = Pager::new(lines, 3);
        assert_eq!(pager.pages(), 2);
        assert_eq!(pager.current().len(), 1);
        assert!(pager.next_page());
        assert_eq!(pager.current()[0].text(), "two");
        assert_eq!(pager.current().len(), 2);
    }

    #[test]
    fn a_break_at_a_full_page_does_not_add_an_empty_page() {
        let lines = vec![
            text_line("one"),
            text_line("two").start_page(),
            text_line("three"),
        ];
        let pager = Pager::new(lines, 2);
        assert_eq!(pager.pages(), 2);
        assert_eq!(pager.current().len(), 1);
    }

    const HEADER_RULEBOOK: &str = r#"
characteristics:
  physique: { name: Physique, min: 1, max: 14, default: 7 }
  intellect: { name: Intellect, min: 1, max: 14, default: 9 }
resources:
  health: { name: Health, min: 0, max: 10 }
  focus: { name: Focus, min: 0, max: 5 }
starting_character:
  characteristics: { physique: 7, intellect: 9 }
  resources: { health: 8, focus: 3 }
"#;

    fn header(config: TopBar) -> Header {
        let rulebook = Rulebook::load(HEADER_RULEBOOK)
            .expect("rulebook loads")
            .rulebook;
        let character = Character::from_starting(&rulebook);
        Header::new(&rulebook, &character, &config)
    }

    #[test]
    fn the_header_reads_resources_and_characteristics() {
        let config = TopBar {
            resources: vec!["health".to_string(), "focus".to_string()],
            characteristics: vec!["intellect".to_string(), "physique".to_string()],
            height: None,
        };
        let header = header(config);
        assert_eq!(header.resources_text(), "Health 8/10  Focus 3/5");
        assert_eq!(header.characteristics_text(), "Intellect 9  Physique 7");
        assert!(!header.is_empty());
    }

    #[test]
    fn the_header_keeps_the_configured_order_and_skips_unknown_ids() {
        let config = TopBar {
            resources: vec!["missing".to_string(), "focus".to_string()],
            characteristics: vec!["physique".to_string()],
            height: None,
        };
        let header = header(config);
        assert_eq!(header.resources_text(), "Focus 3/5");
        assert_eq!(header.characteristics_text(), "Physique 7");
    }

    #[test]
    fn an_empty_header_has_no_text() {
        let header = header(TopBar::default());
        assert!(header.is_empty());
        assert!(header.resources_text().is_empty());
        assert!(header.characteristics_text().is_empty());
    }

    const SHEET_RULEBOOK: &str = r#"
characteristics:
  physique: { name: Physique, min: 1, max: 14, default: 7 }
abilities:
  might: { name: Might, characteristic: physique }
perks:
  sure_footed: { name: Sure-footed }
conditions:
  bruised: { name: Bruised, duration: 2 }
items:
  rope: { name: Rope, weight: 2 }
  torch: { name: Torch, weight: 1 }
  key: { name: Key }
resources:
  focus: { name: Focus, min: 0, max: 5 }
spells:
  fireball:
    name: Fireball
    ability: might
    cost: { resource: focus, amount: 2 }
    check: { difficulty: 8 }
starting_character:
  characteristics: { physique: 7 }
  abilities: { might: 2 }
  perks: [sure_footed]
  inventory: [rope, torch, key]
  resources: { focus: 3 }
"#;

    fn sheet(config: ReaderConfig) -> Panel {
        let rulebook = Rulebook::load(SHEET_RULEBOOK)
            .expect("rulebook loads")
            .rulebook;
        let mut character = Character::from_starting(&rulebook);
        character.add_condition(&rulebook, "bruised");
        Panel::sheet(&rulebook, &character, &config, 80)
    }

    fn inventory(config: ReaderConfig) -> Panel {
        let rulebook = Rulebook::load(SHEET_RULEBOOK)
            .expect("rulebook loads")
            .rulebook;
        let mut character = Character::from_starting(&rulebook);
        character.add_item(&rulebook, "rope");
        Panel::inventory(&rulebook, &character, &config, 80)
    }

    fn texts(panel: &Panel) -> Vec<String> {
        panel.lines.iter().map(Line::text).collect()
    }

    #[test]
    fn the_sheet_lists_every_group_with_its_entries() {
        let panel = sheet(ReaderConfig::default());
        assert_eq!(panel.title, "Sheet");
        let lines = texts(&panel);
        for expected in [
            "Characteristics",
            "Physique 7",
            "Abilities",
            "Might 2",
            "Perks",
            "Sure-footed",
            "Conditions",
            "Bruised",
            "Spells",
            "Fireball (Might, 2 Focus)",
        ] {
            assert!(
                lines.contains(&expected.to_string()),
                "{expected:?} in {lines:?}"
            );
        }
        let heading = panel
            .lines
            .iter()
            .position(|line| line.text() == "Characteristics")
            .unwrap();
        assert_eq!(panel.lines[heading].spans[0].tone, Tone::Accent);
    }

    #[test]
    fn the_sheet_follows_the_configured_group_order() {
        let config = ReaderConfig {
            sheet_order: vec![SheetGroup::Spells, SheetGroup::Characteristics],
            ..ReaderConfig::default()
        };
        let lines = texts(&sheet(config));
        assert_eq!(lines[0], "Spells");
        assert_eq!(lines[1], "Fireball (Might, 2 Focus)");
        assert!(lines.contains(&"Characteristics".to_string()));
        assert!(!lines.contains(&"Perks".to_string()));
    }

    #[test]
    fn an_empty_group_shows_none() {
        let rulebook = Rulebook::load("").expect("rulebook loads").rulebook;
        let character = Character::from_starting(&rulebook);
        let config = ReaderConfig {
            sheet_order: vec![SheetGroup::Spells],
            ..ReaderConfig::default()
        };
        let panel = Panel::sheet(&rulebook, &character, &config, 80);
        assert_eq!(texts(&panel), vec!["Spells", "None"]);
        assert_eq!(panel.lines[1].spans[0].tone, Tone::Dim);
    }

    #[test]
    fn every_sheet_group_starts_a_page() {
        let panel = sheet(ReaderConfig::default());
        let mut pager = Pager::new(panel.lines, 10);
        assert_eq!(pager.current()[0].text(), "Characteristics");
        assert!(pager.next_page());
        assert_eq!(pager.current()[0].text(), "Abilities");
        assert!(pager.next_page());
        assert_eq!(pager.current()[0].text(), "Perks");
    }

    #[test]
    fn the_sheet_pages_without_losing_a_group() {
        let panel = sheet(ReaderConfig::default());
        let mut pager = Pager::new(panel.lines, 3);
        assert!(pager.pages() > 1);
        let mut seen = Vec::new();
        loop {
            seen.extend(pager.current().iter().map(Line::text));
            if !pager.next_page() {
                break;
            }
        }
        let headings: Vec<String> = seen
            .into_iter()
            .filter(|line| line.as_str() == "Characteristics" || line.as_str() == "Spells")
            .collect();
        assert_eq!(headings, vec!["Characteristics", "Spells"]);
    }

    #[test]
    fn the_log_keeps_the_newest_entries_first() {
        let mut log = Log::new(3);
        for entry in ["one", "two", "three", "four"] {
            log.push(entry);
        }
        assert_eq!(log.len(), 3);
        assert_eq!(
            log.entries().collect::<Vec<_>>(),
            vec!["two", "three", "four"]
        );
        let panel = log.panel(80);
        assert_eq!(panel.title, "Log");
        assert_eq!(texts(&panel), vec!["four", "three", "two"]);
    }

    #[test]
    fn a_log_with_no_capacity_keeps_nothing() {
        let mut log = Log::new(0);
        log.push("one");
        assert!(log.is_empty());
        assert_eq!(log.capacity(), 0);
    }

    #[test]
    fn an_empty_log_shows_a_notice() {
        let panel = Log::new(5).panel(40);
        assert_eq!(texts(&panel), vec!["No entries yet."]);
        assert_eq!(panel.lines[0].spans[0].tone, Tone::Dim);
    }

    #[test]
    fn log_entries_wrap_on_their_own() {
        let mut log = Log::new(5);
        log.push("a long entry that needs more than one row to fit");
        let panel = log.panel(20);
        assert!(panel.lines.len() > 1);
        assert!(panel.lines.iter().all(|line| line.width() <= 20));
    }

    #[test]
    fn the_inventory_lists_names_weights_and_stack_counts() {
        let panel = inventory(ReaderConfig::default());
        assert_eq!(panel.title, "Inventory");
        assert_eq!(texts(&panel), vec!["Key", "Rope x2 (2)", "Torch (1)"]);
    }

    #[test]
    fn inventory_lines_name_their_item() {
        let panel = inventory(ReaderConfig::default());
        let items: Vec<&str> = panel
            .lines
            .iter()
            .filter_map(|line| line.item.as_deref())
            .collect();
        assert_eq!(items, vec!["key", "rope", "torch"]);
    }

    #[test]
    fn the_inventory_pages_by_item_count() {
        let config = ReaderConfig {
            items_per_page: 1,
            ..ReaderConfig::default()
        };
        let panel = inventory(config);
        let mut pager = Pager::new(panel.lines, 10);
        assert_eq!(pager.pages(), 3);
        assert_eq!(pager.current()[0].text(), "Key");
        assert!(pager.next_page());
        assert_eq!(pager.current()[0].text(), "Rope x2 (2)");
        assert!(pager.next_page());
        assert_eq!(pager.current()[0].text(), "Torch (1)");
    }

    #[test]
    fn item_count_and_weight_share_one_line() {
        let config = ReaderConfig {
            items_per_page: 2,
            ..ReaderConfig::default()
        };
        let panel = inventory(config);
        let mut pager = Pager::new(panel.lines, 10);
        assert_eq!(pager.current().len(), 2);
        assert!(pager.next_page());
        assert_eq!(pager.current().len(), 1);
        assert_eq!(pager.current()[0].text(), "Torch (1)");
    }

    #[test]
    fn an_empty_inventory_shows_a_notice() {
        let rulebook = Rulebook::load("").expect("rulebook loads").rulebook;
        let character = Character::from_starting(&rulebook);
        let panel = Panel::inventory(&rulebook, &character, &ReaderConfig::default(), 40);
        assert_eq!(texts(&panel), vec!["Empty inventory."]);
        assert_eq!(panel.lines[0].spans[0].tone, Tone::Dim);
    }
}
