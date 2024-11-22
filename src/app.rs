use crate::{
    any::Any,
    cli_args::JqCliArgs,
    jq_process::{JqOutput, JqProcessBuilder},
    lines::Lines,
    rect_set::RectSet,
    terminal::Terminal,
    text_state_set::TextStateSet,
    tmp_file::TmpFile,
};
use anyhow::Error;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use futures::StreamExt;
use ratatui::{layout::Rect, style::Stylize, text::Line, Frame};
use std::{io::Error as IoError, path::Path, time::Duration};
use tokio::{
    sync::mpsc::{error::TryRecvError, UnboundedReceiver, UnboundedSender},
    time::Interval,
};
use tui_widgets::prompts::{State, TextState};

pub struct App {
    event_stream: EventStream,
    input_tmp_file: TmpFile,
    interval: Interval,
    jq_output: JqOutput,
    receiver: UnboundedReceiver<JqOutput>,
    rect_set: RectSet,
    sender: UnboundedSender<JqOutput>,
    text_state_set: TextStateSet,
}

impl App {
    const BAIL_MESSAGE: &'static str = "quitting!";
    const INPUT_BLOCK_TITLE: &'static str = "INPUT";
    const INTERVAL_DURATION: Duration = Duration::from_millis(50);
    const OUTPUT_BLOCK_TITLE: &'static str = "OUTPUT";
    const FLAGS_BLOCK_TITLE: &'static str = "FLAGS";
    const QUERY_BLOCK_TITLE: &'static str = "QUERY";

    pub fn new(input_filepath: Option<&Path>, jq_cli_args: &JqCliArgs, query: String) -> Result<Self, Error> {
        let event_stream = EventStream::new();
        let input_tmp_file = Self::input_tmp_file(input_filepath, jq_cli_args)?;
        let interval = Self::interval();
        let jq_output = JqOutput::empty();
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let rect_set = RectSet::empty();
        let text_state_set = TextStateSet::new(jq_cli_args, query);
        let app = Self {
            event_stream,
            input_tmp_file,
            interval,
            jq_output,
            receiver,
            rect_set,
            sender,
            text_state_set,
        };

        app.ok()
    }

    fn input_tmp_file(input_filepath: Option<&Path>, jq_cli_args: &JqCliArgs) -> Result<TmpFile, IoError> {
        let content = if jq_cli_args.null_input {
            String::new()
        } else if let Some(input_filepath) = input_filepath {
            input_filepath.open()?.buf_reader().read_into_string()?
        } else {
            std::io::stdin().lock().read_into_string()?
        };

        TmpFile::new(content)
    }

    fn interval() -> Interval {
        tokio::time::interval(Self::INTERVAL_DURATION)
    }

    fn update_jq_output(&mut self) {
        let new_jq_output = match self.receiver.try_recv() {
            Ok(new_jq_output) => new_jq_output,
            Err(TryRecvError::Empty) => return,
            Err(try_recv_error) => return try_recv_error.log_as_error(),
        };

        if self.jq_output.instant() < new_jq_output.instant() {
            let offset = self.jq_output.lines_mut().scroll_state_mut().offset();

            self.jq_output = new_jq_output;

            self.jq_output.lines_mut().scroll_state_mut().set_offset(offset);
        }
    }

    fn render_scroll_view<S: AsRef<str> + Default>(frame: &mut Frame, rect: Rect, title: &str, lines: &mut Lines<S>) {
        lines.render_scroll_view(frame, rect.decrement());
        title.block().render_to(frame, rect);
    }

    #[tracing::instrument(skip_all)]
    fn render_input(&mut self, frame: &mut Frame, rect: Rect) {
        Self::render_scroll_view(frame, rect, Self::INPUT_BLOCK_TITLE, self.input_tmp_file.lines_mut());
    }

    #[tracing::instrument(skip_all)]
    fn render_output(&mut self, frame: &mut Frame, rect: Rect) {
        Self::render_scroll_view(frame, rect, Self::OUTPUT_BLOCK_TITLE, self.jq_output.lines_mut());
    }

