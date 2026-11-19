//! The `ui` block of `config.yaml` for a game bundle.
//!
//! The block controls presentation only. The engine never reads it. Every
//! field has a default, so a bundle without the block keeps the player
//! defaults. Unknown fields in the block are rejected.

use oink_core::oink_rulebook::Rulebook;
use oink_core::DEFAULT_MODE;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt;

/// The file name of the document that holds the `ui` block.
const CONFIG_FILE: &str = "config.yaml";

/// The presentation settings of one game bundle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UiConfig {
    pub reader: ReaderConfig,
    pub layout: LayoutConfig,
    pub pages: BTreeMap<String, PageConfig>,
    pub characters: BTreeMap<String, CharacterConfig>,
    pub maps: BTreeMap<String, MapConfig>,
    pub status: StatusConfig,
    pub theme: ThemeConfig,
}

/// A character that a scene can speak with, keyed by the id the story uses
/// in the `# speaker: <id>` and `# portrait: <id>` tags.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CharacterConfig {
    /// The display name of the character.
    pub name: String,
    /// The portrait file, relative to the bundle directory. A player that
    /// can draw pictures loads it when a scene shows the character.
    pub portrait: Option<String>,
}

/// A map layout for a map page, keyed by the id the story uses in the
/// `# map: <id>` tag.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapConfig {
    /// The display name of the map, drawn above the location choices.
    pub label: String,
}

/// The `config.yaml` document, without the fields the engine owns.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    #[serde(default)]
    ui: Option<UiConfig>,
}

impl UiConfig {
    /// Parse the `ui` block of `config.yaml`. Structural errors fail here.
    /// Rulebook references fail in [`UiConfig::validate`], because this
    /// function has no rulebook. A missing or empty block means every
    /// default.
    pub fn from_config(yaml: &str) -> Result<Self, UiConfigError> {
        let raw: RawConfig = serde_yaml::from_str::<Option<RawConfig>>(yaml)?.unwrap_or_default();
        let config = raw.ui.unwrap_or_default();
        config.validate_structure()?;
        Ok(config)
    }

    /// Check the rulebook references and the structural rules again.
    pub fn validate(&self, rulebook: &Rulebook) -> Result<(), UiConfigError> {
        self.validate_structure()?;
        for id in &self.layout.top_bar.resources {
            if !rulebook.resources.contains_key(id) {
                return Err(UiConfigError::UnknownResource(id.clone()));
            }
        }
        for id in &self.layout.top_bar.characteristics {
            if !rulebook.characteristics.contains_key(id) {
                return Err(UiConfigError::UnknownCharacteristic(id.clone()));
            }
        }
        for page in self.pages.values() {
            let Some(ZoneOverride::Fields(fields)) = &page.top_bar else {
                continue;
            };
            for id in fields.resources.iter().flatten() {
                if !rulebook.resources.contains_key(id) {
                    return Err(UiConfigError::UnknownResource(id.clone()));
                }
            }
            for id in fields.characteristics.iter().flatten() {
                if !rulebook.characteristics.contains_key(id) {
                    return Err(UiConfigError::UnknownCharacteristic(id.clone()));
                }
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
        self.check_buttons(&self.layout.bottom_bar.buttons)?;
        for page in self.pages.values() {
            if let Some(ZoneOverride::Fields(fields)) = &page.bottom_bar {
                if let Some(buttons) = &fields.buttons {
                    self.check_buttons(buttons)?;
                }
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

    fn check_buttons(&self, buttons: &[String]) -> Result<(), UiConfigError> {
        for (index, name) in buttons.iter().enumerate() {
            if !BottomBar::BUTTONS.contains(&name.as_str()) {
                return Err(UiConfigError::UnknownButton(name.clone()));
            }
            if buttons[..index].contains(name) {
                return Err(UiConfigError::DuplicateButton(name.clone()));
            }
        }
        Ok(())
    }
}

/// Page sizes and the sheet order of the reader.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReaderConfig {
    pub items_per_page: usize,
    pub log_entries: usize,
    pub save_slots: usize,
    pub sheet_order: Vec<SheetGroup>,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            items_per_page: 8,
            log_entries: 50,
            save_slots: 3,
            sheet_order: SheetGroup::ALL.to_vec(),
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

    /// The heading drawn above the group on the sheet.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Characteristics => "Characteristics",
            Self::Abilities => "Abilities",
            Self::Perks => "Perks",
            Self::Conditions => "Conditions",
            Self::Spells => "Spells",
        }
    }
}

impl fmt::Display for SheetGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The base layout of the reader: top bar, center, bottom bar.
///
/// Every scene mode uses this layout unless `pages` overrides it for the
/// mode's id.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    pub top_bar: TopBar,
    pub bottom_bar: BottomBar,
}

/// The values shown in the top summary bar.
///
/// `height` in pixels is optional; the device metrics give the default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopBar {
    pub resources: Vec<String>,
    pub characteristics: Vec<String>,
    pub height: Option<u32>,
}

