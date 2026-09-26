use anyhow::Result;

use crate::{config, downloader};

pub async fn run(project: Option<&str>, cache_dir: Option<&str>) -> Result<()> {
    println!("Pico Build Tool doctor");
    println!("Platform: {}", std::env::consts::OS);
    println!("Architecture: {}", std::env::consts::ARCH);
    println!("Cache: {}", downloader::get_cache_dir(cache_dir)?.display());

    let cache = downloader::get_cache_dir(cache_dir)?;
    for name in ["cmake", "ninja", "arm-gcc", "pico-sdk", "picotool"] {
        let ready = cache.join(name).is_dir();
        println!("  {:<12} {}", name, if ready { "installed" } else { "missing" });
    }

    if !cfg!(windows) {
        println!("  bundled toolchain: unsupported on this platform (Windows x64 required)");
    }

    if let Some(path) = project {
        let root = config::project_path(path)?;
        let cfg = config::load(&root)?;
        println!("Project: {}", root.display());
        println!("  name: {}", cfg.name);
        println!("  board: {}", cfg.pico_board);
        println!("  CMakeLists.txt: {}", yes(root.join("CMakeLists.txt").exists()));
        println!("  pico.toml: {}", yes(root.join("pico.toml").exists()));
        println!("  source directory: {}", yes(root.join("src").is_dir()));
    } else {
        println!("Project: not supplied");
        println!("Use: pico-build doctor <PROJECT_PATH>");
    }
    Ok(())
}

fn yes(value: bool) -> &'static str {
    if value { "present" } else { "missing" }
}
