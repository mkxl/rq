use crate::{any::Any, channel::Channel};
use anyhow::Error;
use derive_more::From;
use std::{
    collections::VecDeque,
    io::{Error as IoError, IsTerminal},
    marker::Unpin,
    os::fd::AsFd,
    path::Path,
};
use tokio::{
    io::AsyncBufReadExt,
    sync::mpsc::{error::TryRecvError, UnboundedSender},
};

pub struct Input {
    channel: Channel<Result<String, IoError>>,
    lines: VecDeque<String>,
}

impl Input {
    pub fn empty() -> Self {
        let channel = Channel::new();
        let lines = VecDeque::new();

        Self { channel, lines }
    }

    pub async fn from_filepath(filepath: &Path) -> Result<Self, IoError> {
        filepath.open().await?.buf_reader().ok()
    }

    pub fn from_stdin() -> Self {
        let stdin = tokio::io::stdin();

        // NOTE: without this, `rq` (run by itself, with no stdin input) becomes laggy
        // TODO: figure out why
        if stdin.as_fd().is_terminal() {
            Self::empty()
        } else {
            stdin.buf_reader().into()
        }
    }

    async fn read_lines<B: AsyncBufReadExt + Unpin>(buf_reader: B, sender: UnboundedSender<Result<String, IoError>>) {
        let mut lines = buf_reader.lines();

        while let Some(line_res) = lines.next_line().await.transpose() {
            // NOTE: we don't want to end early for send errors (we don't hold onto the spawned read_lines() task, so
            // retrieving a returned error from the task is not possible), but we do want to terminate for io reading next
            // line errors, so we log and ignore any errors forwarding along string results
            sender.send(line_res).log_if_error();
        }
    }

    pub async fn next_lines(&mut self) -> Result<VecDeque<String>, Error> {
        loop {
            match self.channel.receiver.try_recv() {
                Ok(line_res) => self.lines.push_back(line_res?),
                Err(TryRecvError::Empty) => break,
                Err(err) => return err.err(),
            }
        }

        if self.lines.is_empty() {
            std::future::pending().await
        } else {
            self.lines.mem_take().ok()
        }
    }
}

impl<B: 'static + AsyncBufReadExt + Send + Unpin> From<B> for Input {
    fn from(buf_reader: B) -> Self {
        let input = Input::empty();

        Self::read_lines(buf_reader, input.channel.sender.clone()).spawn_task();

        input
    }
}
