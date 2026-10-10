//! Thin launcher for the C/C++ extractor helper (ADR-0034).
//!
//! The helper is a separate executable, `insight-extract-cpp.exe`, shipped next to
//! `insight.exe`, so libclang is only ever loaded by the helper.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

pub fn helper_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().context("current exe has no folder")?;
    let path = dir.join(format!("insight-extract-cpp{}", std::env::consts::EXE_SUFFIX));
    if !path.exists() {
        bail!("C/C++ extractor helper not found at {}", path.display());
    }
    Ok(path)
}

/// Builds the extractor command; the caller streams its stdout (NDJSON).
pub fn command(root: &Path, compdbs: &[PathBuf], schema: u32) -> Result<Command> {
    let mut cmd = Command::new(helper_path()?);
    cmd.arg("--root").arg(root).arg("--schema").arg(schema.to_string());
    for c in compdbs {
        cmd.arg("--compdb").arg(c);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::piped());
    Ok(cmd)
}