/// The buttons shown in the bottom bar, in display order.
///
/// `height` in pixels is optional; the device metrics give the default.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BottomBar {
    pub buttons: Vec<String>,
    pub height: Option<u32>,
}

impl Default for BottomBar {
    fn default() -> Self {
        Self {
            buttons: ["sheet", "inventory", "log", "menu"]
                .iter()
                .map(|name| name.to_string())
                .collect(),
            height: None,
        }
    }
}

impl BottomBar {
    /// The button names a bottom bar can list.
    pub const BUTTONS: [&str; 4] = ["sheet", "inventory", "log", "menu"];
}

/// The layout override of one scene mode, keyed by the mode id.
///
/// A zone key the block omits keeps the base value. The value `false`
/// removes the zone on that page, and `true` keeps the base zone. A mapping
/// replaces the fields it lists, and the other fields inherit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PageConfig {
    pub top_bar: Option<ZoneOverride<TopBarFields>>,
    pub bottom_bar: Option<ZoneOverride<BottomBarFields>>,
}

/// One zone of a page block: `false` or `true`, or a field mapping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum ZoneOverride<Fields> {
    /// `false` removes the zone; `true` keeps the base zone.
    Switch(bool),
    /// A mapping of the fields that change.
    Fields(Fields),
}

/// The top bar fields a page block can override.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TopBarFields {
    pub resources: Option<Vec<String>>,
    pub characteristics: Option<Vec<String>>,
    pub height: Option<u32>,
}

/// The bottom bar fields a page block can override.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BottomBarFields {
    pub buttons: Option<Vec<String>>,
    pub height: Option<u32>,
}

/// The layout of one scene mode, after the page block merged into the base.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageLayout {
    pub top_bar: Option<TopBar>,
    pub bottom_bar: Option<BottomBar>,
    /// Notes for the host: a mode id without a page block uses the base
    /// layout and names itself here.
    pub warnings: Vec<String>,
}

