//! Player state for the full-screen player: the current page, the
//! highlighted choice, the scroll position, and the character status. The
//! event loop turns terminal events into `Action`s and the view reads the
//! state back.

use oink_core::markup::{self, Run};
use oink_core::oink_rulebook::{Character, Item, Rulebook, StateChange};
use oink_core::{CheckRecord, Engine, EngineError, Event, DEFAULT_MODE};
use oink_ui::{
    CharacterConfig, Log, MapConfig, Pager, Panel, ReaderConfig, StatusConfig, Tone, View,
};
use ratatui::layout::{Position, Rect};
use std::collections::BTreeMap;

use crate::saves::Saves;

/// The presentation settings the player reads from the `ui` block.
#[derive(Debug, Clone, Default)]
pub struct Presentation {
    pub reader: ReaderConfig,
    pub status: StatusConfig,
    /// The characters a scene can speak with, keyed by id.
    pub characters: BTreeMap<String, CharacterConfig>,
    /// The map layouts of the map pages, keyed by id.
    pub maps: BTreeMap<String, MapConfig>,
}

/// Which screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The cover picture and the title, before the first scene.
    Cover,
    /// A scene with choices.
    Page,
    /// The ending.
    End,
    /// The save and load slots.
    Menu,
    /// The detail page of one inventory item.
    Item,
    /// A panel over the page: the sheet, the inventory, or the log.
    Panel(View),
}

/// What the player did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    /// A choice by 0-based index, from a number key.
    Pick(usize),
    /// Take the highlighted choice, or leave the cover and ending screens.
    /// A click also leaves the cover.
    Confirm,
    ScrollUp(u16),
    ScrollDown(u16),
    PageUp,
    PageDown,
    ScrollTop,
    ScrollBottom,
    Click(Position),
    Hover(Position),
    /// Open a panel: the sheet, the inventory, or the log.
    Open(View),
    /// Leave the open panel.
    Close,
    Quit,
}

/// The sign and color of a state change notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Gain,
    Loss,
    Condition,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub text: String,
}

/// What a menu row does with its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    Save,
    Load,
}

/// One row of the save and load menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuRow {
    /// The slot number, starting at 1.
    pub slot: usize,
    pub kind: MenuKind,
}

/// One scene, ready to draw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    /// The scene mode from the `# mode: <id>` tag, `prose` by default.
    pub mode: String,
    pub paragraphs: Vec<Vec<Run>>,
    pub checks: Vec<CheckRecord>,
    pub notices: Vec<Notice>,
    pub choices: Vec<Vec<Run>>,
    /// The name of the character speaking, resolved from the
    /// `# speaker: <id>` tag.
    pub speaker: Option<String>,
    /// The label of the map, resolved from the `# map: <id>` tag.
    pub map: Option<String>,
    /// The character id whose portrait to draw, from the
    /// `# portrait: <id>` tag or the speaker. Only a character with a
    /// configured portrait file appears here.
    pub portrait: Option<String>,
    /// The scene picture, from the `# image: <file>` tag, relative to the
    /// bundle directory.
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceStatus {
    pub name: String,
    pub value: i32,
    pub max: Option<i32>,
}

/// The character sheet summary for the status bar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    pub resources: Vec<ResourceStatus>,
    pub conditions: Vec<String>,
    pub level: Option<u32>,
}

pub struct App {
    pub title: String,
    pub screen: Screen,
    pub page: Page,
    pub status: Status,
    /// The highlighted choice.
    pub selected: usize,
    /// Rows scrolled off the top of the prose.
    pub scroll: u16,
    /// The largest useful scroll, set by the view on every draw.
    pub max_scroll: u16,
    /// Rows the prose area holds, set by the view on every draw.
    pub viewport: u16,
    /// Where each choice was drawn, set by the view on every draw.
    pub choice_areas: Vec<Rect>,
    /// The open panel, paged by the view.
    pub panel_pager: Pager,
    /// The session log behind the log panel.
    pub log: Log,
    pub quit: bool,
    /// The highlighted row of the save and load menu.
    pub menu_selection: usize,
    /// The last message of the menu, when a slot failed or a save worked.
    pub menu_message: Option<String>,
    /// A warning for the status bar, shown on the next draw and then cleared.
    pub notice: Option<String>,
    /// Where each menu row was drawn, with its row index, set by the view on
    /// every draw.
    pub menu_areas: Vec<(usize, Rect)>,
    /// The inventory item whose detail page is open.
    pub item_detail: Option<String>,
    /// The highlighted inventory item.
    pub item_selection: usize,
    /// The item ids of the open inventory, in drawing order.
    pub panel_items: Vec<String>,
    /// Where each inventory row was drawn, set by the view on every draw.
    pub item_areas: Vec<(String, Rect)>,
    /// The paged detail of the open item.
    pub item_pager: Pager,
    /// The column width the item detail was built for.
    item_columns: usize,
    /// The rows the item detail was built for.
    item_rows: usize,
    /// The numbered save slots of the bundle.
    saves: Saves,
    /// The reader settings: page sizes and sheet order.
    reader: ReaderConfig,
    /// The characters a scene can speak with, keyed by id.
    characters: BTreeMap<String, CharacterConfig>,
    /// The map layouts of the map pages, keyed by id.
    maps: BTreeMap<String, MapConfig>,
    status_config: StatusConfig,
    engine: Engine,
    /// The column width the panel lines were built for.
    panel_columns: usize,
    /// The view the panel lines were built for.
    panel_view: Option<View>,
    /// Set when the character or the log changed and the panel must rebuild.
    panel_stale: bool,
    /// The screen the panel was opened from.
    panel_return: Screen,
}

