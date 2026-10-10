//! `path`: algorithmic reachability with receipts (R2-12, ADR-0047, R4-B).
//!
//! The caller always chooses which hops count (`follow`); there is no default set.
//! Follow tokens are exact predicates over the `edges` table (SCHEMA.md §5):
//! - `call`: `kind='call'` with a target (exact and possible; possible is labeled);
//! - `handoff`: `kind='value-flow' AND subkind='handoff'` (a function value passed to external code);
//! - `literal`: function → literal node (`link/literal`) → every other function containing it;
//! - `names`: function → literal node → the declaration it names (`link/names`).
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
}

impl Follow {
    pub fn parse_list(s: &str) -> Result<Vec<Follow>> {
        let mut out = Vec::new();
        for t in s.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            out.push(match t {
                "call" => Follow::Call,
                "handoff" => Follow::Handoff,
                "literal" => Follow::Literal,
                "names" => Follow::Names,
                other => bail!("unknown follow token `{other}`; valid: call, handoff, literal, names"),
            });
        }
        if out.is_empty() {
            bail!("--follow is required: choose the hop kinds (call, handoff, literal, names); there is no default (ADR-0047)");
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
    let mut stmt = conn.prepare("SELECT nid, path FROM nodes WHERE path = ?1 OR path LIKE ?2 ESCAPE '\\' OR path LIKE ?3 ESCAPE '\\' OR path LIKE ?4 ESCAPE '\\'")?;
    let esc = spec.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    let rows: Vec<(i64, String)> = stmt
        .query_map(params![spec, format!("% › {esc}"), format!("ext:%:{esc}"), format!("%/{esc}")], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
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
    let mut add = |from: i64, to: i64, label: &'static str, eid: i64| {
        g.out.entry(from).or_default().push((to, label, eid));
        g.rev.entry(to).or_default().push((from, label, eid));
    };
    let f = |x| opts.follow.contains(&x);
    let edges = |sql: &str| -> Result<Vec<(i64, i64, i64, String)>> {
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    };
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
    let mut starts = Vec::new();
    let mut stmt = conn.prepare("SELECT to_nid FROM edges WHERE from_nid = ?1 AND kind = 'value-flow' AND subkind = 'returns'")?;
    for f in from {
        let s = resolve(conn, f)?;
        starts.push(s);
        for r in stmt.query_map([s], |r| r.get::<_, i64>(0))? {
            starts.push(r?);
        }
    }
    starts.sort();
    starts.dedup();
    let mut targets: Vec<i64> = to.iter().map(|t| resolve(conn, t)).collect::<Result<_>>()?;
    targets.sort();
    targets.dedup();
    let g = load(conn, opts)?;

    let (fwd, bwd) = (!starts.is_empty(), !targets.is_empty());
    let is_target: BTreeSet<i64> = targets.iter().copied().collect();
    // Forward from the starts (a route ends at its target, never continues through it); backward from the targets.
    let depth = if fwd { bfs(&g.out, &starts, &is_target) } else { HashMap::new() };
    let to_target = if bwd { bfs(&g.rev, &targets, &BTreeSet::new()) } else { HashMap::new() };
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
    for n in keep.iter().filter(|n| !is_target.contains(n)) {
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
    let mut stmt = conn.prepare("SELECT path FROM nodes WHERE nid = ?1")?;
    for n in &keep {
        nodes.insert(*n, stmt.query_row([n], |r| r.get::<_, String>(0))?);
    }
    let cycles = cycles(&keep, &hops);
    // Display order: distance from the starts, or (reverse only) from the open end toward the targets.
    let depth = if fwd {
        depth
    } else {
        let max = keep.iter().filter_map(|n| to_target.get(n)).max().copied().unwrap_or(0);
        keep.iter().filter_map(|n| to_target.get(n).map(|d| (*n, max - d))).collect()
    };
    Ok(PathResult { starts, targets, nodes, hops, cycles, depth })
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
    let mut files: HashMap<String, Vec<u8>> = HashMap::new();
    let mut rstmt = conn.prepare("SELECT role, file, byte_start, byte_end, line, col FROM receipts WHERE eid = ?1 ORDER BY rowid")?;
    for n in order {
        out.push_str(&format!("\n{}\n", name(&n)));
        for h in by_from.get(&n).into_iter().flatten() {
            out.push_str(&format!("  --{}--> {}\n", h.label, name(&h.to)));
            if !receipts {
                continue;
            }
            for eid in &h.eids {
                let rows: Vec<(String, String, i64, i64, i64, i64)> =
                    rstmt.query_map([eid], |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?, x.get(3)?, x.get(4)?, x.get(5)?)))?.collect::<Result<_, _>>()?;
                for (role, file, s, e, line, col) in rows {
                    if !files.contains_key(&file) {
                        let content: Vec<u8> = conn.query_row("SELECT content FROM files WHERE path = ?1", [&file], |x| x.get(0)).unwrap_or_default();
                        files.insert(file.clone(), content);
                    }
                    let bytes = &files[&file];
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
