//! `insight install`: one step from a build to a working MCP server in Claude Code.
//!
//! 1. Copies `insight` and its extractor helpers into Insight's own `bin` folder, so the
//!    registered server keeps running while the build folder is rebuilt (on Windows a
//!    running exe cannot be overwritten; the old copy is moved aside, which is allowed).
//! 2. Registers that copy with Claude Code at user scope (works in every folder), replacing
//!    an earlier registration of the same name.
//! 3. Checks what each language needs and says what is missing.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

const HELPERS: &[&str] = &["insight-extract-rust", "insight-extract-cpp"];

/// The MCP server name: development builds register separately from releases.
fn server_name() -> &'static str {
    if cfg!(debug_assertions) { "insight-dev" } else { "insight" }
}

pub fn install(register: bool) -> Result<String> {
    let mut out = String::new();
    let here = std::env::current_exe()?;
    let from = here.parent().context("the insight executable has no folder")?;
    let bin = insight_solver::paths::cache_root()?.join("bin");
    std::fs::create_dir_all(&bin)?;
    let exe = |name: &str| format!("{name}{}", std::env::consts::EXE_SUFFIX);
    let installed = bin.join(exe("insight"));
    if here != installed {
        copy_replacing(&here, &installed)?;
        out.push_str(&format!("copied insight to {}\n", installed.display()));
        for h in HELPERS {
            let src = from.join(exe(h));
            if src.exists() {
                copy_replacing(&src, &bin.join(exe(h)))?;
                out.push_str(&format!("copied {h}\n"));
            } else {
                out.push_str(&format!("not found next to insight: {h} (build the whole workspace: `cargo build`)\n"));
            }
        }
    } else {
        out.push_str(&format!("already running from {}\n", bin.display()));
    }

    out.push_str("\nlanguage support:\n");
    for (lang, ok, how) in checks(&bin) {
        out.push_str(&format!("  {lang:11} {}  {how}\n", if ok { "ok     " } else { "MISSING" }));
    }

    let cmd = format!("claude mcp add -s user {} -- \"{}\" mcp", server_name(), installed.display());
    if !register {
        out.push_str(&format!("\nTo register with Claude Code:\n  {cmd}\n"));
        return Ok(out);
    }
    if claude(&["--version"]).is_err() {
        out.push_str(&format!("\nClaude Code's `claude` command was not found. Register by hand:\n  {cmd}\n"));
        return Ok(out);
    }
    // Replace an earlier registration (a missing one is not an error).
    let _ = claude(&["mcp", "remove", "-s", "user", server_name()]);
    let installed_str = installed.display().to_string();
    match claude(&["mcp", "add", "-s", "user", server_name(), "--", &installed_str, "mcp"]) {
        Ok(_) => out.push_str(&format!(
            "\nregistered MCP server `{}` with Claude Code (user scope).\nStart a new Claude Code session (or run /mcp) and ask about a project; `insight init <name> <folder>` creates one.\n",
            server_name()
        )),
        Err(e) => out.push_str(&format!("\nregistering failed ({e:#}). Register by hand:\n  {cmd}\n")),
    }
    Ok(out)
}

/// Copies over `dest`, moving a running copy aside first.
fn copy_replacing(src: &Path, dest: &Path) -> Result<()> {
    if dest.exists() && std::fs::copy(src, dest).is_ok() {
        return Ok(());
    }
    if dest.exists() {
        let old = dest.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(dest, &old).with_context(|| format!("moving aside {}", dest.display()))?;
    }
    std::fs::copy(src, dest).with_context(|| format!("copying to {}", dest.display()))?;
    Ok(())
}

/// Runs Claude Code's CLI (through the shell on Windows, where it is a `.cmd`).
fn claude(args: &[&str]) -> Result<String> {
    let mut c = if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg("claude");
        c
    } else {
        Command::new("claude")
    };
    let out = c.args(args).output().context("running claude")?;
    anyhow::ensure!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr).trim());
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// What each language needs, probed by running it (never by its presence on PATH alone).
fn checks(bin: &Path) -> Vec<(&'static str, bool, String)> {
    let runs = |prog: &str, arg: &str| {
        let mut c = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.arg("/C").arg(prog);
            c
        } else {
            Command::new(prog)
        };
        c.arg(arg).output().is_ok_and(|o| o.status.success())
    };
    let libclang = libclang_found();
    vec![
        ("TypeScript", runs("node", "--version"), "Node.js (the project's own TypeScript is used)".into()),
        ("Rust", runs("cargo", "--version") && bin.join(format!("insight-extract-rust{}", std::env::consts::EXE_SUFFIX)).exists(), "cargo + the bundled rust-analyzer helper".into()),
        (
            "C/C++",
            libclang.is_some() && bin.join(format!("insight-extract-cpp{}", std::env::consts::EXE_SUFFIX)).exists(),
            libclang.map_or("libclang (install LLVM, e.g. `winget install LLVM.LLVM`, or set LIBCLANG_PATH)".into(), |p| format!("libclang at {}", p.display())),
        ),
    ]
}

fn libclang_found() -> Option<PathBuf> {
    let name = if cfg!(windows) { "libclang.dll" } else if cfg!(target_os = "macos") { "libclang.dylib" } else { "libclang.so" };
    let mut dirs: Vec<PathBuf> = std::env::var_os("LIBCLANG_PATH").map(PathBuf::from).into_iter().collect();
    if cfg!(windows) {
        dirs.push(PathBuf::from(r"C:\Program Files\LLVM\bin"));
    } else {
        dirs.extend(["/usr/lib", "/usr/local/lib", "/usr/lib/llvm/lib", "/opt/homebrew/opt/llvm/lib"].map(PathBuf::from));
    }
    dirs.into_iter().map(|d| d.join(name)).find(|p| p.exists())
}
