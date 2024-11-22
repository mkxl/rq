use crate::{any::Any, lines::Lines};
use anyhow::Error;
use std::{fs::File, time::Instant};
use tokio::{process::Command, sync::mpsc::UnboundedSender};

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

    pub fn lines_mut(&mut self) -> &mut Lines<String> {
        &mut self.lines
    }
}

pub struct JqProcessBuilder<'a> {
    pub input_file: File,
    pub flags: &'a str,
    pub query: &'a str,
    pub sender: UnboundedSender<JqOutput>,
}

impl<'a> JqProcessBuilder<'a> {
    const JQ_EXECUTABLE_NAME: &'static str = "jq";

    pub fn build(self) -> Result<JqProcess, Error> {
        let instant = Instant::now();
        let command = Command::new(Self::JQ_EXECUTABLE_NAME);
        let Some(args) = shlex::split(self.flags) else {
            anyhow::bail!("unable to split flags for the shell")
        };
        let mut jq_process = JqProcess {
            instant,
            command,
            sender: self.sender,
        };

        jq_process.command.args(args).arg(self.query).stdin(self.input_file);

        jq_process.ok()
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

    #[tracing::instrument(skip_all, fields(command = ?self.command))]
    pub async fn run(mut self) {
        self.run_helper().await.log_if_error();
    }
}