impl App {
    /// Start the story. With a cover the player opens on the cover screen.
    pub fn new(
        mut engine: Engine,
        has_cover: bool,
        presentation: Presentation,
        saves: Saves,
    ) -> Result<Self, EngineError> {
        let Presentation {
            reader,
            status,
            characters,
            maps,
        } = presentation;
        let event = engine.start()?;
        let mut app = Self {
            title: engine.data.config.title.clone(),
            screen: Screen::Page,
            page: Page::default(),
            status: Status::default(),
            selected: 0,
            scroll: 0,
            max_scroll: 0,
            viewport: 0,
            choice_areas: Vec::new(),
            panel_pager: Pager::new(Vec::new(), 0),
            log: Log::from_config(&reader),
            quit: false,
            menu_selection: 0,
            menu_message: None,
            notice: None,
            menu_areas: Vec::new(),
            item_detail: None,
            item_selection: 0,
            panel_items: Vec::new(),
            item_areas: Vec::new(),
            item_pager: Pager::new(Vec::new(), 0),
            item_columns: 0,
            item_rows: 0,
            saves,
            reader,
            characters,
            maps,
            status_config: status,
            engine,
            panel_columns: 0,
            panel_view: None,
            panel_stale: false,
            panel_return: Screen::Page,
        };
        app.show(event);
        if has_cover {
            app.screen = Screen::Cover;
        }
        Ok(app)
    }

    pub fn update(&mut self, action: Action) -> Result<(), EngineError> {
        if action == Action::Quit {
            self.quit = true;
            return Ok(());
        }
        match self.screen {
            Screen::Cover => {
                if matches!(action, Action::Confirm | Action::Click(_)) {
                    self.screen = if self.page.choices.is_empty() {
                        Screen::End
                    } else {
                        Screen::Page
                    };
                }
            }
            Screen::End => match action {
                Action::Confirm => self.quit = true,
                Action::Open(view) => self.open_panel(view),
                other => self.scroll_by(other),
            },
            Screen::Menu => match action {
                Action::Up => self.menu_selection = self.menu_selection.saturating_sub(1),
                Action::Down => {
                    let last = self.menu_rows().len().saturating_sub(1);
                    self.menu_selection = (self.menu_selection + 1).min(last);
                }
                Action::Confirm => self.activate_menu()?,
                Action::Click(position) => {
                    if let Some(index) = self.menu_at(position) {
                        self.menu_selection = index;
                        self.activate_menu()?;
                    }
                }
                Action::Hover(position) => {
                    if let Some(index) = self.menu_at(position) {
                        self.menu_selection = index;
                    }
                }
                Action::Close => self.screen = self.panel_return,
                Action::Open(view) => self.open_panel(view),
                _ => {}
            },
            Screen::Item => match action {
                Action::Close => self.screen = Screen::Panel(View::Inventory),
                Action::Open(view) => {
                    self.item_detail = None;
                    self.open_panel(view);
                }
                Action::PageUp | Action::ScrollUp(_) => {
                    self.item_pager.previous_page();
                }
                Action::PageDown | Action::ScrollDown(_) => {
                    self.item_pager.next_page();
                }
                Action::ScrollTop => while self.item_pager.previous_page() {},
                Action::ScrollBottom => while self.item_pager.next_page() {},
                _ => {}
            },
            Screen::Panel(view) => match action {
                Action::Close => self.screen = self.panel_return,
                Action::Open(view) => self.open_panel(view),
                Action::PageUp | Action::ScrollUp(_) => {
                    self.panel_pager.previous_page();
                }
                Action::PageDown | Action::ScrollDown(_) => {
                    self.panel_pager.next_page();
                }
                Action::ScrollTop => while self.panel_pager.previous_page() {},
                Action::ScrollBottom => while self.panel_pager.next_page() {},
                Action::Up if view == View::Inventory => {
                    self.select_item(self.item_selection.saturating_sub(1));
                }
                Action::Down if view == View::Inventory => {
                    self.select_item(self.item_selection + 1);
                }
                Action::Confirm if view == View::Inventory => {
                    if let Some(id) = self.panel_items.get(self.item_selection).cloned() {
                        self.open_item(&id);
                    }
                }
                Action::Click(position) if view == View::Inventory => {
                    if let Some((index, id)) = self.item_at(position) {
                        self.item_selection = index;
                        self.open_item(&id);
                    }
                }
                Action::Hover(position) if view == View::Inventory => {
                    if let Some((index, _)) = self.item_at(position) {
                        self.select_item(index);
                    }
                }
                _ => {}
            },
            Screen::Page => match action {
                Action::Up => self.selected = self.selected.saturating_sub(1),
                Action::Down => {
                    let last = self.page.choices.len().saturating_sub(1);
                    self.selected = (self.selected + 1).min(last);
                }
                Action::Pick(index) if index < self.page.choices.len() => self.choose(index)?,
                Action::Pick(_) => {}
                Action::Confirm => {
                    if self.selected < self.page.choices.len() {
                        self.choose(self.selected)?;
                    }
                }
                Action::Open(view) => self.open_panel(view),
                Action::Click(position) => {
                    if let Some(index) = self.choice_at(position) {
                        self.choose(index)?;
                    }
                }
                Action::Hover(position) => {
                    if let Some(index) = self.choice_at(position) {
                        self.selected = index;
                    }
                }
                other => self.scroll_by(other),
            },
        }
        Ok(())
    }

    fn open_panel(&mut self, view: View) {
        match view {
            View::Sheet | View::Inventory | View::Log => {
                if !matches!(self.screen, Screen::Panel(_) | Screen::Menu | Screen::Item) {
                    self.panel_return = self.screen;
                }
                self.item_detail = None;
                self.screen = Screen::Panel(view);
            }
            View::Menu => {
                if !matches!(self.screen, Screen::Panel(_) | Screen::Menu | Screen::Item) {
                    self.panel_return = self.screen;
                }
                self.item_detail = None;
                self.menu_selection = 0;
                self.menu_message = None;
                self.menu_areas.clear();
                self.screen = Screen::Menu;
            }
            View::Reader => {}
        }
    }

    /// Move the inventory highlight and page it into view.
    fn select_item(&mut self, index: usize) {
        if self.panel_items.is_empty() {
            return;
        }
        self.item_selection = index.min(self.panel_items.len() - 1);
        self.reveal_item();
    }

    /// Turn the inventory to the page that holds the selected item.
    fn reveal_item(&mut self) {
        let Some(id) = self.panel_items.get(self.item_selection) else {
            return;
        };
        let Some(index) = self
            .panel_pager
            .lines()
            .iter()
            .position(|line| line.item.as_deref() == Some(id.as_str()))
        else {
            return;
        };
        let page = self.panel_pager.page_of_line(index);
        self.panel_pager.set_page(page);
    }

