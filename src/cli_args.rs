use crate::{any::Any, app::App};
use anyhow::Error;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct CliArgs {
    #[arg(long = "logs")]
    logs_filepath: Option<PathBuf>,

    #[arg(long = "out")]
    output_filepath: Option<PathBuf>,

    input_filepath: Option<PathBuf>,
}

impl CliArgs {
    fn init_tracing(&self) -> Result<(), Error> {
        let Some(logs_filepath) = &self.logs_filepath else {
            return ().ok();
        };
        let logs_file = logs_filepath.create()?;

        // TODO: consider using tracing-appender for writing to a file
        tracing_subscriber::fmt().with_writer(logs_file).json().init();

        ().ok()
    }

    pub async fn run(self) -> Result<(), Error> {
        self.init_tracing()?;

        let output_value = App::new(self.input_filepath.as_deref())?.run().await?;

        if let Some(output_filepath) = &self.output_filepath {
            output_value.write_all_and_flush(output_filepath.create()?)?;
        } else {
            output_value.write_all_and_flush(std::io::stdout().lock())?;
        }

        ().ok()
    }
}
