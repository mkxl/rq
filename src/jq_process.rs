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
    pub cli_flags: &'a str,
    pub filter: &'a str,
    pub input: &'a [u8],
    pub jq_outputs_sender: UnboundedSender<Result<JqOutput, Error>>,
}

impl<'a> JqProcessBuilder<'a> {
    const JQ_EXECUTABLE_NAME: &'static str = "jq";
    const DEFAULT_FILTER: &'static str = ".";

    // TODO-d9feca: figure out why ok_or_error requires turbofish
    pub fn build(self) -> Result<JqProcess, Error> {
        let instant = Instant::now();
        let args =
            shlex::split(self.cli_flags).ok_or_error::<Vec<String>>("unable to split cli-flags for the shell")?;
        let filter = if self.filter.is_empty() {
            Self::DEFAULT_FILTER
        } else {
            self.filter
        };
        let mut command = Command::new(Self::JQ_EXECUTABLE_NAME);
        let jq_outputs_sender = self.jq_outputs_sender;

        command
            .args(args)
            .arg(filter)
            .stdin(self.input.tempfile()?)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        JqProcess {
            instant,
            command,
            jq_outputs_sender,
        }
        .ok()
    }
}

pub struct JqProcess {
    instant: Instant,
    command: Command,
    jq_outputs_sender: UnboundedSender<Result<JqOutput, Error>>,
}

impl JqProcess {
    // TODO:
    // - TODO-d9feca
    // - determine if this is useful: [https://docs.rs/tokio/latest/tokio/process/index.html#droppingcancellation]
    // - figure out how to cancel previously started processes
    //   - some join!(command, other) type thing where other can be set or told to cancel on updates/new calls to
    //     this function
    #[tracing::instrument(skip(self), fields(command = ?self.command), err)]
    async fn jq_output(&mut self) -> Result<JqOutput, Error> {
        let output = self.command.output().await?;

        anyhow::ensure!(
            output.status.success(),
            "[{status}] {stderr:?}",
            status = output.status,
            stderr = output.stderr.to_str()?
        );

        JqOutput::new(self.instant, output.stdout.to_str()?).ok()
    }

    pub async fn run(mut self) {
        let jq_output_res = self.jq_output().await;

        self.jq_outputs_sender.send(jq_output_res).log_if_error();
    }
}
