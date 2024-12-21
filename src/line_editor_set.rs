use crate::{any::Any, cli_args::JqCliArgs};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::style::{Modifier, Style};
use tui_textarea::{CursorMove, TextArea};

pub struct LineEditor {
    text_area: TextArea<'static>,
}

impl LineEditor {
    const STYLE_FOCUSED: Style = Style::new().add_modifier(Modifier::REVERSED);
    const STYLE_UNFOCUSED: Style = Style::new();
    const MAX_HISTORIES: usize = 2048;

    pub fn new(title: &'static str, focused: bool, value: String) -> Self {
        let mut text_area = value.some().convert::<TextArea>();
        let cursor_style = if focused {
            Self::STYLE_FOCUSED
        } else {
            Self::STYLE_UNFOCUSED
        };

        text_area.set_block(title.block());
        text_area.set_cursor_style(cursor_style);
        text_area.set_cursor_line_style(Self::STYLE_UNFOCUSED);
        text_area.set_max_histories(Self::MAX_HISTORIES);
        text_area.move_cursor(CursorMove::End);

        Self { text_area }
    }

    pub fn text_area(&self) -> &TextArea<'static> {
        &self.text_area
    }

    pub fn is_focused(&self) -> bool {
        self.text_area.cursor_style() == Self::STYLE_FOCUSED
    }

    pub fn toggle_focus(&mut self) {
        let cursor_style = if self.is_focused() {
            Self::STYLE_UNFOCUSED
        } else {
            Self::STYLE_FOCUSED
        };

        self.text_area.set_cursor_style(cursor_style);
    }

    pub fn content(&self) -> &str {
        &self.text_area.lines()[0]
    }
}

pub struct LineEditorSet {
    cli_flags: LineEditor,
    filter: LineEditor,
}

impl LineEditorSet {
    const BLOCK_TITLE_FILTER: &'static str = "FILTER";
    const BLOCK_TITLE_CLI_FLAGS: &'static str = "CLI-FLAGS";
    const FOCUSED_FILTER: bool = true;
    const FOCUSED_CLI_FLAGS: bool = false;

    pub fn new(jq_cli_args: &JqCliArgs, initial_filter: Option<String>) -> Self {
        let cli_flags = LineEditor::new(
            Self::BLOCK_TITLE_CLI_FLAGS,
            Self::FOCUSED_CLI_FLAGS,
            jq_cli_args.to_string(),
        );
        let filter = LineEditor::new(
            Self::BLOCK_TITLE_FILTER,
            Self::FOCUSED_FILTER,
            initial_filter.unwrap_or_default(),
        );

        Self { cli_flags, filter }
    }

    pub fn cli_flags(&self) -> &LineEditor {
        &self.cli_flags
    }

    pub fn filter(&self) -> &LineEditor {
        &self.filter
    }

    fn toggle_focus(&mut self) {
        self.cli_flags.toggle_focus();
        self.filter.toggle_focus();
    }

    fn active_mut(&mut self) -> &mut LineEditor {
        if self.filter.is_focused() {
            &mut self.filter
        } else {
            &mut self.cli_flags
        }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) -> bool {
        // NOTE: returns if the content changed:
        // - [https://docs.rs/tui-textarea/latest/tui_textarea/struct.TextArea.html#method.undo]
        // - [https://docs.rs/tui-textarea/latest/tui_textarea/struct.TextArea.html#method.redo]
        // - [https://docs.rs/tui-textarea/latest/tui_textarea/struct.TextArea.html#method.input]
        match key_event {
            KeyEvent { code: KeyCode::Tab, .. } => self.toggle_focus().with(false),
            KeyEvent { code: KeyCode::Up, .. } => self.active_mut().text_area.undo(),
            KeyEvent {
                code: KeyCode::Down, ..
            } => self.active_mut().text_area.redo(),
            _key_event => self.active_mut().text_area.input(key_event),
        }
    }
}
