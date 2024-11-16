use crate::{any::Any, lines::Lines};
use anyhow::Error;
use std::{fs::File, time::Instant};
use tokio::{process::Command, sync::mpsc::UnboundedSender};
use tracing::Level;

pub struct JqOutput {
    instant: Instant,
    lines: Lines<String>,
}

impl JqOutput {
    pub fn new(instant: Instant, content: String) -> Self {
        let lines = Lines::new(content);

        Self { instant, lines }
    }

    pub fn empty() -> Self {
        Self::new(Instant::now(), String::new())
    }

    pub fn instant(&self) -> Instant {
        self.instant
    }

    pub fn lines(&mut self) -> &mut Lines<String> {
        &mut self.lines
    }
}

pub struct JqProcessBuilder<'a> {
    input_file: File,
    query: &'a str,
    sender: UnboundedSender<JqOutput>,
}

impl<'a> JqProcessBuilder<'a> {
    const JQ_EXECUTABLE_NAME: &'static str = "jq";
    const ARGS: [&'static str; 1] = ["--compact-output"];

    pub fn new(input_file: File, query: &'a str, sender: UnboundedSender<JqOutput>) -> Self {
        Self {
            input_file,
            query,
            sender,
        }
    }

    pub fn build(self) -> JqProcess {
        let instant = Instant::now();
        let mut command = Command::new(Self::JQ_EXECUTABLE_NAME);

        command.args(Self::ARGS).arg(self.query).stdin(self.input_file);

        JqProcess {
            instant,
            command,
            sender: self.sender,
        }
    }
}

pub struct JqProcess {
    instant: Instant,
    command: Command,
    sender: UnboundedSender<JqOutput>,
}

impl JqProcess {
    // TODO:
    // - determine if this is useful: [https://docs.rs/tokio/latest/tokio/process/index.html#droppingcancellation]
    // - figure out how to cancel previously started processes
    //   - some join!(command, other) type thing where other can be set or told to cancel on updates/new calls to
    //     this function
    async fn run_helper(&mut self) -> Result<(), Error> {
        let output = self.command.output().await?;

        anyhow::ensure!(output.status.success(), output.stderr.into_string()?);

        let content = output.stdout.into_string()?;
        let jq_output = JqOutput::new(self.instant, content);

        self.sender.send(jq_output)?;

        ().ok()
    }

    #[tracing::instrument(level = Level::WARN, skip(self), fields(command = ?self.command))]
    pub async fn run(mut self) {
        self.run_helper().await.log_if_error();
    }
}