    /// The item under a clicked or hovered position of the inventory.
    fn item_at(&self, position: Position) -> Option<(usize, String)> {
        let (id, _) = self
            .item_areas
            .iter()
            .find(|(_, area)| area.contains(position))?;
        let index = self.panel_items.iter().position(|item| item == id)?;
        Some((index, id.clone()))
    }

    /// Open the detail page of one item.
    fn open_item(&mut self, id: &str) {
        self.item_detail = Some(id.to_string());
        self.item_columns = 0;
        self.item_rows = 0;
        self.item_pager = Pager::new(Vec::new(), 0);
        self.screen = Screen::Item;
    }

    /// The item whose detail page is open.
    pub fn detail_item(&self) -> Option<&Item> {
        let id = self.item_detail.as_deref()?;
        self.engine.data.rulebook.items.get(id)
    }

    /// The name of the open item, for the header.
    pub fn detail_title(&self) -> String {
        self.detail_item()
            .map(|item| item.name.clone())
            .unwrap_or_else(|| "Item".to_string())
    }

    /// The picture of the open item, relative to the bundle directory.
    pub fn detail_image(&self) -> Option<&str> {
        self.detail_item()?.image.as_deref()
    }

    /// The scene picture, relative to the bundle directory.
    pub fn scene_image(&self) -> Option<&str> {
        self.page.image.as_deref()
    }

    /// The picture to draw on the current screen, relative to the bundle.
    pub fn image_path(&self) -> Option<&str> {
        match self.screen {
            Screen::Item => self.detail_image(),
            Screen::Page | Screen::End => self.scene_image(),
            _ => None,
        }
    }

    /// The rows of the save and load menu: save and load for every slot.
    pub fn menu_rows(&self) -> Vec<MenuRow> {
        (1..=self.saves.slots())
            .flat_map(|slot| {
                [
                    MenuRow {
                        slot,
                        kind: MenuKind::Save,
                    },
                    MenuRow {
                        slot,
                        kind: MenuKind::Load,
                    },
                ]
            })
            .collect()
    }

    /// True when a slot holds a save file.
    pub fn slot_filled(&self, slot: usize) -> bool {
        self.saves.exists(slot)
    }

    /// The label of one menu row.
    pub fn menu_label(&self, row: MenuRow) -> String {
        match row.kind {
            MenuKind::Save => format!("Save to slot {}", row.slot),
            MenuKind::Load if self.slot_filled(row.slot) => format!("Load slot {}", row.slot),
            MenuKind::Load => format!("Load slot {} (empty)", row.slot),
        }
    }

    /// Run the selected menu row. A save stays in the menu to show its
    /// message; a load returns to the restored scene.
    fn activate_menu(&mut self) -> Result<(), EngineError> {
        let rows = self.menu_rows();
        let Some(row) = rows.get(self.menu_selection).copied() else {
            return Ok(());
        };
        match row.kind {
            MenuKind::Save => {
                let result = self
                    .engine
                    .snapshot()
                    .map_err(|error| error.to_string())
                    .and_then(|text| self.saves.write(row.slot, &text));
                self.menu_message = Some(match result {
                    Ok(()) => format!("Saved slot {}.", row.slot),
                    Err(error) => format!("Cannot save: {error}"),
                });
            }
            MenuKind::Load => {
                let text = match self.saves.read(row.slot) {
                    Ok(Some(text)) => text,
                    Ok(None) => {
                        self.menu_message = Some(format!("Slot {} is empty.", row.slot));
                        return Ok(());
                    }
                    Err(error) => {
                        self.menu_message = Some(format!("Cannot load: {error}"));
                        return Ok(());
                    }
                };
                match self.engine.restore(&text) {
                    Ok(event) => {
                        self.log.push(format!("Loaded slot {}.", row.slot));
                        self.show(event);
                    }
                    Err(error) => {
                        self.menu_message = Some(format!("Cannot load: {error}"));
                    }
                }
            }
        }
        Ok(())
    }

    fn menu_at(&self, position: Position) -> Option<usize> {
        self.menu_areas
            .iter()
            .find(|(_, area)| area.contains(position))
            .map(|(index, _)| *index)
    }

    fn choice_at(&self, position: Position) -> Option<usize> {
        self.choice_areas
            .iter()
            .position(|area| area.contains(position))
    }

    fn scroll_by(&mut self, action: Action) {
        let page = self.viewport.saturating_sub(1).max(1);
        self.scroll = match action {
            Action::ScrollUp(rows) => self.scroll.saturating_sub(rows),
            Action::ScrollDown(rows) => self.scroll.saturating_add(rows),
            Action::PageUp => self.scroll.saturating_sub(page),
            Action::PageDown => self.scroll.saturating_add(page),
            Action::ScrollTop => 0,
            Action::ScrollBottom => self.max_scroll,
            _ => self.scroll,
        }
        .min(self.max_scroll);
    }

    fn choose(&mut self, index: usize) -> Result<(), EngineError> {
        let event = self.engine.choose(index)?;
        self.show(event);
        Ok(())
    }

    /// The panel for `view` at `columns` cells wide.
    fn panel(&self, view: View, columns: usize) -> Panel {
        let rulebook = &self.engine.data.rulebook;
        let character = self.engine.character();
        match view {
            View::Sheet => Panel::sheet(rulebook, &character, &self.reader, columns),
            View::Inventory => Panel::inventory(rulebook, &character, &self.reader, columns),
            _ => self.log.panel(columns),
        }
    }

    /// Rebuild the open panel when the content, the view, or the column
    /// width changed. The view calls this on every draw with the column
    /// geometry; the page position survives a rebuild.
    pub fn sync_panel(&mut self, view: View, columns: usize, rows: usize) {
        if self.panel_view != Some(view) || self.panel_columns != columns || self.panel_stale {
            let page = self.panel_pager.page();
            let selected = self.panel_items.get(self.item_selection).cloned();
            let panel = self.panel(view, columns);
            self.panel_items = item_ids(&panel.lines);
            self.item_selection = selected
                .and_then(|id| self.panel_items.iter().position(|item| *item == id))
                .unwrap_or(0);
            self.panel_pager = Pager::new(panel.lines, rows);
            let last = self.panel_pager.pages().saturating_sub(1);
            while self.panel_pager.page() < page.min(last) {
                self.panel_pager.next_page();
            }
            self.panel_view = Some(view);
            self.panel_columns = columns;
            self.panel_stale = false;
            self.item_areas.clear();
            self.reveal_item();
        } else {
            self.panel_pager.set_rows(rows);
        }
    }

