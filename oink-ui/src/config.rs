//! Optional `ui.yaml` file for a game bundle.
//!
//! The file controls presentation only. The engine never reads it. Every
//! field has a default, so a story without the file keeps the player
//! defaults. Unknown fields are rejected.

use oink_core::oink_rulebook::Rulebook;
use serde::Deserialize;
use std::fmt;

/// The file name inside a game bundle.
pub const UI_FILE: &str = "ui.yaml";

/// The only file format this player understands.
pub const VERSION: u32 = 1;

/// The presentation settings of one game bundle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub version: u32,
    pub reader: ReaderConfig,
    pub status: StatusConfig,
    pub theme: ThemeConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            version: VERSION,
            reader: ReaderConfig::default(),
            status: StatusConfig::default(),
            theme: ThemeConfig::default(),
        }
    }
}

impl UiConfig {
    /// Parse `ui.yaml`. Structural errors fail here. Rulebook references
    /// fail in [`UiConfig::validate`], because this function has no rulebook.
    /// An empty file means every default.
    pub fn from_yaml(yaml: &str) -> Result<Self, UiConfigError> {
        let config: Self = serde_yaml::from_str::<Option<Self>>(yaml)?.unwrap_or_default();
        config.validate_structure()?;
        Ok(config)
    }

    /// Check the rulebook references and the structural rules again.
    pub fn validate(&self, rulebook: &Rulebook) -> Result<(), UiConfigError> {
        self.validate_structure()?;
        for id in &self.reader.top_bar.resources {
            if !rulebook.resources.contains_key(id) {
                return Err(UiConfigError::UnknownResource(id.clone()));
            }
        }
        for id in &self.reader.top_bar.characteristics {
            if !rulebook.characteristics.contains_key(id) {
                return Err(UiConfigError::UnknownCharacteristic(id.clone()));
            }
        }
        if let Some(resources) = &self.status.resources {
            for id in resources {
                if !rulebook.resources.contains_key(id) {
                    return Err(UiConfigError::UnknownResource(id.clone()));
                }
            }
        }
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), UiConfigError> {
        if self.version != VERSION {
            return Err(UiConfigError::Version(self.version));
        }
        for (field, value) in [
            ("reader.items_per_page", self.reader.items_per_page),
            ("reader.log_entries", self.reader.log_entries),
            ("reader.save_slots", self.reader.save_slots),
        ] {
            if value == 0 {
                return Err(UiConfigError::Zero { field });
            }
        }
        for (index, group) in self.reader.sheet_order.iter().enumerate() {
            if self.reader.sheet_order[..index].contains(group) {
                return Err(UiConfigError::DuplicateSheetGroup(*group));
            }
        }
        for (field, value) in [
            ("theme.eink.paper", self.theme.eink.paper),
            ("theme.eink.ink", self.theme.eink.ink),
            ("theme.eink.dim", self.theme.eink.dim),
            ("theme.eink.accent", self.theme.eink.accent),
        ] {
            if value > 15 {
                return Err(UiConfigError::Gray4OutOfRange { field, value });
            }
        }
        Ok(())
    }
}

/// Page sizes and the top summary bar of the reader.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReaderConfig {
    pub items_per_page: usize,
    pub log_entries: usize,
    pub save_slots: usize,
    pub sheet_order: Vec<SheetGroup>,
    pub top_bar: TopBar,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            items_per_page: 8,
            log_entries: 50,
            save_slots: 3,
            sheet_order: SheetGroup::ALL.to_vec(),
            top_bar: TopBar::default(),
        }
    }
}

/// One group of the character sheet, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SheetGroup {
    Characteristics,
    Abilities,
    Perks,
    Conditions,
    Spells,
}

impl SheetGroup {
    pub const ALL: [Self; 5] = [
        Self::Characteristics,
        Self::Abilities,
        Self::Perks,
        Self::Conditions,
        Self::Spells,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Characteristics => "characteristics",
            Self::Abilities => "abilities",
            Self::Perks => "perks",
            Self::Conditions => "conditions",
            Self::Spells => "spells",
        }
    }
}

impl fmt::Display for SheetGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The values shown in the top summary bar.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopBar {
    pub resources: Vec<String>,
    pub characteristics: Vec<String>,
}

/// The values shown in the terminal player's status bar.
///
/// `None` means the player default: every resource, the level, and the
/// conditions. An explicit empty list hides the resources.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StatusConfig {
    pub resources: Option<Vec<String>>,
    pub show_level: Option<bool>,
    pub show_conditions: Option<bool>,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            resources: None,
            show_level: Some(true),
            show_conditions: Some(true),
        }
    }
}

/// The palette per device class.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeConfig {
    pub eink: EinkTheme,
    pub terminal: TerminalTheme,
}

/// Gray4 levels, from 0 (black) to 15 (white).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EinkTheme {
    pub paper: u8,
    pub ink: u8,
    pub dim: u8,
    pub accent: u8,
}

impl Default for EinkTheme {
    fn default() -> Self {
        Self {
            paper: 15,
            ink: 0,
            dim: 7,
            accent: 4,
        }
    }
}

/// The three terminal colors a story can change. The semantic styles keep
/// their standard colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerminalTheme {
    pub text: ColorChoice,
    pub background: ColorChoice,
    pub accent: ColorChoice,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        Self {
            text: ColorChoice::Default,
            background: ColorChoice::Default,
            accent: ColorChoice::Default,
        }
    }
}

/// A terminal palette entry, or `default` for the terminal theme.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorChoice {
    #[default]
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
}

