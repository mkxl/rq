use crate::any::Any;
use anyhow::Error;
use std::{fs::File, time::Instant};
use tokio::{process::Command, sync::mpsc::UnboundedSender};

pub struct JqOutput {
    instant: Instant,
    value: String,
}

impl JqOutput {
    pub fn empty() -> Self {
        let instant = Instant::now();
        let value = String::new();

        Self { instant, value }
    }

    pub fn instant(&self) -> Instant {
        self.instant
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn take_value(&mut self) -> String {
        std::mem::take(&mut self.value)
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
    pub async fn run(&mut self) -> Result<(), Error> {
        let output = self.command.output().await?;

        anyhow::ensure!(output.status.success(), output.stderr.into_string()?);

        let value = output.stdout.into_string()?;
        let jq_output = JqOutput {
            instant: self.instant,
            value,
        };

        self.sender.send(jq_output)?;

        ().ok()
    }
}
