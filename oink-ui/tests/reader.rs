#![cfg(feature = "render")]

mod support;

use oink_core::oink_rulebook::{Character, Rulebook};
use oink_core::{Choice, Event};
use oink_ui::{
    draw_empty, draw_page, draw_panel, draw_reader, page_size, Action, BarButton, Focus, Header,
    Metrics, Pager, Panel, ReaderConfig, Scene, Theme, TopBar, UiState, View, Zones,
};

const RULEBOOK: &str = r#"
characteristics:
  physique: { name: Physique, min: 1, max: 14, default: 7 }
resources:
  health: { name: Health, min: 0, max: 10 }
starting_character:
  characteristics: { physique: 7 }
  resources: { health: 8 }
"#;

#[test]
fn the_empty_reader_draws_the_three_zones() {
    let metrics = Metrics::new(320, 240, 28, 36);
    let zones = Zones::from_metrics(metrics);
    let theme = Theme::default();
    let mut framebuffer = support::Framebuffer::<320, 240>::new();

    draw_empty(&mut framebuffer, metrics, theme).unwrap();

    assert_eq!(zones.top.size.height, 28);
    assert_eq!(zones.center.size.height, 176);
    assert_eq!(zones.bottom.size.height, 36);

    assert_eq!(framebuffer.pixel(0, 0), theme.paper);
    assert_eq!(framebuffer.pixel(0, 27), theme.dim);
    assert_eq!(framebuffer.pixel(0, 28), theme.paper);
    assert_eq!(framebuffer.pixel(0, 203), theme.paper);
    assert_eq!(framebuffer.pixel(0, 204), theme.dim);
    assert_eq!(framebuffer.pixel(0, 239), theme.paper);
}

#[test]
fn the_snapshot_is_written_as_a_pgm() {
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    draw_empty(
        &mut framebuffer,
        Metrics::new(320, 240, 28, 36),
        Theme::default(),
    )
    .unwrap();

    let path = framebuffer.write_pgm("empty-reader").unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"P5\n320 240\n15\n"));
    assert_eq!(bytes.len(), "P5\n320 240\n15\n".len() + 320 * 240);
}

#[test]
fn the_center_page_draws_and_turns() {
    let metrics = Metrics::new(320, 240, 28, 36);
    let size = page_size(metrics);
    let prose = "The pass is long and the wind is cold. ".repeat(12);
    let scene = Scene::from_event(
        &Event::Scene {
            mode: oink_core::DEFAULT_MODE.to_string(),
            speaker: None,
            mood: None,
            portrait: None,
            map: None,
            image: None,
            text: vec![prose],
            choices: vec![
                Choice {
                    index: 0,
                    text: "Climb higher".to_string(),
                },
                Choice {
                    index: 1,
                    text: "Turn back".to_string(),
                },
            ],
        },
        Vec::new(),
    );
    let mut pager = Pager::from_scene(&scene, size.columns, size.rows);
    assert!(pager.pages() > 1);

    let theme = Theme::default();
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    draw_page(&mut framebuffer, metrics, theme, &pager).unwrap();

    let has_ink = (28..204).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.ink));
    assert!(has_ink, "the prose draws in the center zone");
    assert_eq!(framebuffer.pixel(0, 220), theme.paper);

    while pager.next_page() {}
    draw_page(&mut framebuffer, metrics, theme, &pager).unwrap();
    let path = framebuffer.write_pgm("center-page").unwrap();
    assert!(path.exists());
}

#[test]
fn the_bars_show_the_sheet_values_and_the_buttons() {
    let rulebook = Rulebook::load(RULEBOOK).expect("rulebook loads").rulebook;
    let character = Character::from_starting(&rulebook);
    let header = Header::new(
        &rulebook,
        &character,
        &TopBar {
            resources: vec!["health".to_string()],
            characteristics: vec!["physique".to_string()],
            height: None,
        },
    );
    assert_eq!(header.resources_text(), "Health 8/10");
    assert_eq!(header.characteristics_text(), "Physique 7");

    let metrics = Metrics::new(320, 240, 28, 36);
    let theme = Theme::default();
    let mut state = UiState::new();
    state.set_choices(1);
    assert_eq!(state.handle(Action::Down), oink_ui::Effect::None);
    assert_eq!(state.focus(), Focus::Bar);

    let size = page_size(metrics);
    let pager = Pager::from_scene(&Scene::default(), size.columns, size.rows);
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    draw_reader(&mut framebuffer, metrics, theme, &pager, &header, &state).unwrap();

    let top_text = (4..24).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.dim));
    assert!(top_text, "the top bar shows the summary values");
    let selected = (204..240).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.ink));
    assert!(selected, "the focused button draws a filled highlight");
    let buttons = (204..240).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.dim));
    assert!(buttons, "the bar labels draw in the dim tone");

    let path = framebuffer.write_pgm("reader").unwrap();
    assert!(path.exists());
}

