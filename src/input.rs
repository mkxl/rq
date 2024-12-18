use crate::{any::Any, channel::Channel};
use anyhow::Error;
use derive_more::From;
use std::{collections::VecDeque, io::Error as IoError, marker::Unpin, path::Path};
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
        filepath.open().await?.buf_reader().convert::<Self>().ok()
    }

    pub fn from_stdin() -> Self {
        tokio::io::stdin().buf_reader().into()
    }

    async fn read_lines<B: AsyncBufReadExt + Unpin>(sender: UnboundedSender<Result<String, IoError>>, buf_reader: B) {
        let mut lines = buf_reader.lines();

        // NOTE: we don't want to terminate for send errors (we don't hold onto the spawned read_lines() task, so
        // returning an error here doesn't make sense), but we do want to terminate for io reading next line errors,
        // so we log and ignore any errors forwarding along string results
        while let Some(line_res) = lines.next_line().await.transpose() {
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

        Self::read_lines(input.channel.sender.clone(), buf_reader).spawn_task();

        input
    }
}