    /// Rebuild the open item detail when the item or the geometry changed.
    /// The current page survives a rebuild.
    pub fn sync_item(&mut self, columns: usize, rows: usize) {
        if self.item_columns != columns
            || self.item_rows != rows
            || self.item_pager.lines().is_empty()
        {
            let page = self.item_pager.page();
            let panel = self.item_panel(columns);
            self.item_pager = Pager::new(panel.lines, rows);
            self.item_pager.set_page(page);
            self.item_columns = columns;
            self.item_rows = rows;
        } else {
            self.item_pager.set_rows(rows);
        }
    }

    /// The detail page of the open item: description, weight, tags, and
    /// modifiers. The picture draws above this text, so the lines start
    /// with the description.
    fn item_panel(&self, columns: usize) -> Panel {
        let Some(item) = self.detail_item() else {
            return Panel {
                title: "Item".to_string(),
                lines: text_lines("This item is not in the rulebook.", columns, Tone::Dim),
            };
        };
        let rulebook = &self.engine.data.rulebook;
        let mut lines = Vec::new();
        if let Some(description) = &item.description {
            lines.extend(text_lines(description, columns, Tone::Ink));
            lines.push(oink_ui::Line::blank());
        }
        let weight = if item.weight > 0 {
            format!("Weight {}", item.weight)
        } else {
            "Weightless".to_string()
        };
        lines.extend(text_lines(&weight, columns, Tone::Dim));
        if !item.tags.is_empty() {
            let tags: Vec<&str> = item
                .tags
                .iter()
                .map(|id| {
                    rulebook
                        .tags
                        .get(id)
                        .map(|tag| tag.name.as_str())
                        .unwrap_or(id)
                })
                .collect();
            lines.extend(text_lines(
                &format!("Tags: {}", tags.join(", ")),
                columns,
                Tone::Dim,
            ));
        }
        if item.consumable {
            lines.extend(text_lines("Consumable", columns, Tone::Dim));
        }
        let modifiers = item_modifiers(item, rulebook);
        if !modifiers.is_empty() {
            lines.push(oink_ui::Line::blank());
            lines.extend(text_lines("Modifiers", columns, Tone::Accent));
            for modifier in modifiers {
                lines.extend(text_lines(&modifier, columns, Tone::Ink));
            }
        }
        Panel {
            title: item.name.clone(),
            lines,
        }
    }

    /// The character id whose portrait the view should draw, if any.
    pub fn portrait(&self) -> Option<&str> {
        self.page.portrait.as_deref()
    }

    /// The portrait file of a character, relative to the bundle directory.
    pub fn portrait_path(&self, id: &str) -> Option<&str> {
        self.characters.get(id)?.portrait.as_deref()
    }

