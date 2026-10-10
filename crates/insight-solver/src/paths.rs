//! Where Insight keeps things (R2-16, R2-33).
//!
//! Debug builds use the `Insight-dev` channel so a development build never
//! touches the installed app's projects or caches (R2-33). `INSIGHT_HOME`
//! overrides both roots (used by tests).

use std::path::PathBuf;

use anyhow::{Context, Result};

pub fn channel() -> &'static str {
    if cfg!(debug_assertions) { "Insight-dev" } else { "Insight" }
}

/// `%APPDATA%\<channel>`: projects and their INSIGHT.md (roaming, small).
pub fn data_root() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("INSIGHT_HOME") {
        return Ok(PathBuf::from(home).join("data"));
    }
    Ok(PathBuf::from(std::env::var("APPDATA").context("APPDATA is not set")?).join(channel()))
}

/// `%LOCALAPPDATA%\<channel>`: caches (exported trees, snapshots, cargo targets).
pub fn cache_root() -> Result<PathBuf> {
    if let Ok(home) = std::env::var("INSIGHT_HOME") {
        return Ok(PathBuf::from(home).join("cache"));
    }
    Ok(PathBuf::from(std::env::var("LOCALAPPDATA").context("LOCALAPPDATA is not set")?).join(channel()))
}

pub fn project_dir(name: &str) -> Result<PathBuf> {
    Ok(data_root()?.join("projects").join(name))
}

/// Short roots keep exported trees clear of MAX_PATH (R3-18).
pub fn tree_dir(commit: &str) -> Result<PathBuf> {
    Ok(cache_root()?.join("t").join(&commit[..12.min(commit.len())]))
}

pub fn snapshot_path(project: &str, repo: &str, key: &str) -> Result<PathBuf> {
    Ok(cache_root()?.join("snapshots").join(project).join(format!("{repo}-{key}.sqlite")))
}

/// One private cargo target per (project, toolchain) (R3-16).
pub fn cargo_target_dir(project: &str, toolchain: &str) -> Result<PathBuf> {
    let safe: String = toolchain.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '_' }).collect();
    Ok(cache_root()?.join("cargo-target").join(project).join(safe))
}
