use anyhow::{anyhow, Result};
use log::info;
use std::fs;
use std::path::Path;

use crate::config::{self, ProjectConfig};

pub async fn init_project(name: &str, board: &str) -> Result<()> {
    config::validate_board(board)?;
    let root = Path::new(name);
    anyhow::ensure!(!root.exists(), "project directory already exists: {}", root.display());
    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("include"))?;

    let config = ProjectConfig {
        name: root.file_name().and_then(|s| s.to_str()).unwrap_or(name).to_string(),
        pico_board: board.to_string(),
        ..Default::default()
    };

    fs::write(root.join("CMakeLists.txt"), generate_cmake(&config.name))?;
    fs::write(root.join("src/main.c"), generate_main_c())?;
    fs::write(root.join("pico.toml"), toml::to_string_pretty(&config)?)?;
    fs::write(root.join(".gitignore"), GITIGNORE)?;
    fs::write(root.join("README.md"), generate_readme(&config))?;

    info!("Initialized {}", root.display());
    Ok(())
}

fn generate_cmake(name: &str) -> String {
    format!(r#"cmake_minimum_required(VERSION 3.13)

include(${{CMAKE_CURRENT_LIST_DIR}}/pico_sdk_import.cmake)

project({name} C CXX ASM)

pico_sdk_init()

add_executable(${{PROJECT_NAME}}
    src/main.c
)

target_link_libraries(${{PROJECT_NAME}} pico_stdlib)
pico_add_extra_outputs(${{PROJECT_NAME}})
"#)
}

fn generate_main_c() -> &'static str {
    r#"#include "pico/stdlib.h"
#include <stdio.h>

int main(void) {
    stdio_init_all();
    const uint led = PICO_DEFAULT_LED_PIN;
    gpio_init(led);
    gpio_set_dir(led, GPIO_OUT);

    while (true) {
        gpio_put(led, 1);
        sleep_ms(250);
        gpio_put(led, 0);
        sleep_ms(250);
    }
}
"#
}

const GITIGNORE: &str = r#"# Pico build output
build/
*.elf
*.uf2
*.bin
*.hex
*.map
CMakeFiles/
CMakeCache.txt
cmake_install.cmake
build.ninja
.ninja_deps
.ninja_log

# Tool-generated SDK import file
pico_sdk_import.cmake
"#;

fn generate_readme(config: &ProjectConfig) -> String {
    format!(r#"# {name}

Raspberry Pi Pico project managed by pico-build-tool.

## Build

    pico-build build .

The tool installs its pinned Windows build dependencies into ~/.pico-build-tool/cache.

## Board

{board}

Change pico_board in pico.toml to another board supported by the installed Pico SDK.
"#, name = config.name, board = config.pico_board)
}
