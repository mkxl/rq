use crate::{
    any::Any,
    jq_process::{JqOutput, JqProcessBuilder},
    terminal::Terminal,
    tmp_file::TmpFile,
};
use anyhow::Error;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    Frame,
};
use std::{io::Error as IoError, path::Path, time::Duration};
use tokio::{
    sync::mpsc::{error::TryRecvError, UnboundedReceiver, UnboundedSender},
    time::Interval,
};
use tui_widgets::prompts::{FocusState, State, TextState};

pub struct App {
    event_stream: EventStream,
    input_tmp_file: TmpFile,
    interval: Interval,
    jq_output: JqOutput,
    query_text_state: TextState<'static>,
    receiver: UnboundedReceiver<JqOutput>,
    sender: UnboundedSender<JqOutput>,
}

impl App {
    const INTERVAL_DURATION: Duration = Duration::from_millis(50);

    pub fn new(input_filepath: Option<&Path>) -> Result<Self, Error> {
        let event_stream = EventStream::new();
        let input_tmp_file = Self::input_tmp_file(input_filepath)?;
        let interval = Self::interval();
        let jq_output = JqOutput::empty();
        let query_text_state = Self::query_text_state();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let app = Self {
            event_stream,
            input_tmp_file,
            interval,
            jq_output,
            query_text_state,
            receiver,
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
            Err(try_recv_error) => try_recv_error.log_as_error().with(return),
        };

        if self.jq_output.instant() < new_jq_output.instant() {
            self.jq_output = new_jq_output;
        }
    }

    fn areas(area: Rect) -> (Rect, Rect, Rect) {
        let layout = Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]);
        let [top_area, query_area] = layout.areas(area);
        let layout = Layout::horizontal([Constraint::Ratio(1, 2); 2]);
        let [input_area, output_area] = layout.areas(top_area);

        (input_area, output_area, query_area)
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        self.input_tmp_file
            .content()
            .paragraph()
            .bordered_block("INPUT")
            .render_to(frame, area);
    }

    fn render_output(&self, frame: &mut Frame, area: Rect) {
        self.jq_output
            .value()
            .paragraph()
            .bordered_block("OUTPUT")
            .render_to(frame, area);
    }

    // NOTE:
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/text_prompt.rs.html#75] TextPrompt.draw() calls frame.set_cursor_position()
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/text_prompt.rs.html#86] TextPrompt.render() mutates TextState cursor field
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/prompt.rs.html#183] TextState.push() mutates TextState.position field
    // - i choose to render the cursor separately as i want to keep the terminal's actual cursor hidden
    fn render_query(&self, frame: &mut Frame, area: Rect) {
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
            .bordered_block("QUERY");

        paragraph.render_to(frame, area);
    }

    fn render(&mut self, frame: &mut Frame) {
        let (input_area, output_area, query_area) = Self::areas(frame.area());

        self.render_input(frame, input_area);
        self.render_output(frame, output_area);
        self.render_query(frame, query_area);
    }

    fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<(), Error> {
        let old_query_text_state_value_hash_code = self.query_text_state.value().hash_code();

        self.query_text_state.handle_key_event(*key_event);

        let new_query_text_state_value = self.query_text_state.value();

        if old_query_text_state_value_hash_code == new_query_text_state_value.hash_code() {
            return ().ok();
        }

        let mut jq_process = JqProcessBuilder {
            input_file: self.input_tmp_file.file()?,
            sender: self.sender.clone(),
            query: new_query_text_state_value,
        }
        .build();

        tokio::spawn(async move {
            jq_process.run().await.log_if_error();
        });

        ().ok()
    }

    fn handle_event(&mut self, event: &Event) -> Result<(), Error> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('q'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => anyhow::bail!("quit"),
            Event::Key(key_event) => self.handle_key_event(key_event)?,
            ignored_event => tracing::debug!(?ignored_event),
        }

        ().ok()
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let mut terminal = Terminal::new()?;

        loop {
            tokio::select! {
                _instant = self.interval.tick() => {
                    self.update_jq_output();
                    terminal.inner().draw(|frame| self.render(frame))?;
                }
                event_res_opt = self.event_stream.next() => {
                    let Some(event_res) = event_res_opt else { anyhow::bail!("event stream ended unexpectedly") };

                    self.handle_event(&event_res?)?;
                }
            }
        }
    }
}
