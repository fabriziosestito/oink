//! Logical actions and the focus state machine.
//!
//! Touch zones, keys, and device buttons all produce an [`Action`]. The host
//! feeds the action to [`UiState`] and receives an [`Effect`] to execute.

/// A screen the reader can show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Reader,
    Sheet,
    Inventory,
    Log,
    Menu,
}

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
    /// The buttons the bar shows. The menu stays hidden until save and load
    /// land.
    pub const ALL: [Self; 3] = [Self::Sheet, Self::Inventory, Self::Log];
    pub const COUNT: usize = Self::ALL.len();

    pub const fn view(self) -> View {
        match self {
            Self::Sheet => View::Sheet,
            Self::Inventory => View::Inventory,
            Self::Log => View::Log,
            Self::Menu => View::Menu,
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
    Open(View),
    Quit,
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
            Action::Confirm if self.view == View::Reader => match self.focus {
                Focus::Choices if self.choice_count > 0 => Effect::Choose(self.choice),
                Focus::Choices => Effect::None,
                Focus::Bar => self.open(self.highlighted_button().view()),
            },
            Action::Up | Action::Down | Action::Left | Action::Right | Action::Confirm => {
                Effect::None
            }
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
        assert_eq!(state.highlighted_button(), BarButton::Log);
        state.handle(Action::Right);
        assert_eq!(state.highlighted_button(), BarButton::Sheet);
    }

    #[test]
    fn the_menu_is_not_on_the_bar_before_saves_land() {
        assert!(!BarButton::ALL.contains(&BarButton::Menu));
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
    fn opening_the_reader_closes_the_current_view() {
        let mut state = with_choices(1);
        state.handle(Action::Open(View::Log));
        assert_eq!(state.handle(Action::Open(View::Reader)), Effect::Close);
        assert_eq!(state.view(), View::Reader);
    }
}
