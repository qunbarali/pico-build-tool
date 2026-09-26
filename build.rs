use std::{env,fs,path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=PICO_BUNDLE_DIR");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR missing"));
    let target = out.join("embedded_deps.rs");
    let dir = env::var_os("PICO_BUNDLE_DIR").map(PathBuf::from);
    let specs = [
        ("cmake", "cmake.zip"),
        ("ninja", "ninja.zip"),
        ("arm-gcc", "arm-gcc.zip"),
        ("pico-sdk", "pico-sdk.zip"),
        ("picotool", "picotool.zip"),
    ];
    let mut source = String::from("pub struct EmbeddedDependency { pub name: &'static str, pub archive: &'static [u8] }\npub fn dependencies() -> &'static [EmbeddedDependency] { &[\n");
    for (name, filename) in specs {
        let Some(dir) = dir.as_ref() else { continue };
        let path = dir.join(filename);
        if path.is_file() {
            let escaped = path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
            source.push_str(&format!("EmbeddedDependency {{ name: \"{name}\", archive: include_bytes!(\"{escaped}\") }},\n"));
            println!("cargo:rerun-if-changed={escaped}");
        }
    }
    source.push_str("] }\n");
    fs::write(target, source).expect("write embedded dependency source");
}
