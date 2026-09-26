use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use crate::embedded;
use walkdir::WalkDir;

const CMAKE_SHA256: &str = "4d52ebab7193a698651639ed80d8d04fd903358843572cf44c7fd234cb7c26ab";
const NINJA_SHA256: &str = "07fc8261b42b20e71d1720b39068c2e14ffcee6396b76fb7a795fb460b78dc65";
const ARM_SHA256: &str = "b40db54536d2fdf0ff21f4316b56c1fc4d3b782b792c5b298bcbeaf5eccedb96";
const PICOTOOL_SHA256: &str = "68730be0813f8f35be2cca147cf7f1572662d5dcf0f5ba468e02a6dd9e85db2b";
// Pico SDK is currently verified by its version and required SDK marker; its archive hash should be pinned once the release artifact hash is confirmed.

#[derive(Clone, Copy)]
struct Dependency {
    name: &'static str,
    version: &'static str,
    url: &'static str,
    sha256: Option<&'static str>,
    executable: Option<&'static str>,
}

const DEPENDENCIES: &[Dependency] = &[
    Dependency { name: "cmake", version: "4.4.3", url: "https://github.com/Kitware/CMake/releases/download/v4.4.3/cmake-4.4.3-windows-x86_64.zip", sha256: Some(CMAKE_SHA256), executable: Some("cmake.exe") },
    Dependency { name: "ninja", version: "1.13.2", url: "https://github.com/ninja-build/ninja/releases/download/v1.13.2/ninja-win.zip", sha256: Some(NINJA_SHA256), executable: Some("ninja.exe") },
    Dependency { name: "arm-gcc", version: "15.2.rel1", url: "https://developer.arm.com/-/media/Files/downloads/gnu/15.2.rel1/binrel/arm-gnu-toolchain-15.2.rel1-mingw-w64-i686-arm-none-eabi.zip", sha256: Some(ARM_SHA256), executable: Some("arm-none-eabi-gcc.exe") },
    Dependency { name: "pico-sdk", version: "2.3.1", url: "https://github.com/raspberrypi/pico-sdk/archive/refs/tags/2.3.1.zip", sha256: None, executable: None },
    Dependency { name: "picotool", version: "2.3.1", url: "https://github.com/raspberrypi/pico-sdk-tools/releases/download/v2.3.1-0/picotool-2.3.1-x64-win.zip", sha256: Some(PICOTOOL_SHA256), executable: Some("picotool.exe") },
];

pub fn get_cache_dir(custom_dir: Option<&str>) -> Result<PathBuf> {
    if let Some(dir) = custom_dir { return Ok(PathBuf::from(dir)); }
    home::home_dir().map(|p| p.join(".pico-build-tool").join("cache"))
        .ok_or_else(|| anyhow!("could not determine home directory"))
}

pub async fn setup_dependencies(force: bool, cache_dir: Option<&str>) -> Result<()> {
    if !cfg!(windows) { return Err(anyhow!("the bundled dependency installer currently supports Windows x64 only")); }
    let cache = get_cache_dir(cache_dir)?;
    fs::create_dir_all(&cache)?;
    install_embedded_dependencies(force, &cache)?;
    for dep in DEPENDENCIES {
        let target = cache.join(dep.name);
        if !force && dependency_ready(dep, &target) {
            info!("[READY] {} {}", dep.name, dep.version);
            continue;
        }
        if target.exists() { fs::remove_dir_all(&target)?; }
        download_and_extract(dep, &cache).await?;
    }
    Ok(())
}

pub async fn check_status(cache_dir: Option<&str>) -> Result<()> {
    let cache = get_cache_dir(cache_dir)?;
    println!("Pico Build Tool dependency status");
    println!("Cache: {}", cache.display());
    for dep in DEPENDENCIES {
        let target = cache.join(dep.name);
        let state = if dependency_ready(dep, &target) { "READY" } else { "MISSING/INVALID" };
        println!("{:<12} {:<10} {}", dep.name, dep.version, state);
    }
    Ok(())
}

pub fn dependency_paths(cache_dir: Option<&str>) -> Result<DependencyPaths> {
    let cache = get_cache_dir(cache_dir)?;
    let find = |name: &str| -> Result<PathBuf> {
        let dep = DEPENDENCIES.iter().find(|d| d.name == name).unwrap();
        let root = cache.join(name);
        anyhow::ensure!(dependency_ready(dep, &root), "dependency '{}' is not installed; run 'pico-build setup'", name);
        Ok(root)
    };
    Ok(DependencyPaths {
        cmake: find("cmake")?, ninja: find("ninja")?, arm_gcc: find("arm-gcc")?,
        pico_sdk: find("pico-sdk")?, picotool: find("picotool")?,
    })
}

#[derive(Debug, Clone)]
pub struct DependencyPaths {
    pub cmake: PathBuf,
    pub ninja: PathBuf,
    pub arm_gcc: PathBuf,
    pub pico_sdk: PathBuf,
    pub picotool: PathBuf,
}

fn dependency_ready(dep: &Dependency, root: &Path) -> bool {
    if !root.is_dir() { return false; }
    let marker = root.join(".pico-build-tool.json");
    let Ok(text) = fs::read_to_string(marker) else { return false; };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else { return false; };
    if value.get("version").and_then(|v| v.as_str()) != Some(dep.version) { return false; }
    match dep.executable {
        Some(exe) => find_named(root, exe).is_some(),
        None => root_contains_sdk(root),
    }
}

