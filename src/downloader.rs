use anyhow::{Result, anyhow, Context};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write;
use log::info;
use indicatif::{ProgressBar, ProgressStyle};
use futures::stream::StreamExt;

const DEPENDENCIES: &[(&str, &str)] = &[
    ("cmake", "https://github.com/Kitware/CMake/releases/download/v3.27.8/cmake-3.27.8-windows-x86_64.zip"),
    ("ninja", "https://github.com/ninja-build/ninja/releases/download/v1.11.1/ninja-win.zip"),
    ("arm-gcc", "https://developer.arm.com/-/media/Files/downloads/gnu/13.2.Rel1/binrel/arm-gnu-toolchain-13.2.rel1-mingw-w64-i686-arm-none-eabi.zip"),
    ("pico-sdk", "https://github.com/raspberrypi/pico-sdk/archive/refs/tags/2.0.0.zip"),
];

pub async fn setup_dependencies(force: bool, cache_dir: Option<&str>) -> Result<()> {
    let cache_path = get_cache_dir(cache_dir)?;
    fs::create_dir_all(&cache_path)?;
    info!("Cache directory: {}", cache_path.display());
    
    for (name, url) in DEPENDENCIES {
        let dep_dir = cache_path.join(name);
        if dep_dir.exists() && !force {
            info!("[CACHED] {}", name);
            continue;
        }
        if force && dep_dir.exists() {
            fs::remove_dir_all(&dep_dir)?;
        }
        info!("Downloading {}...", name);
        download_and_extract(name, url, &cache_path).await?;
    }
    info!("All dependencies ready");
    Ok(())
}

pub async fn check_status() -> Result<()> {
    let cache_path = get_cache_dir(None)?;
    println!("\nDependency Status:");
    println!("Cache: {}\n", cache_path.display());
    
    for (name, _) in DEPENDENCIES {
        let status = if cache_path.join(name).exists() { "OK" } else { "MISSING" };
        println!("{:<15} {}", name, status);
    }
    Ok(())
}

async fn download_and_extract(name: &str, url: &str, cache_dir: &Path) -> Result<()> {
    let response = reqwest::Client::new().get(url).send().await?;
    let total_size = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar().template("{bar:40}").unwrap());
    
    let mut file = tempfile::NamedTempFile::new()?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk??)?;
        pb.inc(chunk?.len() as u64);
    }
    pb.finish();
    
    let target_dir = cache_dir.join(name);
    fs::create_dir_all(&target_dir)?;
    extract_archive(file.path(), &target_dir)?;
    info!("Extracted: {}", name);
    Ok(())
}

fn extract_archive(archive_path: &Path, target_dir: &Path) -> Result<()> {
    if let Some(ext) = archive_path.extension().and_then(|e| e.to_str()) {
        match ext {
            "zip" => {
                let file = fs::File::open(archive_path)?;
                zip::ZipArchive::new(file)?.extract(target_dir)?;
            }
            _ => return Err(anyhow!("Unsupported format: {}", ext)),
        }
    }
    Ok(())
}

fn get_cache_dir(custom_dir: Option<&str>) -> Result<PathBuf> {
    if let Some(dir) = custom_dir {
        Ok(PathBuf::from(dir))
    } else if let Some(home) = home::home_dir() {
        Ok(home.join(".pico-build-tool/cache"))
    } else {
        Err(anyhow!("No cache directory found"))
    }
}
