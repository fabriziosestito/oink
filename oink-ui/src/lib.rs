//! Shared reader layout for oink players.
//!
//! The crate draws through the `embedded-graphics` `DrawTarget` trait and has
//! no file, network, or terminal access. Device players supply the target,
//! the screen metrics, and the input events.
//!
//! The [`config`] module is always available. It reads `ui.yaml` and has no
//! renderer dependency, so the terminal player can reuse the presentation
//! settings. The drawing modules sit behind the `render` feature.

pub mod config;

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
    ColorChoice, EinkTheme, ReaderConfig, SheetGroup, StatusConfig, TerminalTheme, ThemeConfig,
    TopBar, UiConfig, UiConfigError,
};
#[cfg(feature = "render")]
pub use input::{Action, BarButton, Effect, Focus, UiState, View};
#[cfg(feature = "render")]
pub use layout::Zones;
#[cfg(feature = "render")]
pub use metrics::Metrics;
#[cfg(feature = "render")]
pub use render::draw_empty;
#[cfg(feature = "render")]
pub use theme::Theme;