impl UiConfig {
    /// The layout for the scene mode `mode`.
    ///
    /// The `prose` mode is the base layout by definition and never warns.
    /// Every other id without a `pages` block uses the base layout and adds
    /// a warning.
    pub fn resolve_page(&self, mode: &str) -> PageLayout {
        let mut warnings = Vec::new();
        if mode != DEFAULT_MODE && !self.pages.contains_key(mode) {
            warnings.push(format!(
                "unknown scene mode `{mode}`: using the base layout"
            ));
        }
        let Some(page) = self.pages.get(mode) else {
            return PageLayout {
                top_bar: Some(self.layout.top_bar.clone()),
                bottom_bar: Some(self.layout.bottom_bar.clone()),
                warnings,
            };
        };
        let top_bar = match &page.top_bar {
            None => Some(self.layout.top_bar.clone()),
            Some(ZoneOverride::Switch(true)) => Some(self.layout.top_bar.clone()),
            Some(ZoneOverride::Switch(false)) => None,
            Some(ZoneOverride::Fields(fields)) => Some(TopBar {
                resources: fields
                    .resources
                    .clone()
                    .unwrap_or_else(|| self.layout.top_bar.resources.clone()),
                characteristics: fields
                    .characteristics
                    .clone()
                    .unwrap_or_else(|| self.layout.top_bar.characteristics.clone()),
                height: fields.height.or(self.layout.top_bar.height),
            }),
        };
        let bottom_bar = match &page.bottom_bar {
            None => Some(self.layout.bottom_bar.clone()),
            Some(ZoneOverride::Switch(true)) => Some(self.layout.bottom_bar.clone()),
            Some(ZoneOverride::Switch(false)) => None,
            Some(ZoneOverride::Fields(fields)) => Some(BottomBar {
                buttons: fields
                    .buttons
                    .clone()
                    .unwrap_or_else(|| self.layout.bottom_bar.buttons.clone()),
                height: fields.height.or(self.layout.bottom_bar.height),
            }),
        };
        PageLayout {
            top_bar,
            bottom_bar,
            warnings,
        }
    }
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

/// A problem in the `ui` block of `config.yaml`.
#[derive(Debug)]
pub enum UiConfigError {
    Yaml(serde_yaml::Error),
    Zero { field: &'static str },
    DuplicateSheetGroup(SheetGroup),
    UnknownButton(String),
    DuplicateButton(String),
    Gray4OutOfRange { field: &'static str, value: u8 },
    UnknownResource(String),
    UnknownCharacteristic(String),
}

impl fmt::Display for UiConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(error) => write!(f, "invalid {CONFIG_FILE}: {error}"),
            Self::Zero { field } => {
                write!(f, "{CONFIG_FILE} `ui.{field}` must be greater than zero")
            }
            Self::DuplicateSheetGroup(group) => {
                write!(
                    f,
                    "{CONFIG_FILE} `ui.reader.sheet_order` lists `{group}` twice"
                )
            }
            Self::UnknownButton(name) => write!(
                f,
                "{CONFIG_FILE} `ui.layout.bottom_bar.buttons` names unknown button `{name}`"
            ),
            Self::DuplicateButton(name) => write!(
                f,
                "{CONFIG_FILE} `ui.layout.bottom_bar.buttons` lists `{name}` twice"
            ),
            Self::Gray4OutOfRange { field, value } => write!(
                f,
                "{CONFIG_FILE} `ui.{field}` is {value}, but Gray4 values range from 0 to 15"
            ),
            Self::UnknownResource(id) => {
                write!(f, "{CONFIG_FILE} `ui` names unknown resource `{id}`")
            }
            Self::UnknownCharacteristic(id) => {
                write!(f, "{CONFIG_FILE} `ui` names unknown characteristic `{id}`")
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
    fn a_full_block_parses() {
        let yaml = r#"
ui:
  reader:
    items_per_page: 4
    log_entries: 20
    save_slots: 2
    sheet_order: [abilities, spells, characteristics]
  layout:
    top_bar:
      resources: [health]
      characteristics: [physique]
      height: 40
    bottom_bar:
      buttons: [log, sheet]
      height: 48
  pages:
    dialog:
      top_bar:
        height: 96
      bottom_bar: false
  characters:
    keeper:
      name: The Shrine-Keeper
    oka:
      name: Oka
      portrait: portraits/oka.png
  maps:
    ravine:
      label: The Ravine
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
        let config = UiConfig::from_config(yaml).expect("config parses");
        assert_eq!(config.reader.items_per_page, 4);
        assert_eq!(
            config.reader.sheet_order,
            vec![
                SheetGroup::Abilities,
                SheetGroup::Spells,
                SheetGroup::Characteristics
            ]
        );
        assert_eq!(config.layout.top_bar.resources, vec!["health"]);
        assert_eq!(config.layout.top_bar.height, Some(40));
        assert_eq!(
            config.layout.bottom_bar.buttons,
            vec!["log".to_string(), "sheet".to_string()]
        );
        assert_eq!(config.layout.bottom_bar.height, Some(48));
        assert!(config.pages.contains_key("dialog"));
        assert_eq!(
            config.characters["keeper"].name,
            "The Shrine-Keeper".to_string()
        );
        assert_eq!(
            config.characters["oka"].portrait,
            Some("portraits/oka.png".to_string())
        );
        assert_eq!(config.maps["ravine"].label, "The Ravine".to_string());
        assert_eq!(config.status.resources, Some(vec!["focus".to_string()]));
        assert_eq!(config.status.show_level, Some(false));
        assert_eq!(config.theme.eink.paper, 14);
        assert_eq!(config.theme.terminal.text, ColorChoice::White);
        assert_eq!(config.theme.terminal.accent, ColorChoice::Cyan);
        config.validate(&rulebook()).expect("references resolve");
    }

    #[test]
    fn the_block_sits_beside_the_title() {
        let yaml = "title: The High Pass\nui:\n  status:\n    show_level: false\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        assert_eq!(config.status.show_level, Some(false));
        assert_eq!(config.reader, ReaderConfig::default());
    }

    #[test]
    fn a_missing_block_uses_the_defaults() {
        let config = UiConfig::from_config("title: The High Pass\n").expect("config parses");
        assert_eq!(config, UiConfig::default());
    }

    #[test]
    fn an_empty_document_uses_the_defaults() {
        assert_eq!(
            UiConfig::from_config("").expect("config parses"),
            UiConfig::default()
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let error = UiConfig::from_config("ui:\n  surprise: true\n").unwrap_err();
        assert!(error.to_string().contains("unknown field"), "{error}");

        let error =
            UiConfig::from_config("ui:\n  characters:\n    keeper:\n      nope: 1\n").unwrap_err();
        assert!(error.to_string().contains("unknown field"), "{error}");

        let error =
            UiConfig::from_config("ui:\n  maps:\n    ravine:\n      nope: 1\n").unwrap_err();
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn a_character_needs_a_name_and_a_map_needs_a_label() {
        let error = UiConfig::from_config("ui:\n  characters:\n    keeper: {}\n").unwrap_err();
        assert!(error.to_string().contains("name"), "{error}");

        let error = UiConfig::from_config("ui:\n  maps:\n    ravine: {}\n").unwrap_err();
        assert!(error.to_string().contains("label"), "{error}");
    }

    #[test]
    fn zero_page_sizes_are_rejected() {
        for field in ["items_per_page", "log_entries", "save_slots"] {
            let yaml = format!("ui:\n  reader:\n    {field}: 0\n");
            let error = UiConfig::from_config(&yaml).unwrap_err();
            assert!(error.to_string().contains(field), "{error}");
        }
    }

    #[test]
    fn gray4_values_are_checked() {
        for field in ["paper", "ink", "dim", "accent"] {
            let yaml = format!("ui:\n  theme:\n    eink:\n      {field}: 16\n");
            let error = UiConfig::from_config(&yaml).unwrap_err();
            assert!(error.to_string().contains(field), "{error}");
        }
    }

    #[test]
    fn duplicate_sheet_groups_are_rejected() {
        let error =
            UiConfig::from_config("ui:\n  reader:\n    sheet_order: [perks, perks]\n").unwrap_err();
        assert!(error.to_string().contains("perks"), "{error}");
    }

    #[test]
    fn sheet_groups_have_a_heading_label() {
        assert_eq!(SheetGroup::Characteristics.label(), "Characteristics");
        assert_eq!(SheetGroup::Spells.label(), "Spells");
        assert_eq!(SheetGroup::Perks.as_str(), "perks");
    }

    #[test]
    fn rulebook_references_are_checked() {
        let yaml = "ui:\n  layout:\n    top_bar:\n      resources: [missing]\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");

        let yaml = "ui:\n  layout:\n    top_bar:\n      characteristics: [missing]\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");

        let yaml = "ui:\n  pages:\n    dialog:\n      top_bar:\n        resources: [missing]\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");

        let yaml =
            "ui:\n  pages:\n    dialog:\n      top_bar:\n        characteristics: [missing]\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");

        let yaml = "ui:\n  status:\n    resources: [missing]\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let error = config.validate(&rulebook()).unwrap_err();
        assert!(error.to_string().contains("missing"), "{error}");
    }

