use serde::Serialize;
use tauri::Emitter;
use tauri_plugin_shell::{process::CommandEvent, ShellExt};

#[derive(Debug, Clone, Serialize)]
struct OutputEvent {
    stream: &'static str,
    text: String,
}

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

    let (mut events, _child) = command
        .spawn()
        .map_err(|e| format!("failed to start pico-build: {e}"))?;

    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut code = None;

    while let Some(event) = events.recv().await {
        match event {
            CommandEvent::Stdout(bytes) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                stdout.push_str(&text);
                let _ = app.emit(
                    "pico-build-output",
                    OutputEvent {
                        stream: "stdout",
                        text,
                    },
                );
            }
            CommandEvent::Stderr(bytes) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                stderr.push_str(&text);
                let _ = app.emit(
                    "pico-build-output",
                    OutputEvent {
                        stream: "stderr",
                        text,
                    },
                );
            }
            CommandEvent::Error(message) => {
                stderr.push_str(&message);
                stderr.push('\n');
                let _ = app.emit(
                    "pico-build-output",
                    OutputEvent {
                        stream: "stderr",
                        text: format!("{message}\n"),
                    },
                );
            }
            CommandEvent::Terminated(payload) => {
                code = payload.code;
            }
            _ => {}
        }
    }

    Ok(CliResult {
        code,
        success: code == Some(0),
        stdout,
        stderr,
    })
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![run_cli])
        .run(tauri::generate_context!())
        .expect("error while running pico-build desktop application");
}
