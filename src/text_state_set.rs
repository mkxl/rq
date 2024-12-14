use crate::{any::Any, cli_args::JqCliArgs};
use crossterm::event::KeyEvent;
use tui_widgets::prompts::{FocusState, State, TextState};

pub struct TextStateSet {
    flags: TextState<'static>,
    query: TextState<'static>,
}

impl TextStateSet {
    const INITIAL_FOCUSED_FLAGS: FocusState = FocusState::Unfocused;
    const INITIAL_FOCUSED_QUERY: FocusState = FocusState::Focused;

    pub fn new(jq_cli_args: &JqCliArgs, initial_query: Option<String>) -> Self {
        let flags = Self::text_state(Self::INITIAL_FOCUSED_FLAGS, jq_cli_args.to_string());
        let query = Self::text_state(Self::INITIAL_FOCUSED_QUERY, initial_query.unwrap_or_default());

        Self { flags, query }
    }

    fn text_state(focus_state: FocusState, value: String) -> TextState<'static> {
        let mut text_state = TextState::new().with_focus(focus_state).with_value(value);

        text_state.move_end();

        text_state
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

        // NOTE: return did the content change
        prev_hash_code != text_state.hash_code()
    }
}
