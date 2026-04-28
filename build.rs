fn main() {
    // Re-run if HEAD or refs change.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
    println!("cargo:rerun-if-env-changed=GIT_SHA");

    // Prefer an explicit GIT_SHA env var (set by Homebrew formula or other
    // hermetic build environments that have no .git directory).
    if let Ok(sha) = std::env::var("GIT_SHA")
        && !sha.is_empty()
    {
        println!("cargo:rustc-env=GIT_SHA={sha}");
        return;
    }

    // Try the exact tag pointing at HEAD (gives a clean "v0.3.0" on release builds).
    let tag = std::process::Command::new("git")
        .args(["tag", "--points-at", "HEAD", "--sort=-version:refname"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.lines().next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty());

    if let Some(t) = tag {
        println!("cargo:rustc-env=GIT_SHA={t}");
        return;
    }

    // Fall back to short commit SHA for dev builds.
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_SHA={sha}");
}
