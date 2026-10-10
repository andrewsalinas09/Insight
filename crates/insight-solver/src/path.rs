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

pub fn path(conn: &Connection, from: &str, to: &[String], opts: &Options) -> Result<PathResult> {
    let start = resolve(conn, from)?;
    let mut starts = vec![start];
    let mut stmt = conn.prepare("SELECT to_nid FROM edges WHERE from_nid = ?1 AND kind = 'value-flow' AND subkind = 'returns'")?;
    for r in stmt.query_map([start], |r| r.get::<_, i64>(0))? {
        starts.push(r?);
    }
    let targets: Vec<i64> = to.iter().map(|t| resolve(conn, t)).collect::<Result<_>>()?;
    let g = load(conn, opts)?;

    // Forward from the starts (with distances), backward from the targets.
    let mut depth: HashMap<i64, usize> = HashMap::new();
    let mut q: VecDeque<i64> = VecDeque::new();
    for s in &starts {
        depth.insert(*s, 0);
        q.push_back(*s);
    }
    let is_target: BTreeSet<i64> = targets.iter().copied().collect();
    while let Some(n) = q.pop_front() {
        let d = depth[&n];
        // A route ends at its target; it never continues through it.
        if is_target.contains(&n) {
            continue;
        }
        for (m, _, _) in g.out.get(&n).into_iter().flatten() {
            if !depth.contains_key(m) {
                depth.insert(*m, d + 1);
                q.push_back(*m);
            }
        }
    }
    if opts.max_depth.is_some() && targets.is_empty() {
        bail!("--max-depth needs --to (it bounds routes between two ends)");
    }
    // Distance to the nearest target, walking edges backward.
    let mut to_target: HashMap<i64, usize> = HashMap::new();
    let mut q: VecDeque<i64> = VecDeque::new();
    for t in &targets {
        to_target.insert(*t, 0);
        q.push_back(*t);
    }
    while let Some(n) = q.pop_front() {
        let d = to_target[&n];
        for (m, _, _) in g.rev.get(&n).into_iter().flatten() {
            if !to_target.contains_key(m) {
                to_target.insert(*m, d + 1);
                q.push_back(*m);
            }
        }
    }
    let limit = opts.max_depth.unwrap_or(usize::MAX);
    let on_route = |n: &i64| targets.is_empty() || to_target.get(n).is_some_and(|b| depth.get(n).is_some_and(|f| f.saturating_add(*b) <= limit));
    let keep: BTreeSet<i64> = depth.keys().copied().filter(|n| on_route(n)).collect();

    let mut hops: BTreeMap<(i64, i64, &str), Vec<i64>> = BTreeMap::new();
    for n in keep.iter().filter(|n| !is_target.contains(n)) {
        for (m, label, eid) in g.out.get(n).into_iter().flatten() {
            let within = targets.is_empty() || depth[n].saturating_add(1).saturating_add(to_target.get(m).copied().unwrap_or(usize::MAX)) <= limit;
            // Progress: a hop moves further from the start or closer to a target.
            let progress = targets.is_empty()
                || is_target.contains(m)
                || depth.get(m).is_some_and(|dm| *dm > depth[n])
                || to_target.get(m).is_some_and(|tm| to_target.get(n).is_some_and(|tn| tm < tn));
            if keep.contains(m) && within && progress {
                hops.entry((*n, *m, label)).or_default().push(*eid);
            }
        }
    }
    let mut hops: Vec<Hop> = hops.into_iter().map(|((from, to, label), eids)| Hop { from, to, label: label.to_string(), eids }).collect();
    prune(&mut hops, &starts, &targets);
    let keep: BTreeSet<i64> = if targets.is_empty() {
        keep
    } else {
        hops.iter().flat_map(|h| [h.from, h.to]).chain(starts.iter().copied()).chain(targets.iter().copied()).collect()
    };

    let mut nodes = BTreeMap::new();
    let mut stmt = conn.prepare("SELECT path FROM nodes WHERE nid = ?1")?;
    for n in keep.iter().chain(&starts).chain(&targets) {
        nodes.insert(*n, stmt.query_row([n], |r| r.get::<_, String>(0))?);
    }
    let cycles = cycles(&keep, &hops);
    Ok(PathResult { starts, targets, nodes, hops, cycles, depth })
}

/// Removes hops that are not on any real route, to a fixpoint:
/// - a literal connects two *different* functions: `F → lit → F` is not a hop;
/// - (with targets) a node other than a start or target needs a way in and a way out.
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
        if !targets.is_empty() {
            let has_in: BTreeSet<i64> = hops.iter().map(|h| h.to).collect();
            let has_out: BTreeSet<i64> = hops.iter().map(|h| h.from).collect();
            let fixed = |n: &i64| starts.contains(n) || targets.contains(n);
            hops.retain(|h| (fixed(&h.from) || has_in.contains(&h.from)) && (fixed(&h.to) || has_out.contains(&h.to)));
        }
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
pub fn render(conn: &Connection, r: &PathResult) -> Result<String> {
    let mut out = String::new();
    let name = |n: &i64| r.nodes.get(n).cloned().unwrap_or_default();
    out.push_str(&format!("start: {}\n", r.starts.iter().map(name).collect::<Vec<_>>().join(", ")));
    if !r.targets.is_empty() {
        out.push_str(&format!("to:    {}\n", r.targets.iter().map(name).collect::<Vec<_>>().join(", ")));
    }
    out.push_str(&format!("{} nodes, {} hops\n", r.nodes.len(), r.hops.len()));
    if r.nodes.is_empty() {
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
