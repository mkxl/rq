use crate::any::Any;
use derive_more::From;
use std::{io::Error as IoError, path::Path};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader, Lines, Stdin},
};
use tokio_util::either::Either;

#[derive(From)]
pub struct Input {
    lines: Option<Lines<BufReader<Either<File, Stdin>>>>,
}

impl Input {
    pub async fn from_filepath(filepath: &Path) -> Result<Self, IoError> {
        filepath
            .open_tokio()
            .await?
            .left_tokio()
            .buf_reader_tokio()
            .lines()
            .some()
            .convert::<Self>()
            .ok()
    }

    pub fn from_stdin() -> Self {
        tokio::io::stdin()
            .right_tokio()
            .buf_reader_tokio()
            .lines()
            .some()
            .into()
    }

    pub fn empty() -> Self {
        None.into()
    }

    pub async fn next_line(&mut self) -> Result<String, IoError> {
        let Some(lines) = &mut self.lines else { std::future::pending().await };
        let Some(line) = lines.next_line().await? else { std::future::pending().await };

        line.ok()
    }
}
