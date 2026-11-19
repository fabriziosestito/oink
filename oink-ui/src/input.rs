//! Logical actions and the focus state machine.
//!
//! Touch zones, keys, and device buttons all produce an [`Action`]. The host
//! feeds the action to [`UiState`] and receives an [`Effect`] to execute.

use crate::content::View;

/// Where the focus sits in the reader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Choices,
    Bar,
}

/// A button on the bottom bar, in drawing order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarButton {
    Sheet,
    Inventory,
    Log,
    Menu,
}

impl BarButton {
    /// The buttons the bar shows, in drawing order.
    pub const ALL: [Self; 4] = [Self::Sheet, Self::Inventory, Self::Log, Self::Menu];
    pub const COUNT: usize = Self::ALL.len();

    pub const fn view(self) -> View {
        match self {
            Self::Sheet => View::Sheet,
            Self::Inventory => View::Inventory,
            Self::Log => View::Log,
            Self::Menu => View::Menu,
        }
    }

    /// The label drawn on the bottom bar.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Sheet => "Sheet",
            Self::Inventory => "Inventory",
            Self::Log => "Log",
            Self::Menu => "Menu",
        }
    }
}

/// A logical action from touch, keys, or buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    PageUp,
    PageDown,
    /// Pick a choice by zero-based index, from a number key or a tap.
    Pick(usize),
    Open(View),
    Quit,
}

/// A host key that maps to a reader action.
///
/// The enum is host-agnostic: a simulator, a keyboard, and a device with
/// buttons all translate their events to [`Key`] or send an [`Action`]
/// directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Backspace,
    Escape,
    PageUp,
    PageDown,
    /// A choice number from 1 to 9. Zero and values above 9 do nothing.
    Digit(u8),
    Sheet,
    Inventory,
    Log,
    Menu,
}

impl Key {
    /// The action for this key, or `None` when the key has no function yet.
    ///
    /// Digits pick a choice directly. `Escape` quits, and `Backspace` leaves
    /// an open view. The menu key opens the save and load slots.
    pub fn action(self) -> Option<Action> {
        match self {
            Self::Up => Some(Action::Up),
            Self::Down => Some(Action::Down),
            Self::Left => Some(Action::Left),
            Self::Right => Some(Action::Right),
            Self::Enter => Some(Action::Confirm),
            Self::Backspace => Some(Action::Back),
            Self::Escape => Some(Action::Quit),
            Self::PageUp => Some(Action::PageUp),
            Self::PageDown => Some(Action::PageDown),
            Self::Digit(number) if (1..=9).contains(&number) => {
                Some(Action::Pick(usize::from(number - 1)))
            }
            Self::Digit(_) => None,
            Self::Sheet => Some(Action::Open(View::Sheet)),
            Self::Inventory => Some(Action::Open(View::Inventory)),
            Self::Log => Some(Action::Open(View::Log)),
            Self::Menu => Some(Action::Open(View::Menu)),
        }
    }
}

/// What the host must do after an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    None,
    Choose(usize),
    Open(View),
    Close,
    PageUp,
    PageDown,
    Quit,
}

/// The open view, the focused control, and the highlighted choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiState {
    view: View,
    focus: Focus,
    choice: usize,
    choice_count: usize,
    bar: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

impl UiState {
    pub const fn new() -> Self {
        Self {
            view: View::Reader,
            focus: Focus::Choices,
            choice: 0,
            choice_count: 0,
            bar: 0,
        }
    }

    pub const fn view(self) -> View {
        self.view
    }

    pub const fn focus(self) -> Focus {
        self.focus
    }

    pub const fn highlighted_choice(self) -> usize {
        self.choice
    }

    pub const fn choice_count(self) -> usize {
        self.choice_count
    }

    pub const fn highlighted_button(self) -> BarButton {
        BarButton::ALL[self.bar]
    }

    /// Set the number of choices in the current scene and clamp the highlight.
    pub fn set_choices(&mut self, count: usize) {
        self.choice_count = count;
        self.choice = self.choice.min(count.saturating_sub(1));
    }

    /// Apply one action and return the work the host must do.
    ///
    /// Choice movement and confirmation work only in the reader. An open
    /// sheet, inventory, log, or menu owns its own navigation, so the story
    /// never advances behind it.
    pub fn handle(&mut self, action: Action) -> Effect {
        match action {
            Action::Quit => Effect::Quit,
            Action::Back => self.back(),
            Action::Open(view) => self.open(view),
            Action::Up if self.view == View::Reader => {
                self.move_up();
                Effect::None
            }
            Action::Down if self.view == View::Reader => {
                self.move_down();
                Effect::None
            }
            Action::Left if self.view == View::Reader => {
                if self.focus == Focus::Bar {
                    self.bar = (self.bar + BarButton::COUNT - 1) % BarButton::COUNT;
                }
                Effect::None
            }
            Action::Right if self.view == View::Reader => {
                if self.focus == Focus::Bar {
                    self.bar = (self.bar + 1) % BarButton::COUNT;
                }
                Effect::None
            }
            Action::PageUp => Effect::PageUp,
            Action::PageDown => Effect::PageDown,
            Action::Pick(index) if self.view == View::Reader => self.pick(index),
            Action::Confirm if self.view == View::Reader => match self.focus {
                Focus::Choices if self.choice_count > 0 => Effect::Choose(self.choice),
                Focus::Choices => Effect::None,
                Focus::Bar => self.open(self.highlighted_button().view()),
            },
            Action::Up
            | Action::Down
            | Action::Left
            | Action::Right
            | Action::Confirm
            | Action::Pick(_) => Effect::None,
        }
    }