    fn display_name(&self, id: &str) -> String {
        self.characters
            .get(id)
            .map(|character| character.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn map_label(&self, id: &str) -> String {
        self.maps
            .get(id)
            .map(|map| map.label.clone())
            .unwrap_or_else(|| id.to_string())
    }

    fn show(&mut self, event: Event) {
        let checks = self.engine.take_checks();
        let changes = self.engine.take_changes();
        let rulebook = &self.engine.data.rulebook;
        let notices: Vec<Notice> = changes
            .iter()
            .map(|change| notice(change, rulebook))
            .collect();
        for change_notice in &notices {
            self.log.push(change_notice.text.clone());
        }
        self.panel_stale = true;
        let (mode, speaker, portrait_tag, map, image, text, choices, screen) = match event {
            Event::Scene {
                mode,
                speaker,
                portrait,
                map,
                image,
                text,
                choices,
                ..
            } => (
                mode,
                speaker,
                portrait,
                map,
                image,
                text,
                choices,
                Screen::Page,
            ),
            Event::TheEnd { text } => (
                DEFAULT_MODE.to_string(),
                None,
                None,
                None,
                None,
                text,
                Vec::new(),
                Screen::End,
            ),
        };
        let speaker_name = speaker.as_deref().map(|id| self.display_name(id));
        let map_label = map.as_deref().map(|id| self.map_label(id));
        // The portrait to draw: the explicit tag wins, then the speaker's
        // own portrait. A character without a configured portrait file
        // draws name only.
        let portrait = portrait_tag.or(speaker).filter(|id| {
            self.characters
                .get(id)
                .and_then(|character| character.portrait.as_ref())
                .is_some()
        });
        self.page = Page {
            mode,
            paragraphs: text.iter().map(|p| markup::parse(p)).collect(),
            checks,
            notices,
            choices: choices.iter().map(|c| markup::parse(&c.text)).collect(),
            speaker: speaker_name,
            map: map_label,
            portrait,
            image,
        };
        self.status = status(&self.engine, &self.status_config);
        self.screen = screen;
        self.selected = 0;
        self.scroll = 0;
        self.choice_areas.clear();
        self.menu_message = None;
        self.menu_areas.clear();
        self.item_detail = None;
        self.item_selection = 0;
        self.panel_items.clear();
        self.item_areas.clear();
    }
}

/// The item ids of a panel, in drawing order. Wrapped lines repeat the id
/// and only the first keeps it.
fn item_ids(lines: &[oink_ui::Line]) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for line in lines {
        let Some(id) = &line.item else {
            continue;
        };
        if ids.last() != Some(id) {
            ids.push(id.clone());
        }
    }
    ids
}

/// One plain text block wrapped to `columns`, as panel lines.
fn text_lines(text: &str, columns: usize, tone: Tone) -> Vec<oink_ui::Line> {
    crate::wrap::wrap_plain(text, columns)
        .into_iter()
        .map(|text| oink_ui::Line {
            spans: vec![oink_ui::Span {
                text,
                emphasis: markup::Emphasis::default(),
                tone,
            }],
            ..Default::default()
        })
        .collect()
}

/// The readable flat modifiers of an item, one line each.
fn item_modifiers(item: &Item, rulebook: &Rulebook) -> Vec<String> {
    let name = |display: Option<&str>, id: &str| display.unwrap_or(id).to_string();
    let mut lines = Vec::new();
    for (id, value) in &item.modifiers.characteristics {
        let label = name(
            rulebook.characteristics.get(id).map(|c| c.name.as_str()),
            id,
        );
        lines.push(format!("{label} {value:+}"));
    }
    for (id, value) in &item.modifiers.abilities {
        let label = name(rulebook.abilities.get(id).map(|a| a.name.as_str()), id);
        lines.push(format!("{label} {value:+}"));
    }
    for (id, value) in &item.modifiers.tags {
        let label = name(rulebook.tags.get(id).map(|t| t.name.as_str()), id);
        lines.push(format!("{label} tag {value:+}"));
    }
    lines
}

fn status(engine: &Engine, config: &StatusConfig) -> Status {
    let rulebook = &engine.data.rulebook;
    let character = engine.character();
    let resources = match &config.resources {
        Some(ids) => ids
            .iter()
            .filter_map(|id| resource_status(rulebook, &character, id))
            .collect(),
        None => rulebook
            .resources
            .keys()
            .filter_map(|id| resource_status(rulebook, &character, id))
            .collect(),
    };
    let conditions = if config.show_conditions.unwrap_or(true) {
        character
            .condition_ids()
            .map(|id| name(rulebook.conditions.get(id).map(|c| c.name.as_str()), id))
            .collect()
    } else {
        Vec::new()
    };
    let level = if config.show_level.unwrap_or(true) {
        rulebook.levelling.as_ref().map(|_| character.level())
    } else {
        None
    };
    Status {
        resources,
        conditions,
        level,
    }
}

fn resource_status(rulebook: &Rulebook, character: &Character, id: &str) -> Option<ResourceStatus> {
    let resource = rulebook.resources.get(id)?;
    character.resource(id).map(|value| ResourceStatus {
        name: resource.name.clone(),
        value,
        max: character.resource_max(rulebook, id),
    })
}

fn name(display: Option<&str>, id: &str) -> String {
    display.unwrap_or(id).to_string()
}

/// A notice with display names in place of ids.
fn notice(change: &StateChange, rulebook: &Rulebook) -> Notice {
    let item = |id: &str| name(rulebook.items.get(id).map(|i| i.name.as_str()), id);
    let perk = |id: &str| name(rulebook.perks.get(id).map(|p| p.name.as_str()), id);
    let condition = |id: &str| name(rulebook.conditions.get(id).map(|c| c.name.as_str()), id);
    let environment = |id: &str| name(rulebook.environments.get(id).map(|e| e.name.as_str()), id);
    let tag = |id: &str| name(rulebook.tags.get(id).map(|t| t.name.as_str()), id);
    let ability = |id: &str| name(rulebook.abilities.get(id).map(|a| a.name.as_str()), id);
    let resource = |id: &str| name(rulebook.resources.get(id).map(|r| r.name.as_str()), id);
    let (kind, text) = match change {
        StateChange::ItemAdded(id) => (NoticeKind::Gain, format!("item added: {}", item(id))),
        StateChange::ItemRemoved(id) => (NoticeKind::Loss, format!("item removed: {}", item(id))),
        StateChange::PerkAdded(id) => (NoticeKind::Gain, format!("perk added: {}", perk(id))),
        StateChange::PerkRemoved(id) => (NoticeKind::Loss, format!("perk removed: {}", perk(id))),
        StateChange::AbilityGranted(id) => (
            NoticeKind::Gain,
            format!("ability granted: {}", ability(id)),
        ),
        StateChange::ConditionAdded(id) => (
            NoticeKind::Condition,
            format!("condition added: {}", condition(id)),
        ),
        StateChange::ConditionRefreshed(id) => (
            NoticeKind::Condition,
            format!("condition refreshed: {}", condition(id)),
        ),
        StateChange::ConditionRemoved(id) => (
            NoticeKind::Condition,
            format!("condition removed: {}", condition(id)),
        ),
        StateChange::EnvironmentEntered(id) => (
            NoticeKind::Note,
            format!("environment entered: {}", environment(id)),
        ),
        StateChange::EnvironmentCleared(id) => (
            NoticeKind::Note,
            format!("environment cleared: {}", environment(id)),
        ),
        StateChange::TagAdded(id) => (NoticeKind::Note, format!("tag added: {}", tag(id))),
        StateChange::TagRemoved(id) => (NoticeKind::Note, format!("tag removed: {}", tag(id))),
        StateChange::ResourceChanged { id, from, to } => {
            let kind = if to >= from {
                NoticeKind::Gain
            } else {
                NoticeKind::Loss
            };
            (kind, format!("{} {from} → {to}", resource(id)))
        }
    };
    Notice { kind, text }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use oink_core::data::GameData;

    pub(crate) const RULEBOOK: &str = "
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
  key:
    name: Brass Key
    description: A small brass key.
    image: art/key.png
    weight: 1
  rope:
    name: Rope
    description: Hemp rope, dry and strong.
    weight: 2
conditions:
  bruised: { name: Bruised, duration: 2 }
resources:
  health: { name: Health, min: 0, max: 10 }
starting_character:
  characteristics: { body: 5 }
  abilities: { might: 1 }
  inventory: [rope]
  resources: { health: 10 }
";

    pub(crate) const STORY: &str = r#"
EXTERNAL roll_check(ability, difficulty, tags, modifier)
EXTERNAL add_item(id)
EXTERNAL add_condition(id)
EXTERNAL spend_resource(id, amount)
A door, and a *long* corridor behind it.
+ [Open the **door**]
    ~ add_item("key")
    ~ add_condition("bruised")
    ~ spend_resource("health", 7)
    ~ roll_check("might", 7, "", 0)
    -> inside
+ [Leave]
    You leave.
    -> END
=== inside ===
# mode: dialog
# speaker: keeper
# image: art/inside.png
Inside.
+ [Sit]
    -> crossroads
=== crossroads ===
# mode: map
# map: valley
# portrait: keeper
Two ways down.
+ [The bridge]
    -> END
+ [The scree]
    -> END
"#;

    pub(crate) fn engine() -> Engine {
        let data = GameData::from_yaml("title: Têst", RULEBOOK).unwrap();
        let mut engine = Engine::new(STORY, data).unwrap();
        engine.set_seed(7);
        engine
    }

    pub(crate) fn app() -> App {
        App::new(engine(), false, Presentation::default(), saves()).unwrap()
    }

    fn saves() -> Saves {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "oink-app-saves-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        ));
        Saves::new(dir, 3)
    }

    #[test]
    fn the_first_page_parses_markup_and_reads_the_sheet() {
        let app = app();
        assert_eq!(app.screen, Screen::Page);
        assert_eq!(app.page.paragraphs.len(), 1);
        assert_eq!(app.page.paragraphs[0][1].text, "long");
        assert!(app.page.paragraphs[0][1].emphasis.italic);
        assert_eq!(app.page.choices.len(), 2);
        assert!(app.page.choices[0][1].emphasis.bold);
        assert_eq!(
            app.status.resources,
            vec![ResourceStatus {
                name: "Health".to_string(),
                value: 10,
                max: Some(10),
            }]
        );
        assert!(app.status.conditions.is_empty());
        assert_eq!(app.status.level, None);
    }

    #[test]
    fn status_resources_follow_the_ui_config() {
        let config = StatusConfig {
            resources: Some(Vec::new()),
            ..StatusConfig::default()
        };
        let app = App::new(
            engine(),
            false,
            Presentation {
                status: config,
                ..Presentation::default()
            },
            saves(),
        )
        .unwrap();
        assert!(app.status.resources.is_empty());

        let config = StatusConfig {
            resources: Some(vec!["health".to_string()]),
            ..StatusConfig::default()
        };
        let app = App::new(
            engine(),
            false,
            Presentation {
                status: config,
                ..Presentation::default()
            },
            saves(),
        )
        .unwrap();
        assert_eq!(app.status.resources[0].name, "Health");
    }

    #[test]
    fn status_conditions_follow_the_ui_config() {
        let config = StatusConfig {
            show_conditions: Some(false),
            ..StatusConfig::default()
        };
        let mut app = App::new(
            engine(),
            false,
            Presentation {
                status: config,
                ..Presentation::default()
            },
            saves(),
        )
        .unwrap();
        app.update(Action::Pick(0)).unwrap();
        assert!(app.status.conditions.is_empty());
    }

    #[test]
    fn a_cover_shows_first_and_confirm_or_a_click_leaves_it() {
        let mut app = App::new(engine(), true, Presentation::default(), saves()).unwrap();
        assert_eq!(app.screen, Screen::Cover);
        app.update(Action::Down).unwrap();
        assert_eq!(app.screen, Screen::Cover);
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.screen, Screen::Page);

        let mut app = App::new(engine(), true, Presentation::default(), saves()).unwrap();
        app.update(Action::Click(Position::new(3, 3))).unwrap();
        assert_eq!(app.screen, Screen::Page);
        assert_eq!(app.page.choices.len(), 2, "the click picks nothing yet");
    }

