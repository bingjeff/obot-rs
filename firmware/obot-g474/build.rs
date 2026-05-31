use std::{env, process::Command};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo");
    println!("cargo:rustc-link-search={manifest_dir}");
    println!("cargo:rerun-if-changed=link.x");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/index");

    if let Some(branch_ref) = git_head_ref() {
        println!("cargo:rerun-if-changed=../../.git/{branch_ref}");
    }

    println!(
        "cargo:rustc-env=OBOT_RS_FIRMWARE_VERSION={}",
        git_version().unwrap_or_else(|| "unknown".to_string())
    );
}

fn git_version() -> Option<String> {
    let output = Command::new("git")
        .args(["describe", "--always", "--dirty=+dirty"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8(output.stdout).ok()?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_string())
}

fn git_head_ref() -> Option<String> {
    let head = std::fs::read_to_string("../../.git/HEAD").ok()?;
    head.strip_prefix("ref: ")
        .map(|head_ref| head_ref.trim().to_string())
}
