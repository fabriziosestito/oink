use oink_core::oink_rulebook::{Character, Rulebook};
#[test]
fn dump() {
    let rulebook = Rulebook::load(&std::fs::read_to_string("../examples/high-pass/rulebook.yaml").unwrap()).unwrap().rulebook;
    let character = Character::from_starting(&rulebook);
    let config = oink_ui::config::ReaderConfig::default();
    let panel = oink_ui::content::Panel::sheet(&rulebook, &character, &config, 40);
    for line in &panel.lines { println!("{}", line.text()); }
}
