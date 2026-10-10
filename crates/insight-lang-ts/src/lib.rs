//! Thin launcher for the TypeScript/JavaScript extractor (ADR-0034, R2-15).
//!
//! The extractor script is embedded in the binary and written into the cache
//! under its content hash, so `insight.exe` is self-contained. It runs on the
//! system's Node, loading the repo's own `typescript` package.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, bail};

const SCRIPT: &str = include_str!("../../../extractors/ts/extract.mjs");

/// Finds a working Node by running it (never trust PATH alone).
pub fn node_version() -> Result<String> {
    let out = Command::new("node").arg("--version").output().context("`node` could not be run; install Node.js")?;
    if !out.status.success() {
        bail!("`node --version` failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Writes the embedded script under `cache/extractors/` and returns its path.
pub fn script_path(cache_root: &Path) -> Result<PathBuf> {
    let hash = fnv1a(SCRIPT.as_bytes());
    let path = cache_root.join("extractors").join(format!("ts-{hash:016x}.mjs"));
    if !path.exists() {
        std::fs::create_dir_all(path.parent().expect("has parent"))?;
        std::fs::write(&path, SCRIPT)?;
    }
    Ok(path)
}

/// Builds the extractor command; the caller streams its stdout (NDJSON).
pub fn command(cache_root: &Path, root: &Path, projects: &[String], schema: u32) -> Result<Command> {
    let mut cmd = Command::new("node");
    cmd.arg(script_path(cache_root)?).arg("--root").arg(root).arg("--schema").arg(schema.to_string());
    for p in projects {
        cmd.arg("--project").arg(p);
    }
    cmd.stdin(Stdio::null()).stdout(Stdio::piped());
    Ok(cmd)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x100000001b3))
}
