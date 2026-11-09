//! Semantic styles. Colors come from the terminal's 16-color palette, never
//! from fixed RGB values, so the player follows the user's theme.

use oink_ui::{ColorChoice, TerminalTheme};
use ratatui::style::{Color, Modifier, Style};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub title: Style,
    pub rule: Style,
    pub prose: Style,
    pub choice_number: Style,
    pub choice: Style,
    /// Patched over every span of the highlighted choice.
    pub choice_selected: Style,
    pub check: Style,
    pub success: Style,
    pub critical_success: Style,
    pub failure: Style,
    pub critical_failure: Style,
    pub gain: Style,
    pub loss: Style,
    pub condition: Style,
    pub note: Style,
    pub the_end: Style,
    pub status: Style,
    pub hint: Style,
    pub resource_full: Style,
    pub resource_half: Style,
    pub resource_low: Style,
}

impl Theme {
    /// Colors unless `NO_COLOR` is set to a non-empty value.
    pub fn detect() -> Self {
        if no_color() {
            Self::monochrome()
        } else {
            Self::colored()
        }
    }

    /// The terminal palette from `ui.yaml`. `NO_COLOR` wins over the file.
    pub fn from_config(config: &TerminalTheme) -> Self {
        if no_color() {
            Self::monochrome()
        } else {
            let mut theme = Self::colored();
            theme.apply_terminal(config);
            theme
        }
    }

    fn apply_terminal(&mut self, config: &TerminalTheme) {
        if let Some(text) = color(config.text) {
            self.prose = self.prose.fg(text);
            self.choice = self.choice.fg(text);
            self.status = self.status.fg(text);
        }
        if let Some(accent) = color(config.accent) {
            self.title = self.title.fg(accent);
            self.choice_number = self.choice_number.fg(accent);
            self.note = self.note.fg(accent);
        }
        if let Some(background) = color(config.background) {
            self.title = self.title.bg(background);
            self.rule = self.rule.bg(background);
            self.prose = self.prose.bg(background);
            self.choice_number = self.choice_number.bg(background);
            self.choice = self.choice.bg(background);
            self.choice_selected = self.choice_selected.bg(background);
            self.check = self.check.bg(background);
            self.success = self.success.bg(background);
            self.critical_success = self.critical_success.bg(background);
            self.failure = self.failure.bg(background);
            self.critical_failure = self.critical_failure.bg(background);
            self.gain = self.gain.bg(background);
            self.loss = self.loss.bg(background);
            self.condition = self.condition.bg(background);
            self.note = self.note.bg(background);
            self.the_end = self.the_end.bg(background);
            self.status = self.status.bg(background);
            self.hint = self.hint.bg(background);
            self.resource_full = self.resource_full.bg(background);
            self.resource_half = self.resource_half.bg(background);
            self.resource_low = self.resource_low.bg(background);
        }
    }

    pub fn colored() -> Self {
        let plain = Style::new();
        Self {
            title: plain.add_modifier(Modifier::BOLD),
            rule: plain.add_modifier(Modifier::DIM),
            prose: plain,
            choice_number: plain.fg(Color::Cyan).add_modifier(Modifier::BOLD),
            choice: plain,
            choice_selected: plain.add_modifier(Modifier::REVERSED),
            check: plain.add_modifier(Modifier::DIM),
            success: plain.fg(Color::Green),
            critical_success: plain.fg(Color::Green).add_modifier(Modifier::BOLD),
            failure: plain.fg(Color::Red),
            critical_failure: plain.fg(Color::Red).add_modifier(Modifier::BOLD),
            gain: plain.fg(Color::Green),
            loss: plain.fg(Color::Red),
            condition: plain.fg(Color::Yellow),
            note: plain.fg(Color::Blue),
            the_end: plain.fg(Color::Magenta).add_modifier(Modifier::BOLD),
            status: plain,
            hint: plain.add_modifier(Modifier::DIM),
            resource_full: plain.fg(Color::Green),
            resource_half: plain.fg(Color::Yellow),
            resource_low: plain.fg(Color::Red).add_modifier(Modifier::BOLD),
        }
    }

