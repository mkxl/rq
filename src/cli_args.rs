use crate::{any::Any, app::App};
use anyhow::Error;
use clap::{Args, Parser};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    path::{Path, PathBuf},
};
use tracing_subscriber::{
    filter::LevelFilter, fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

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
    log_filepath: Option<PathBuf>,

    #[arg(long = "log-level", default_value_t = LevelFilter::INFO)]
    log_level_filter: LevelFilter,

    #[arg(long = "out")]
    output_filepath: Option<PathBuf>,

    #[command(flatten)]
    jq_cli_args: JqCliArgs,

    #[arg(long)]
    query: Option<String>,

    input_filepath: Option<PathBuf>,
}

impl CliArgs {
    const FMT_SPAN: FmtSpan = FmtSpan::CLOSE;
    const DEFAULT_LOG_FILEPATH_STR: &'static str = "/dev/null";

    fn default_log_filepath() -> &'static Path {
        Path::new(Self::DEFAULT_LOG_FILEPATH_STR)
    }

    async fn init_tracing(&self) -> Result<(), Error> {
        // TODO:
        // - consider using tracing-appender for writing to a file
        // - let log_filepath = self.log_filepath.as_deref().unwrap_or_else(Self::default_log_filepath);
        let log_filepath = if let Some(log_filepath) = &self.log_filepath {
            log_filepath.as_path()
        } else {
            Self::default_log_filepath()
        };
        let log_file = log_filepath.create().await?.into_std().await;
        let log_layer = tracing_subscriber::fmt::layer()
            .with_span_events(Self::FMT_SPAN)
            .with_writer(log_file)
            .json()
            .with_filter(self.log_level_filter);

        tracing_subscriber::registry()
            .with(console_subscriber::spawn())
            .with(log_layer)
            .init()
            .ok()
    }

    pub async fn run(self) -> Result<(), Error> {
        self.init_tracing().await?;

        let input_filepath = self.input_filepath.as_deref();
        let output_value = App::new(input_filepath, &self.jq_cli_args, self.query)
            .await?
            .run()
            .await?;

        if let Some(output_filepath) = &self.output_filepath {
            output_filepath.create().await?.into_std().await.left()
        } else {
            std::io::stdout().right()
        }
        .write_all_and_flush(output_value)?
        .ok()
    }
}
