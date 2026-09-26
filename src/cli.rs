use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;

#[derive(Parser)]
#[command(name = "pico-build", version = "1.2.0", about = "Standalone Raspberry Pi Pico build tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[arg(short, long, global = true)]
    pub quiet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Clone a GitHub repository
    Clone {
        repo_url: String,
        #[arg(short, long, env = "GITHUB_TOKEN")]
        token: Option<String>,
        #[arg(short, long)]
        branch: Option<String>,
        #[arg(short, long, default_value = "./pico-projects")]
        output: String,
    },
    /// Build an existing Pico project and produce a flashable UF2
    Build {
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: String,
        #[arg(short, long, default_value = "./build")]
        output: String,
        #[arg(short, long, default_value = "release")]
        profile: String,
        #[arg(long)]
        skip_deps_check: bool,
    },
    /// Clone and build a GitHub repository, leaving the flashable artifacts on disk
    CloneBuild {
        repo_url: String,
        #[arg(short, long, env = "GITHUB_TOKEN")]
        token: Option<String>,
        #[arg(short, long)]
        branch: Option<String>,
        #[arg(short, long, default_value = "./build")]
        build_output: String,
        #[arg(long, default_value = "release")]
        profile: String,
    },
    /// Download and verify the pinned Windows toolchain
    Setup {
        #[arg(short, long)]
        force: bool,
        #[arg(short, long)]
        cache_dir: Option<String>,
    },
    /// Check the installed toolchain
    Status,
    /// Run a complete environment and project diagnostic
    Doctor {
        #[arg(value_name = "PROJECT_PATH")]
        path: Option<String>,
    },
    /// Create a new Pico project
    Init {
        name: String,
        #[arg(long, default_value = "pico")]
        board: String,
    },
    /// Convert an ELF to a flashable UF2 using the bundled picotool
    GenerateUf2 {
        elf_file: String,
        #[arg(short, long)]
        output: Option<String>,
        #[arg(long, default_value = "rp2040")]
        family: String,
    },
    /// Remove build artifacts
    Clean {
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: String,
        #[arg(short, long, default_value = "./build")]
        output: String,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            Commands::Clone { repo_url, token, branch, output } =>
                crate::github::clone_repository(repo_url, token.as_deref(), branch.as_deref(), output).await,
            Commands::Build { path, output, profile, skip_deps_check } =>
                crate::builder::build_project(path, output, profile, *skip_deps_check).await,
            Commands::CloneBuild { repo_url, token, branch, build_output, profile } => {
                let output = env::current_dir()?.join(build_output);
                let workspace = tempfile::tempdir()?;
                let clone_path = workspace.path().join("project");
                let clone_path_str = clone_path.to_string_lossy().to_string();
                let output_str = output.to_string_lossy().to_string();
                crate::github::clone_repository(repo_url, token.as_deref(), branch.as_deref(), &clone_path_str).await?;
                crate::builder::build_project(&clone_path_str, &output_str, profile, false).await
            }
            Commands::Setup { force, cache_dir } =>
                crate::downloader::setup_dependencies(*force, cache_dir.as_deref()).await,
            Commands::Status => crate::downloader::check_status().await,
            Commands::Doctor { path } => crate::doctor::run(path.as_deref()).await,
            Commands::Init { name, board } => crate::project::init_project(name, board).await,
            Commands::GenerateUf2 { elf_file, output, family } =>
                crate::uf2_generator::generate_uf2(elf_file, output.as_deref(), family).await,
            Commands::Clean { path, output } =>
                crate::builder::clean_project(path, output).await,
        }
    }
}
