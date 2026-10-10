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

/// Project and repo names become folder and file names, and over MCP they come
/// from the agent: anything that could leave Insight's folders is refused.
pub fn check_name(name: &str) -> Result<&str> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
        && !name.ends_with([' ', '.']);
    anyhow::ensure!(ok, "`{name}` is not a valid name: use letters, digits, space, `-`, `_` or `.` (at most 64, not starting with `.`)");
    Ok(name)
}

pub fn project_dir(name: &str) -> Result<PathBuf> {
    Ok(data_root()?.join("projects").join(check_name(name)?))
}

pub fn snapshots_dir(project: &str) -> Result<PathBuf> {
    Ok(cache_root()?.join("snapshots").join(check_name(project)?))
}

/// The read-only, path-shaped copy of a snapshot's files (R4-B), so the agent's own Grep and Read see snapshot bytes.
pub fn frozen_source_dir(snapshot: &std::path::Path) -> Result<PathBuf> {
    let stem = snapshot.file_stem().context("snapshot has no name")?.to_string_lossy().to_string();
    Ok(cache_root()?.join("src").join(stem))
}

/// Short roots keep exported trees clear of MAX_PATH (R3-18).
pub fn tree_dir(commit: &str) -> Result<PathBuf> {
    Ok(cache_root()?.join("t").join(&commit[..12.min(commit.len())]))
}

pub fn snapshot_path(project: &str, repo: &str, key: &str) -> Result<PathBuf> {
    Ok(snapshots_dir(project)?.join(format!("{}-{key}.sqlite", check_name(repo)?)))
}

/// One private cargo target per (project, toolchain) (R3-16).
pub fn cargo_target_dir(project: &str, toolchain: &str) -> Result<PathBuf> {
    let safe: String = toolchain.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '_' }).collect();
    Ok(cache_root()?.join("cargo-target").join(check_name(project)?).join(safe))
}