    /// Bold, italic, dim, and reverse video only.
    pub fn monochrome() -> Self {
        let plain = Style::new();
        Self {
            title: plain.add_modifier(Modifier::BOLD),
            rule: plain.add_modifier(Modifier::DIM),
            prose: plain,
            choice_number: plain.add_modifier(Modifier::BOLD),
            choice: plain,
            choice_selected: plain.add_modifier(Modifier::REVERSED),
            check: plain.add_modifier(Modifier::DIM),
            success: plain,
            critical_success: plain.add_modifier(Modifier::BOLD),
            failure: plain.add_modifier(Modifier::ITALIC),
            critical_failure: plain.add_modifier(Modifier::BOLD | Modifier::ITALIC),
            gain: plain,
            loss: plain,
            condition: plain.add_modifier(Modifier::ITALIC),
            note: plain.add_modifier(Modifier::DIM),
            the_end: plain.add_modifier(Modifier::BOLD),
            status: plain,
            hint: plain.add_modifier(Modifier::DIM),
            resource_full: plain,
            resource_half: plain,
            resource_low: plain.add_modifier(Modifier::BOLD),
        }
    }

    /// The style for a resource at `value` out of `max`.
    pub fn resource(&self, value: i32, max: Option<i32>) -> Style {
        let Some(max) = max.filter(|max| *max > 0) else {
            return self.status;
        };
        let ratio = f64::from(value) / f64::from(max);
        if ratio > 0.5 {
            self.resource_full
        } else if ratio > 0.25 {
            self.resource_half
        } else {
            self.resource_low
        }
    }
}

fn no_color() -> bool {
    std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty())
}

fn color(choice: ColorChoice) -> Option<Color> {
    match choice {
        ColorChoice::Default => None,
        ColorChoice::Black => Some(Color::Black),
        ColorChoice::Red => Some(Color::Red),
        ColorChoice::Green => Some(Color::Green),
        ColorChoice::Yellow => Some(Color::Yellow),
        ColorChoice::Blue => Some(Color::Blue),
        ColorChoice::Magenta => Some(Color::Magenta),
        ColorChoice::Cyan => Some(Color::Cyan),
        ColorChoice::Gray => Some(Color::Gray),
        ColorChoice::DarkGray => Some(Color::DarkGray),
        ColorChoice::LightRed => Some(Color::LightRed),
        ColorChoice::LightGreen => Some(Color::LightGreen),
        ColorChoice::LightYellow => Some(Color::LightYellow),
        ColorChoice::LightBlue => Some(Color::LightBlue),
        ColorChoice::LightMagenta => Some(Color::LightMagenta),
        ColorChoice::LightCyan => Some(Color::LightCyan),
        ColorChoice::White => Some(Color::White),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_style_follows_the_ratio() {
        let theme = Theme::colored();
        assert_eq!(theme.resource(10, Some(10)), theme.resource_full);
        assert_eq!(theme.resource(6, Some(10)), theme.resource_full);
        assert_eq!(theme.resource(5, Some(10)), theme.resource_half);
        assert_eq!(theme.resource(3, Some(10)), theme.resource_half);
        assert_eq!(theme.resource(2, Some(10)), theme.resource_low);
        assert_eq!(theme.resource(0, Some(10)), theme.resource_low);
        assert_eq!(theme.resource(3, None), theme.status);
        assert_eq!(theme.resource(3, Some(0)), theme.status);
    }

    #[test]
    fn the_config_maps_to_ansi_colors() {
        let config = TerminalTheme {
            text: ColorChoice::White,
            background: ColorChoice::Black,
            accent: ColorChoice::Cyan,
        };
        let mut theme = Theme::colored();
        theme.apply_terminal(&config);
        assert_eq!(theme.prose.fg, Some(Color::White));
        assert_eq!(theme.prose.bg, Some(Color::Black));
        assert_eq!(theme.title.fg, Some(Color::Cyan));
        assert_eq!(theme.choice_number.fg, Some(Color::Cyan));
        assert_eq!(theme.note.fg, Some(Color::Cyan));
        // Semantic styles keep their colors.
        assert_eq!(theme.success.fg, Some(Color::Green));
        assert_eq!(theme.failure.fg, Some(Color::Red));
    }

    #[test]
    fn default_colors_leave_the_theme_alone() {
        let config = TerminalTheme::default();
        let mut theme = Theme::colored();
        let before = theme;
        theme.apply_terminal(&config);
        assert_eq!(theme, before);
    }
}
