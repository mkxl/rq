use crate::any::Any;
use std::{
    fs::File,
    io::{Error as IoError, Write},
    path::{Path, PathBuf},
};
use tempfile::TempDir;

// TODO: handle [https://docs.rs/tempfile/latest/tempfile/struct.TempDir.html#resource-leaking]
pub struct TmpFile {
    content: String,
    filepath: PathBuf,
    temp_dir: TempDir,
}

impl TmpFile {
    const FILENAME: &'static str = "file";

    pub fn new(content: String) -> Result<Self, IoError> {
        let temp_dir = tempfile::tempdir()?;
        let filepath = Self::create_file(temp_dir.path(), &content)?;
        let tmp_file = Self {
            content,
            filepath,
            temp_dir,
        };

        tmp_file.ok()
    }

    fn create_file(dirpath: &Path, content: &str) -> Result<PathBuf, IoError> {
        let filepath = dirpath.join(Self::FILENAME);

        filepath.create()?.write_all(content.as_bytes())?;

        filepath.ok()
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn file(&self) -> Result<File, IoError> {
        self.filepath.open()
    }
}
