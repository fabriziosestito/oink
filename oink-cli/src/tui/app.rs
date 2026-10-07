//! Player state for the full-screen player: the current page, the
//! highlighted choice, the scroll position, and the character status. The
//! event loop turns terminal events into `Action`s and the view reads the
//! state back.

use oink_core::oink_rulebook::{Rulebook, StateChange};
use oink_core::{CheckRecord, Engine, EngineError, Event};
use ratatui::layout::{Position, Rect};

use crate::markup::{self, Run};

/// Which screen is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// The cover picture and the title, before the first scene.
    Cover,
    /// A scene with choices.
    Page,
    /// The ending.
    End,
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

/// One scene, ready to draw.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Page {
    pub paragraphs: Vec<Vec<Run>>,
    pub checks: Vec<CheckRecord>,
    pub notices: Vec<Notice>,
    pub choices: Vec<Vec<Run>>,
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
    pub quit: bool,
    engine: Engine,
}

impl App {
    /// Start the story. With a cover the player opens on the cover screen.
    pub fn new(mut engine: Engine, has_cover: bool) -> Result<Self, EngineError> {
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
            quit: false,
            engine,
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
                    self.screen = Screen::Page;
                }
            }
            Screen::End => match action {
                Action::Confirm => self.quit = true,
                other => self.scroll_by(other),
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

    fn show(&mut self, event: Event) {
        let checks = self.engine.take_checks();
        let changes = self.engine.take_changes();
        let rulebook = &self.engine.data.rulebook;
        let notices = changes
            .iter()
            .map(|change| notice(change, rulebook))
            .collect();
        let (text, choices, screen) = match event {
            Event::Scene { text, choices } => (text, choices, Screen::Page),
            Event::TheEnd { text } => (text, Vec::new(), Screen::End),
        };
        self.page = Page {
            paragraphs: text.iter().map(|p| markup::parse(p)).collect(),
            checks,
            notices,
            choices: choices.iter().map(|c| markup::parse(&c.text)).collect(),
        };
        self.status = status(&self.engine);
        self.screen = screen;
        self.selected = 0;
        self.scroll = 0;
        self.choice_areas.clear();
    }
}

fn status(engine: &Engine) -> Status {
    let rulebook = &engine.data.rulebook;
    let character = engine.character();
    Status {
        resources: rulebook
            .resources
            .iter()
            .filter_map(|(id, resource)| {
                character.resource(id).map(|value| ResourceStatus {
                    name: resource.name.clone(),
                    value,
                    max: character.resource_max(rulebook, id),
                })
            })
            .collect(),
        conditions: character
            .condition_ids()
            .map(|id| name(rulebook.conditions.get(id).map(|c| c.name.as_str()), id))
            .collect(),
        level: rulebook.levelling.as_ref().map(|_| character.level()),
    }
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
  key: { name: Brass Key }
conditions:
  bruised: { name: Bruised, duration: 2 }
resources:
  health: { name: Health, min: 0, max: 10 }
starting_character:
  characteristics: { body: 5 }
  abilities: { might: 1 }
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
Inside.
+ [Sit]
    -> END
"#;

    pub(crate) fn engine() -> Engine {
        let data = GameData::from_yaml("title: Têst", RULEBOOK).unwrap();
        let mut engine = Engine::new(STORY, data).unwrap();
        engine.set_seed(7);
        engine
    }

    pub(crate) fn app() -> App {
        App::new(engine(), false).unwrap()
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
    fn a_cover_shows_first_and_confirm_or_a_click_leaves_it() {
        let mut app = App::new(engine(), true).unwrap();
        assert_eq!(app.screen, Screen::Cover);
        app.update(Action::Down).unwrap();
        assert_eq!(app.screen, Screen::Cover);
        app.update(Action::Confirm).unwrap();
        assert_eq!(app.screen, Screen::Page);

        let mut app = App::new(engine(), true).unwrap();
        app.update(Action::Click(Position::new(3, 3))).unwrap();
        assert_eq!(app.screen, Screen::Page);
        assert_eq!(app.page.choices.len(), 2, "the click picks nothing yet");
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
        assert_eq!(app.screen, Screen::End);
        assert!(!app.quit);
        app.update(Action::Confirm).unwrap();
        assert!(app.quit);
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
        let mut app = App::new(engine(), true).unwrap();
        app.update(Action::Quit).unwrap();
        assert!(app.quit);
    }
}
