use crate::{any::Any, app::App};
use anyhow::Error;
use clap::{Args, Parser};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    path::PathBuf,
};
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct JqCliArgs {
    #[arg(long)]
    pub compact_output: bool,

    #[arg(long)]
    pub null_input: bool,

    #[arg(long)]
    pub raw_input: bool,

    #[arg(long)]
    pub raw_output: bool,

    #[arg(long)]
    pub slurp: bool,
}

impl Display for JqCliArgs {
    // NOTE: including a trailing space is okay bc when user goes to edit the flags they're gonna want to add a space
    // after anyways
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        if self.compact_output {
            formatter.write_str("--compact-output ")?;
        }

        if self.null_input {
            formatter.write_str("--null-input ")?;
        }

        if self.raw_input {
            formatter.write_str("--raw-input ")?;
        }

        if self.raw_output {
            formatter.write_str("--raw-output ")?;
        }

        if self.slurp {
            formatter.write_str("--slurp ")?;
        }

        ().ok()
    }
}

#[derive(Parser)]
pub struct CliArgs {
    #[arg(long = "logs")]
    logs_filepath: Option<PathBuf>,

    #[arg(long = "out")]
    output_filepath: Option<PathBuf>,

    #[command(flatten)]
    jq_cli_args: JqCliArgs,

    #[arg(default_value = "", long)]
    query: String,

    input_filepath: Option<PathBuf>,
}

impl CliArgs {
    const FMT_SPAN: FmtSpan = FmtSpan::CLOSE;

    fn init_tracing(&self) -> Result<(), Error> {
        let Some(logs_filepath) = &self.logs_filepath else {
            return ().ok();
        };
        let logs_file = logs_filepath.create()?;

        // TODO: consider using tracing-appender for writing to a file
        tracing_subscriber::fmt()
            .with_span_events(Self::FMT_SPAN)
            .with_writer(logs_file)
            .json()
            .init();

        ().ok()
    }

    pub async fn run(self) -> Result<(), Error> {
        self.init_tracing()?;

        let input_filepath = self.input_filepath.as_deref();
        let output_value = App::new(input_filepath, &self.jq_cli_args, self.query)?.run().await?;

        if let Some(output_filepath) = &self.output_filepath {
            output_filepath.create()?.left()
        } else {
            std::io::stdout().lock().right()
        }
        .write_all_and_flush(output_value)?;

        ().ok()
    }
}
