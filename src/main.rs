mod any;
mod app;
mod cli_args;
mod jq_process;
mod lines;
mod rect_set;
mod terminal;
mod tmp_file;

use crate::cli_args::CliArgs;
use anyhow::Error;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Error> {
    CliArgs::parse().run().await
}
