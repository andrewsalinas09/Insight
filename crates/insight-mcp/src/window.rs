//! What the window draws: the loaded snapshot as nested boxes and typed lines
//! (ADR-0008, ADR-0021), and source text under the agent's deny rules.
//!
//! Boxes: folders (from file paths; a chain of single-child folders is one box),
//! then the code's own containers, plus fields and statics, plus library symbols
//! grouped as libraries › owner › symbol. Lines: calls, possible calls, handoffs,
//! reads, writes and references taken, counted per (from, to, kind).

use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use serde_json::{Value, json};

use crate::Loaded;

/// Line kinds, as the window numbers them.
const LINE_KINDS: &[(&str, &str, &str, u8)] = &[
    ("call", "", "exact", 0),
    ("call", "", "possible", 1),
    ("value-flow", "handoff", "exact", 2),
    ("read", "", "exact", 3),
    ("write", "", "exact", 4),
    ("access", "", "exact", 5),
];

pub(crate) fn graph(l: &Loaded) -> Result<Value> {
    let conn = l.db.conn();
    let mut nodes: Vec<Value> = Vec::new();
    let mut known: HashMap<i64, ()> = HashMap::new();
    let mut synth: i64 = 0;
    let mut groups: HashMap<String, i64> = HashMap::new();

    // Containers.
    struct Row {
        nid: i64,
        parent: Option<i64>,
        kind: String,
        name: String,
        file: Option<String>,
        lang: Option<String>,
        a: Option<i64>,
        b: Option<i64>,
    }
    let rows: Vec<Row> = {
        let mut stmt = conn.prepare("SELECT nid, parent_nid, subkind, name, file, lang, line_start, line_end FROM main.nodes WHERE kind = 'container' ORDER BY nid")?;
        stmt.query_map([], |r| Ok(Row { nid: r.get(0)?, parent: r.get(1)?, kind: r.get::<_, Option<String>>(2)?.unwrap_or_default(), name: r.get(3)?, file: r.get(4)?, lang: r.get(5)?, a: r.get(6)?, b: r.get(7)? }))?
            .collect::<Result<_, _>>()?
    };

    // Folder boxes for top-level containers, from their files' folders.
    let folders: Vec<String> = {
        let mut f: Vec<String> = rows.iter().filter(|r| r.parent.is_none()).filter_map(|r| r.file.as_deref().map(folder_of)).collect();
        f.sort();
        f.dedup();
        f
    };
    let tree = FolderTree::build(&folders);
    let mut folder_id: HashMap<String, i64> = HashMap::new();
    for (path, parent, label) in &tree.boxes {
        synth -= 1;
        folder_id.insert(path.clone(), synth);
        nodes.push(json!({"id": synth, "p": parent.as_ref().map(|p| folder_id[p]), "k": "folder", "n": label}));
    }

    for r in &rows {
        let p = match r.parent {
            Some(p) => Some(p),
            None => r.file.as_deref().map(folder_of).and_then(|f| tree.owner(&f)).map(|f| folder_id[&f]),
        };
        known.insert(r.nid, ());
        nodes.push(json!({"id": r.nid, "p": p, "k": r.kind, "n": r.name, "f": r.file, "a": r.a, "b": r.b, "l": r.lang}));
    }

    // Fields and statics of types, files and modules (function locals stay in the code).
    {
        let mut stmt = conn.prepare(
            "SELECT n.nid, n.parent_nid, n.subkind, n.name, n.file, n.line_start, n.line_end FROM main.nodes n JOIN main.nodes c ON c.nid = n.parent_nid \
             WHERE n.kind = 'state' AND n.subkind IN ('field', 'static') AND c.subkind IN ('type', 'file', 'module')",
        )?;
        let found: Vec<(i64, i64, Value)> = stmt
            .query_map([], |r| {
                let (nid, p): (i64, i64) = (r.get(0)?, r.get(1)?);
                let row = json!({"id": nid, "p": p, "k": r.get::<_, String>(2)?, "n": r.get::<_, String>(3)?, "f": r.get::<_, Option<String>>(4)?, "a": r.get::<_, Option<i64>>(5)?, "b": r.get::<_, Option<i64>>(6)?});
                Ok((nid, p, row))
            })?
            .collect::<Result<_, _>>()?;
        for (nid, p, row) in found {
            if known.contains_key(&p) {
                known.insert(nid, ());
                nodes.push(row);
            }
        }
    }

    // Library symbols: ext:<lang>:<library>:<owner path + name> -> libraries › library › owner › symbol.
    {
        let mut stmt = conn.prepare("SELECT nid, path, name FROM main.nodes WHERE kind = 'external'")?;
        let found: Vec<(i64, String, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        let mut group = |key: String, make: &mut dyn FnMut(i64) -> Value, nodes: &mut Vec<Value>| -> i64 {
            if let Some(id) = groups.get(&key) {
                return *id;
            }
            synth -= 1;
            groups.insert(key, synth);
            nodes.push(make(synth));
            synth
        };
        let libs = if found.is_empty() { None } else { Some(group("libs".into(), &mut |id| json!({"id": id, "p": null, "k": "libroot", "n": "libraries"}), &mut nodes)) };
        for (nid, path, name) in found {
            let Some((lib, symbol)) = split_external(&path) else { continue };
            let libs = libs.unwrap_or_default();
            let lib_id = group(format!("lib:{lib}"), &mut |id| json!({"id": id, "p": libs, "k": "lib", "n": lib}), &mut nodes);
            let segs: Vec<&str> = symbol.split("::").flat_map(|s| s.split('.')).filter(|s| !s.is_empty()).collect();
            let parent = if segs.len() > 1 {
                let owner = segs[segs.len() - 2].to_string();
                group(format!("libt:{lib}:{}", segs[..segs.len() - 1].join("::")), &mut |id| json!({"id": id, "p": lib_id, "k": "libtype", "n": owner}), &mut nodes)
            } else {
                lib_id
            };
            known.insert(nid, ());
            nodes.push(json!({"id": nid, "p": parent, "k": "ext", "n": name}));
        }
    }

    // Typed lines between drawn nodes: [from, to, kind, count, sites], where each site is
    // where one occurrence is written ([line, col, end_line, end_col], 1-based), so the
    // window can tie a line to the code and walk a function's calls in the order written.
    let mut edges: Vec<Value> = Vec::new();
    {
        let mut stmt = conn.prepare(
            "WITH first AS (SELECT eid, min(rowid) AS rid FROM main.receipts WHERE role NOT IN ('value', 'decl') GROUP BY eid) \
             SELECT e.from_nid, e.to_nid, e.kind, coalesce(e.subkind, ''), e.resolution, r.line, r.col, r.end_line, r.end_col \
             FROM main.edges e LEFT JOIN first ON first.eid = e.eid LEFT JOIN main.receipts r ON r.rowid = first.rid \
             WHERE e.to_nid IS NOT NULL",
        )?;
        type Found = (i64, i64, String, String, String, Option<i64>, Option<i64>, Option<i64>, Option<i64>);
        let found: Vec<Found> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)))?
            .collect::<Result<_, _>>()?;
        // (from, to, kind) -> (count, sites)
        type Lines = BTreeMap<(i64, i64, u8), (i64, Vec<[i64; 4]>)>;
        let mut lines = Lines::new();
        for (f, t, kind, sub, res, l0, c0, l1, c1) in found {
            // `pointee` refines a call's target; it is still a call.
            let sub = if sub == "pointee" { "" } else { sub.as_str() };
            let Some(k) = LINE_KINDS.iter().find(|(k, s, r, _)| *k == kind && *s == sub && *r == res).map(|x| x.3) else { continue };
            if !(known.contains_key(&f) && known.contains_key(&t)) {
                continue;
            }
            let x = lines.entry((f, t, k)).or_default();
            x.0 += 1;
            if let (Some(l0), Some(c0), Some(l1), Some(c1)) = (l0, c0, l1, c1) {
                x.1.push([l0, c0, l1, c1]);
            }
        }
        for ((f, t, k), (c, mut sites)) in lines {
            sites.sort();
            sites.dedup();
            edges.push(json!([f, t, k, c, sites]));
        }
    }

    let repo: String = conn.query_row("SELECT value FROM main.meta WHERE key = 'repo'", [], |r| r.get(0)).unwrap_or_default();
    let commit: String = conn.query_row("SELECT value FROM main.meta WHERE key = 'commit'", [], |r| r.get(0)).unwrap_or_default();
    Ok(json!({
        "project": l.project,
        "snapshot": crate::file_name(&l.main),
        "repo": repo,
        "commit": commit,
        "nodes": nodes,
        "edges": edges,
    }))
}

