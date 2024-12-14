use crate::{any::Any, channel::Channel};
use derive_more::From;
use std::{io::Error as IoError, marker::Unpin, path::Path};
use tokio::{
    io::{AsyncBufRead, AsyncBufReadExt, Lines},
    sync::mpsc::{error::SendError, UnboundedSender},
};

type LinesItem = Result<Option<String>, IoError>;

#[derive(From)]
pub struct Input {
    channel: Channel<LinesItem>,
}

impl Input {
    pub fn empty() -> Self {
        Channel::new().into()
    }

    pub async fn from_filepath(filepath: &Path) -> Result<Self, IoError> {
        filepath
            .open_tokio()
            .await?
            .buf_reader_tokio()
            .lines()
            .convert::<Self>()
            .ok()
    }

    pub fn from_stdin() -> Self {
        tokio::io::stdin().buf_reader_tokio().lines().into()
    }

    async fn read_lines<R: AsyncBufRead + Unpin>(
        sender: UnboundedSender<LinesItem>,
        mut lines: Lines<R>,
    ) -> Result<(), SendError<LinesItem>> {
        loop {
            sender.send(lines.next_line().await)?;
        }
    }

    pub async fn next_line(&mut self) -> Result<String, IoError> {
        match self.channel.receiver.recv().unwrap_or_pending().await? {
            Some(line) => line.ok(),
            None => std::future::pending().await,
        }
    }
}

impl<R: 'static + AsyncBufRead + Send + Unpin> From<Lines<R>> for Input {
    fn from(lines: Lines<R>) -> Self {
        let channel = Channel::new();

        Self::read_lines(channel.sender.clone(), lines).spawn_task();

        channel.into()
    }
}
