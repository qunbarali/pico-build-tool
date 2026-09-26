use serde::Serialize;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

#[derive(Debug, Serialize)]
struct CliResult {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    success: bool,
}

#[tauri::command]
async fn run_cli(app: tauri::AppHandle, args: Vec<String>) -> Result<CliResult, String> {
    let command = app
        .shell()
        .sidecar("pico-build")
        .map_err(|e| format!("failed to prepare pico-build sidecar: {e}"))?
        .args(args);

    let output = command
        .output()
        .await
        .map_err(|e| format!("failed to execute pico-build: {e}"))?;

    Ok(CliResult {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![run_cli])
        .run(tauri::generate_context!())
        .expect("error while running pico-build desktop application");
}