/// A snapshot file's text for the window, under the same deny rules as `source`.
pub(crate) fn file_text(l: &Loaded, file: &str) -> Result<String> {
    let repo = &l.repos[0];
    anyhow::ensure!(!repo.denies(file), "`{file}` is covered by the agent's deny rules");
    let hidden = l.repos.iter().any(|r| !r.denied.is_empty());
    l.db.set_hide_content(false);
    let content = l.db.conn().query_row::<Vec<u8>, _, _>("SELECT content FROM main.files WHERE path = ?1", [file], |r| r.get(0));
    l.db.set_hide_content(hidden);
    Ok(String::from_utf8_lossy(&content?).into_owned())
}

fn folder_of(file: &str) -> String {
    file.rsplit_once('/').map_or(String::new(), |(d, _)| d.to_string())
}

/// `ext:<lang>:<library>:<symbol>` -> (library, symbol). The library part may itself
/// hold `::` (a module path); the split is at the last single `:`.
fn split_external(path: &str) -> Option<(String, String)> {
    let rest = path.strip_prefix("ext:")?;
    let (_, rest) = rest.split_once(':')?;
    let b = rest.as_bytes();
    let single = (0..b.len()).rev().find(|&i| b[i] == b':' && (i == 0 || b[i - 1] != b':') && b.get(i + 1) != Some(&b':'));
    Some(match single {
        Some(i) => (rest[..i].to_string(), rest[i + 1..].to_string()),
        None => (rest.to_string(), String::new()),
    })
}