fn install_embedded_dependencies(force: bool, cache: &Path) -> Result<()> {
    for embedded_dep in embedded::dependencies() {
        let Some(dep) = DEPENDENCIES.iter().find(|d| d.name == embedded_dep.name) else { continue };
        let target = cache.join(dep.name);
        if !force && dependency_ready(dep, &target) { continue; }
        install_archive(dep, embedded_dep.archive, cache)?;
        info!("Installed bundled {} {}", dep.name, dep.version);
    }
    Ok(())
}

fn install_archive(dep: &Dependency, bytes: &[u8], cache: &Path) -> Result<()> {
    let actual = hex_string(&Sha256::digest(bytes));
    if let Some(expected) = dep.sha256 {
        anyhow::ensure!(actual.eq_ignore_ascii_case(expected),
            "bundled checksum mismatch for {}: expected {}, got {}", dep.name, expected, actual);
    }
    let stage = cache.join(format!(".{}.staging", dep.name));
    if stage.exists() { fs::remove_dir_all(&stage)?; }
    fs::create_dir_all(&stage)?;
    let archive = NamedTempFile::new()?;
    fs::write(archive.path(), bytes)?;
    extract_zip(archive.path(), &stage).with_context(|| format!("failed to extract bundled {}", dep.name))?;
    let target = cache.join(dep.name);
    if target.exists() { fs::remove_dir_all(&target)?; }
    fs::rename(&stage, &target)?;
    let marker = serde_json::json!({
        "name": dep.name, "version": dep.version, "url": dep.url, "sha256": actual, "bundled": true
    });
    fs::write(target.join(".pico-build-tool.json"), serde_json::to_vec_pretty(&marker)?)?;
    anyhow::ensure!(dependency_ready(dep, &target), "bundled dependency '{}' failed validation", dep.name);
    Ok(())
}

async fn download_and_extract(dep: &Dependency, cache: &Path) -> Result<()> {
    info!("Downloading {} {}...", dep.name, dep.version);
    let client = Client::builder()
        .user_agent("pico-build-tool/1.1")
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(600))
        .build()?;
    let response = client.get(dep.url).send().await?.error_for_status()
        .with_context(|| format!("download failed: {}", dep.url))?;

    let total = response.content_length().unwrap_or(0);
    let pb = if total > 0 { ProgressBar::new(total) } else { ProgressBar::new_spinner() };
    pb.set_style(ProgressStyle::with_template("{spinner:.green} {bar:40} {bytes}/{total_bytes}").unwrap());

    let mut archive = NamedTempFile::new()?;
    let mut hasher = Sha256::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let bytes = chunk?;
        archive.write_all(&bytes)?;
        hasher.update(&bytes);
        pb.inc(bytes.len() as u64);
    }
    pb.finish_and_clear();

    let actual = hex_string(&hasher.finalize());
    if let Some(expected) = dep.sha256 {
        anyhow::ensure!(actual.eq_ignore_ascii_case(expected),
            "checksum mismatch for {}: expected {}, got {}", dep.name, expected, actual);
    }

    let stage = cache.join(format!(".{}.staging", dep.name));
    if stage.exists() { fs::remove_dir_all(&stage)?; }
    fs::create_dir_all(&stage)?;
    extract_zip(archive.path(), &stage).with_context(|| format!("failed to extract {}", dep.name))?;

    let target = cache.join(dep.name);
    if target.exists() { fs::remove_dir_all(&target)?; }
    fs::rename(&stage, &target)?;

    let marker = serde_json::json!({
        "name": dep.name, "version": dep.version, "url": dep.url, "sha256": actual
    });
    fs::write(target.join(".pico-build-tool.json"), serde_json::to_vec_pretty(&marker)?)?;
    anyhow::ensure!(dependency_ready(dep, &target), "installed dependency '{}' failed validation", dep.name);
    Ok(())
}

fn extract_zip(archive_path: &Path, target_dir: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let relative = entry.enclosed_name().ok_or_else(|| anyhow!("unsafe archive path: {}", entry.name()))?.to_path_buf();
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 { return Err(anyhow!("symlink entries are not allowed: {}", entry.name())); }
        }
        let output = target_dir.join(&relative);
        if entry.is_dir() { fs::create_dir_all(&output)?; continue; }
        if let Some(parent) = output.parent() { fs::create_dir_all(parent)?; }
        let mut out = fs::File::create(&output)?;
        io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

fn find_named(root: &Path, name: &str) -> Option<PathBuf> {
    WalkDir::new(root).follow_links(false).into_iter().filter_map(Result::ok)
        .find(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.into_path())
}

pub fn find_named_public(root: &Path, name: &str) -> Option<PathBuf> { find_named(root, name) }

fn root_contains_sdk(root: &Path) -> bool {
    WalkDir::new(root).max_depth(5).follow_links(false).into_iter().filter_map(Result::ok)
        .any(|e| e.file_type().is_file() && e.file_name() == "pico_sdk_init.cmake")
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn finds_executable_recursively() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("nested/bin")).unwrap();
        fs::write(dir.path().join("nested/bin/test.exe"), b"x").unwrap();
        assert_eq!(find_named(dir.path(), "test.exe").unwrap(), dir.path().join("nested/bin/test.exe"));
    }
}
