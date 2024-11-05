use crate::{
    any::Any,
    jq_process::{JqOutput, JqProcessBuilder},
    lines::Lines,
    rect_set::RectSet,
    terminal::Terminal,
    tmp_file::TmpFile,
};
use anyhow::Error;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use futures::StreamExt;
use ratatui::{layout::Rect, style::Stylize, text::Line, widgets::StatefulWidget, Frame};
use std::{io::Error as IoError, path::Path, time::Duration};
use tokio::{
    sync::mpsc::{error::TryRecvError, UnboundedReceiver, UnboundedSender},
    time::Interval,
};
use tui_widgets::{
    prompts::{FocusState, State, TextState},
    scrollview::{ScrollView, ScrollViewState},
};

pub struct App {
    event_stream: EventStream,
    input_scroll_view_state: ScrollViewState,
    input_tmp_file: TmpFile,
    interval: Interval,
    jq_output: JqOutput,
    output_scroll_view_state: ScrollViewState,
    query_text_state: TextState<'static>,
    receiver: UnboundedReceiver<JqOutput>,
    rect_set: RectSet,
    sender: UnboundedSender<JqOutput>,
}

impl App {
    const INPUT_BLOCK_TITLE: &'static str = "INPUT";
    const INTERVAL_DURATION: Duration = Duration::from_millis(50);
    const OUTPUT_BLOCK_TITLE: &'static str = "OUTPUT";
    const QUERY_BLOCK_TITLE: &'static str = "QUERY";

    pub fn new(input_filepath: Option<&Path>) -> Result<Self, Error> {
        let event_stream = EventStream::new();
        let input_scroll_view_state = ScrollViewState::new();
        let input_tmp_file = Self::input_tmp_file(input_filepath)?;
        let interval = Self::interval();
        let jq_output = JqOutput::empty();
        let output_scroll_view_state = ScrollViewState::new();
        let query_text_state = Self::query_text_state();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let rect_set = RectSet::empty();
        let app = Self {
            event_stream,
            input_scroll_view_state,
            input_tmp_file,
            interval,
            jq_output,
            output_scroll_view_state,
            query_text_state,
            receiver,
            rect_set,
            sender,
        };

        app.ok()
    }

    fn input_tmp_file(input_filepath: Option<&Path>) -> Result<TmpFile, IoError> {
        let content = if let Some(input_filepath) = input_filepath {
            input_filepath.open()?.buf_reader().read_into_string()
        } else {
            std::io::stdin().lock().read_into_string()
        }?;

        TmpFile::new(content)
    }

    fn interval() -> Interval {
        tokio::time::interval(Self::INTERVAL_DURATION)
    }