#[test]
fn the_sheet_panel_draws_its_title_and_pages() {
    let rulebook = Rulebook::load(RULEBOOK).expect("rulebook loads").rulebook;
    let character = Character::from_starting(&rulebook);
    let metrics = Metrics::new(320, 240, 28, 36);
    let size = page_size(metrics);
    let panel = Panel::sheet(
        &rulebook,
        &character,
        &ReaderConfig::default(),
        size.columns,
    );
    assert_eq!(panel.title, "Sheet");
    assert!(panel.lines.iter().any(|line| line.text() == "Physique 7"));

    let mut pager = Pager::new(panel.lines.clone(), size.rows);
    assert!(
        pager.pages() > 1,
        "the default groups fill more than a page"
    );

    let theme = Theme::default();
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    draw_panel(&mut framebuffer, metrics, theme, &panel, &pager).unwrap();

    let title = (0..24).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.ink));
    assert!(title, "the panel title takes the top bar");
    let heading = (28..204).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.accent));
    assert!(heading, "group headings draw in the accent tone");
    let back = (210..236).any(|y| (0..320).any(|x| framebuffer.pixel(x, y) == theme.dim));
    assert!(back, "the bottom bar shows how to leave the view");

    while pager.next_page() {}
    draw_panel(&mut framebuffer, metrics, theme, &panel, &pager).unwrap();
    let path = framebuffer.write_pgm("sheet").unwrap();
    assert!(path.exists());
}

#[test]
fn the_reader_reports_its_touch_targets() {
    let metrics = Metrics::new(320, 240, 28, 36);
    let size = page_size(metrics);
    let theme = Theme::default();
    let mut state = UiState::new();
    state.set_choices(2);

    let scene = Scene::from_event(
        &Event::Scene {
            mode: oink_core::DEFAULT_MODE.to_string(),
            speaker: None,
            mood: None,
            portrait: None,
            map: None,
            image: None,
            text: vec!["One short line.".to_string()],
            choices: vec![
                Choice {
                    index: 0,
                    text: "First".to_string(),
                },
                Choice {
                    index: 1,
                    text: "Second".to_string(),
                },
            ],
        },
        Vec::new(),
    );
    let header = Header::default();
    let pager = Pager::from_scene(&scene, size.columns, size.rows);
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    let areas = draw_reader(&mut framebuffer, metrics, theme, &pager, &header, &state).unwrap();

    assert_eq!(areas.choices().len(), 2);
    assert_eq!(areas.choice_at(areas.choices()[0].1.top_left), Some(0));
    assert_eq!(areas.choice_at(areas.choices()[1].1.top_left), Some(1));
    assert_eq!(
        areas.action_at(areas.choices()[0].1.top_left),
        Some(Action::Pick(0))
    );
    assert_eq!(
        areas.action_at(areas.choices()[1].1.top_left),
        Some(Action::Pick(1))
    );
    assert_eq!(areas.buttons().len(), 4);
    assert_eq!(
        areas.button_at(areas.buttons()[0].1.top_left),
        Some(BarButton::Sheet)
    );
    assert_eq!(
        areas.action_at(areas.buttons()[0].1.top_left),
        Some(Action::Open(View::Sheet))
    );
    assert_eq!(
        areas.action_at(embedded_graphics::geometry::Point::new(0, 0)),
        None
    );
    assert!(areas.previous_page().is_none());
    assert!(areas.next_page().is_none());

    let rulebook = Rulebook::load(RULEBOOK).expect("rulebook loads").rulebook;
    let character = Character::from_starting(&rulebook);
    let panel = Panel::sheet(
        &rulebook,
        &character,
        &ReaderConfig::default(),
        size.columns,
    );
    let mut pager = Pager::new(panel.lines.clone(), size.rows);
    let areas = draw_panel(&mut framebuffer, metrics, theme, &panel, &pager).unwrap();
    assert!(areas.previous_page().is_none());
    let next = areas.next_page().expect("a second page");
    assert!(areas.next_page_at(next.top_left));
    assert_eq!(areas.action_at(next.top_left), Some(Action::PageDown));
    assert!(areas
        .back()
        .is_some_and(|back| areas.back_at(back.top_left)));
    assert_eq!(
        areas.action_at(areas.back().unwrap().top_left),
        Some(Action::Back)
    );

    pager.next_page();
    let areas = draw_panel(&mut framebuffer, metrics, theme, &panel, &pager).unwrap();
    let previous = areas.previous_page().expect("a previous page");
    assert!(areas.previous_page_at(previous.top_left));
    assert_eq!(areas.action_at(previous.top_left), Some(Action::PageUp));
}

#[test]
fn the_page_arrows_have_fingertip_targets() {
    let metrics = Metrics::new(320, 240, 28, 36);
    let size = page_size(metrics);
    let theme = Theme::default();
    let rulebook = Rulebook::load(RULEBOOK).expect("rulebook loads").rulebook;
    let character = Character::from_starting(&rulebook);
    let panel = Panel::sheet(
        &rulebook,
        &character,
        &ReaderConfig::default(),
        size.columns,
    );
    let mut pager = Pager::new(panel.lines.clone(), size.rows);
    pager.next_page();
    let mut framebuffer = support::Framebuffer::<320, 240>::new();
    let areas = draw_panel(&mut framebuffer, metrics, theme, &panel, &pager).unwrap();

    let previous = areas.previous_page().expect("a previous page");
    let next = areas.next_page().expect("a next page");
    let cell = metrics.font.character_width();
    let line_height = metrics.font.line_height();
    assert!(
        previous.size.width > cell && previous.size.height > line_height,
        "the previous arrow hit box must be wider and taller than the glyph: {previous:?}"
    );
    assert!(
        next.size.width > cell && next.size.height > line_height,
        "the next arrow hit box must be wider and taller than the glyph: {next:?}"
    );
    assert!(
        previous.top_left.x + previous.size.width as i32 <= next.top_left.x,
        "the two arrow hit boxes must not overlap: {previous:?} {next:?}"
    );
    assert!(!areas.previous_page_at(next.top_left));
}