    #[test]
    fn an_ending_during_start_survives_the_cover() {
        let data = GameData::from_yaml("title: Têst", RULEBOOK).unwrap();
        let engine = Engine::new("The pass is closed.\n-> END\n", data).unwrap();
        let mut app = App::new(engine, true, Presentation::default(), saves()).unwrap();
        assert_eq!(app.screen, Screen::Cover);
        assert!(app.page.choices.is_empty());
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.screen, Screen::End);
        assert!(!app.quit);
        app.update(Action::Confirm).unwrap();
        assert!(app.quit);
    }

    #[test]
    fn the_highlight_moves_and_clamps() {
        let mut app = app();
        app.update(Action::Up).unwrap();
        assert_eq!(app.selected, 0);
        app.update(Action::Down).unwrap();
        assert_eq!(app.selected, 1);
        app.update(Action::Down).unwrap();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn picking_advances_and_collects_checks_and_notices() {
        let mut app = app();
        app.update(Action::Pick(5)).unwrap();
        assert_eq!(app.page.choices.len(), 2, "out of range picks are ignored");
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(app.page.paragraphs[0][0].text, "Inside.");
        assert_eq!(app.page.checks.len(), 1);
        assert_eq!(app.page.checks[0].ability, "might");
        assert_eq!(
            app.page.notices,
            vec![
                Notice {
                    kind: NoticeKind::Gain,
                    text: "item added: Brass Key".to_string(),
                },
                Notice {
                    kind: NoticeKind::Condition,
                    text: "condition added: Bruised".to_string(),
                },
                Notice {
                    kind: NoticeKind::Loss,
                    text: "Health 10 → 3".to_string(),
                },
            ]
        );
        assert_eq!(app.status.resources[0].value, 3);
        assert_eq!(app.status.conditions, vec!["Bruised".to_string()]);
        app.update(Action::Confirm).unwrap();
        assert_eq!(
            app.screen,
            Screen::Page,
            "the dialog scene leads to the map scene"
        );
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.screen, Screen::End);
        assert!(!app.quit);
        app.update(Action::Confirm).unwrap();
        assert!(app.quit);
    }

    #[test]
    fn every_slot_saves_and_loads_the_same_scene() {
        let mut app = app();
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(app.page.mode, "dialog");
        assert_eq!(app.menu_rows().len(), 6);

        for slot in 1..=3 {
            app.update(Action::Open(View::Menu)).unwrap();
            assert_eq!(app.screen, Screen::Menu);
            app.menu_selection = (slot - 1) * 2;
            assert_eq!(
                app.menu_label(app.menu_rows()[app.menu_selection]),
                format!("Save to slot {slot}")
            );
            app.update(Action::Confirm).unwrap();
            assert_eq!(
                app.menu_message,
                Some(format!("Saved slot {slot}.")),
                "the save reports its slot"
            );
            assert_eq!(
                app.menu_label(app.menu_rows()[app.menu_selection + 1]),
                format!("Load slot {slot}")
            );
            app.update(Action::Close).unwrap();
        }

        for slot in 1..=3 {
            app.update(Action::Open(View::Menu)).unwrap();
            app.menu_selection = (slot - 1) * 2 + 1;
            app.update(Action::Confirm).unwrap();
            assert_eq!(app.screen, Screen::Page);
            assert_eq!(app.page.mode, "dialog");
            assert_eq!(app.page.speaker.as_deref(), Some("keeper"));
            assert_eq!(app.page.choices.len(), 1);
        }
    }

    #[test]
    fn an_empty_slot_reports_itself_and_keeps_the_menu_open() {
        let mut app = app();
        app.update(Action::Open(View::Menu)).unwrap();
        assert_eq!(app.menu_label(app.menu_rows()[1]), "Load slot 1 (empty)");
        app.menu_selection = 3;
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.screen, Screen::Menu);
        assert_eq!(app.menu_message.as_deref(), Some("Slot 2 is empty."));
        app.update(Action::Close).unwrap();
        assert_eq!(app.screen, Screen::Page);
    }

    #[test]
    fn a_menu_click_selects_a_row() {
        let mut app = app();
        app.update(Action::Open(View::Menu)).unwrap();
        app.menu_areas = vec![(0, Rect::new(0, 10, 20, 1)), (1, Rect::new(0, 11, 20, 1))];
        app.update(Action::Click(Position::new(5, 11))).unwrap();
        assert_eq!(app.menu_selection, 1);
        assert_eq!(
            app.menu_message.as_deref(),
            Some("Slot 1 is empty."),
            "a click activates the row"
        );
        app.update(Action::Close).unwrap();
        assert_eq!(app.screen, Screen::Page);
    }

    #[test]
    fn a_dialog_scene_names_the_speaker_and_a_map_scene_labels_the_map() {
        let mut app = app();
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(app.page.mode, "dialog");
        assert_eq!(
            app.page.image.as_deref(),
            Some("art/inside.png"),
            "the scene carries its picture path"
        );
        assert_eq!(
            app.page.speaker.as_deref(),
            Some("keeper"),
            "an unknown character falls back to its id"
        );
        assert_eq!(
            app.page.portrait, None,
            "a speaker without a portrait file draws name only"
        );
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.page.mode, "map");
        assert_eq!(app.page.map.as_deref(), Some("valley"));
        assert_eq!(app.page.speaker, None);
        assert_eq!(
            app.page.portrait, None,
            "the portrait tag needs a configured character"
        );
        assert_eq!(app.page.choices.len(), 2);

        let presentation = Presentation {
            characters: [(
                "keeper".to_string(),
                CharacterConfig {
                    name: "The Keeper".to_string(),
                    portrait: Some("keeper.png".to_string()),
                },
            )]
            .into(),
            ..Presentation::default()
        };
        let mut app = App::new(engine(), false, presentation, saves()).unwrap();
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(app.page.speaker.as_deref(), Some("The Keeper"));
        assert_eq!(app.page.portrait.as_deref(), Some("keeper"));
        app.update(Action::Confirm).unwrap();
        assert_eq!(
            app.page.portrait.as_deref(),
            Some("keeper"),
            "the portrait tag overrides the speaker"
        );
        assert_eq!(app.page.speaker, None);
    }

    #[test]
    fn clicks_and_hovers_use_the_drawn_areas() {
        let mut app = app();
        app.choice_areas = vec![Rect::new(0, 10, 40, 1), Rect::new(0, 11, 40, 2)];
        app.update(Action::Hover(Position::new(5, 12))).unwrap();
        assert_eq!(app.selected, 1);
        app.update(Action::Hover(Position::new(5, 3))).unwrap();
        assert_eq!(app.selected, 1);
        app.update(Action::Click(Position::new(5, 3))).unwrap();
        assert_eq!(
            app.page.choices.len(),
            2,
            "a click outside the choices does nothing"
        );
        app.update(Action::Click(Position::new(39, 11))).unwrap();
        assert_eq!(app.screen, Screen::End);
    }

    #[test]
    fn scrolling_clamps_to_the_drawn_range() {
        let mut app = app();
        app.max_scroll = 10;
        app.viewport = 5;
        app.update(Action::ScrollDown(3)).unwrap();
        assert_eq!(app.scroll, 3);
        app.update(Action::PageDown).unwrap();
        assert_eq!(app.scroll, 7);
        app.update(Action::ScrollBottom).unwrap();
        assert_eq!(app.scroll, 10);
        app.update(Action::ScrollDown(3)).unwrap();
        assert_eq!(app.scroll, 10);
        app.update(Action::PageUp).unwrap();
        assert_eq!(app.scroll, 6);
        app.update(Action::ScrollTop).unwrap();
        assert_eq!(app.scroll, 0);
        app.update(Action::ScrollUp(1)).unwrap();
        assert_eq!(app.scroll, 0);
    }

    #[test]
    fn quit_works_on_every_screen() {
        let mut app = App::new(engine(), true, Presentation::default(), saves()).unwrap();
        app.update(Action::Quit).unwrap();
        assert!(app.quit);
    }

    #[test]
    fn panels_open_close_and_return_to_their_screen() {
        let mut app = app();
        app.update(Action::Open(View::Sheet)).unwrap();
        assert_eq!(app.screen, Screen::Panel(View::Sheet));
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(
            app.screen,
            Screen::Panel(View::Sheet),
            "a panel ignores the page keys"
        );
        app.update(Action::Open(View::Log)).unwrap();
        assert_eq!(app.screen, Screen::Panel(View::Log));
        app.update(Action::Close).unwrap();
        assert_eq!(app.screen, Screen::Page, "the panel returns to the page");

        let mut app = App::new(engine(), true, Presentation::default(), saves()).unwrap();
        app.update(Action::Open(View::Sheet)).unwrap();
        assert_eq!(
            app.screen,
            Screen::Cover,
            "the cover ignores the panel keys"
        );
    }

    #[test]
    fn the_menu_opens_over_the_page_and_back_returns() {
        let mut opened = app();
        opened.update(Action::Open(View::Menu)).unwrap();
        assert_eq!(opened.screen, Screen::Menu);
        assert_eq!(opened.menu_selection, 0);
        opened.update(Action::Close).unwrap();
        assert_eq!(opened.screen, Screen::Page, "Backspace returns to the page");

        let mut ended = app();
        ended.update(Action::Pick(1)).unwrap();
        assert_eq!(ended.screen, Screen::End);
        ended.update(Action::Open(View::Menu)).unwrap();
        assert_eq!(ended.screen, Screen::Menu);
        ended.update(Action::Close).unwrap();
        assert_eq!(ended.screen, Screen::End, "the menu returns to the ending");
    }

    #[test]
    fn the_ending_keeps_the_panels() {
        let mut app = app();
        app.update(Action::Pick(1)).unwrap();
        assert_eq!(app.screen, Screen::End);
        app.update(Action::Open(View::Sheet)).unwrap();
        assert_eq!(app.screen, Screen::Panel(View::Sheet));
        app.update(Action::Close).unwrap();
        assert_eq!(app.screen, Screen::End);
        app.update(Action::Confirm).unwrap();
        assert!(app.quit);
    }

    #[test]
    fn the_log_records_the_notices_newest_first() {
        let mut app = app();
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(app.log.len(), 3);
        let newest: Vec<&str> = app.log.entries().rev().collect();
        assert_eq!(newest[0], "Health 10 → 3");
        assert_eq!(newest[2], "item added: Brass Key");
    }

    #[test]
    fn the_log_respects_the_configured_cap() {
        let reader = ReaderConfig {
            log_entries: 3,
            ..ReaderConfig::default()
        };
        let mut app = App::new(
            engine(),
            false,
            Presentation {
                reader,
                ..Presentation::default()
            },
            saves(),
        )
        .unwrap();
        for number in 0..5 {
            app.log.push(format!("entry {number}"));
        }
        assert_eq!(app.log.len(), 3);
        let oldest: Vec<&str> = app.log.entries().collect();
        assert_eq!(oldest[0], "entry 2", "the oldest entries drop first");
    }

    #[test]
    fn a_panel_pages_and_a_rebuild_keeps_the_position() {
        let mut app = app();
        app.update(Action::Pick(0)).unwrap();
        app.update(Action::Open(View::Log)).unwrap();
        app.sync_panel(View::Log, 20, 1);
        let pages = app.panel_pager.pages();
        assert!(pages > 1, "{}", pages);
        app.update(Action::PageDown).unwrap();
        assert_eq!(app.panel_pager.page(), 1);
        app.update(Action::ScrollUp(3)).unwrap();
        assert_eq!(app.panel_pager.page(), 0, "the wheel pages the panel too");
        app.update(Action::PageDown).unwrap();
        assert_eq!(app.panel_pager.page(), 1);

        app.sync_panel(View::Log, 10, 1);
        assert!(
            app.panel_pager.pages() > pages,
            "a narrower column splits into more pages"
        );
        assert_eq!(
            app.panel_pager.page(),
            1,
            "the position survives the rebuild"
        );
    }

    #[test]
    fn the_sheet_panel_lists_the_character() {
        let mut app = app();
        app.update(Action::Open(View::Sheet)).unwrap();
        app.sync_panel(View::Sheet, 40, 10);
        let lines: Vec<String> = app
            .panel_pager
            .lines()
            .iter()
            .map(|line| line.text())
            .collect();
        assert!(lines.contains(&"Characteristics".to_string()));
        assert!(lines.contains(&"Body 5".to_string()));
        assert!(lines.contains(&"Might 1".to_string()));
    }

    #[test]
    fn the_inventory_panel_lists_the_stacks() {
        let mut app = app();
        app.update(Action::Pick(0)).unwrap();
        app.update(Action::Open(View::Inventory)).unwrap();
        app.sync_panel(View::Inventory, 40, 10);
        let lines: Vec<String> = app
            .panel_pager
            .lines()
            .iter()
            .map(|line| line.text())
            .collect();
        assert!(
            lines.iter().any(|line| line.starts_with("Brass Key")),
            "{lines:?}"
        );
    }

    fn inventory() -> App {
        let mut app = app();
        app.update(Action::Pick(0)).unwrap();
        app.update(Action::Open(View::Inventory)).unwrap();
        app.sync_panel(View::Inventory, 40, 10);
        app
    }

    #[test]
    fn the_inventory_selects_one_item_at_a_time() {
        let mut app = inventory();
        assert_eq!(app.panel_items, ["key", "rope"]);
        assert_eq!(app.item_selection, 0);
        app.update(Action::Down).unwrap();
        assert_eq!(app.item_selection, 1);
        app.update(Action::Down).unwrap();
        assert_eq!(app.item_selection, 1, "the last item holds the highlight");
        app.update(Action::Up).unwrap();
        assert_eq!(app.item_selection, 0);
        app.update(Action::Up).unwrap();
        assert_eq!(app.item_selection, 0, "the first item holds the highlight");
    }

    #[test]
    fn enter_opens_an_item_and_back_returns_to_the_list() {
        let mut app = inventory();
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.screen, Screen::Item);
        assert_eq!(app.detail_title(), "Brass Key");
        app.update(Action::Close).unwrap();
        assert_eq!(app.screen, Screen::Panel(View::Inventory));
        assert_eq!(app.item_selection, 0, "the list keeps the item that opened");
    }

    #[test]
    fn the_item_detail_shows_the_description_and_weight() {
        let mut app = inventory();
        app.update(Action::Confirm).unwrap();
        app.sync_item(40, 10);
        let lines: Vec<String> = app
            .item_pager
            .lines()
            .iter()
            .map(|line| line.text())
            .collect();
        assert!(lines.contains(&"A small brass key.".to_string()));
        assert!(lines.contains(&"Weight 1".to_string()));
    }

    #[test]
    fn image_paths_come_from_the_scene_and_the_item() {
        let mut app = app();
        assert_eq!(app.image_path(), None, "the first page has no picture");
        app.update(Action::Pick(0)).unwrap();
        assert_eq!(
            app.image_path(),
            Some("art/inside.png"),
            "a scene picture comes from the mode tag block"
        );
        app.update(Action::Open(View::Inventory)).unwrap();
        app.sync_panel(View::Inventory, 40, 10);
        assert_eq!(
            app.image_path(),
            None,
            "the inventory list draws no picture"
        );
        app.update(Action::Confirm).unwrap();
        assert_eq!(
            app.image_path(),
            Some("art/key.png"),
            "an item picture comes from the rulebook"
        );
        app.update(Action::Close).unwrap();
        app.update(Action::Down).unwrap();
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.image_path(), None, "the rope has no picture");
    }
}