    #[test]
    fn unknown_and_duplicate_buttons_are_rejected() {
        let yaml = "ui:\n  layout:\n    bottom_bar:\n      buttons: [nope]\n";
        let error = UiConfig::from_config(yaml).unwrap_err();
        assert!(
            error.to_string().contains("unknown button `nope`"),
            "{error}"
        );

        let yaml = "ui:\n  layout:\n    bottom_bar:\n      buttons: [log, log]\n";
        let error = UiConfig::from_config(yaml).unwrap_err();
        assert!(error.to_string().contains("`log` twice"), "{error}");

        let yaml = "ui:\n  pages:\n    dialog:\n      bottom_bar:\n        buttons: [nope]\n";
        let error = UiConfig::from_config(yaml).unwrap_err();
        assert!(
            error.to_string().contains("unknown button `nope`"),
            "{error}"
        );
    }

    #[test]
    fn the_prose_mode_is_the_base_layout() {
        let yaml = "title: T\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let page = config.resolve_page("prose");
        assert_eq!(page.warnings, Vec::<String>::new());
        assert_eq!(page.top_bar, Some(TopBar::default()));
        assert_eq!(page.bottom_bar, Some(BottomBar::default()));

        let yaml = "ui:\n  layout:\n    top_bar:\n      resources: [health]\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let page = config.resolve_page("prose");
        assert_eq!(page.top_bar.unwrap().resources, vec!["health"]);
    }

