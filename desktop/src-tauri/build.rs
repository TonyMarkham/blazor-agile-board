use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Get the target triple for this build
    let target_triple = env::var("TARGET").expect("TARGET not set");

    // Determine source binary path based on profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let workspace_root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let source_binary = workspace_root
        .join("target")
        .join(&profile)
        .join(format!("pm-server{}", env::consts::EXE_SUFFIX));

    // Destination path for the sidecar
    let binaries_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("binaries");

    let dest_binary = binaries_dir.join(format!("pm-server-{}", target_triple));

    // Create binaries directory if it doesn't exist
    fs::create_dir_all(&binaries_dir).ok();

    // If source binary exists, copy it. Otherwise, build it first.
    if source_binary.exists() {
        println!("cargo:warning=Copying {} to {:?}", profile, dest_binary);
        fs::copy(&source_binary, &dest_binary).expect("Failed to copy pm-server binary");
    } else {
        println!("cargo:warning=pm-server binary not found, building it...");

        // Build pm-server in the appropriate profile
        let status = Command::new("cargo")
            .args(["build", "-p", "pm-server"])
            .args(if profile == "release" {
                vec!["--release"]
            } else {
                vec![]
            })
            .current_dir(&workspace_root)
            .status()
            .expect("Failed to build pm-server");

        if !status.success() {
            panic!("Failed to build pm-server");
        }

        // Now copy it
        fs::copy(&source_binary, &dest_binary)
            .expect("Failed to copy pm-server binary after building");
    }

    tauri_build::build()
}
