//! The project glossary (ADR-0056): terms for this codebase, each a plain definition
//! plus the code it names, kept by Claude with the `glossary` tool. It lives in the
//! project's INSIGHT.md, in its own ```` ```insight-glossary ```` block (TOML), so it
//! travels with the project's other notes and the settings parser never sees it. The
//! window shows it while you read code: hovering a name shows its terms.
//!
//! Code names are stored as node paths (stable across snapshots) and resolved against
//! the loaded snapshot whenever they're read; a path that no longer resolves is shown
//! as stale, never dropped silently.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::Loaded;

const FENCE: &str = "```insight-glossary";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct Glossary {
    #[serde(default)]
    pub term: Vec<Term>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Term {
    pub name: String,
    pub means: String,
    /// Node paths of the code this term names (types, variables, functions).
    #[serde(default)]
    pub names: Vec<String>,
}

pub(crate) fn file(project: &str) -> Result<PathBuf> {
    Ok(insight_solver::paths::project_dir(project)?.join("INSIGHT.md"))
}

/// The glossary block's TOML and where it sits in the file (line range of the fence).
fn find(text: &str) -> Result<(String, Option<(usize, usize)>)> {
    let lines: Vec<&str> = text.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == FENCE) else { return Ok((String::new(), None)) };
    let end = lines[start + 1..].iter().position(|l| l.trim() == "```").map(|i| start + 1 + i).context("INSIGHT.md: the ```insight-glossary block is not closed")?;
    Ok((lines[start + 1..end].join("\n"), Some((start, end))))
}

pub(crate) fn read(path: &Path) -> Result<Glossary> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let (toml_src, _) = find(&text)?;
    toml::from_str(&toml_src).context("INSIGHT.md glossary")
}

fn write(path: &Path, g: &Glossary) -> Result<()> {
    let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let body = toml::to_string(g)?;
    let block = format!("{FENCE}\n{}```", body);
    let (_, at) = find(&text)?;
    let out = match at {
        Some((s, e)) => {
            let lines: Vec<&str> = text.lines().collect();
            let mut v: Vec<String> = lines[..s].iter().map(|l| l.to_string()).collect();
            v.push(block);
            v.extend(lines[e + 1..].iter().map(|l| l.to_string()));
            v.join("\n") + "\n"
        }
        None => format!(
            "{}\n## Glossary\n\nTerms for this codebase, kept by Claude with the `glossary` tool; the Insight window shows them when you hover a name.\n\n{block}\n",
            text.trim_end()
        ),
    };
    std::fs::write(path, out).with_context(|| format!("writing {}", path.display()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TermIn {
    name: String,
    means: String,
    #[serde(default)]
    names: Vec<String>,
}

/// `glossary` tool: list, or set and remove terms. Every code name must resolve.
pub(crate) fn tool(l: &Loaded, a: &Value) -> Result<(String, bool)> {
    let path = file(&l.project)?;
    let mut g = read(&path)?;
    let set: Vec<TermIn> = serde_json::from_value(a.get("set").cloned().unwrap_or(json!([]))).context("`set` is a list of {name, means, names}")?;
    let remove: Vec<String> = serde_json::from_value(a.get("remove").cloned().unwrap_or(json!([]))).context("`remove` is a list of term names")?;
    if set.is_empty() && remove.is_empty() {
        let mut o = format!("{} term(s) in {}\n", g.term.len(), path.display());
        for t in &g.term {
            o.push_str(&format!("\n**{}**: {}\n", t.name, t.means));
            for n in &t.names {
                let ok = resolve_path(l, n).is_some();
                o.push_str(&format!("  - {n}{}\n", if ok { "" } else { " (stale: not in this snapshot)" }));
            }
        }
        return Ok((o, false));
    }
    for r in &remove {
        let before = g.term.len();
        g.term.retain(|t| !t.name.eq_ignore_ascii_case(r));
        if g.term.len() == before {
            bail!("no term `{r}` to remove");
        }
    }
    let mut changed = Vec::new();
    for t in set {
        anyhow::ensure!(!t.name.trim().is_empty() && !t.means.trim().is_empty(), "a term needs a `name` and what it `means`");
        let mut names = Vec::new();
        for r in &t.names {
            let id = insight_solver::path::resolve(l.db.conn(), r).with_context(|| format!("term `{}`: `{r}`", t.name))?;
            let p: String = l.db.conn().query_row("SELECT path FROM main.nodes WHERE nid = ?1", [id], |x| x.get(0))?;
            if !names.contains(&p) {
                names.push(p);
            }
        }
        let term = Term { name: t.name.trim().to_string(), means: t.means.trim().to_string(), names };
        match g.term.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&term.name)) {
            Some(x) => *x = term,
            None => g.term.push(term),
        }
        changed.push(t.name);
    }
    g.term.sort_by_key(|t| t.name.to_lowercase());
    write(&path, &g)?;
    Ok((format!("glossary: {} term(s) set, {} removed; {} in all ({})", changed.len(), remove.len(), g.term.len(), path.display()), true))
}

fn resolve_path(l: &Loaded, p: &str) -> Option<i64> {
    l.db.conn().query_row("SELECT nid FROM main.nodes WHERE path = ?1", [p], |r| r.get(0)).ok()
}

/// The glossary for the window: each term with its names resolved to node ids
/// (null when stale).
pub(crate) fn for_window(l: &Loaded) -> Result<Value> {
    let g = read(&file(&l.project)?)?;
    Ok(json!(g.term.iter().map(|t| json!({
        "name": t.name,
        "means": t.means,
        "names": t.names.iter().map(|p| json!({"path": p, "id": resolve_path(l, p)})).collect::<Vec<_>>(),
    })).collect::<Vec<_>>()))
}

/// Whether a term exists, for `[label](term:name)` citations.
pub(crate) fn has(project: &str, name: &str) -> Result<bool> {
    Ok(read(&file(project)?)?.term.iter().any(|t| t.name.eq_ignore_ascii_case(name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_round_trips_and_leaves_the_rest_alone() {
        let dir = std::env::temp_dir().join(format!("insight-gloss-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("INSIGHT.md");
        std::fs::write(&p, "# P\n\nprose\n\n```insight\ntagging = \"on-request\"\n```\n").unwrap();
        let g = Glossary { term: vec![Term { name: "tick".into(), means: "one sample".into(), names: vec!["a › b".into()] }] };
        write(&p, &g).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("```insight\ntagging"));
        assert_eq!(read(&p).unwrap().term[0].name, "tick");
        let g2 = Glossary { term: vec![Term { name: "row".into(), means: "a sample row".into(), names: vec![] }] };
        write(&p, &g2).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert_eq!(text.matches(FENCE).count(), 1);
        assert_eq!(read(&p).unwrap().term[0].name, "row");
        std::fs::remove_dir_all(&dir).ok();
    }
}
