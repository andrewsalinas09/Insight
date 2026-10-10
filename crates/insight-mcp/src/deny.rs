//! The agent's file deny rules, mirrored (R3-03): a file the agent may not Read
//! is not served by `source`, not written to the frozen-source folder, and
//! hides `files.content` from `sql`.
//!
//! Sources: Claude Code's `permissions.deny` `Read(<pattern>)` entries in
//! `~/.claude/settings.json` and the repo's `.claude/settings.json` and
//! `.claude/settings.local.json`. Patterns follow gitignore style:
//! - `//abs` is absolute;
//! - `~/p` is under the home folder;
//! - `/p` is relative to the settings file's project;
//! - anything else is relative to the repo, and a bare name matches at any depth.

use std::path::{Path, PathBuf};

enum Rule {
    /// A glob over the repo-relative path (forward slashes).
    Rel(String),
    /// A glob over the absolute path (forward slashes).
    Abs(String),
}

pub struct DenyRules {
    rules: Vec<Rule>,
    /// Each rule as written, for `status`.
    pub sources: Vec<String>,
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
}

fn slashes(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

impl DenyRules {
    pub fn load(repo_root: Option<&Path>) -> DenyRules {
        let mut out = DenyRules { rules: Vec::new(), sources: Vec::new() };
        if let Some(h) = home() {
            out.read(&h.join(".claude").join("settings.json"), &h.join(".claude"));
        }
        if let Some(r) = repo_root {
            for f in ["settings.json", "settings.local.json"] {
                out.read(&r.join(".claude").join(f), r);
            }
        }
        out
    }

    fn read(&mut self, file: &Path, base: &Path) {
        let Ok(text) = std::fs::read_to_string(file) else { return };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return };
        let Some(deny) = v.pointer("/permissions/deny").and_then(|d| d.as_array()) else { return };
        for d in deny.iter().filter_map(|d| d.as_str()) {
            let Some(p) = d.strip_prefix("Read(").and_then(|p| p.strip_suffix(')')) else { continue };
            let rule = if let Some(abs) = p.strip_prefix("//") {
                Rule::Abs(format!("/{abs}"))
            } else if let Some(rest) = p.strip_prefix("~/") {
                match home() {
                    Some(h) => Rule::Abs(format!("{}/{rest}", slashes(&h))),
                    None => continue,
                }
            } else if let Some(rest) = p.strip_prefix('/') {
                Rule::Abs(format!("{}/{rest}", slashes(base)))
            } else {
                let p = p.strip_prefix("./").unwrap_or(p);
                if p.trim_end_matches('/').contains('/') { Rule::Rel(p.to_string()) } else { Rule::Rel(format!("**/{p}")) }
            };
            self.rules.push(rule);
            self.sources.push(format!("{d} ({})", file.display()));
        }
    }


    /// True if the agent may not read `rel` (a repo-relative path) of the repo at `repo_root`.
    pub fn denies(&self, repo_root: Option<&Path>, rel: &str) -> bool {
        let abs = repo_root.map(|r| format!("{}/{rel}", slashes(r)));
        self.rules.iter().any(|r| match r {
            Rule::Rel(g) => matches(g, rel),
            Rule::Abs(g) => abs.as_deref().is_some_and(|a| {
                // `//c/Users/…` and `//C:/Users/…` both name `C:\Users\…`.
                let drive = a.get(..2).filter(|d| d.ends_with(':')).map(|d| format!("/{}{}", &d[..1], &a[2..]));
                matches(g, &format!("/{a}")) || matches(g, a) || drive.is_some_and(|d| matches(g, &d))
            }),
        })
    }
}

/// gitignore-style glob, case-insensitive (Windows paths): `*` and `?` stay within a
/// segment, `**` crosses segments, and a pattern that names a folder covers its contents.
fn matches(glob: &str, path: &str) -> bool {
    let g = glob.trim_end_matches('/');
    m(g.as_bytes(), path.as_bytes()) || m(format!("{g}/**").as_bytes(), path.as_bytes())
}

fn m(p: &[u8], s: &[u8]) -> bool {
    match p.first() {
        None => s.is_empty(),
        Some(b'*') if p.get(1) == Some(&b'*') => {
            let rest = &p[2..];
            if let Some(after) = rest.strip_prefix(b"/") {
                m(after, s) || (0..s.len()).any(|i| s[i] == b'/' && m(after, &s[i + 1..]))
            } else {
                (0..=s.len()).any(|i| m(rest, &s[i..]))
            }
        }
        Some(b'*') => (0..=s.len()).take_while(|i| *i == 0 || s[i - 1] != b'/').any(|i| m(&p[1..], &s[i..])),
        Some(b'?') => s.first().is_some_and(|c| *c != b'/') && m(&p[1..], &s[1..]),
        Some(c) => s.first().is_some_and(|x| x.eq_ignore_ascii_case(c)) && m(&p[1..], &s[1..]),
    }
}

#[cfg(test)]
mod tests {
    use super::matches;

    #[test]
    fn globs() {
        assert!(matches("**/.env", ".env"));
        assert!(matches("**/.env", "app/.env"));
        assert!(!matches("**/.env", "app/.env.example"));
        assert!(matches("**/.env*", "app/.env.local"));
        assert!(matches("secrets/", "secrets/a/b.txt"));
        assert!(matches("secrets", "secrets/key.pem"));
        assert!(matches("src/*.rs", "src/main.rs"));
        assert!(!matches("src/*.rs", "src/a/main.rs"));
        assert!(matches("src/**/*.rs", "src/a/b/main.rs"));
        assert!(matches("/C:/Users/me/**", "/c:/users/me/x/y"));
    }
}
