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

pub async fn build_project(path: &str, output: &str, profile: &str, skip_deps_check: bool, cache_dir: Option<&str>) -> Result<()> {
    let project = config::project_path(path)?;
    let profile = normalize_profile(profile)?;

    let deps = if skip_deps_check {
        downloader::dependency_paths(cache_dir)?
    } else {
        downloader::setup_dependencies(false, cache_dir).await?;
        downloader::dependency_paths(cache_dir)?
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run(&mut configure, "CMake configure").await?;

    let mut build = Command::new(ninja);
    build.current_dir(&build_dir)
        .arg("-v")
        .env("PICO_SDK_PATH", &sdk)
        .env("PICO_TOOLCHAIN_PATH", &arm_bin)
        .env("PATH", &path_env)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    run(&mut build, "Ninja build").await?;

    let artifacts = require_artifacts(&build_dir, &cfg.name)?;
    info!("Build complete: {}", build_dir.display());
    info!("Flashable UF2: {}", artifacts.uf2.display());
    info!("ELF: {}", artifacts.elf.display());
    Ok(())
}

#[derive(Debug)]
struct BuildArtifacts {
    elf: PathBuf,
    uf2: PathBuf,
}

fn require_artifacts(build_dir: &Path, project_name: &str) -> Result<BuildArtifacts> {
    let elf = build_dir.join(format!("{project_name}.elf"));
    let uf2 = build_dir.join(format!("{project_name}.uf2"));
    anyhow::ensure!(elf.is_file(), "build succeeded but ELF was not produced: {}", elf.display());
    anyhow::ensure!(uf2.is_file(), "build succeeded but flashable UF2 was not produced: {}", uf2.display());
    let metadata = fs::metadata(&uf2)?;
    anyhow::ensure!(metadata.len() > 0, "flashable UF2 is empty: {}", uf2.display());
    anyhow::ensure!(metadata.len() % 512 == 0, "UF2 has invalid size ({} bytes): {}", metadata.len(), uf2.display());
    validate_uf2_blocks(&uf2)?;
    Ok(BuildArtifacts { elf, uf2 })
}

pub async fn clean_project(path: &str, output: &str) -> Result<()> {
    let project = config::project_path(path)?;
    let requested = resolve_output(&project, output);
    let build_root = project.join("build").canonicalize().unwrap_or_else(|_| project.join("build"));
    let build_dir = requested.canonicalize().unwrap_or(requested);
    anyhow::ensure!(build_dir != project && build_dir.starts_with(&build_root),
        "refusing to delete path outside the project build directory: {}", build_dir.display());

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
    let output = command.output().await
        .with_context(|| format!("failed to start {label}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    print!("{stdout}");
    eprint!("{stderr}");

    if !output.status.success() {
        let combined = format!("{stdout}{stderr}");
        report_compiler_diagnostics(&combined);
        anyhow::bail!("{label} failed with exit code {:?}", output.status.code());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Diagnostic {
    severity: &'static str,
    file: Option<String>,
    line: Option<u32>,
    column: Option<u32>,
    message: String,
}

fn report_compiler_diagnostics(output: &str) {
    let diagnostics = parse_diagnostics(output);
    if diagnostics.is_empty() {
        return;
    }

    let errors = diagnostics.iter().filter(|d| d.severity == "ERROR").count();
    let warnings = diagnostics.iter().filter(|d| d.severity == "WARNING").count();
    println!("\\nBuild diagnostics: {errors} error(s), {warnings} warning(s)");

    for diagnostic in diagnostics {
        let location = match (&diagnostic.file, diagnostic.line, diagnostic.column) {
            (Some(file), Some(line), Some(column)) => format!("{file}:{line}:{column}"),
            (Some(file), Some(line), None) => format!("{file}:{line}"),
            (Some(file), None, _) => file.clone(),
            _ => "build system".to_string(),
        };
        println!("\\n[{}] {location}\\n  {}", diagnostic.severity, diagnostic.message.trim());
    }
}

fn parse_diagnostics(output: &str) -> Vec<Diagnostic> {
    let mut result = Vec::new();

    for line in output.lines() {
        if let Some(diagnostic) = parse_gcc_diagnostic(line) {
            push_unique_diagnostic(&mut result, diagnostic);
            continue;
        }
        if let Some(diagnostic) = parse_cmake_diagnostic(line) {
            push_unique_diagnostic(&mut result, diagnostic);
        }
    }

    result
}

fn push_unique_diagnostic(result: &mut Vec<Diagnostic>, diagnostic: Diagnostic) {
    if !result.contains(&diagnostic) {
        result.push(diagnostic);
    }
}

fn parse_gcc_diagnostic(line: &str) -> Option<Diagnostic> {
    let lower = line.to_ascii_lowercase();
    let (marker, severity) = if let Some(index) = lower.find(": error:") {
        (index, "ERROR")
    } else if let Some(index) = lower.find(": warning:") {
        (index, "WARNING")
    } else if let Some(index) = lower.find(": fatal error:") {
        (index, "ERROR")
    } else {
        return None;
    };

    let prefix = line[..marker].trim();
    let message = line[marker..]
        .split_once(':')
        .and_then(|(_, rest)| rest.split_once(':').map(|(_, msg)| msg))
        .unwrap_or("")
        .trim()
        .to_string();

    let (file, line_number, column) = parse_source_location(prefix);
    Some(Diagnostic {
        severity,
        file,
        line: line_number,
        column,
        message: if message.is_empty() { line.trim().to_string() } else { message },
    })
}

fn parse_source_location(location: &str) -> (Option<String>, Option<u32>, Option<u32>) {
    let mut parts = location.rsplitn(3, ':');
    let last = parts.next().unwrap_or_default();
    let second = parts.next().unwrap_or_default();
    let first = parts.next();

    if let (Ok(column), Ok(line)) = (last.parse::<u32>(), second.parse::<u32>()) {
        if let Some(file) = first {
            return (Some(file.trim().to_string()), Some(line), Some(column));
        }
    }

    let mut parts = location.rsplitn(2, ':');
    let last = parts.next().unwrap_or_default();
    let file = parts.next();
    if let (Some(file), Ok(line)) = (file, last.parse::<u32>()) {
        return (Some(file.trim().to_string()), Some(line), None);
    }

    (Some(location.trim().to_string()), None, None)
}

fn parse_cmake_diagnostic(line: &str) -> Option<Diagnostic> {
    let lower = line.to_ascii_lowercase();
    let (needle, severity) = if lower.contains("cmake error at ") {
        ("cmake error at ", "ERROR")
    } else if lower.contains("cmake warning at ") {
        ("cmake warning at ", "WARNING")
    } else {
        return None;
    };

    let start = lower.find(needle)? + needle.len();
    let location_and_message = line[start..].trim();
    let colon = location_and_message.rfind(':')?;
    let location = location_and_message[..colon].trim();
    let message = location_and_message[colon + 1..].trim();

    let source_location = location.split_whitespace().next().unwrap_or(location);
    let (file, line_number, column) = parse_source_location(source_location);
    Some(Diagnostic {
        severity,
        file,
        line: line_number,
        column,
        message: message.trim_matches(|c| c == '(' || c == ')').to_string(),
    })
}


fn validate_uf2_blocks(path: &Path) -> Result<()> {
    const BLOCK_SIZE: usize = 512;
    const MAGIC0: u32 = 0x0A32_4655;
    const MAGIC1: u32 = 0x9E5D_5157;
    let bytes = fs::read(path).with_context(|| format!("failed to read UF2 {}", path.display()))?;
    anyhow::ensure!(bytes.len() >= BLOCK_SIZE, "UF2 is too small to be valid: {}", path.display());
    for (index, block) in bytes.chunks_exact(BLOCK_SIZE).enumerate() {
        let magic0 = u32::from_le_bytes(block[0..4].try_into().unwrap());
        let magic1 = u32::from_le_bytes(block[4..8].try_into().unwrap());
        let end_magic = u32::from_le_bytes(block[508..512].try_into().unwrap());
        anyhow::ensure!(magic0 == MAGIC0 && magic1 == MAGIC1 && end_magic == MAGIC1,
            "invalid UF2 block {} in {}", index, path.display());
    }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gcc_error_with_windows_path() {
        let diagnostics = parse_diagnostics(
            r#"C:\projects\blink\src\main.c:27:5: error: 'foo' undeclared"#
        );
        assert_eq!(diagnostics, vec![Diagnostic {
            severity: "ERROR",
            file: Some(r#"C:\projects\blink\src\main.c"#.to_string()),
            line: Some(27),
            column: Some(5),
            message: "'foo' undeclared".to_string(),
        }]);
    }

    #[test]
    fn parses_gcc_warning_without_column() {
        let diagnostics = parse_diagnostics("src/main.c:12: warning: unused variable 'x'");
        assert_eq!(diagnostics[0].severity, "WARNING");
        assert_eq!(diagnostics[0].file.as_deref(), Some("src/main.c"));
        assert_eq!(diagnostics[0].line, Some(12));
        assert_eq!(diagnostics[0].column, None);
    }

    #[test]
    fn parses_cmake_error() {
        let diagnostics = parse_diagnostics(
            "CMake Error at CMakeLists.txt:18 (add_executable): Cannot find source file"
        );
        assert_eq!(diagnostics[0].severity, "ERROR");
        assert_eq!(diagnostics[0].file.as_deref(), Some("CMakeLists.txt"));
        assert_eq!(diagnostics[0].line, Some(18));
        assert!(diagnostics[0].message.contains("Cannot find source file"));
    }

    #[test]
    fn removes_duplicate_diagnostics() {
        let diagnostics = parse_diagnostics(
            "src/main.c:7:2: error: expected ';'\nsrc/main.c:7:2: error: expected ';'"
        );
        assert_eq!(diagnostics.len(), 1);
    }
}
