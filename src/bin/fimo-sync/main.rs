mod cli;
mod sync;

use crate::cli::Cli;
use clap::Parser;
use crate::sync::start_sync;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    start_sync(args).await
}