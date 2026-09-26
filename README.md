# pico-build-tool

A standalone Rust CLI for building Raspberry Pi Pico / Pico 2 C/C++ projects on Windows x64.

## Implemented

- Project initialization with board configuration in pico.toml.
- Pinned CMake 4.4.3, Ninja 1.13.2, Arm GNU Toolchain 15.2.rel1, Pico SDK 2.3.1 and picotool 2.3.1 downloads.
- SHA-256 verification for downloadable binary dependencies.
- Staged dependency installation with validation markers.
- Safe ZIP extraction with traversal and symlink rejection.
- CMake + Ninja builds with PICO_BOARD selection.
- UF2 conversion through the official Raspberry Pi picotool.
- GitHub HTTPS cloning with optional GITHUB_TOKEN authentication.
- Build cleanup and dependency status reporting.
- Unit tests for validation paths.

The bundled dependency installer is currently Windows x64 only.

## Quick start

```powershell
cargo build --release
pico-build setup
pico-build init blink --board pico
pico-build build .\blink
pico-build status
pico-build generate-uf2 .\blink\build\blink.elf
```

## GitHub builds

For public repositories:

```powershell
pico-build clone https://github.com/user/repository
pico-build build .\pico-projects\repository
```

For private repositories, prefer an environment variable over putting a token into shell history:

```powershell
$env:GITHUB_TOKEN = "YOUR_TOKEN"
pico-build clone https://github.com/user/private-repository
```

## Configuration

Example pico.toml:

```toml
name = "blink"
version = "0.1.0"
description = "Example"
pico_board = "pico"
```

The board value is passed to CMake as PICO_BOARD after strict character validation. The board must be supported by the installed Pico SDK.

## Dependency provenance

The bundle pins CMake 4.4.3, Ninja 1.13.2, Arm GNU Toolchain 15.2.rel1, Raspberry Pi Pico SDK 2.3.1 and Raspberry Pi picotool 2.3.1.

The SDK and picotool releases come from Raspberry Pi's official repositories. CMake and Ninja come from their official project releases. The Arm compiler comes from Arm's distribution endpoint.

## Commands

```text
pico-build setup [--force]
pico-build status
pico-build init <NAME> [--board <BOARD>]
pico-build clone <REPO_URL> [--branch <BRANCH>]
pico-build clone-build <REPO_URL> [--profile <debug|release>]
pico-build build [PROJECT_PATH] [--profile <debug|release>]
pico-build generate-uf2 <ELF_FILE> [--family <rp2040|...>]
pico-build clean [PROJECT_PATH]
```

## Security

- Never commit GitHub tokens.
- Binary dependency archives are SHA-256 verified where upstream hashes are pinned.
- ZIP path traversal and symlink entries are rejected.
- Downloads are extracted into a staging directory before installation.
- Clone URLs are restricted to HTTPS github.com.

## Development

```powershell
cargo fmt --check
cargo test
cargo check
```

GitHub Actions are not added as automatic triggers. If CI is added later, use manual workflow_dispatch.
