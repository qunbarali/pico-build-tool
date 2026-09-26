use anyhow::{anyhow, Context, Result};
use std::path::Path;
use tokio::process::Command;

use crate::downloader;

pub async fn generate_uf2(elf_file: &str, output: Option<&str>, family: &str, cache_dir: Option<&str>) -> Result<()> {
    let input = Path::new(elf_file);
    anyhow::ensure!(input.is_file(), "ELF file does not exist: {}", input.display());

    let deps = downloader::dependency_paths(cache_dir)?;
    let picotool = downloader::find_named_public(&deps.picotool, "picotool.exe")
        .ok_or_else(|| anyhow!("picotool.exe not found; run 'pico-build setup'"))?;

    let out = output.map(Path::new).map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension("uf2"));
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let status = Command::new(&picotool)
        .arg("uf2")
        .arg("convert")
        .arg(input)
        .arg(&out)
        .arg("--family")
        .arg(family)
        .status()
        .await
        .with_context(|| "failed to start picotool")?;

    anyhow::ensure!(status.success(), "picotool UF2 conversion failed with exit code {:?}", status.code());
    println!("UF2: {}", out.display());
    Ok(())
}
