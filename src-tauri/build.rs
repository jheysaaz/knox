use std::env;
use std::path::PathBuf;

fn main() {
    tauri_build::build();

    // When the `typescript` feature is enabled, set ts-rs export config.
    // Run: cargo test --features typescript --no-default-features
    if env::var_os("CARGO_FEATURE_TYPESCRIPT").is_some() {
        let manifest_dir =
            PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
        let export_dir = manifest_dir.join("..").join("src").join("types-gen");
        println!(
            "cargo:rustc-env=TS_RS_EXPORT_DIR={}",
            export_dir.display()
        );
        // Serialize u64 as TypeScript `number` (fits Number.MAX_SAFE_INTEGER).
        println!("cargo:rustc-env=TS_RS_LARGE_INT=number");
    }

    let target = env::var("TARGET").expect("TARGET not set");
    if target == "x86_64-apple-darwin" {
        panic!("x86_64-apple-darwin is not supported");
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let native_dir = manifest_dir
        .join("third_party")
        .join("native")
        .join(&target);

    // Only emit static link directives when prebuilt archives exist.
    // Otherwise, the -sys crates will find libraries via pkg-config / brew.
    let has_static_libs =
        native_dir.join("libtesseract.a").exists() || native_dir.join("tesseract.lib").exists();

    if has_static_libs {
        println!("cargo:rustc-link-search=native={}", native_dir.display());

        let link_list = native_dir.join("link-libs.txt");
        if let Ok(contents) = std::fs::read_to_string(&link_list) {
            for line in contents.lines() {
                let name = line.trim();
                if !name.is_empty() {
                    println!("cargo:rustc-link-lib=static={}", name);
                }
            }
        } else {
            println!("cargo:rustc-link-lib=static=tesseract");
            println!("cargo:rustc-link-lib=static=lept");
        }

        if target.contains("windows-msvc") {
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
        } else if target.contains("apple-darwin") {
            println!("cargo:rustc-link-lib=c++");
            println!("cargo:rustc-link-lib=z");
        } else if target.contains("linux-gnu") {
            println!("cargo:rustc-link-lib=stdc++");
            println!("cargo:rustc-link-lib=z");
        }
    }
}
