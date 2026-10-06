//! Every Ink example in the reference pages compiles and runs against the
//! demo rulebook, which the pages describe.

use oink_core::data::GameData;
use oink_core::{Engine, Event};

const DEMO_RULEBOOK: &str = include_str!("../../examples/high-pass/rulebook.yaml");

#[test]
fn reference_ink_examples_compile_and_run() {
    let overview = include_str!("../../docs/rulebook.md");
    let minimal_yaml = overview
        .split("```yaml\n")
        .nth(2)
        .unwrap()
        .split("```")
        .next()
        .unwrap();
    for (page, markdown) in [
        ("overview", overview),
        ("checks", include_str!("../../docs/reference/checks.md")),
        ("state", include_str!("../../docs/reference/state.md")),
        ("ink-api", include_str!("../../docs/reference/ink-api.md")),
    ] {
        for block in markdown.split("```ink\n").skip(1) {
            let source = block.split("```").next().unwrap();
            let yaml = if page == "overview" {
                minimal_yaml
            } else {
                DEMO_RULEBOOK
            };
            let data = GameData::from_yaml("title: Documentation", yaml).unwrap();
            let mut engine =
                Engine::new(source, data).unwrap_or_else(|error| panic!("{page}: {error}"));
            engine.set_seed(7);
            let mut event = engine
                .start()
                .unwrap_or_else(|error| panic!("{page}: {error}"));
            let mut steps = 0;
            while let Event::Scene { choices, .. } = event {
                assert!(!choices.is_empty(), "{page}");
                steps += 1;
                assert!(steps < 10, "example did not end: {page}");
                event = engine
                    .choose(0)
                    .unwrap_or_else(|error| panic!("{page}: {error}"));
            }
        }
    }
}
