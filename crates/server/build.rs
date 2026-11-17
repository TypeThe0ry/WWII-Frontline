// Embeds the git short sha (read-only `git rev-parse`) for the startup build-info log line.
use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=FRONTLINE_GIT_SHA={sha}");
    println!("cargo:rerun-if-changed=build.rs");
}
