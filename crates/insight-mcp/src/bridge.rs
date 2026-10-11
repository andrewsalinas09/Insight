//! `insight mcp` as a bridge to the running app (ADR-0031, R2-13, BQ-43).
//!
//! The app serves its [`Server`] on a local named pipe. It publishes the pipe's
//! name and a fresh random token in `<cache root>\run\app.json`, which only the
//! current user can read. A client must send the token as its first line before
//! anything else is answered, so only the user's own processes are served. The
//! pipe also rejects remote clients, and the app creates it as the first
//! instance, so no other process can own that name first.
//!
//! The bridge answers `initialize` and `tools/list` itself. On the first tool
//! call it connects to the app, starting `insight-app` from its own folder if
//! the app isn't running, and relays from then on. With no app to reach
//! (`INSIGHT_NO_APP=1`, not installed, or it fails to start), it runs a server in
//! this process instead, the same backend against the same snapshot cache.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::Server;

/// Where the running app says how to reach it.
pub fn run_file() -> Result<PathBuf> {
    Ok(insight_solver::paths::cache_root()?.join("run").join("app.json"))
}

/// What the app publishes: the pipe and the token a client must present.
pub struct Endpoint {
    pub pipe: String,
    pub token: String,
}

impl Endpoint {
    /// A new endpoint with a random pipe name and token.
    pub fn fresh() -> Result<Endpoint> {
        let hex = |n: usize| -> Result<String> {
            let mut b = vec![0u8; n];
            getrandom::fill(&mut b).map_err(|e| anyhow::anyhow!("no OS randomness: {e}"))?;
            Ok(b.iter().map(|x| format!("{x:02x}")).collect())
        };
        Ok(Endpoint { pipe: format!(r"\\.\pipe\{}-{}", insight_solver::paths::channel().to_lowercase(), hex(8)?), token: hex(32)? })
    }

    /// Publishes it for clients; the app removes the file when it exits.
    pub fn publish(&self) -> Result<()> {
        let f = run_file()?;
        std::fs::create_dir_all(f.parent().context("run file has no folder")?)?;
        let tmp = f.with_extension("tmp");
        std::fs::write(&tmp, json!({"pipe": self.pipe, "token": self.token, "pid": std::process::id()}).to_string())?;
        std::fs::rename(&tmp, &f)?;
        Ok(())
    }

    fn read() -> Option<Endpoint> {
        let v: Value = serde_json::from_str(&std::fs::read_to_string(run_file().ok()?).ok()?).ok()?;
        Some(Endpoint { pipe: v["pipe"].as_str()?.to_string(), token: v["token"].as_str()?.to_string() })
    }

    /// Constant-time token check for the app's side.
    pub fn accepts(&self, line: &str) -> bool {
        let given: Value = serde_json::from_str(line).unwrap_or(Value::Null);
        let given = given["token"].as_str().unwrap_or("").as_bytes();
        let want = self.token.as_bytes();
        given.len() == want.len() && given.iter().zip(want).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    }
}

/// A connection to the app: requests out, one reply line back for each.
struct Link {
    write: std::fs::File,
    read: BufReader<std::fs::File>,
}

impl Link {
    fn connect() -> Result<Link> {
        let ep = Endpoint::read().context("the app isn't running (no run file)")?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let file = loop {
            match std::fs::OpenOptions::new().read(true).write(true).open(&ep.pipe) {
                Ok(f) => break f,
                // All instances busy: the app opens another one at once.
                Err(_) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
                Err(e) => return Err(e).with_context(|| format!("connecting to {}", ep.pipe)),
            }
        };
        let mut link = Link { read: BufReader::new(file.try_clone()?), write: file };
        let ok = link.exchange(&json!({"token": ep.token}).to_string())?;
        anyhow::ensure!(ok.trim() == "ok", "the app refused the connection");
        Ok(link)
    }

