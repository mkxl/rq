use crate::any::Any;
use crossterm::event::KeyEvent;
use tui_widgets::prompts::{FocusState, State, TextState};

pub struct TextStateSet {
    flags: TextState<'static>,
    query: TextState<'static>,
}

impl TextStateSet {
    const INITIAL_FOCUSED_FLAGS: FocusState = FocusState::Unfocused;
    const INITIAL_FOCUSED_QUERY: FocusState = FocusState::Focused;
    const INITIAL_VALUE_FLAGS: &'static str = "--compact-output";
    const INITIAL_VALUE_QUERY: &'static str = "";

    pub fn new() -> Self {
        Self {
            flags: Self::text_state(Self::INITIAL_FOCUSED_FLAGS, Self::INITIAL_VALUE_FLAGS),
            query: Self::text_state(Self::INITIAL_FOCUSED_QUERY, Self::INITIAL_VALUE_QUERY),
        }
    }

    fn text_state(focus_state: FocusState, value: &str) -> TextState {
        TextState::new().with_focus(focus_state).with_value(value)
    }

    pub fn flags(&self) -> &TextState<'static> {
        &self.flags
    }

    pub fn query(&self) -> &TextState<'static> {
        &self.query
    }

    pub fn toggle_focus(&mut self) {
        self.flags.toggle_focus();
        self.query.toggle_focus();
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> bool {
        let text_state = if self.query.is_focused() {
            &mut self.query
        } else {
            &mut self.flags
        };
        let prev_hash_code = text_state.hash_code();

        text_state.handle_key_event(key_event);

        prev_hash_code != text_state.hash_code()
    }
}