    // NOTE:
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/text_prompt.rs.html#75] TextPrompt.draw() calls frame.set_cursor_position()
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/text_prompt.rs.html#86] TextPrompt.render() mutates TextState cursor field
    // - [https://docs.rs/tui-prompts/0.5.0/src/tui_prompts/prompt.rs.html#183] TextState.push() mutates TextState.position field
    // - i choose to render the cursor separately as i want to keep the terminal's actual cursor hidden
    fn render_text_state(frame: &mut Frame, rect: Rect, text_state: &TextState, title: &str) {
        let query_str = text_state.value();
        let cursor_begin = text_state.position();
        let cursor_end = cursor_begin.saturating_add(1);
        let before_cursor_str_span = query_str.substring(..cursor_begin).reset();
        let cursor_str = query_str.substring(cursor_begin..cursor_end);
        let cursor_str = if cursor_str.is_empty() { " " } else { cursor_str };
        let cursor_str_span = if text_state.is_focused() {
            cursor_str.reversed()
        } else {
            cursor_str.reset()
        };
        let after_cursor_str_span = query_str.substring(cursor_end..).reset();
        let line = std::vec![before_cursor_str_span, cursor_str_span, after_cursor_str_span].convert::<Line>();

        line.paragraph().bordered_block(title.bold()).render_to(frame, rect);
    }

    #[tracing::instrument(skip_all)]
    fn render_flags(&self, frame: &mut Frame, rect: Rect) {
        Self::render_text_state(frame, rect, self.text_state_set.flags(), Self::FLAGS_BLOCK_TITLE);
    }

    #[tracing::instrument(skip_all)]
    fn render_query(&self, frame: &mut Frame, rect: Rect) {
        Self::render_text_state(frame, rect, self.text_state_set.query(), Self::QUERY_BLOCK_TITLE);
    }

    #[tracing::instrument(skip_all)]
    fn render(&mut self, frame: &mut Frame) {
        self.rect_set = RectSet::new(frame.area());

        self.render_input(frame, self.rect_set.input);
        self.render_output(frame, self.rect_set.output);
        self.render_query(frame, self.rect_set.query);
        self.render_flags(frame, self.rect_set.flags);
    }

    fn spawn_jq_process(&self) -> Result<(), Error> {
        let jq_process = JqProcessBuilder {
            input_file: self.input_tmp_file.file()?,
            flags: self.text_state_set.flags().value(),
            query: self.text_state_set.query().value(),
            sender: self.sender.clone(),
        }
        .build()?;

        tokio::spawn(jq_process.run());

        ().ok()
    }

    async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<Option<String>, Error> {
        if self.text_state_set.handle_key_event(*key_event) {
            self.spawn_jq_process()?;
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

        // NOTE: allow any recently spawned jq process to run and update self.jq_output before ending the program with
        // this output value
        tokio::time::sleep(Self::INTERVAL_DURATION).await;

        self.jq_output.lines_mut().take_content().some().ok()
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        let position = (mouse_event.column, mouse_event.row).into();

        if self.rect_set.input.contains(position) {
            self.input_tmp_file.lines_mut()
        } else if self.rect_set.output.contains(position) {
            self.jq_output.lines_mut()
        } else {
            return;
        }
        .scroll_state_mut()
        .handle_mouse_event(mouse_event);
    }

    // NOTE:
    // - Ok(Some(output)) => exit program successfully with the given output
    // - Ok(None) => ignore the given input and continue running the program
    // - Err(error) => exit program unsuccessfully with the given error
    #[tracing::instrument(skip(self), fields(?event))]
    async fn handle_event(&mut self, event: &Event) -> Result<Option<String>, Error> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            }) => anyhow::bail!(Self::BAIL_MESSAGE),
            Event::Key(KeyEvent { code: KeyCode::Tab, .. }) => self.text_state_set.toggle_focus().none().ok(),
            Event::Key(key_event) => self.handle_key_event(key_event).await,
            Event::Mouse(mouse_event) => self.handle_mouse_event(*mouse_event).none().ok(),
            ignored_event => tracing::debug!(?ignored_event).none().ok(),
        }
    }

    pub async fn run(&mut self) -> Result<String, Error> {
        let mut terminal = Terminal::new()?;

        // NOTE: spawn jq process to render initial output
        self.spawn_jq_process()?;

        loop {
            tokio::select! {
                _instant = self.interval.tick() => {
                    self.update_jq_output();
                    terminal.inner().draw(|frame| self.render(frame))?;
                }
                event_res_opt = self.event_stream.next() => {
                    let Some(event_res) = event_res_opt else { anyhow::bail!("event stream ended unexpectedly"); };

                    if let Some(output_content) = self.handle_event(&event_res?).await? {
                        return output_content.ok();
                    }
                }
            }
        }
    }
}
