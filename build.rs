// build.rs
use std::process::Command;

fn main() {
    // macOS specific configuration for iconv and TLS linking
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=iconv");
        // Try to find iconv in common locations
        if std::path::Path::new("/usr/lib/libiconv.dylib").exists() {
            println!("cargo:rustc-link-search=native=/usr/lib");
        } else if std::path::Path::new("/opt/homebrew/lib/libiconv.dylib").exists() {
            println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
        } else if std::path::Path::new("/usr/local/lib/libiconv.dylib").exists() {
            println!("cargo:rustc-link-search=native=/usr/local/lib");
        }

        // Link against Security framework for TLS support
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
    }

    let git_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git_commit = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}