//! `path`: algorithmic reachability with receipts (R2-12, ADR-0047, R4-B).
//!
//! The caller always chooses which hops count (`follow`); there is no default set.
//! Follow tokens are exact predicates over the `edges` table (SCHEMA.md §5):
//! - `call`: `kind='call'` with a target (exact and possible; possible is labeled);
//! - `handoff`: `kind='value-flow' AND subkind='handoff'` (a function value passed to external code);
//! - `literal`: function → literal node (`link/literal`) → every other function containing it;
//! - `names`: function → literal node → the declaration it names (`link/names`);
//! - `data`: value moving between states and functions: `value-flow` `assign`, `result`,
//!   `return` and `binds` (argument → parameter or external callee);
//! - `write` / `access`: function → the state it writes / may write;
//! - `read`: state → each function that reads it (read edges walked toward the reader);
//! - `same` (several repos): a declaration ↔ the same declaration in another repo's
//!   snapshot, where the two files have identical content (shared source compiled in both).
//!
//! **Several repos** (`Options::schemas`, one SQLite schema per repo snapshot): nodes are
//! shown as `<schema>:<path>`; equal literals are one node across repos, and `names`
//! also matches a string in one repo to a declaration in another. These cross-repo links
//! are computed from each snapshot's facts at query time (exact equality of content,
//! value or name); what they mean is the caller's judgment.
//!
//! The start may be a function that returns function literals: those are starts too
//! (R3-G #5, from `value-flow/returns` edges). Loops and recursion are never followed
//! forever; they are reported as facts.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use anyhow::{Result, bail};
use insight_engine::rusqlite::{Connection, params};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Follow {
    Call,
    Handoff,
    Literal,
    Names,
    Data,
    Read,
    Write,
    Access,
    Same,
}

const TOKENS: &str = "call, handoff, literal, names, data, read, write, access, same";