/// A problem in `ui.yaml`.
#[derive(Debug)]
pub enum UiConfigError {
    Yaml(serde_yaml::Error),
    Version(u32),
    Zero { field: &'static str },
    DuplicateSheetGroup(SheetGroup),
    Gray4OutOfRange { field: &'static str, value: u8 },
    UnknownResource(String),
    UnknownCharacteristic(String),
}

impl fmt::Display for UiConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(error) => write!(f, "invalid {UI_FILE}: {error}"),
            Self::Version(version) => write!(
                f,
                "{UI_FILE} version {version} is not supported; this player reads version {VERSION}"
            ),
            Self::Zero { field } => write!(f, "{UI_FILE} `{field}` must be greater than zero"),
            Self::DuplicateSheetGroup(group) => {
                write!(f, "{UI_FILE} sheet_order lists `{group}` twice")
            }
            Self::Gray4OutOfRange { field, value } => write!(
                f,
                "{UI_FILE} `{field}` is {value}, but Gray4 values range from 0 to 15"
            ),
            Self::UnknownResource(id) => {
                write!(f, "{UI_FILE} names unknown resource `{id}`")
            }
            Self::UnknownCharacteristic(id) => {
                write!(f, "{UI_FILE} names unknown characteristic `{id}`")
            }
        }
    }
}

impl std::error::Error for UiConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Yaml(error) => Some(error),
            _ => None,
        }
    }
}

impl From<serde_yaml::Error> for UiConfigError {
    fn from(error: serde_yaml::Error) -> Self {
        Self::Yaml(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULEBOOK: &str = r#"
characteristics:
  physique:
    name: Physique
    min: 1
    max: 14
    bonus:
      thresholds:
        - { at: 1, bonus: 0 }
resources:
  health: { name: Health, min: 0, max: 10 }
  focus: { name: Focus, min: 0, max: 5 }
"#;

    fn rulebook() -> Rulebook {
        Rulebook::load(RULEBOOK).expect("rulebook loads").rulebook
    }

    #[test]
    fn a_full_file_parses() {
        let yaml = r#"
version: 1
reader:
  items_per_page: 4
  log_entries: 20
  save_slots: 2
  sheet_order: [abilities, spells, characteristics]
  top_bar:
    resources: [health]
    characteristics: [physique]
status:
  resources: [focus]
  show_level: false
  show_conditions: false
theme:
  eink:
    paper: 14
    ink: 1
    dim: 6
    accent: 3
  terminal:
    text: white
    background: black
    accent: cyan
"#;
        let config = UiConfig::from_yaml(yaml).expect("config parses");
        assert_eq!(config.reader.items_per_page, 4);
        assert_eq!(
            config.reader.sheet_order,
            vec![
                SheetGroup::Abilities,
                SheetGroup::Spells,
                SheetGroup::Characteristics
            ]
        );
        assert_eq!(config.reader.top_bar.resources, vec!["health"]);
        assert_eq!(config.status.resources, Some(vec!["focus".to_string()]));
        assert_eq!(config.status.show_level, Some(false));
        assert_eq!(config.theme.eink.paper, 14);
        assert_eq!(config.theme.terminal.text, ColorChoice::White);
        assert_eq!(config.theme.terminal.accent, ColorChoice::Cyan);
        config.validate(&rulebook()).expect("references resolve");
    }

    #[test]
    fn missing_fields_use_the_defaults() {
        let config = UiConfig::from_yaml("version: 1\n").expect("config parses");
        assert_eq!(config, UiConfig::default());
    }

    #[test]
    fn an_empty_file_uses_the_defaults() {
        assert_eq!(
            UiConfig::from_yaml("").expect("config parses"),
            UiConfig::default()
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let error = UiConfig::from_yaml("version: 1\nsurprise: true\n").unwrap_err();
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn a_future_version_is_rejected() {
        let error = UiConfig::from_yaml("version: 2\n").unwrap_err();
        assert!(
            error.to_string().contains("version 2 is not supported"),
            "{error}"
        );
    }

    #[test]
    fn zero_page_sizes_are_rejected() {
        for field in ["items_per_page", "log_entries", "save_slots"] {
            let yaml = format!("reader:\n  {field}: 0\n");
            let error = UiConfig::from_yaml(&yaml).unwrap_err();
            assert!(error.to_string().contains(field), "{error}");
        }
    }

    #[test]
    fn gray4_values_are_checked() {
        for field in ["paper", "ink", "dim", "accent"] {
            let yaml = format!("theme:\n  eink:\n    {field}: 16\n");
            let error = UiConfig::from_yaml(&yaml).unwrap_err();
            assert!(error.to_string().contains(field), "{error}");
        }
    }

    #[test]
    fn duplicate_sheet_groups_are_rejected() {
        let error = UiConfig::from_yaml("reader:\n  sheet_order: [perks, perks]\n").unwrap_err();
        assert!(error.to_string().contains("perks"), "{error}");
    }

    #[test]
    fn rulebook_references_are_checked() {
        let yaml = "reader:\n  top_bar:\n    resources: [missing]\n";
        let config = UiConfig::from_yaml(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");

        let yaml = "reader:\n  top_bar:\n    characteristics: [missing]\n";
        let config = UiConfig::from_yaml(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");

        let yaml = "status:\n  resources: [missing]\n";
        let config = UiConfig::from_yaml(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");
    }

    #[test]
    fn an_explicit_empty_resource_list_hides_the_status() {
        let config = UiConfig::from_yaml("status:\n  resources: []\n").expect("config parses");
        assert_eq!(config.status.resources, Some(Vec::new()));
        assert_eq!(config.status.show_level, Some(true));
    }
}
