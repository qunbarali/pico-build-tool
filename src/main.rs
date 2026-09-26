mod builder;
mod cli;
mod config;
mod downloader;
mod github;
mod logger;
mod project;
mod uf2_generator;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    logger::init(&cli);
    cli.execute().await
}
