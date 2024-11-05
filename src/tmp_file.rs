use crate::{any::Any, lines::Lines};
use std::{
    fs::File,
    io::Error as IoError,
    path::{Path, PathBuf},
};
use tempfile::TempDir;

// TODO: handle [https://docs.rs/tempfile/latest/tempfile/struct.TempDir.html#resource-leaking]
pub struct TmpFile {
    lines: Lines,
    filepath: PathBuf,
    #[allow(dead_code)]
    temp_dir: TempDir,
}

impl TmpFile {
    const FILENAME: &'static str = "file";

    pub fn new(content: String) -> Result<Self, IoError> {
        let temp_dir = tempfile::tempdir()?;
        let filepath = Self::create_file(temp_dir.path(), &content)?;
        let lines = content.into_lines();
        let tmp_file = Self {
            lines,
            filepath,
            temp_dir,
        };

        tmp_file.ok()
    }

    fn create_file(dirpath: &Path, content: &str) -> Result<PathBuf, IoError> {
        let filepath = dirpath.join(Self::FILENAME);

        content.as_bytes().write_all_and_flush(filepath.create()?)?;

        filepath.ok()
    }

    pub fn lines(&self) -> &Lines {
        &self.lines
    }

    pub fn file(&self) -> Result<File, IoError> {
        self.filepath.open()
    }
}
