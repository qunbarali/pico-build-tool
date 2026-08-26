use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "pico-build")]
#[command(about = "Raspberry Pi Pico standalone build tool with bundled dependencies", long_about = None)]
#[command(version = "1.0.0")]
#[command(author = "Pico Build Tool Contributors")]
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
    /// Clone and build from GitHub repository
    Clone {
        /// GitHub repository URL (e.g., https://github.com/user/repo)
        #[arg(value_name = "REPO_URL")]
        repo_url: String,
        
        /// GitHub personal access token for private repos (optional)
        #[arg(short, long)]
        token: Option<String>,
        
        /// Branch to checkout (default: main/master)
        #[arg(short, long)]
        branch: Option<String>,
        
        /// Output directory for cloned repo
        #[arg(short, long, default_value = "./pico-projects")]
        output: String,
    },
    
    /// Build from existing local project
    Build {
        /// Path to project directory
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: String,
        
        /// Output directory for build artifacts
        #[arg(short, long, default_value = "./build")]
        output: String,
        
        /// Build type (debug or release)
        #[arg(short, long, default_value = "release")]
        profile: String,
        
        /// Skip dependency download/bundle check
        #[arg(long)]
        skip_deps_check: bool,
    },
    
    /// Clone and build in one command
    CloneBuild {
        /// GitHub repository URL
        #[arg(value_name = "REPO_URL")]
        repo_url: String,
        
        /// GitHub personal access token for private repos (optional)
        #[arg(short, long)]
        token: Option<String>,
        
        /// Branch to checkout
        #[arg(short, long)]
        branch: Option<String>,
        
        /// Output directory for build artifacts
        #[arg(short, long, default_value = "./build")]
        build_output: String,
    },
    
    /// Setup/Download all bundled dependencies
    Setup {
        /// Force re-download all dependencies
        #[arg(short, long)]
        force: bool,
        
        /// Custom cache directory
        #[arg(short, long)]
        cache_dir: Option<String>,
    },
    
    /// Check dependency status
    Status {},
    
    /// Initialize a new Pico project
    Init {
        /// Project name
        #[arg(value_name = "PROJECT_NAME")]
        name: String,
        
        /// Use C template (default is CMake + Pico SDK)
        #[arg(long)]
        template: Option<String>,
    },
    
    /// Generate UF2 file from ELF binary
    GenerateUf2 {
        /// Path to ELF file
        #[arg(value_name = "ELF_FILE")]
        elf_file: String,
        
        /// Output UF2 file path
        #[arg(short, long)]
        output: Option<String>,
    },
    
    /// Clean build artifacts
    Clean {
        /// Path to project directory
        #[arg(value_name = "PROJECT_PATH", default_value = ".")]
        path: String,
        
        /// Remove build directory entirely
        #[arg(short, long)]
        all: bool,
    },
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        match &self.command {
            Commands::Clone { repo_url, token, branch, output } => {
                crate::github::clone_repository(repo_url, token.as_deref(), branch.as_deref(), output).await?;
            }
            Commands::Build { path, output, profile, skip_deps_check } => {
                crate::builder::build_project(path, output, profile, *skip_deps_check).await?;
            }
            Commands::CloneBuild { repo_url, token, branch, build_output } => {
                let temp_dir = format!("{}/temp-clone", build_output);
                crate::github::clone_repository(repo_url, token.as_deref(), branch.as_deref(), &temp_dir).await?;
                crate::builder::build_project(&temp_dir, build_output, "release", false).await?;
            }
            Commands::Setup { force, cache_dir } => {
                crate::downloader::setup_dependencies(*force, cache_dir.as_deref()).await?;
            }
            Commands::Status {} => {
                crate::downloader::check_status().await?;
            }
            Commands::Init { name, template } => {
                crate::project::init_project(name, template.as_deref()).await?;
            }
            Commands::GenerateUf2 { elf_file, output } => {
                crate::uf2_generator::generate_uf2(elf_file, output.as_deref()).await?;
            }
            Commands::Clean { path, all } => {
                crate::builder::clean_project(path, *all).await?;
            }
        }
        Ok(())
    }
}