    /// Map one host key and apply the action. Keys with no function return
    /// [`Effect::None`].
    pub fn handle_key(&mut self, key: Key) -> Effect {
        match key.action() {
            Some(action) => self.handle(action),
            None => Effect::None,
        }
    }

    fn pick(&mut self, index: usize) -> Effect {
        if index < self.choice_count {
            self.choice = index;
            self.focus = Focus::Choices;
            Effect::Choose(index)
        } else {
            Effect::None
        }
    }

    fn open(&mut self, view: View) -> Effect {
        if view == View::Reader {
            self.view = View::Reader;
            self.focus = Focus::Choices;
            Effect::Close
        } else {
            self.view = view;
            self.focus = Focus::Choices;
            Effect::Open(view)
        }
    }

    fn back(&mut self) -> Effect {
        if self.view != View::Reader {
            self.view = View::Reader;
            self.focus = Focus::Choices;
            Effect::Close
        } else if self.focus == Focus::Bar {
            self.focus = Focus::Choices;
            Effect::None
        } else {
            Effect::None
        }
    }

    fn move_up(&mut self) {
        match self.focus {
            Focus::Choices if self.choice > 0 => self.choice -= 1,
            Focus::Choices => {
                self.focus = Focus::Bar;
                self.bar = BarButton::COUNT - 1;
            }
            Focus::Bar if self.bar > 0 => self.bar -= 1,
            Focus::Bar => {
                self.focus = Focus::Choices;
                self.choice = self.choice_count.saturating_sub(1);
            }
        }
    }

