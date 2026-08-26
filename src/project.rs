use anyhow::{Result, Context, anyhow};
use std::path::{Path, PathBuf};
use std::fs;
use log::info;
use serde::{Deserialize, Serialize};
use toml;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub pico_board: String,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "pico-project".to_string(),
            version: "0.1.0".to_string(),
            description: None,
            pico_board: "pico".to_string(),
        }
    }
}

/// Initialize a new Pico project with template files
pub async fn init_project(name: &str, template: Option<&str>) -> Result<()> {
    let project_dir = Path::new(name);
    
    if project_dir.exists() {
        return Err(anyhow!("Project directory '{}' already exists", name));
    }
    
    info!("Initializing new Pico project: {}", name);
    
    // Create project structure
    fs::create_dir_all(project_dir)?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("include"))?;
    
    // Create CMakeLists.txt
    let cmake_content = generate_cmake_template(name);
    fs::write(project_dir.join("CMakeLists.txt"), cmake_content)?;
    
    // Create main.c
    let main_c = generate_main_c_template();
    fs::write(project_dir.join("src/main.c"), main_c)?;
    
    // Create project config
    let config = ProjectConfig {
        name: name.to_string(),
        ..Default::default()
    };
    let config_str = toml::to_string_pretty(&config)?;
    fs::write(project_dir.join("pico.toml"), config_str)?;
    
    // Create .gitignore
    fs::write(project_dir.join(".gitignore"), generate_gitignore())?;
    
    // Create README.md
    fs::write(project_dir.join("README.md"), generate_readme(name))?;
    
    info!("✓ Project initialized at: {}", project_dir.display());
    info!("Run 'pico-build build {}' to build the project", name);
    
    Ok(())
}

fn generate_cmake_template(project_name: &str) -> String {
    format!(r#"cmake_minimum_required(VERSION 3.12)

# Initialize the SDK
include(${{CMAKE_CURRENT_LIST_DIR}}/pico_sdk_import.cmake)

project({} C CXX ASM)

# Initialize the Pico SDK
pico_sdk_init()

# Add executable. Default name is the project name, version 0.1
add_executable(${{PROJECT_NAME}}
    src/main.c
)

# Pull in common dependencies
target_link_libraries(${{PROJECT_NAME}} pico_stdlib)

# Create map/bin/hex/uf2 file in addition to ELF.
pico_add_extra_outputs(${{PROJECT_NAME}})
"#, project_name)
}

fn generate_main_c_template() -> String {
    r#"#include <stdio.h>
#include "pico/stdlib.h"

int main() {
    // Initialize stdio
    stdio_init_all();
    
    printf("Hello, Raspberry Pi Pico!\n");
    
    // Set LED GPIO (GP25 on standard Pico)
    const uint LED_PIN = PICO_DEFAULT_LED_PIN;
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);
    
    while (1) {
        gpio_put(LED_PIN, 1);
        sleep_ms(250);
        gpio_put(LED_PIN, 0);
        sleep_ms(250);
    }
    
    return 0;
}
"#.to_string()
}

fn generate_gitignore() -> &'static str {
    r#"# Build directories
build/
*.elf
*.uf2
*.hex
*.bin

# CMake
CMakeFiles/
CMakeCache.txt
cmake_install.cmake
Makefile

# IDE
.vscode/
.idea/
*.code-workspace

# OS
.DS_Store
Thumbs.db

# Dependencies
pico-sdk/
"#
}

fn generate_readme(project_name: &str) -> String {
    format!(r#"# {}

A Raspberry Pi Pico project built with the pico-build-tool.

## Building

```bash
pico-build build .
```

## Flashing

1. Connect your Pico to your computer via USB while holding the BOOTSEL button
2. Copy the generated UF2 file from `build/` to the RPI-RP2 drive

## Requirements

- pico-build-tool (handles all dependencies)

## Project Structure

```
{}
├── src/
│   └── main.c          # Main program
├── include/            # Header files
├── CMakeLists.txt      # Build configuration
├── pico.toml          # Project configuration
└── README.md          # This file
```
"#, project_name, project_name)
}
