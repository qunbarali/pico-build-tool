use anyhow::{anyhow, Context, Result};
use log::info;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use walkdir::WalkDir;

use crate::config;
use crate::downloader::{self, DependencyPaths};

pub async fn build_project(path: &str, output: &str, profile: &str, skip_deps_check: bool) -> Result<()> {
    let project = config::project_path(path)?;
    let profile = normalize_profile(profile)?;

    let deps = if skip_deps_check {
        downloader::dependency_paths()?
    } else {
        downloader::setup_dependencies(false, None).await?;
        downloader::dependency_paths()?
    };

    let sdk = find_sdk_root(&deps.pico_sdk)?;
    ensure_import_file(&project, &sdk)?;
    let cfg = config::load(&project)?;
    let build_dir = resolve_output(&project, output);
    fs::create_dir_all(&build_dir)?;

    let cmake = downloader::find_named_public(&deps.cmake, "cmake.exe")
        .ok_or_else(|| anyhow!("cmake.exe not found; run 'pico-build setup'"))?;
    let ninja = downloader::find_named_public(&deps.ninja, "ninja.exe")
        .ok_or_else(|| anyhow!("ninja.exe not found; run 'pico-build setup'"))?;
    let arm_bin = find_dir_containing(&deps.arm_gcc, "arm-none-eabi-gcc.exe")?;
    let path_env = build_path_env(&deps);

    let mut configure = Command::new(cmake);
    configure
        .arg("-S").arg(&project)
        .arg("-B").arg(&build_dir)
        .arg("-G").arg("Ninja")
        .arg(format!("-DCMAKE_BUILD_TYPE={profile}"))
        .arg(format!("-DPICO_BOARD={}", cfg.pico_board))
        .env("PICO_SDK_PATH", &sdk)
        .env("PICO_TOOLCHAIN_PATH", &arm_bin)
        .env("PATH", &path_env)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    run(&mut configure, "CMake configure").await?;

    let mut build = Command::new(ninja);
    build.current_dir(&build_dir)
        .arg("-v")
        .env("PICO_SDK_PATH", &sdk)
        .env("PICO_TOOLCHAIN_PATH", &arm_bin)
        .env("PATH", &path_env)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    run(&mut build, "Ninja build").await?;

    info!("Build complete: {}", build_dir.display());
    report_artifacts(&build_dir, &cfg.name);
    Ok(())
}

pub async fn clean_project(path: &str, output: &str) -> Result<()> {
    let project = config::project_path(path)?;
    let requested = resolve_output(&project, output);
    let build_dir = requested.canonicalize().unwrap_or(requested);
    anyhow::ensure!(build_dir != project && build_dir.starts_with(&project),
        "refusing to delete path outside project build tree: {}", build_dir.display());

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)
            .with_context(|| format!("failed to remove {}", build_dir.display()))?;
        info!("Removed {}", build_dir.display());
    } else {
        info!("Nothing to clean");
    }
    Ok(())
}

fn normalize_profile(profile: &str) -> Result<&'static str> {
    match profile.to_ascii_lowercase().as_str() {
        "debug" => Ok("Debug"),
        "release" => Ok("Release"),
        _ => Err(anyhow!("unsupported build profile '{}'; use debug or release", profile)),
    }
}

fn resolve_output(project: &Path, output: &str) -> PathBuf {
    let p = PathBuf::from(output);
    if p.is_absolute() { p } else { project.join(p) }
}

fn find_sdk_root(root: &Path) -> Result<PathBuf> {
    if root.join("pico_sdk_init.cmake").is_file() {
        return Ok(root.to_path_buf());
    }
    WalkDir::new(root).max_depth(3).follow_links(false).into_iter()
        .filter_map(Result::ok)
        .find(|e| e.file_type().is_file() && e.file_name() == "pico_sdk_init.cmake")
        .and_then(|e| e.path().parent().map(Path::to_path_buf))
        .ok_or_else(|| anyhow!("pico_sdk_init.cmake not found under {}", root.display()))
}

fn ensure_import_file(project: &Path, sdk: &Path) -> Result<()> {
    let import = project.join("pico_sdk_import.cmake");
    if import.exists() { return Ok(()); }
    let source = sdk.join("external").join("pico_sdk_import.cmake");
    anyhow::ensure!(source.exists(), "Pico SDK import file not found: {}", source.display());
    fs::copy(&source, &import)
        .with_context(|| format!("failed to copy {}", source.display()))?;
    Ok(())
}

fn build_path_env(deps: &DependencyPaths) -> String {
    let dirs = [
        find_dir_containing(&deps.cmake, "cmake.exe").ok(),
        find_dir_containing(&deps.ninja, "ninja.exe").ok(),
        find_dir_containing(&deps.arm_gcc, "arm-none-eabi-gcc.exe").ok(),
        find_dir_containing(&deps.picotool, "picotool.exe").ok(),
    ];
    let existing = env::var_os("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut parts: Vec<String> = dirs.into_iter().flatten()
        .map(|p| p.to_string_lossy().into_owned()).collect();
    parts.push(existing.to_string_lossy().into_owned());
    parts.join(sep)
}

fn find_dir_containing(root: &Path, name: &str) -> Result<PathBuf> {
    let file = downloader::find_named_public(root, name)
        .ok_or_else(|| anyhow!("{} not found under {}", name, root.display()))?;
    file.parent().map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("no parent directory for {}", file.display()))
}

async fn run(command: &mut Command, label: &str) -> Result<()> {
    let status = command.status().await
        .with_context(|| format!("failed to start {}", label))?;
    anyhow::ensure!(status.success(), "{} failed with exit code {:?}", label, status.code());
    Ok(())
}

fn report_artifacts(build_dir: &Path, project_name: &str) {
    for file in [
        build_dir.join(format!("{project_name}.elf")),
        build_dir.join(format!("{project_name}.uf2")),
        build_dir.join(format!("{project_name}.bin")),
        build_dir.join(format!("{project_name}.hex")),
    ] {
        if file.exists() { info!("Artifact: {}", file.display()); }
    }
}
