//! Shared reader layout for oink players.
//!
//! The crate draws through the `embedded-graphics` `DrawTarget` trait and has
//! no file, network, or terminal access. Device players supply the target,
//! the screen metrics, and the input events.

pub mod input;
pub mod layout;
pub mod metrics;
pub mod render;
pub mod theme;

pub use input::{Action, BarButton, Effect, Focus, UiState, View};
pub use layout::Zones;
pub use metrics::Metrics;
pub use render::draw_empty;
pub use theme::Theme;
