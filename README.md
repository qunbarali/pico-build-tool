# pico-build-tool

A standalone Windows x64 desktop application and Rust build engine for building Raspberry Pi Pico / Pico 2 C/C++ projects.

## Major capabilities

- Reproducible, pinned Windows build dependencies.
- SHA-256 verification for pinned binary archives where an upstream digest is available.
- Staged dependency installation and validation.
- Safe ZIP extraction with traversal and symlink rejection.
- CMake + Ninja builds with PICO_BOARD selection.
- Official Raspberry Pi picotool-based UF2 conversion.
- GitHub HTTPS cloning with GITHUB_TOKEN authentication.
- Project scaffolding with pico.toml.
- Safe build cleanup.
- doctor diagnostics for troubleshooting.
- Structured compiler/CMake diagnostics with file, line, column, severity, and message.
- Tauri 2 Windows desktop application with a native installer EXE.
- Bundled pico-build engine as a Tauri sidecar, so the GUI and build engine ship together.
- One-time build preparation downloads the required Pico toolchain archives into a local embedded bundle; subsequent builds reuse verified archives.
- Manual-only GitHub Actions validation.

## Desktop application

The production Windows application uses Tauri 2. Tauri bundles the GUI and the self-contained `pico-build` engine as a sidecar executable. The resulting Windows build includes a standalone application EXE and an NSIS setup EXE. Tauri's Windows distribution produces setup executables or MSI packages; this project uses the setup EXE target. citeturn4search1

The sidecar pattern is intentional: the GUI does not require a separate Pico SDK, CMake, Ninja, ARM GCC or picotool installation. Tauri supports bundling external binaries as sidecars and executing them from the application. citeturn3search0

## One-time dependency preparation

The desktop build runs `scripts/prepare-desktop-build.ps1` automatically before the Tauri build. It creates the local `embedded` bundle and downloads the pinned CMake, Ninja, ARM GNU Toolchain, Pico SDK and picotool archives only when they are missing or invalid. Verified archives are reused on subsequent builds instead of being downloaded again. The archives are then embedded into the `pico-build` sidecar through `PICO_BUNDLE_DIR`.

For a Windows development build:

```powershell
cargo install tauri-cli --version "^2.11.0" --locked
cargo tauri build
```

Tauri 2's official setup supports installing the CLI with Cargo and building with `cargo tauri build`. citeturn4search0

## Quick start

```powershell
cargo build --release
pico-build setup
pico-build init blink --board pico
pico-build build .\blink
pico-build doctor .\blink
```

## Commands

```text
pico-build setup [--force] [--cache-dir <DIR>]
pico-build status
pico-build doctor [PROJECT_PATH]
pico-build init <NAME> [--board <BOARD>]
pico-build clone <REPO_URL> [--branch <BRANCH>] [--output <DIR>]
pico-build clone-build <REPO_URL> [--branch <BRANCH>] [--build-output <DIR>]
pico-build build [PROJECT_PATH] [--profile <debug|release>] [--output <DIR>]
pico-build generate-uf2 <ELF_FILE> [--output <FILE>] [--family <FAMILY>]
pico-build clean [PROJECT_PATH] [--output <DIR>]
```

## GitHub authentication

For public repositories no token is required.

For private repositories, prefer GITHUB_TOKEN so the secret is not placed directly in shell command history:

```powershell
$env:GITHUB_TOKEN = "YOUR_TOKEN"
pico-build clone https://github.com/user/private-repository
```

The clone operation accepts only HTTPS URLs whose host is exactly github.com.

## Project configuration

Example:

```toml
name = "blink"
version = "0.1.0"
description = "Example"
pico_board = "pico"
```

The board value is passed to CMake as PICO_BOARD after strict input validation. It must be supported by the installed Pico SDK.

## Dependency bundle

The current Windows x64 bundle pins:

- CMake 4.4.3
- Ninja 1.13.2
- Arm GNU Toolchain 15.2.rel1
- Raspberry Pi Pico SDK 2.3.1
- Raspberry Pi picotool 2.3.1

The dependency archives are downloaded only during pico-build setup; normal builds reuse the validated cache.

## Diagnostics

Run:

```powershell
pico-build doctor
pico-build doctor .\blink
pico-build status
```

Doctor reports platform, architecture, dependency presence, and basic project configuration.

When a build fails, pico-build captures both CMake and Ninja/compiler output and extracts recognized errors and warnings. GCC-style diagnostics such as `src/main.c:27:5: error: ...` are reported with file, line, column, severity, and message; CMake diagnostics such as `CMake Error at CMakeLists.txt:18 ...` are also recognized. Duplicate diagnostics are suppressed while the original build output remains visible.

## Security

- Do not commit GitHub tokens.
- Prefer GITHUB_TOKEN over command-line secrets.
- Dependency archives are checksum-verified where pinned digests are available.
- ZIP path traversal and symlink entries are rejected.
- Downloads are extracted into staging directories before installation.
- GitHub clone URLs are restricted to HTTPS github.com.

## Development

```powershell
cargo fmt --check
cargo check --all-targets
cargo test --all-targets
```

The repository's GitHub Actions workflow is manual (workflow_dispatch) and does not run automatically on push or pull requests.
