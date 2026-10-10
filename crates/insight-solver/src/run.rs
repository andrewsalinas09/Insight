//! Running project commands (install/check) in Insight-owned trees, with the
//! tripwire (R3-02): while a command runs, the live repo is watched, and any
//! change to it stops the command and reports which files changed.
//!
//! `.git/` is not watched: IDEs refresh the index constantly, and Insight's
//! commands run in its own tree, never against the live repo's git data.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify::{RecursiveMode, Watcher};

pub enum Outcome {
    Ok,
    Failed(i32),
    /// The live repo changed while the command ran.
    Tripped(Vec<PathBuf>),
}

/// Runs `command` through `cmd /C` in `cwd`; output goes to `log`.
pub fn run(command: &str, cwd: &Path, env: &[(String, String)], log: &std::fs::File, watch: &Path) -> Result<Outcome> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)?;
    watcher.watch(watch, RecursiveMode::Recursive).with_context(|| format!("watching {}", watch.display()))?;
    let git = watch.join(".git");

    let mut cmd = shell(command);
    cmd.current_dir(cwd).stdin(Stdio::null()).stdout(log.try_clone()?).stderr(log.try_clone()?);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().with_context(|| format!("starting `{command}`"))?;
    loop {
        let mut changed: Vec<PathBuf> = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            if let Ok(ev) = ev {
                changed.extend(ev.paths.into_iter().filter(|p| !p.starts_with(&git)));
            }
        }
        if !changed.is_empty() {
            child.kill().ok();
            child.wait().ok();
            changed.sort();
            changed.dedup();
            return Ok(Outcome::Tripped(changed));
        }
        if let Some(status) = child.try_wait()? {
            return Ok(if status.success() { Outcome::Ok } else { Outcome::Failed(status.code().unwrap_or(-1)) });
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(windows)]
fn shell(command: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new("cmd");
    // raw_arg: pass the command line through untouched (no re-quoting).
    c.arg("/C").raw_arg(command);
    c
}

#[cfg(not(windows))]
fn shell(command: &str) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(command);
    c
}
