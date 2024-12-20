use crate::{any::Any, scroll::ScrollView};
use anyhow::Error;
use std::{
    io::Error as IoError,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
    process::{ChildStdin, ChildStdout, Command},
    sync::mpsc::UnboundedSender,
};

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

    // TODO-d9feca: figure out why ok_or_error requires turbofish
    pub fn build(self) -> Result<JqProcess, Error> {
        let instant = Instant::now();
        let args = shlex::split(self.flags).ok_or_error::<Vec<String>>("unable to split flags for the shell")?;
        let query = if self.query.is_empty() {
            Self::DEFAULT_QUERY
        } else {
            self.query
        };
        let mut command = Command::new(Self::JQ_EXECUTABLE_NAME);
        let input = self.input.to_vec();
        let jq_outputs_sender = self.jq_outputs_sender;

        command
            .args(args)
            .arg(query)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        JqProcess {
            instant,
            command,
            input,
            jq_outputs_sender,
        }
        .ok()
    }
}

pub struct JqProcess {
    instant: Instant,
    command: Command,
    input: Vec<u8>,
    jq_outputs_sender: UnboundedSender<JqOutput>,
}

impl JqProcess {
    const POST_WRITE_SLEEP_DURATION: Duration = Duration::from_millis(5);

    // NOTE: loop stdout.read_buf() rather than a single call to stdout.read_to_end() to not end early if 0 bytes are
    // read
    async fn read(mut stdout: ChildStdout, content: &mut Vec<u8>) -> Result<(), IoError> {
        loop {
            stdout.read_buf(content).await?;
        }
    }

    // NOTE:
    // - both std::mem::drop() and tokio::time::sleep() seem to be necessary to get the output to render
    // - Self::POST_WRITE_SLEEP_DURATION chosen based off trial and error
    async fn write(mut stdin: ChildStdin, input: &[u8]) -> Result<(), IoError> {
        stdin.write_all_and_flush(input).await?;
        std::mem::drop(stdin);
        tokio::time::sleep(Self::POST_WRITE_SLEEP_DURATION).await;

        ().ok()
    }

    // TODO:
    // - TODO-d9feca
    // - determine if this is useful: [https://docs.rs/tokio/latest/tokio/process/index.html#droppingcancellation]
    // - figure out how to cancel previously started processes
    //   - some join!(command, other) type thing where other can be set or told to cancel on updates/new calls to
    //     this function
    async fn run_helper(mut self) -> Result<(), Error> {
        let mut child = self.command.spawn()?;
        let stdin = child.stdin.take().ok_or_error::<ChildStdin>("unable to get stdin")?;
        let stdout = child.stdout.take().ok_or_error::<ChildStdout>("unable to get stdout")?;
        let mut content = Vec::new();
        let read = Self::read(stdout, &mut content);
        let write = Self::write(stdin, &self.input);

        Self::select(read, write).await?;

        let output = child.wait_with_output().await?;

        anyhow::ensure!(output.status.success(), output.stderr.into_string()?);

        let jq_output = JqOutput::new(self.instant, &content.into_string()?);

        self.jq_outputs_sender.send(jq_output)?;

        ().ok()
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(self) {
        self.run_helper().await.log_if_error();
    }
}
