mod any;
mod app;
mod channel;
mod cli_args;
mod input;
mod jq_process;
mod rect_set;
mod scroll;
mod terminal;
mod text_state_set;

use crate::cli_args::CliArgs;
use anyhow::Error;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Error> {
    CliArgs::parse().run().await
}
