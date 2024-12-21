use crate::{
    any::Any,
    channel::Channel,
    cli_args::JqCliArgs,
    input::Input,
    jq_process::{JqOutput, JqProcessBuilder},
    line_editor_set::LineEditorSet,
    rect_set::RectSet,
    scroll::ScrollView,
    terminal::Terminal,
};
use anyhow::Error;
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use futures::StreamExt;
use ratatui::{layout::Rect, Frame};
use std::{io::Error as IoError, path::Path, time::Duration};
use tokio::time::Interval;

pub struct App {
    event_stream: EventStream,
    input: Input,
    input_scroll_view: ScrollView,
    interval: Interval,
    jq_output: JqOutput,
    jq_outputs: Channel<JqOutput>,
    line_editor_set: LineEditorSet,
    rect_set: RectSet,
}

impl App {
    const INPUT_BLOCK_TITLE: &'static str = "INPUT";
    const INTERVAL_DURATION: Duration = Duration::from_millis(50);
    const OUTPUT_BLOCK_TITLE: &'static str = "OUTPUT";
    const QUIT_MESSAGE: &'static str = "quitting!";

    pub async fn new(
        input_filepath: Option<&Path>,
        jq_cli_args: &JqCliArgs,
        filter: Option<String>,
    ) -> Result<Self, Error> {
        let event_stream = EventStream::new();
        let input = Self::input(input_filepath).await?;
        let input_scroll_view = ScrollView::new();
        let interval = Self::interval();
        let jq_output = JqOutput::empty();
        let jq_outputs = Channel::new();
        let line_editor_set = LineEditorSet::new(jq_cli_args, filter);
        let rect_set = RectSet::empty();
        let app = Self {
            event_stream,
            input,
            input_scroll_view,
            interval,
            jq_output,
            jq_outputs,
            line_editor_set,
            rect_set,
        };

        app.ok()
    }

    async fn input(input_filepath: Option<&Path>) -> Result<Input, IoError> {
        // NOTE:
        // - if both an input filepath and `--null-input` are supplied, let `jq` determine what the output should be
        //   by supplying both stdin and the --null-input flag
        // - otherwise, if no input filepath is supplied, but `--null-input` is, definitely do not read from stdin
        if let Some(input_filepath) = input_filepath {
            Input::from_filepath(input_filepath).await?
        } else {
            Input::from_stdin()
        }
        .ok()
    }

    fn interval() -> Interval {
        tokio::time::interval(Self::INTERVAL_DURATION)
    }

    fn render_scroll_view(frame: &mut Frame, rect: Rect, title: &str, scroll_view: &mut ScrollView) {
        scroll_view.render(frame, rect.decrement());
        title.block().render_to(frame, rect);
    }

    #[tracing::instrument(skip_all)]
    fn render_input(&mut self, frame: &mut Frame, rect: Rect) {
        Self::render_scroll_view(frame, rect, Self::INPUT_BLOCK_TITLE, &mut self.input_scroll_view);
    }

    #[tracing::instrument(skip_all)]
    fn render_output(&mut self, frame: &mut Frame, rect: Rect) {
        Self::render_scroll_view(frame, rect, Self::OUTPUT_BLOCK_TITLE, self.jq_output.scroll_view_mut());
    }

    #[tracing::instrument(skip_all)]
    fn render_cli_flags(&self, frame: &mut Frame, rect: Rect) {
        self.line_editor_set.cli_flags().text_area().render_to(frame, rect);
    }

    #[tracing::instrument(skip_all)]
    fn render_filter(&self, frame: &mut Frame, rect: Rect) {
        self.line_editor_set.filter().text_area().render_to(frame, rect);
    }

    #[tracing::instrument(skip_all)]
    fn render(&mut self, frame: &mut Frame) {
        self.rect_set = RectSet::new(frame.area());

        self.render_input(frame, self.rect_set.input);
        self.render_output(frame, self.rect_set.output);
        self.render_filter(frame, self.rect_set.filter);
        self.render_cli_flags(frame, self.rect_set.cli_flags);
    }

    fn spawn_jq_process(&self) -> Result<(), Error> {
        JqProcessBuilder {
            cli_flags: self.line_editor_set.cli_flags().content(),
            filter: self.line_editor_set.filter().content(),
            input: self.input_scroll_view.content().as_bytes(),
            jq_outputs_sender: self.jq_outputs.sender.clone(),
        }
        .build()?
        .run()
        .spawn_task()
        .unit()
        .ok()
    }

    async fn handle_key_event(&mut self, key_event: &KeyEvent) -> Result<Option<String>, Error> {
        match key_event {
            KeyEvent {
                code: KeyCode::Char('c'),
                modifiers: KeyModifiers::CONTROL,
                ..
            } => anyhow::bail!(Self::QUIT_MESSAGE),
            KeyEvent {
                code: KeyCode::Enter, ..
            } => {
                // NOTE: allow any recently spawned jq process to run and update self.jq_output before ending the
                // program with this output value
                tokio::time::sleep(Self::INTERVAL_DURATION).await;

                self.jq_output.scroll_view_mut().take_content().some().ok()
            }
            _key_event => {
                if self.line_editor_set.handle_key_event(*key_event) {
                    self.spawn_jq_process()?;
                }

                None.ok()
            }
        }
    }

    fn handle_mouse_event(&mut self, mouse_event: MouseEvent) {
        let position = (mouse_event.column, mouse_event.row).into();

        if self.rect_set.input.contains(position) {
            &mut self.input_scroll_view
        } else if self.rect_set.output.contains(position) {
            self.jq_output.scroll_view_mut()
        } else {
            return;
        }
        .handle_mouse_event(mouse_event);
    }

    // NOTE:
    // - Ok(Some(output)) => exit program successfully with the given output
    // - Ok(None) => ignore the given input and continue running the program
    // - Err(error) => exit program unsuccessfully with the given error
    #[tracing::instrument(skip(self), fields(?event))]
    async fn handle_event(&mut self, event: &Event) -> Result<Option<String>, Error> {
        match event {
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
                _instant = self.interval.tick() => terminal.inner().draw(|frame| self.render(frame))?.unit(),
                lines_res = self.input.next_lines() => {
                    self.input_scroll_view.extend(&lines_res?);
                    self.spawn_jq_process()?;
                }
                jq_output = self.jq_outputs.receiver.recv().unwrap_or_pending() => {
                    // NOTE: keep scroll offset if the output changes
                    if self.jq_output.instant() < jq_output.instant() {
                        self.jq_output = jq_output.with_scroll_view_offset(&self.jq_output);
                    }
                }
                event_res = self.event_stream.next().unwrap_or_pending() => {
                    if let Some(output_content) = self.handle_event(&event_res?).await? {
                        return output_content.ok();
                    }
                }
            }
        }
    }
}
