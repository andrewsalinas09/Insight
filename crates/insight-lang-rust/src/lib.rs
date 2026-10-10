//! Thin launcher for the Rust extractor helper (ADR-0034, R2-27).
//!
//! The helper is a separate executable, `insight-extract-rust.exe`, shipped
//! next to `insight.exe`, so rust-analyzer's crates never link into the CLI.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn helper_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().context("current exe has no folder")?;
    let path = dir.join(format!("insight-extract-rust{}", std::env::consts::EXE_SUFFIX));
    if !path.exists() {
        bail!("Rust extractor helper not found at {}", path.display());
    }
    Ok(path)
}

/// Builds the extractor command; the caller streams its stdout (NDJSON).
pub fn command(root: &Path, packages: &[String], schema: u32) -> Result<Command> {
    let mut cmd = Command::new(helper_path()?);
    cmd.arg("--root").arg(root).arg("--schema").arg(schema.to_string());
    for p in packages {
        cmd.arg("--package").arg(p);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::piped());
    Ok(cmd)
}
