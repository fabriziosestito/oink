//! Scene tags from Ink, parsed into engine events.
//!
//! Ink passes tags through as text. The engine reads the tags of the scene
//! contract in `AGENTS.md`:
//!
//! - `# mode: <id>` selects the presentation page of a scene.
//! - `# speaker: <id>` names the character that speaks in a dialog scene.
//! - `# mood: <id>` selects the portrait variant of the speaker.
//! - `# portrait: <id>` overrides the portrait of the scene.
//! - `# map: <id>` selects the map layout of a map scene.
//! - `# image: <file>` adds a picture to the scene, relative to the bundle.
//!
//! Every other tag stays unknown to the engine. Each tag applies to the
//! whole scene, and the last tag of a kind wins.

/// The mode of a scene without a `mode` tag: the base reader layout.
pub const DEFAULT_MODE: &str = "prose";

/// The value of the last `<key>: <id>` tag, or `None` without one.
///
/// A tag counts when it reads `<key>: <id>` after optional spaces, with the
/// key in any case. Later tags win, so a scene that crosses knots takes the
/// value of the knot the reader stands in last. An empty id does not
/// overwrite a previous value. The id keeps its case.
fn value_from_tags(key: &str, tags: &[String]) -> Option<String> {
    let prefix = format!("{key}:");
    let mut value = None;
    for tag in tags {
        let tag = tag.trim_start();
        if !tag.to_lowercase().starts_with(&prefix) {
            continue;
        }
        let id = tag[prefix.len()..].trim();
        if !id.is_empty() {
            value = Some(id.to_string());
        }
    }
    value
}

/// The `mode` id of the last scene tag, or `None` without one.
pub fn mode_from_tags(tags: &[String]) -> Option<String> {
    value_from_tags("mode", tags)
}

/// The `speaker` id of the last scene tag, or `None` without one.
pub fn speaker_from_tags(tags: &[String]) -> Option<String> {
    value_from_tags("speaker", tags)
}

/// The `mood` id of the last scene tag, or `None` without one.
pub fn mood_from_tags(tags: &[String]) -> Option<String> {
    value_from_tags("mood", tags)
}

/// The `portrait` id of the last scene tag, or `None` without one.
pub fn portrait_from_tags(tags: &[String]) -> Option<String> {
    value_from_tags("portrait", tags)
}

/// The `map` id of the last scene tag, or `None` without one.
pub fn map_from_tags(tags: &[String]) -> Option<String> {
    value_from_tags("map", tags)
}

/// The picture file of the last scene tag, or `None` without one.
///
/// The file path is relative to the bundle directory.
pub fn image_from_tags(tags: &[String]) -> Option<String> {
    value_from_tags("image", tags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn the_mode_tag_gives_the_id() {
        assert_eq!(
            mode_from_tags(&tags(&["mode: prose"])),
            Some("prose".to_string())
        );
        assert_eq!(
            mode_from_tags(&tags(&["mode: dialog"])),
            Some("dialog".to_string())
        );
    }

    #[test]
    fn spaces_around_the_tag_and_id_do_not_matter() {
        assert_eq!(
            mode_from_tags(&tags(&["  mode:   map  "])),
            Some("map".to_string())
        );
    }

    #[test]
    fn the_uppercase_key_is_accepted() {
        assert_eq!(
            mode_from_tags(&tags(&["MODE: dialog"])),
            Some("dialog".to_string())
        );
    }

    #[test]
    fn other_tags_are_not_modes() {
        assert_eq!(
            mode_from_tags(&tags(&["speaker: guard", "mood: angry"])),
            None
        );
    }

    #[test]
    fn every_contract_tag_gives_its_value() {
        let tags = tags(&[
            "mode: dialog",
            "speaker: keeper",
            "mood: calm",
            "portrait: keeper",
            "map: ravine",
            "image: art/shrine.png",
        ]);
        assert_eq!(mode_from_tags(&tags), Some("dialog".to_string()));
        assert_eq!(speaker_from_tags(&tags), Some("keeper".to_string()));
        assert_eq!(mood_from_tags(&tags), Some("calm".to_string()));
        assert_eq!(portrait_from_tags(&tags), Some("keeper".to_string()));
        assert_eq!(map_from_tags(&tags), Some("ravine".to_string()));
        assert_eq!(image_from_tags(&tags), Some("art/shrine.png".to_string()));
    }

    #[test]
    fn the_image_tag_keeps_its_path() {
        assert_eq!(
            image_from_tags(&tags(&["image: art/the-ravine.png", "mode: map"])),
            Some("art/the-ravine.png".to_string())
        );
        assert_eq!(image_from_tags(&tags(&["image:"])), None);
    }

    #[test]
    fn the_same_rules_hold_for_every_tag() {
        let tags = tags(&["  SPEAKER:   keeper  ", "speaker:", "MOOD: calm"]);
        assert_eq!(speaker_from_tags(&tags), Some("keeper".to_string()));
        assert_eq!(mood_from_tags(&tags), Some("calm".to_string()));
        assert_eq!(map_from_tags(&tags), None);
        assert_eq!(portrait_from_tags(&tags), None);
    }

    #[test]
    fn the_last_mode_tag_wins() {
        assert_eq!(
            mode_from_tags(&tags(&["mode: prose", "mode: dialog"])),
            Some("dialog".to_string())
        );
    }

    #[test]
    fn an_empty_id_does_not_overwrite() {
        assert_eq!(
            mode_from_tags(&tags(&["mode: prose", "mode:"])),
            Some("prose".to_string())
        );
        assert_eq!(mode_from_tags(&tags(&["mode:"])), None);
    }

    #[test]
    fn no_tags_gives_no_mode() {
        assert_eq!(mode_from_tags(&[]), None);
    }
}