    fn exchange(&mut self, line: &str) -> Result<String> {
        self.write.write_all(line.as_bytes())?;
        self.write.write_all(b"\n")?;
        self.write.flush()?;
        let mut reply = String::new();
        anyhow::ensure!(self.read.read_line(&mut reply)? > 0, "the app closed the connection");
        Ok(reply)
    }

    /// Connects, starting the app first if it isn't running.
    fn connect_or_launch() -> Result<Link> {
        if let Ok(l) = Link::connect() {
            return Ok(l);
        }
        let here = std::env::current_exe()?;
        let app = here.with_file_name(format!("insight-app{}", std::env::consts::EXE_SUFFIX));
        anyhow::ensure!(app.exists(), "{} is not installed", app.display());
        // The app outlives this bridge, so it must not inherit the agent's stdio pipes
        // (the agent would never see them close). Any process started from here on
        // Windows inherits every inheritable handle, `cmd /C start` included; Explorer
        // starts it like a double-click, with none of ours.
        #[cfg(windows)]
        let mut cmd = {
            let mut c = std::process::Command::new("explorer.exe");
            c.arg(&app);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = std::process::Command::new(&app);
        cmd.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        cmd.spawn().with_context(|| format!("starting {}", app.display()))?;
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match Link::connect() {
                Ok(l) => return Ok(l),
                Err(e) if Instant::now() >= deadline => return Err(e.context("the app didn't come up within 20 s")),
                Err(_) => std::thread::sleep(Duration::from_millis(200)),
            }
        }
    }
}

/// True if a running app answers on its published pipe.
pub fn app_running() -> bool {
    Link::connect().is_ok()
}

enum Mode {
    /// Not connected yet.
    Pending,
    App(Link),
    Local,
}

/// `insight mcp`: stdio MCP, relayed to the app when it can be reached.
pub fn run_stdio() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    let mut local = Server::default();
    let mut mode = if std::env::var_os("INSIGHT_NO_APP").is_some() { Mode::Local } else { Mode::Pending };
    for line in stdin.lock().lines() {
        let line = line?;
        let method = serde_json::from_str::<Value>(&line).ok().and_then(|m| m["method"].as_str().map(String::from)).unwrap_or_default();
        // Only state-touching calls go to the app; the handshake and tool list are the same everywhere.
        let reply = if method != "tools/call" {
            local.handle_line(&line)
        } else {
            if let Mode::Pending = mode {
                mode = match Link::connect_or_launch() {
                    Ok(l) => Mode::App(l),
                    Err(e) => {
                        eprintln!("insight mcp: no window ({e:#}); serving in this process");
                        Mode::Local
                    }
                };
            }
            match &mut mode {
                Mode::App(link) => match link.exchange(&line) {
                    Ok(r) => Some(r.trim_end().to_string()).filter(|s| !s.is_empty()),
                    // The app went away: try it once more, then answer here.
                    Err(_) => match Link::connect_or_launch().and_then(|mut l| l.exchange(&line).map(|r| (l, r))) {
                        Ok((l, r)) => {
                            mode = Mode::App(l);
                            Some(r.trim_end().to_string()).filter(|s| !s.is_empty())
                        }
                        Err(e) => {
                            eprintln!("insight mcp: lost the window ({e:#}); serving in this process");
                            mode = Mode::Local;
                            local.handle_line(&line)
                        }
                    },
                },
                _ => local.handle_line(&line),
            }
        };
        if let Some(r) = reply {
            stdout.write_all(r.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_must_match_exactly() {
        let ep = Endpoint { pipe: String::new(), token: "ab12".into() };
        assert!(ep.accepts(r#"{"token":"ab12"}"#));
        assert!(!ep.accepts(r#"{"token":"ab13"}"#));
        assert!(!ep.accepts(r#"{"token":"ab1"}"#));
        assert!(!ep.accepts(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call"}"#));
        assert!(!ep.accepts("not json"));
    }
}