    #[test]
    fn a_mode_without_a_block_warns_and_keeps_the_base() {
        let yaml = "title: T\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let page = config.resolve_page("dialog");
        assert_eq!(page.top_bar, Some(TopBar::default()));
        assert_eq!(page.bottom_bar, Some(BottomBar::default()));
        assert_eq!(
            page.warnings,
            vec!["unknown scene mode `dialog`: using the base layout".to_string()]
        );

        let yaml = "ui:\n  pages:\n    dialog:\n      top_bar: false\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let page = config.resolve_page("dialog");
        assert!(page.top_bar.is_none());
        assert!(page.warnings.is_empty());
    }

    #[test]
    fn a_page_block_merges_field_by_field() {
        let yaml = r#"
ui:
  layout:
    top_bar:
      resources: [health, focus]
      characteristics: [physique]
      height: 40
    bottom_bar:
      buttons: [sheet, inventory, log]
      height: 48
  pages:
    dialog:
      top_bar:
        height: 96
      bottom_bar: false
    map:
      top_bar: false
      bottom_bar: false
"#;
        let config = UiConfig::from_config(yaml).expect("config parses");

        let page = config.resolve_page("dialog");
        let top = page.top_bar.expect("the dialog page keeps the top bar");
        assert_eq!(top.resources, vec!["health", "focus"]);
        assert_eq!(top.characteristics, vec!["physique"]);
        assert_eq!(top.height, Some(96));
        assert!(page.bottom_bar.is_none());

        let page = config.resolve_page("map");
        assert!(page.top_bar.is_none());
        assert!(page.bottom_bar.is_none());
        assert!(page.warnings.is_empty());

        let page = config.resolve_page("prose");
        let top = page.top_bar.expect("the base layout keeps both bars");
        assert_eq!(top.height, Some(40));
        assert_eq!(page.bottom_bar.unwrap().height, Some(48));
    }

    #[test]
    fn a_page_field_mapping_replaces_only_the_listed_values() {
        let yaml = r#"
ui:
  layout:
    top_bar:
      resources: [health, focus]
      characteristics: [physique]
    bottom_bar:
      buttons: [sheet, inventory, log]
  pages:
    dialog:
      top_bar:
        resources: [focus]
      bottom_bar:
        height: 20
"#;
        let config = UiConfig::from_config(yaml).expect("config parses");
        let page = config.resolve_page("dialog");
        let top = page.top_bar.expect("the mapping keeps the bar");
        assert_eq!(top.resources, vec!["focus"]);
        assert_eq!(top.characteristics, vec!["physique"]);
        assert_eq!(top.height, None);
        let bottom = page.bottom_bar.expect("the mapping keeps the bar");
        assert_eq!(
            bottom.buttons,
            vec![
                "sheet".to_string(),
                "inventory".to_string(),
                "log".to_string()
            ]
        );
        assert_eq!(bottom.height, Some(20));
    }

    #[test]
    fn the_switch_true_keeps_the_base_zone() {
        let yaml = "ui:\n  pages:\n    dialog:\n      top_bar: true\n      bottom_bar: true\n";
        let config = UiConfig::from_config(yaml).expect("config parses");
        let page = config.resolve_page("dialog");
        assert_eq!(page.top_bar, Some(TopBar::default()));
        assert_eq!(page.bottom_bar, Some(BottomBar::default()));
    }

    #[test]
    fn an_unknown_field_in_a_page_block_is_rejected() {
        // The untagged zone override reports the bad mapping as a variant
        // mismatch, not as an unknown field. The mapping still fails.
        let error =
            UiConfig::from_config("ui:\n  pages:\n    dialog:\n      top_bar:\n        nope: 1\n")
                .unwrap_err();
        assert!(
            error.to_string().contains("did not match any variant"),
            "{error}"
        );
    }

    #[test]
    fn an_explicit_empty_resource_list_hides_the_status() {
        let config =
            UiConfig::from_config("ui:\n  status:\n    resources: []\n").expect("config parses");
        assert_eq!(config.status.resources, Some(Vec::new()));
        assert_eq!(config.status.show_level, Some(true));
    }
}