/// Folder boxes: every folder holding code, and every folder where paths branch;
/// a chain of single-child folders becomes one box labeled with the joined path.
struct FolderTree {
    /// (full path, parent box path, label), parents before children.
    boxes: Vec<(String, Option<String>, String)>,
}

impl FolderTree {
    fn build(folders: &[String]) -> FolderTree {
        // Count children of every folder prefix.
        let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut all: BTreeMap<String, bool> = BTreeMap::new(); // path -> holds code directly
        for f in folders {
            all.insert(f.clone(), true);
            let mut cur = f.as_str();
            while let Some((parent, _)) = cur.rsplit_once('/') {
                all.entry(parent.to_string()).or_insert(false);
                cur = parent;
            }
            if !f.is_empty() {
                all.entry(String::new()).or_insert(false);
            }
        }
        for p in all.keys() {
            if p.is_empty() {
                continue;
            }
            let parent = p.rsplit_once('/').map_or(String::new(), |(d, _)| d.to_string());
            children.entry(parent).or_default().push(p.clone());
        }
        // A folder gets a box if it holds code or branches; the root never does.
        let boxed = |p: &str| !p.is_empty() && (all[p] || children.get(p).is_some_and(|c| c.len() > 1));
        let mut boxes = Vec::new();
        fn walk(p: &str, parent_box: Option<&str>, children: &BTreeMap<String, Vec<String>>, boxed: &dyn Fn(&str) -> bool, boxes: &mut Vec<(String, Option<String>, String)>) {
            let here = if boxed(p) {
                let label = match parent_box {
                    Some(b) => p.strip_prefix(b).map(|s| s.trim_start_matches('/')).unwrap_or(p).to_string(),
                    None => p.to_string(),
                };
                boxes.push((p.to_string(), parent_box.map(String::from), label));
                Some(p)
            } else {
                parent_box
            };
            for c in children.get(p).into_iter().flatten() {
                walk(c, here, children, boxed, boxes);
            }
        }
        walk("", None, &children, &boxed, &mut boxes);
        FolderTree { boxes }
    }

    /// The box a file in `folder` belongs to (`None` for files at the repo root).
    fn owner(&self, folder: &str) -> Option<String> {
        self.boxes.iter().any(|(p, _, _)| p == folder).then(|| folder.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn externals() {
        assert_eq!(split_external("ext:rust:std:collections::HashMap::new"), Some(("std".into(), "collections::HashMap::new".into())));
        assert_eq!(split_external("ext:ts:typescript:Program.getSourceFiles"), Some(("typescript".into(), "Program.getSourceFiles".into())));
        assert_eq!(split_external("ext:cpp::printf"), Some(("".into(), "printf".into())));
    }

    #[test]
    fn folders_join_single_child_chains() {
        let t = FolderTree::build(&["a/b/c".into(), "a/b/d".into(), "x/y".into()]);
        let got: Vec<(String, Option<String>, String)> = t.boxes;
        assert_eq!(
            got,
            vec![
                ("a/b".into(), None, "a/b".into()),
                ("a/b/c".into(), Some("a/b".into()), "c".into()),
                ("a/b/d".into(), Some("a/b".into()), "d".into()),
                ("x/y".into(), None, "x/y".into()),
            ]
        );
    }
}
