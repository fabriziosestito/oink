//! Semantic styles. Colors come from the terminal's 16-color palette, never
//! from fixed RGB values, so the player follows the user's theme.

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
        match std::env::var_os("NO_COLOR") {
            Some(value) if !value.is_empty() => Self::monochrome(),
            _ => Self::colored(),
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
}