impl Follow {
    pub fn parse_list(s: &str) -> Result<Vec<Follow>> {
        let mut out = Vec::new();
        for t in s.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            out.push(match t {
                "call" => Follow::Call,
                "handoff" => Follow::Handoff,
                "literal" => Follow::Literal,
                "names" => Follow::Names,
                "data" => Follow::Data,
                "read" => Follow::Read,
                "write" => Follow::Write,
                "access" => Follow::Access,
                "same" => Follow::Same,
                other => bail!("unknown follow token `{other}`; valid: {TOKENS}"),
            });
        }
        if out.is_empty() {
            bail!("--follow is required: choose the hop kinds ({TOKENS}); there is no default (ADR-0047)");
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
}

pub struct Options {
    pub follow: Vec<Follow>,
    /// Skip literals with more occurrences than this (R4-B follow constraint).
    pub max_occurrences: Option<usize>,
    /// Keep only routes of at most this many hops (R4-B follow constraint). Needs a target.
    pub max_depth: Option<usize>,
    /// The repo snapshots to search, as SQLite schema names (empty: `main` only).
    pub schemas: Vec<String>,
}

/// Node and edge ids across schemas: `schema index << 40 | rowid`.
const SHIFT: u32 = 40;
fn gid(schema: usize, id: i64) -> i64 {
    ((schema as i64) << SHIFT) | id
}
fn split(g: i64) -> (usize, i64) {
    ((g >> SHIFT) as usize, g & ((1i64 << SHIFT) - 1))
}
fn schema_list(opts: &Options) -> Vec<String> {
    if opts.schemas.is_empty() { vec!["main".to_string()] } else { opts.schemas.clone() }
}

/// One hop: possibly several edges (call sites) between the same two nodes.
#[derive(Debug, Clone)]
pub struct Hop {
    pub from: i64,
    pub to: i64,
    /// `call`, `call (possible)`, `handoff`, `literal`, `names`.
    pub label: String,
    pub eids: Vec<i64>,
}

#[derive(Debug)]
pub struct PathResult {
    pub starts: Vec<i64>,
    pub targets: Vec<i64>,
    /// nid -> path, for every node on the result.
    pub nodes: BTreeMap<i64, String>,
    pub hops: Vec<Hop>,
    /// Strongly connected groups (size > 1) and self-loops among the result's nodes.
    pub cycles: Vec<Vec<i64>>,
    /// Breadth-first distance from the starts, for display order.
    pub depth: HashMap<i64, usize>,
    /// The schemas searched (for receipts).
    pub schemas: Vec<String>,
}

impl PathResult {
    /// True if a hop with this label goes from a node whose path ends with `from` to one ending with `to`.
    pub fn has_hop(&self, from: &str, to: &str, label: &str) -> bool {
        self.hops.iter().any(|h| h.label == label && ends(&self.nodes[&h.from], from) && ends(&self.nodes[&h.to], to))
    }
}

fn ends(path: &str, suffix: &str) -> bool {
    path == suffix || path.ends_with(&format!(" › {suffix}")) || path.ends_with(&format!("/{suffix}")) || path.ends_with(&format!(":{suffix}"))
}

/// Resolves a node by exact path or a unique suffix starting at a ` › ` or `/` boundary (BQ-10).
pub fn resolve(conn: &Connection, spec: &str) -> Result<i64> {
    resolve_in(conn, &["main".to_string()], spec)
}

/// Like [`resolve`], across schemas (returns a global id). `schema:` before the spec
/// narrows it to one repo's snapshot.
pub fn resolve_in(conn: &Connection, schemas: &[String], spec: &str) -> Result<i64> {
    let (only, spec) = match spec.split_once(':') {
        Some((s, rest)) if schemas.len() > 1 && schemas.iter().any(|x| x == s) => (Some(s), rest),
        _ => (None, spec),
    };
    let esc = spec.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let mut rows: Vec<(i64, String)> = Vec::new();
    for (i, s) in schemas.iter().enumerate().filter(|(_, s)| only.is_none_or(|o| o == s.as_str())) {
        let mut stmt = conn.prepare(&format!("SELECT nid, path FROM {s}.nodes WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\' OR path LIKE ?3 ESCAPE '\\' OR path LIKE ?4 ESCAPE '\\'"))?;
        let found: Vec<(i64, String)> = stmt
            .query_map(params![spec, format!("% › {esc}"), format!("ext:%:{esc}"), format!("%/{esc}")], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        rows.extend(found.into_iter().map(|(n, p)| (gid(i, n), if schemas.len() > 1 { format!("{s}:{p}") } else { p })));
    }
    match rows.len() {
        1 => Ok(rows[0].0),
        0 => bail!("no node matches `{spec}`"),
        _ => {
            let list: Vec<String> = rows.iter().take(20).map(|(_, p)| format!("  {p}")).collect();
            bail!("`{spec}` is ambiguous ({} matches); use a longer suffix:\n{}", rows.len(), list.join("\n"))
        }
    }
}

struct Graph {
    /// from -> [(to, label, eid)]
    out: HashMap<i64, Vec<(i64, &'static str, i64)>>,
    rev: HashMap<i64, Vec<(i64, &'static str, i64)>>,
}

fn load(conn: &Connection, opts: &Options) -> Result<Graph> {
    let mut g = Graph { out: HashMap::new(), rev: HashMap::new() };
    let schemas = schema_list(opts);
    // One literal node per value across repos: every schema's `lit:…` maps to the first one's.
    let mut lit_canon: HashMap<i64, i64> = HashMap::new();
    if schemas.len() > 1 {
        let mut first: HashMap<String, i64> = HashMap::new();
        for (i, s) in schemas.iter().enumerate() {
            let mut stmt = conn.prepare(&format!("SELECT nid, path FROM {s}.nodes WHERE kind = 'literal'"))?;
            let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
            for (nid, p) in rows {
                let gl = gid(i, nid);
                let c = *first.entry(p).or_insert(gl);
                lit_canon.insert(gl, c);
            }
        }
    }
    let canon = |n: i64| lit_canon.get(&n).copied().unwrap_or(n);
    let mut add = |from: i64, to: i64, label: &'static str, eid: i64| {
        g.out.entry(from).or_default().push((to, label, eid));
        g.rev.entry(to).or_default().push((from, label, eid));
    };
    let f = |x| opts.follow.contains(&x);
    // Every schema's rows of one query (written for `edges`), as global ids.
    let edges = |sql: &str| -> Result<Vec<(i64, i64, i64, String)>> {
        let mut all = Vec::new();
        for (i, s) in schemas.iter().enumerate() {
            let mut stmt = conn.prepare(&sql.replace("FROM edges", &format!("FROM {s}.edges")))?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, String>(3)?)))?;
            for row in rows {
                let (eid, a, b, x) = row?;
                all.push((gid(i, eid), canon(gid(i, a)), canon(gid(i, b)), x));
            }
        }
        Ok(all)
    };
    if f(Follow::Same) && schemas.len() > 1 {
        // Identical files (by hash), then their nodes by (kind, path inside the file): hash lookups.
        let files_of = |s: &str| -> Result<HashMap<String, Vec<String>>> {
            let mut stmt = conn.prepare(&format!("SELECT sha256, path FROM {s}.files"))?;
            let mut m: HashMap<String, Vec<String>> = HashMap::new();
            for r in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
                let (sha, p) = r?;
                m.entry(sha).or_default().push(p);
            }
            Ok(m)
        };
        let nodes_in = |s: &str, file: &str| -> Result<HashMap<(String, String), i64>> {
            let mut stmt = conn.prepare(&format!("SELECT nid, kind, path FROM {s}.nodes WHERE file = ?1"))?;
            let rows = stmt.query_map([file], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)))?;
            let mut m = HashMap::new();
            for r in rows {
                let (nid, kind, p) = r?;
                m.insert((kind, p.get(file.len()..).unwrap_or("").to_string()), nid);
            }
            Ok(m)
        };
        let all_files: Vec<HashMap<String, Vec<String>>> = schemas.iter().map(|s| files_of(s)).collect::<Result<_>>()?;
        for i in 0..schemas.len() {
            for j in i + 1..schemas.len() {
                for (sha, fa_list) in &all_files[i] {
                    let Some(fb_list) = all_files[j].get(sha) else { continue };
                    for fa in fa_list {
                        let na = nodes_in(&schemas[i], fa)?;
                        for fb in fb_list {
                            for (k, y) in nodes_in(&schemas[j], fb)? {
                                if let Some(x) = na.get(&k) {
                                    add(gid(i, *x), gid(j, y), "same code", -1);
                                    add(gid(j, y), gid(i, *x), "same code", -1);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if f(Follow::Call) {
        for (eid, from, to, res) in edges("SELECT eid, from_nid, to_nid, resolution FROM edges WHERE kind = 'call' AND to_nid IS NOT NULL")? {
            add(from, to, if res == "possible" { "call (possible)" } else { "call" }, eid);
        }
    }
    if f(Follow::Handoff) {
        for (eid, from, to, _) in edges("SELECT eid, from_nid, to_nid, '' FROM edges WHERE kind = 'value-flow' AND subkind = 'handoff'")? {
            add(from, to, "handoff", eid);
        }
    }
    if f(Follow::Data) {
        for (eid, from, to, sub) in edges("SELECT eid, from_nid, to_nid, coalesce(subkind, '') || '/' || resolution FROM edges WHERE to_nid IS NOT NULL AND ((kind = 'value-flow' AND subkind IN ('assign', 'result', 'return')) OR kind = 'binds')")? {
            let label = match sub.as_str() {
                "assign/exact" => "assign",
                "result/exact" => "result",
                "return/exact" => "return",
                "ref/exact" => "binds (ref)",
                "pointer/exact" => "binds (pointer)",
                "value/exact" => "binds",
                "ref/possible" => "binds (ref, possible)",
                "pointer/possible" => "binds (pointer, possible)",
                _ => "binds (possible)",
            };
            add(from, to, label, eid);
        }
    }
    if f(Follow::Write) {
        for (eid, from, to, _) in edges("SELECT eid, from_nid, to_nid, '' FROM edges WHERE kind = 'write' AND to_nid IS NOT NULL")? {
            add(from, to, "write", eid);
        }
    }
    if f(Follow::Access) {
        for (eid, from, to, _) in edges("SELECT eid, from_nid, to_nid, '' FROM edges WHERE kind = 'access' AND to_nid IS NOT NULL")? {
            add(from, to, "access", eid);
        }
    }
    if f(Follow::Read) {
        for (eid, from, to, _) in edges("SELECT eid, from_nid, to_nid, '' FROM edges WHERE kind = 'read' AND to_nid IS NOT NULL")? {
            add(to, from, "read by", eid);
        }
    }
    if f(Follow::Literal) || f(Follow::Names) {
        let occ = edges("SELECT eid, from_nid, to_nid, '' FROM edges WHERE kind = 'link' AND subkind = 'literal'")?;
        let mut count: HashMap<i64, usize> = HashMap::new();
        for (_, _, lit, _) in &occ {
            *count.entry(*lit).or_default() += 1;
        }
        let allowed = |lit: &i64| opts.max_occurrences.is_none_or(|m| count[lit] <= m);
        for (eid, from, lit, _) in &occ {
            if !allowed(lit) {
                continue;
            }
            add(*from, *lit, "contains literal", *eid);
            if f(Follow::Literal) {
                add(*lit, *from, "literal", *eid);
            }
        }
        if f(Follow::Names) {
            for (eid, lit, decl, _) in edges("SELECT eid, from_nid, to_nid, '' FROM edges WHERE kind = 'link' AND subkind = 'names'")? {
                if allowed(&lit) {
                    add(lit, decl, "names", eid);
                }
            }
            // Across repos: a string in one repo equal to a declared name in another.
            if schemas.len() > 1 {
                // String literals by value, and declared names (ordinal `#k` stripped), per schema.
                let mut strings: Vec<HashMap<String, i64>> = Vec::new();
                let mut decls: Vec<Vec<(String, i64)>> = Vec::new();
                for (i, s) in schemas.iter().enumerate() {
                    let mut stmt = conn.prepare(&format!("SELECT nid, name FROM {s}.nodes WHERE kind = 'literal' AND path LIKE 'lit:\"%'"))?;
                    let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
                    strings.push(rows.into_iter().map(|(n, v)| (v, canon(gid(i, n)))).collect());
                    let mut stmt = conn.prepare(&format!("SELECT nid, name FROM {s}.nodes WHERE kind = 'container' AND subkind IN ('function', 'type', 'module') AND anonymous = 0"))?;
                    let rows: Vec<(i64, String)> = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
                    decls.push(rows.into_iter().map(|(n, name)| (name.split('#').next().unwrap_or(&name).to_string(), gid(i, n))).collect());
                }
                // Across repos: a string in one repo equal to a declared name in another.
                for (i, lits) in strings.iter().enumerate() {
                    for (_, ds) in decls.iter().enumerate().filter(|(j, _)| *j != i) {
                        for (name, d) in ds {
                            if let Some(lit) = lits.get(name)
                                && count.contains_key(lit) && allowed(lit) {
                                    add(*lit, *d, "names", -1);
                                }
                        }
                    }
                }
            }
        }
    }
    Ok(g)
}

/// Breadth-first distances from `seeds` over `adj`, never continuing through a node in `stop`.
fn bfs(adj: &HashMap<i64, Vec<(i64, &'static str, i64)>>, seeds: &[i64], stop: &BTreeSet<i64>) -> HashMap<i64, usize> {
    let mut dist: HashMap<i64, usize> = HashMap::new();
    let mut q: VecDeque<i64> = VecDeque::new();
    for s in seeds {
        dist.insert(*s, 0);
        q.push_back(*s);
    }
    while let Some(n) = q.pop_front() {
        let d = dist[&n];
        if stop.contains(&n) {
            continue;
        }
        for (m, _, _) in adj.get(&n).into_iter().flatten() {
            if !dist.contains_key(m) {
                dist.insert(*m, d + 1);
                q.push_back(*m);
            }
        }
    }
    dist
}

/// Reachability between two ends (R2-12, R4-B). Either end may be open:
/// - `from` and `to`: every hop on some route from a start to a target;
/// - only `from`: everything reachable forward (targets are terminal);
/// - only `to`: everything that reaches a target (reverse reachability).
///
/// Ends are sets: several starts or several targets.
pub fn path(conn: &Connection, from: &[String], to: &[String], opts: &Options) -> Result<PathResult> {
    if from.is_empty() && to.is_empty() {
        bail!("give a start (`from`), a target (`to`), or both");
    }
    let schemas = schema_list(opts);
    let mut starts = Vec::new();
    for f in from {
        let s = resolve_in(conn, &schemas, f)?;
        starts.push(s);
        let (i, nid) = split(s);
        let mut stmt = conn.prepare(&format!("SELECT to_nid FROM {}.edges WHERE from_nid = ?1 AND kind = 'value-flow' AND subkind = 'returns'", schemas[i]))?;
        for r in stmt.query_map([nid], |r| r.get::<_, i64>(0))? {
            starts.push(gid(i, r?));
        }
    }
    starts.sort();
    starts.dedup();
    let mut targets: Vec<i64> = to.iter().map(|t| resolve_in(conn, &schemas, t)).collect::<Result<_>>()?;
    targets.sort();
    targets.dedup();
    let g = load(conn, opts)?;

    let (fwd, bwd) = (!starts.is_empty(), !targets.is_empty());
    let is_target: BTreeSet<i64> = targets.iter().copied().collect();
    // Forward from the starts (a route ends at its target, never continues through it); backward from the targets.
    // An external leaf is one node for every call site of a symbol: a route may start or
    // end there, but never passes through it (that would join unrelated call sites).
    let mut externals: BTreeSet<i64> = BTreeSet::new();
    for (i, s) in schemas.iter().enumerate() {
        let mut stmt = conn.prepare(&format!("SELECT nid FROM {s}.nodes WHERE kind = 'external'"))?;
        for r in stmt.query_map([], |r| r.get::<_, i64>(0))? {
            externals.insert(gid(i, r?));
        }
    }
    // Hops out of an external leaf only where a route can begin there: it is a start, or
    // (reverse reachability) the open end the search reached.
    let pass_through = |n: &i64| !externals.contains(n) || starts.contains(n) || !fwd;
    let fwd_stop: BTreeSet<i64> = is_target.iter().copied().chain(externals.iter().copied().filter(|n| !starts.contains(n))).collect();
    let bwd_stop: BTreeSet<i64> = externals.iter().copied().filter(|n| !is_target.contains(n)).collect();
    let depth = if fwd { bfs(&g.out, &starts, &fwd_stop) } else { HashMap::new() };
    let to_target = if bwd { bfs(&g.rev, &targets, &bwd_stop) } else { HashMap::new() };
    let limit = opts.max_depth.unwrap_or(usize::MAX);
    let route_len = |n: &i64| -> Option<usize> {
        match (fwd, bwd) {
            (true, true) => Some(depth.get(n)?.saturating_add(*to_target.get(n)?)),
            (true, false) => depth.get(n).copied(),
            _ => to_target.get(n).copied(),
        }
    };
    let reached: Vec<i64> = if fwd { depth.keys().copied().collect() } else { to_target.keys().copied().collect() };
    let keep: BTreeSet<i64> = reached.into_iter().filter(|n| route_len(n).is_some_and(|l| l <= limit)).collect();

    let mut hops: BTreeMap<(i64, i64, &str), Vec<i64>> = BTreeMap::new();
    for n in keep.iter().filter(|n| !is_target.contains(n) && pass_through(n)) {
        for (m, label, eid) in g.out.get(n).into_iter().flatten() {
            if !keep.contains(m) {
                continue;
            }
            let within = match (fwd, bwd) {
                (true, true) => depth[n].saturating_add(1).saturating_add(to_target[m]) <= limit,
                (true, false) => depth[n] < limit,
                _ => to_target[m] < limit,
            };
            // Progress (both ends): a hop moves further from the start or closer to a target.
            let progress = !(fwd && bwd)
                || is_target.contains(m)
                || depth.get(m).is_some_and(|dm| *dm > depth[n])
                || to_target.get(m).is_some_and(|tm| tm < &to_target[n]);
            if within && progress {
                hops.entry((*n, *m, label)).or_default().push(*eid);
            }
        }
    }
    let mut hops: Vec<Hop> = hops.into_iter().map(|((from, to, label), eids)| Hop { from, to, label: label.to_string(), eids }).collect();
    prune(&mut hops, &starts, &targets);
    let keep: BTreeSet<i64> = hops.iter().flat_map(|h| [h.from, h.to]).chain(starts.iter().copied()).chain(targets.iter().copied()).collect();

    let mut nodes = BTreeMap::new();
    for n in &keep {
        let (i, nid) = split(*n);
        let p: String = conn.query_row(&format!("SELECT path FROM {}.nodes WHERE nid = ?1", schemas[i]), [nid], |r| r.get(0))?;
        nodes.insert(*n, if schemas.len() > 1 { format!("{}:{p}", schemas[i]) } else { p });
    }
    let cycles = cycles(&keep, &hops);
    // Display order: distance from the starts, or (reverse only) from the open end toward the targets.
    let depth = if fwd {
        depth
    } else {
        let max = keep.iter().filter_map(|n| to_target.get(n)).max().copied().unwrap_or(0);
        keep.iter().filter_map(|n| to_target.get(n).map(|d| (*n, max - d))).collect()
    };
    Ok(PathResult { starts, targets, nodes, hops, cycles, depth, schemas })
}

/// Removes hops that are not on any real route, to a fixpoint:
/// - a literal connects two *different* functions: `F → lit → F` is not a hop;
/// - with starts, a node other than a start needs a way in; with targets, a node other than a target needs a way out.
fn prune(hops: &mut Vec<Hop>, starts: &[i64], targets: &[i64]) {
    loop {
        let before = hops.len();
        // Literal nodes: predecessors via `contains literal`, successors via `literal`/`names`.
        let mut preds: HashMap<i64, BTreeSet<i64>> = HashMap::new();
        let mut succs: HashMap<i64, BTreeSet<i64>> = HashMap::new();
        let mut names_out: BTreeSet<i64> = BTreeSet::new();
        for h in hops.iter() {
            match h.label.as_str() {
                "contains literal" => {
                    preds.entry(h.to).or_default().insert(h.from);
                }
                "literal" => {
                    succs.entry(h.from).or_default().insert(h.to);
                }
                "names" => {
                    names_out.insert(h.from);
                }
                _ => {}
            }
        }
        hops.retain(|h| match h.label.as_str() {
            "contains literal" => names_out.contains(&h.to) || succs.get(&h.to).is_some_and(|s| s.iter().any(|g| *g != h.from)),
            "literal" => preds.get(&h.from).is_some_and(|p| p.iter().any(|f| *f != h.to)),
            _ => true,
        });
        let has_in: BTreeSet<i64> = hops.iter().map(|h| h.to).collect();
        let has_out: BTreeSet<i64> = hops.iter().map(|h| h.from).collect();
        hops.retain(|h| {
            (starts.is_empty() || starts.contains(&h.from) || has_in.contains(&h.from))
                && (targets.is_empty() || targets.contains(&h.to) || has_out.contains(&h.to))
        });
        if hops.len() == before {
            return;
        }
    }
}

/// Tarjan's strongly connected components over the result; self-loops count as cycles.
fn cycles(keep: &BTreeSet<i64>, hops: &[Hop]) -> Vec<Vec<i64>> {
    let mut adj: HashMap<i64, Vec<i64>> = HashMap::new();
    for h in hops {
        adj.entry(h.from).or_default().push(h.to);
    }
    struct St {
        index: HashMap<i64, usize>,
        low: HashMap<i64, usize>,
        on: BTreeSet<i64>,
        stack: Vec<i64>,
        next: usize,
        out: Vec<Vec<i64>>,
    }
    fn strong(v: i64, adj: &HashMap<i64, Vec<i64>>, s: &mut St) {
        s.index.insert(v, s.next);
        s.low.insert(v, s.next);
        s.next += 1;
        s.stack.push(v);
        s.on.insert(v);
        for w in adj.get(&v).into_iter().flatten() {
            if !s.index.contains_key(w) {
                strong(*w, adj, s);
                let lw = s.low[w];
                let lv = s.low.get_mut(&v).expect("set");
                *lv = (*lv).min(lw);
            } else if s.on.contains(w) {
                let iw = s.index[w];
                let lv = s.low.get_mut(&v).expect("set");
                *lv = (*lv).min(iw);
            }
        }
        if s.low[&v] == s.index[&v] {
            let mut comp = Vec::new();
            while let Some(w) = s.stack.pop() {
                s.on.remove(&w);
                comp.push(w);
                if w == v {
                    break;
                }
            }
            let self_loop = comp.len() == 1 && adj.get(&v).is_some_and(|a| a.contains(&v));
            if comp.len() > 1 || self_loop {
                comp.sort();
                s.out.push(comp);
            }
        }
    }
    let mut s = St { index: HashMap::new(), low: HashMap::new(), on: BTreeSet::new(), stack: Vec::new(), next: 0, out: Vec::new() };
    for v in keep {
        if !s.index.contains_key(v) {
            strong(*v, &adj, &mut s);
        }
    }
    s.out
}

/// Renders a result as text: nodes in breadth-first order, each with its outgoing hops and receipts.
/// With `receipts` false, hops are listed without their source lines.
pub fn render(conn: &Connection, r: &PathResult, receipts: bool) -> Result<String> {
    let mut out = String::new();
    let name = |n: &i64| r.nodes.get(n).cloned().unwrap_or_default();
    let open = |v: &[i64]| if v.is_empty() { "(open)".to_string() } else { v.iter().map(name).collect::<Vec<_>>().join(", ") };
    out.push_str(&format!("start: {}\n", open(&r.starts)));
    out.push_str(&format!("to:    {}\n", open(&r.targets)));
    out.push_str(&format!("{} nodes, {} hops\n", r.nodes.len(), r.hops.len()));
    if r.hops.is_empty() {
        out.push_str("no path\n");
        return Ok(out);
    }
    let mut order: Vec<i64> = r.nodes.keys().copied().collect();
    order.sort_by_key(|n| (r.depth.get(n).copied().unwrap_or(usize::MAX), name(n)));
    let mut by_from: BTreeMap<i64, Vec<&Hop>> = BTreeMap::new();
    for h in &r.hops {
        by_from.entry(h.from).or_default().push(h);
    }
    let mut files: HashMap<(usize, String), Vec<u8>> = HashMap::new();
    for n in order {
        out.push_str(&format!("\n{}\n", name(&n)));
        for h in by_from.get(&n).into_iter().flatten() {
            out.push_str(&format!("  --{}--> {}\n", h.label, name(&h.to)));
            if !receipts {
                continue;
            }
            for eid in h.eids.iter().filter(|e| **e >= 0) {
                let (i, eid) = split(*eid);
                let schema = &r.schemas[i];
                let mut rstmt = conn.prepare(&format!("SELECT role, file, byte_start, byte_end, line, col FROM {schema}.receipts WHERE eid = ?1 ORDER BY rowid"))?;
                let rows: Vec<(String, String, i64, i64, i64, i64)> =
                    rstmt.query_map([eid], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?, x.get(4)?, x.get(5)?)))?.collect::<Result<_, _>>()?;
                for (role, file, s, e, line, col) in rows {
                    let key = (i, file.clone());
                    if !files.contains_key(&key) {
                        let content: Vec<u8> = conn.query_row(&format!("SELECT content FROM {schema}.files WHERE path = ?1"), [&file], |x| x.get(0)).unwrap_or_default();
                        files.insert(key.clone(), content);
                    }
                    let bytes = &files[&key];
                    let file = if r.schemas.len() > 1 { format!("{schema}:{file}") } else { file };
                    let text = String::from_utf8_lossy(bytes.get(s as usize..e as usize).unwrap_or_default());
                    let mut snippet: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    if snippet.chars().count() > 90 {
                        snippet = snippet.chars().take(90).collect::<String>() + "…";
                    }
                    out.push_str(&format!("      {role:7} {file}:{line}:{col}  {snippet}\n"));
                }
            }
        }
    }
    // Links found by exact equality (a shared literal, a string equal to a name, identical
    // source in another repo) are shortcuts, not compiler facts: listed for review.
    let review: Vec<&Hop> = r.hops.iter().filter(|h| matches!(h.label.as_str(), "literal" | "names" | "same code")).collect();
    if !review.is_empty() {
        out.push_str(&format!("\nto review ({}): equality links, not compiler facts; confirm each by reading both ends:\n", review.len()));
        for h in review {
            out.push_str(&format!("  {} --{}--> {}\n", name(&h.from), h.label, name(&h.to)));
        }
    }
    if !r.cycles.is_empty() {
        out.push_str("\ncycles (reported, not followed):\n");
        for c in &r.cycles {
            if c.len() == 1 {
                out.push_str(&format!("  recursive: {} reaches itself\n", name(&c[0])));
            } else {
                out.push_str(&format!("  cycle among: {}\n", c.iter().map(name).collect::<Vec<_>>().join(" | ")));
            }
        }
    }
    Ok(out)
}