    fn move_down(&mut self) {
        match self.focus {
            Focus::Choices if self.choice + 1 < self.choice_count => self.choice += 1,
            Focus::Choices => {
                self.focus = Focus::Bar;
                self.bar = 0;
            }
            Focus::Bar if self.bar + 1 < BarButton::COUNT => self.bar += 1,
            Focus::Bar => {
                self.focus = Focus::Choices;
                self.choice = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_choices(count: usize) -> UiState {
        let mut state = UiState::new();
        state.set_choices(count);
        state
    }

    #[test]
    fn the_reader_starts_on_the_first_choice() {
        let state = with_choices(3);
        assert_eq!(state.view(), View::Reader);
        assert_eq!(state.focus(), Focus::Choices);
        assert_eq!(state.highlighted_choice(), 0);
    }

    #[test]
    fn down_and_up_move_the_highlight_and_cross_to_the_bar() {
        let mut state = with_choices(2);
        assert_eq!(state.handle(Action::Down), Effect::None);
        assert_eq!(state.highlighted_choice(), 1);
        assert_eq!(state.handle(Action::Down), Effect::None);
        assert_eq!(state.focus(), Focus::Bar);
        assert_eq!(state.highlighted_button(), BarButton::Sheet);
        assert_eq!(state.handle(Action::Up), Effect::None);
        assert_eq!(state.focus(), Focus::Choices);
        assert_eq!(state.highlighted_choice(), 1);
    }

    #[test]
    fn confirm_chooses_the_highlighted_choice() {
        let mut state = with_choices(3);
        state.handle(Action::Down);
        assert_eq!(state.handle(Action::Confirm), Effect::Choose(1));
    }

    #[test]
    fn confirm_on_the_bar_opens_the_button_view() {
        let mut state = with_choices(1);
        state.handle(Action::Down);
        state.handle(Action::Right);
        assert_eq!(state.highlighted_button(), BarButton::Inventory);
        assert_eq!(state.handle(Action::Confirm), Effect::Open(View::Inventory));
        assert_eq!(state.view(), View::Inventory);
    }

    #[test]
    fn back_returns_to_the_reader() {
        let mut state = with_choices(1);
        state.handle(Action::Open(View::Sheet));
        assert_eq!(state.view(), View::Sheet);
        assert_eq!(state.handle(Action::Back), Effect::Close);
        assert_eq!(state.view(), View::Reader);
        assert_eq!(state.focus(), Focus::Choices);
    }

    #[test]
    fn back_from_the_reader_clears_the_bar_focus() {
        let mut state = with_choices(1);
        state.handle(Action::Down);
        assert_eq!(state.focus(), Focus::Bar);
        assert_eq!(state.handle(Action::Back), Effect::None);
        assert_eq!(state.focus(), Focus::Choices);
    }

    #[test]
    fn choices_clamp_when_the_scene_shrinks() {
        let mut state = with_choices(5);
        for _ in 0..4 {
            state.handle(Action::Down);
        }
        assert_eq!(state.highlighted_choice(), 4);
        state.set_choices(2);
        assert_eq!(state.highlighted_choice(), 1);
        state.set_choices(0);
        assert_eq!(state.highlighted_choice(), 0);
        assert_eq!(state.handle(Action::Confirm), Effect::None);
    }

    #[test]
    fn the_bar_wraps_in_both_directions() {
        let mut state = with_choices(1);
        state.handle(Action::Down);
        state.handle(Action::Left);
        assert_eq!(state.highlighted_button(), BarButton::Menu);
        state.handle(Action::Right);
        assert_eq!(state.highlighted_button(), BarButton::Sheet);
    }

    #[test]
    fn the_menu_is_on_the_bar() {
        assert!(BarButton::ALL.contains(&BarButton::Menu));
        assert_eq!(BarButton::COUNT, 4);
    }

    #[test]
    fn every_button_has_a_label() {
        assert_eq!(BarButton::Sheet.label(), "Sheet");
        assert_eq!(BarButton::Inventory.label(), "Inventory");
        assert_eq!(BarButton::Log.label(), "Log");
        assert_eq!(BarButton::Menu.label(), "Menu");
    }

    #[test]
    fn an_open_view_does_not_advance_the_story() {
        let mut state = with_choices(3);
        state.handle(Action::Open(View::Sheet));
        assert_eq!(state.handle(Action::Confirm), Effect::None);
        assert_eq!(state.handle(Action::Down), Effect::None);
        assert_eq!(state.highlighted_choice(), 0);
        assert_eq!(state.view(), View::Sheet);
    }

    #[test]
    fn paging_and_quit_pass_through() {
        let mut state = with_choices(1);
        assert_eq!(state.handle(Action::PageUp), Effect::PageUp);
        assert_eq!(state.handle(Action::PageDown), Effect::PageDown);
        assert_eq!(state.handle(Action::Quit), Effect::Quit);
    }

    #[test]
    fn number_keys_pick_a_choice_directly() {
        let mut state = with_choices(3);
        assert_eq!(state.handle_key(Key::Digit(2)), Effect::Choose(1));
        assert_eq!(state.highlighted_choice(), 1);
        assert_eq!(state.handle_key(Key::Digit(9)), Effect::None);
        assert_eq!(state.handle_key(Key::Digit(0)), Effect::None);
    }

    #[test]
    fn letter_keys_open_the_views() {
        for (key, view) in [
            (Key::Sheet, View::Sheet),
            (Key::Inventory, View::Inventory),
            (Key::Log, View::Log),
        ] {
            let mut state = with_choices(1);
            assert_eq!(state.handle_key(key), Effect::Open(view));
            assert_eq!(state.view(), view);
            assert_eq!(state.handle_key(Key::Backspace), Effect::Close);
            assert_eq!(state.view(), View::Reader);
        }
    }

    #[test]
    fn the_menu_key_opens_the_slots() {
        let mut state = with_choices(1);
        assert_eq!(state.handle_key(Key::Menu), Effect::Open(View::Menu));
        assert_eq!(state.view(), View::Menu);
        assert_eq!(state.handle_key(Key::Backspace), Effect::Close);
        assert_eq!(state.view(), View::Reader);
    }

    #[test]
    fn picking_outside_the_reader_does_nothing() {
        let mut state = with_choices(3);
        state.handle_key(Key::Sheet);
        assert_eq!(state.handle_key(Key::Digit(1)), Effect::None);
        assert_eq!(state.view(), View::Sheet);
    }

    #[test]
    fn every_action_has_a_key() {
        let keys = [
            Key::Up,
            Key::Down,
            Key::Left,
            Key::Right,
            Key::Enter,
            Key::Backspace,
            Key::Escape,
            Key::PageUp,
            Key::PageDown,
            Key::Digit(1),
            Key::Sheet,
            Key::Inventory,
            Key::Log,
            Key::Menu,
        ];
        let actions: Vec<Action> = keys.iter().filter_map(|key| key.action()).collect();
        for expected in [
            Action::Up,
            Action::Down,
            Action::Left,
            Action::Right,
            Action::Confirm,
            Action::Back,
            Action::Quit,
            Action::PageUp,
            Action::PageDown,
            Action::Pick(0),
            Action::Open(View::Sheet),
            Action::Open(View::Inventory),
            Action::Open(View::Log),
            Action::Open(View::Menu),
        ] {
            assert!(actions.contains(&expected), "{expected:?} has no key");
        }
    }

    #[test]
    fn opening_the_reader_closes_the_current_view() {
        let mut state = with_choices(1);
        state.handle(Action::Open(View::Log));
        assert_eq!(state.handle(Action::Open(View::Reader)), Effect::Close);
        assert_eq!(state.view(), View::Reader);
    }
}