    fn query_text_state() -> TextState<'static> {
        TextState::new().with_focus(FocusState::Focused)
    }

    fn update_jq_output(&mut self) {
        let new_jq_output = match self.receiver.try_recv() {
            Ok(new_jq_output) => new_jq_output,
            Err(TryRecvError::Empty) => return,
            Err(try_recv_error) => return try_recv_error.log_as_error(),
        };

        if self.jq_output.instant() < new_jq_output.instant() {
            self.jq_output = new_jq_output;
        }
    }

    fn render_scroll_view<S: AsRef<str>>(
        frame: &mut Frame,
        title: &str,
        rect: Rect,
        lines: &Lines<S>,
        state: &mut ScrollViewState,
    ) {
        // TODO: handle
        #[allow(clippy::cast_possible_truncation)]
        let height = rect.height.max(lines.len() as u16);
        let scroll_view_rect = Rect::new(0, 0, rect.width, height);
        let mut scroll_view = ScrollView::new(scroll_view_rect.as_size());
        let paragraph = lines.content().paragraph().bordered_block(title);

        // NOTE: formerly: [paragraph.render_to(frame, rect);]
        scroll_view.render_widget(paragraph, scroll_view_rect);
        scroll_view.render(rect, frame.buffer_mut(), state);
    }

    fn render_input(&mut self, frame: &mut Frame, rect: Rect) {
        Self::render_scroll_view(
            frame,
            Self::INPUT_BLOCK_TITLE,
            rect,
            self.input_tmp_file.lines(),
            &mut self.input_scroll_view_state,
        );
    }

    fn render_output(&mut self, frame: &mut Frame, rect: Rect) {
        Self::render_scroll_view(
            frame,
            Self::OUTPUT_BLOCK_TITLE,
            rect,
            &self.jq_output.value().into_lines(),
            &mut self.output_scroll_view_state,
        );
    }

    // NOTE:
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/text_prompt.rs.html#75] TextPrompt.draw() calls frame.set_cursor_position()
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/text_prompt.rs.html#86] TextPrompt.render() mutates TextState cursor field
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/prompt.rs.html#183] TextState.push() mutates TextState.position field
    // - i choose to render the cursor separately as i want to keep the terminal's actual cursor hidden
    fn render_query(&self, frame: &mut Frame, rect: Rect) {
        let query_str = self.query_text_state.value();
        let cursor_begin = self.query_text_state.position();
        let cursor_end = cursor_begin.saturating_add(1);
        let before_cursor_str_span = query_str.substr(..cursor_begin).reset();
        let cursor_str = query_str.substr(cursor_begin..cursor_end);
        let cursor_str_span = (if cursor_str.is_empty() { " " } else { cursor_str }).reversed();
        let after_cursor_str_span = query_str.substr(cursor_end..).reset();
        let paragraph = std::vec![before_cursor_str_span, cursor_str_span, after_cursor_str_span]
            .convert::<Line>()
            .paragraph()
            .bordered_block(Self::QUERY_BLOCK_TITLE);

        paragraph.render_to(frame, rect);
    }

    fn render(&mut self, frame: &mut Frame) {
        self.rect_set = RectSet::new(frame.area());

        self.render_input(frame, self.rect_set.input);
        self.render_output(frame, self.rect_set.output);
        self.render_query(frame, self.rect_set.query);
    }

    async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<Option<String>, Error> {
        let old_query_text_state_value_hash_code = self.query_text_state.value().hash_code();

        self.query_text_state.handle_key_event(*key_event);

        let new_query_text_state_value = self.query_text_state.value();

        if old_query_text_state_value_hash_code != new_query_text_state_value.hash_code() {
            let mut jq_process = JqProcessBuilder::new(
                self.input_tmp_file.file()?,
                new_query_text_state_value,
                self.sender.clone(),
            )
            .build();

            tokio::spawn(async move {
                jq_process.run().await.log_if_error();
            });
        }

        if !std::matches!(
            key_event,
            KeyEvent {
                code: KeyCode::Enter,
                ..
            }
        ) {
            return None.ok();
        }

        // NOTE: allow any recently spawned jq process to run and update self.jq_output
        tokio::time::sleep(Self::INTERVAL_DURATION).await;

        self.jq_output.take_value().some().ok()
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        let position = (mouse_event.column, mouse_event.row).into();
        let scroll_view_state = if self.rect_set.input.contains(position) {
            &mut self.input_scroll_view_state
        } else if self.rect_set.output.contains(position) {
            &mut self.output_scroll_view_state
        } else {
            return;
        };

        match mouse_event.kind {
            MouseEventKind::ScrollDown => scroll_view_state.scroll_down(),
            MouseEventKind::ScrollUp => scroll_view_state.scroll_up(),
            ignored_mouse_event_kind => tracing::debug!(?ignored_mouse_event_kind),
        }
    }

    async fn handle_event(&mut self, event: &Event) -> Result<Option<String>, Error> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => String::new().some().ok(),
            Event::Key(key_event) => self.handle_key_event(key_event).await,
            Event::Mouse(mouse_event) => self.handle_mouse_event(*mouse_event).none().ok(),
            ignored_event => tracing::debug!(?ignored_event).none().ok(),
        }
    }

    pub async fn run(&mut self) -> Result<String, Error> {
        let mut terminal = Terminal::new()?;

        loop {
            tokio::select! {
                _instant = self.interval.tick() => {
                    self.update_jq_output();
                    terminal.inner().draw(|frame| self.render(frame))?;
                }
                event_res_opt = self.event_stream.next() => {
                    let Some(event_res) = event_res_opt else { anyhow::bail!("event stream ended unexpectedly"); };

                    if let Some(output_value) = self.handle_event(&event_res?).await? {
                        return output_value.ok();
                    }
                }
            }
        }
    }
}
