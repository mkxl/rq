use crate::{any::Any, scroll::ScrollView};
use anyhow::Error;
use std::{process::Stdio, time::Instant};
use tokio::{process::Command, sync::mpsc::UnboundedSender};

pub struct JqOutput {
    instant: Instant,
    scroll_view: ScrollView,
}

impl JqOutput {
    pub fn new(instant: Instant, content: &str) -> Self {
        let scroll_view = content.lines().collect();

        Self { instant, scroll_view }
    }

    pub fn empty() -> Self {
        Self::new(Instant::now(), "")
    }

    pub fn instant(&self) -> Instant {
        self.instant
    }

    pub fn scroll_view_mut(&mut self) -> &mut ScrollView {
        &mut self.scroll_view
    }

    pub fn with_scroll_view_offset(mut self, other: &Self) -> Self {
        self.scroll_view.set_offset(other.scroll_view.offset());

        self
    }
}

pub struct JqProcessBuilder<'a> {
    pub flags: &'a str,
    pub query: &'a str,
    pub input: &'a [u8],
    pub jq_outputs_sender: UnboundedSender<JqOutput>,
}

impl<'a> JqProcessBuilder<'a> {
    const JQ_EXECUTABLE_NAME: &'static str = "jq";
    const DEFAULT_QUERY: &'static str = ".";

    pub fn build(self) -> Result<JqProcess, Error> {
        let instant = Instant::now();
        let command = Command::new(Self::JQ_EXECUTABLE_NAME);
        let Some(args) = shlex::split(self.flags) else { anyhow::bail!("unable to split flags for the shell") };
        let query = if self.query.is_empty() {
            Self::DEFAULT_QUERY
        } else {
            self.query
        };
        let (pipe_reader, mut pipe_writer) = std::pipe::pipe()?;
        let mut jq_process = JqProcess {
            instant,
            command,
            jq_outputs_sender: self.jq_outputs_sender,
        };

        pipe_writer.write_all_and_flush(self.input)?;

        jq_process
            .command
            .args(args)
            .arg(query)
            .stdin(pipe_reader)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        jq_process.ok()
    }
}

pub struct JqProcess {
    instant: Instant,
    command: Command,
    jq_outputs_sender: UnboundedSender<JqOutput>,
}

impl JqProcess {
    // TODO:
    // - determine if this is useful: [https://docs.rs/tokio/latest/tokio/process/index.html#droppingcancellation]
    // - figure out how to cancel previously started processes
    //   - some join!(command, other) type thing where other can be set or told to cancel on updates/new calls to
    //     this function
    async fn run_helper(&mut self) -> Result<(), Error> {
        tracing::warn!(command_begin = ?self.command);

        tracing::warn!("waiting for output");

        let output = self.command.output().await?;

        tracing::warn!(output_num_bytes_read = output.stdout.len());

        anyhow::ensure!(output.status.success(), output.stderr.into_string()?);

        tracing::warn!("making jq_output");

        let jq_output = JqOutput::new(self.instant, output.stdout.as_str()?);

        self.jq_outputs_sender.send(jq_output)?;

        tracing::warn!("sent jq_output");

        ().ok()
    }

    #[tracing::instrument(skip_all, fields(command = ?self.command))]
    pub async fn run(mut self) {
        self.run_helper().await.log_if_error();
    }
}
