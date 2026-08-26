mod cli;
mod config;
mod downloader;
mod github;
mod builder;
mod uf2_generator;
mod project;
mod logger;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logger
    logger::init(&cli);
    
    // Route to appropriate command
    cli.execute().await?;
    
    Ok(())
}
