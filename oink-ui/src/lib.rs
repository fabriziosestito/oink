//! Shared reader layout for oink players.
//!
//! The crate draws through the `embedded-graphics` `DrawTarget` trait and has
//! no file, network, or terminal access. Device players supply the target,
//! the screen metrics, and the input events.
//!
//! The [`config`] and [`content`] modules are always available. They read
//! the `ui` block of `config.yaml` and build the center page without a
//! renderer dependency, so the terminal player can reuse the presentation
//! settings. The drawing modules sit behind the `render` feature.

pub mod config;
pub mod content;

#[cfg(feature = "render")]
pub mod input;
#[cfg(feature = "render")]
pub mod layout;
#[cfg(feature = "render")]
pub mod metrics;
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "render")]
pub mod theme;

pub use config::{
    BottomBar, BottomBarFields, CharacterConfig, ColorChoice, EinkTheme, LayoutConfig, MapConfig,
    PageConfig, PageLayout, ReaderConfig, SheetGroup, StatusConfig, TerminalTheme, ThemeConfig,
    TopBar, TopBarFields, UiConfig, UiConfigError, ZoneOverride,
};
pub use content::{BarValue, Header, Line, Log, Pager, Panel, Scene, Span, Tone, View};
#[cfg(feature = "render")]
pub use input::{Action, BarButton, Effect, Focus, Key, UiState};
#[cfg(feature = "render")]
pub use layout::{DirtyZones, Zones};
#[cfg(feature = "render")]
pub use metrics::{FontSize, Metrics};
#[cfg(feature = "render")]
pub use render::{draw_empty, draw_page, draw_panel, draw_reader, page_size, HitAreas, PageSize};
#[cfg(feature = "render")]
pub use theme::Theme;
