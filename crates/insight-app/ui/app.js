// The Insight window: the loaded snapshot as nested boxes laid out left to right
// (ELK layered), drawn on a canvas. Boxes open and close (double-click, zoom or
// focus), the layout reshapes with an animation, and the box you acted on stays put.
// Claude's views (`show`) arrive as events: boxes to open, highlights, notes, steps,
// and layout: chains (a route laid out in one row), an order within a box, and ELK
// options. The user can do the same by hand (R2-09).
"use strict";
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const elk = new ELK();
const CW = 7.4, LH = 15; // world units per code character and line
const $ = (id) => document.getElementById(id);

// ---------- settings (kept in this window only) ----------
const DEFAULTS = {
  openAt: 15, closePast: 20, anim: 600, reshape: true, openBy: "click", gap: 14, col: 36,
  lines: { call: true, possible: true, handoff: true, lib: false, write: true, read: false, access: false },
};
let P = structuredClone(DEFAULTS);
try { const s = JSON.parse(localStorage.getItem("insight-window") ?? "null"); if (s) P = { ...P, ...s, lines: { ...P.lines, ...(s.lines ?? {}) } }; } catch {}
const save = () => { try { localStorage.setItem("insight-window", JSON.stringify(P)); } catch {} };

// ---------- the graph ----------
const KIND = { folder: "folder", file: "file", module: "module", type: "type", function: "function", block: "block", field: "field", static: "static", libroot: "libraries", lib: "library", libtype: "library type", ext: "library function" };
const ORDER = { folder: 0, file: 1, module: 1, field: 2, static: 2, type: 3, function: 4, block: 5, libroot: 9 };
const LIBK = new Set(["libroot", "lib", "libtype", "ext"]);
const STATEK = new Set(["field", "static"]);
const LINE = {
  call: { label: "calls", note: "your code calling your code" },
  possible: { label: "possible calls", note: "one of several targets (dynamic dispatch)" },
  handoff: { label: "handoffs", note: "a function passed on, to run later" },
  lib: { label: "calls into libraries", note: "" },
  write: { label: "writes", note: "function → field or static" },
  read: { label: "reads", note: "field or static → function" },
  access: { label: "references taken", note: "function takes a reference" },
};
let G = null, root = null, byId = new Map(), E = [], totals = {}, callsOf = new Map(), HOME = new Set();

function setGraph(g) {
  G = g;
  byId = new Map();
  root = { id: 0, k: "root", n: g.project, children: [] };
  byId.set(0, root);
  for (const n of g.nodes) { n.children = []; byId.set(n.id, n); }
  for (const n of g.nodes) { const p = byId.get(n.p ?? 0) ?? root; n.parent = p; p.children.push(n); }
  (function walk(n, path) {
    n.path = path.concat(n); n.depth = path.length;
    n.children.sort((a, b) => (ORDER[a.k] ?? 6) - (ORDER[b.k] ?? 6) || (a.a ?? 0) - (b.a ?? 0) || a.n.localeCompare(b.n));
    for (const c of n.children) walk(c, n.path);
  })(root, []);
  // Lines as drawn. Reads are drawn the way the data moves: from the field to the reader.
  // `src`/`tgt` keep the stored direction (the function doing it first); `sites` are
  // where each occurrence is written: [line, col, end line, end col].
  E = g.edges.map(([f, t, k, c, sites]) => {
    let a = byId.get(f), b = byId.get(t), kind = ["call", "possible", "handoff", "read", "write", "access"][k];
    if ((kind === "call" || kind === "possible") && LIBK.has(b.k)) kind = "lib";
    const src = a, tgt = b;
    if (kind === "read") [a, b] = [b, a];
    return { a, b, kind, c, src, tgt, sites: sites ?? [] };
  });
  totals = {}; for (const e of E) totals[e.kind] = (totals[e.kind] ?? 0) + e.c;
  callsOf = new Map();
  for (const e of E) {
    if (!["call", "possible", "handoff"].includes(e.kind)) continue;
    const a = fnOf(e.a), b = fnOf(e.b);
    if (!a || !b || a === b) continue;
    (callsOf.get(a) ?? callsOf.set(a, { out: new Set(), in: new Set() }).get(a)).out.add(b);
    (callsOf.get(b) ?? callsOf.set(b, { out: new Set(), in: new Set() }).get(b)).in.add(a);
  }
  HOME = new Set([root, ...root.children]);
  open = new Set(HOME); zoomOpened.clear(); userOrder.clear();
  L = { boxes: new Map(), edges: [] };
  selected = hovered = hoverEdge = mapStop = null; V = null; step = -1; lit = new Set(); notesOn = new Map();
  traceReset();
}
const fnOf = (n) => { while (n && n.k !== "function" && n.k !== "ext") n = n.parent; return n; };
const lines = (n) => (n.a ? n.b - n.a + 1 : 0);
const isUnder = (n, anc) => n.path[anc.depth] === anc;
const count = (n, pred) => { let c = 0; (function w(x) { for (const y of x.children) { if (pred(y)) c++; w(y); } })(n); return c; };
const short = (n) => (n.k === "file" ? n.n.split("/").pop() : n.n);
const pathText = (n) => n.path.slice(1).map((x) => x.n).join(" › ");

// Source text, fetched per file when a function's code is first shown.
const files = new Map();
async function ensureFiles(nodes) {
  const want = [...new Set([...nodes].filter((n) => n.k === "function" && n.f && !files.has(n.f)).map((n) => n.f))];
  await Promise.all(want.map(async (f) => {
    try { files.set(f, (await invoke("file", { path: f })).split("\n")); }
    catch (e) { files.set(f, [`(${e})`]); }
  }));
}
const codeOf = (n) => (files.get(n.f) ?? []).slice(n.a - 1, n.b).map((l) => l.replace(/\t/g, "    "));

// ---------- what is open ----------
let open = new Set();
const zoomOpened = new Map(); // box -> camera scale when zooming opened it
const isCode = (n) => n.k === "function";
const shows = (n) => open.has(n) && (isCode(n) || n.children.length > 0);
const expanded = (n) => open.has(n) && !isCode(n) && n.children.length > 0;
function rep(n) { let cur = root; for (let d = 1; d < n.path.length; d++) { if (!expanded(cur)) break; cur = n.path[d]; } return cur; }
function openPathTo(set, n) { for (const a of n.path) if (a !== n) set.add(a); }

// ---------- Claude's view ----------
let V = null;             // the view on screen (resolved ids), or null
let lit = new Set();      // highlighted nodes
let notesOn = new Map();  // node -> [text]
const userOrder = new Map(); // parent -> children in the order the user set (R2-09 parity with layout.order)

function viewNodes(ids) { return (ids ?? []).map((id) => byId.get(id)).filter(Boolean); }
function applyView(v) {
  V = v; step = -1; selected = null; stopPlay();
  if (TR) exitTrace(false);
  lit = new Set(viewNodes(v.highlight));
  notesOn = new Map();
  for (const n of v.notes ?? []) { const x = n.on != null ? byId.get(n.on) : root; if (x) (notesOn.get(x) ?? notesOn.set(x, []).get(x)).push(n.text); }
  $("title").hidden = !v.title; $("title").textContent = v.title ?? ""; $("title").title = v.title ?? "";
  inspect(null);
  if ((v.steps ?? []).length) { goStep(0); if (v.play) startPlay(); return; }
  if (v.trace) { traceFromView(v.trace); bar(); transport(); return; }
  const s = viewOpen();
  const focus = v.focus != null ? byId.get(v.focus) : null;
  const fitTo = focus ? [focus] : [...lit];
  setOpen(s, null, (lay) => fitAll(lay, fitTo));
  bar(); transport();
}
function viewOpen() {
  const s = new Set(HOME);
  if (!V) return s;
  for (const n of viewNodes(V.open)) { openPathTo(s, n); s.add(n); }
  for (const n of [...lit, ...viewNodes(V.focus != null ? [V.focus] : [])]) openPathTo(s, n);
  for (const c of V.layout?.chain ?? []) for (const n of viewNodes(c)) openPathTo(s, n);
  return s;
}
function fitAll(lay, ns) {
  const bs = ns.map((m) => lay.boxes.get(rep(m))).filter(Boolean);
  return bs.length ? fit(union(bs), 0.85) : fit(lay.boxes.get(root), 0.96);
}

// ---------- layout (ELK layered, left to right, nested) ----------
const measure = document.createElement("canvas").getContext("2d");
const monoFont = () => getComputedStyle(document.body).getPropertyValue("--mono");
const prefix = (n) => (n.k === "function" ? "ƒ " : n.k === "type" ? "◆ " : n.k === "static" ? "static " : n.k === "lib" ? "lib " : "");
function closedSize(n) {
  measure.font = "500 12px " + monoFont();
  const w = Math.min(Math.max(measure.measureText(prefix(n) + short(n)).width, 40) + 24, 260);
  if (STATEK.has(n.k)) return { width: w, height: 24 };
  if (n.k === "ext" || n.k === "libtype") return { width: w, height: 28 };
  return { width: Math.max(w, 70), height: n.k === "function" || n.k === "block" ? 34 : 44 };
}
function codeSize(n) {
  const src = codeOf(n);
  const wide = Math.min(100, Math.max(30, ...src.map((l) => l.length)));
  return { width: 20 + wide * CW, height: 34 + src.length * LH };
}
// The order of a box's children: Claude's `layout.order` first, then the user's moves.
function orderedChildren(n) {
  const want = [];
  for (const id of V?.layout?.order ?? []) { const x = byId.get(id); if (x && x.parent === n) want.push(x); }
  for (const x of userOrder.get(n) ?? []) if (!want.includes(x)) want.push(x);
  if (!want.length) return { list: n.children, forced: false };
  return { list: [...want, ...n.children.filter((x) => !want.includes(x))], forced: true };
}
let L = { boxes: new Map(), edges: [] };
let laying = 0;
async function relayout(anchor) {
  if (!G) return;
  const my = ++laying;
  $("busy").hidden = false;
  await ensureFiles(open);
  if (my !== laying) return;
  const nodes = new Map();
  function build(n) {
    const e = { id: String(n.id) };
    nodes.set(n, e);
    if (expanded(n)) {
      const { list, forced } = orderedChildren(n);
      e.children = list.map(build);
      e.layoutOptions = { "elk.padding": "[top=30,left=12,bottom=12,right=12]" };
      if (forced) Object.assign(e.layoutOptions, { "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES", "elk.layered.crossingMinimization.forceNodeModelOrder": "true" });
    } else Object.assign(e, shows(n) ? codeSize(n) : closedSize(n));
    return e;
  }
  const chains = (V?.layout?.chain ?? []).map(viewNodes).filter((c) => c.length > 1);
  const g = {
    id: "0",
    layoutOptions: {
      "elk.algorithm": "layered", "elk.direction": "RIGHT", "elk.hierarchyHandling": "INCLUDE_CHILDREN",
      "elk.edgeRouting": "ORTHOGONAL", "elk.layered.spacing.nodeNodeBetweenLayers": String(P.col), "elk.spacing.nodeNode": String(P.gap),
      "elk.layered.spacing.edgeNodeBetweenLayers": "14", "elk.padding": "[top=20,left=20,bottom=20,right=20]",
      // Chains keep their row best with network-simplex placement.
      ...(chains.length ? { "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX" } : {}),
      ...(V?.layout?.options ?? {}),
    },
  };
  const { list: top, forced: topForced } = orderedChildren(root);
  g.children = top.map(build);
  if (topForced) Object.assign(g.layoutOptions, { "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES", "elk.layered.crossingMinimization.forceNodeModelOrder": "true" });
  nodes.set(root, g);
  const lca = (a, b) => { let x = root; for (let d = 1; d < Math.min(a.path.length, b.path.length) && a.path[d] === b.path[d]; d++) x = a.path[d]; return x; };
  // Lines of the shown kinds, lifted to the deepest boxes on screen, merged per kind.
  const agg = new Map();
  for (const e of E) {
    if (!P.lines[e.kind]) continue;
    const a = rep(e.a), b = rep(e.b);
    if (a === b || isUnder(a, b) || isUnder(b, a)) continue;
    const key = a.id + ">" + b.id + ">" + e.kind;
    const x = agg.get(key) ?? agg.set(key, { a, b, kind: e.kind, c: 0, under: [] }).get(key);
    x.c += e.c; x.under.push(e);
  }
  const meta = new Map();
  let i = 0;
  for (const e of agg.values()) {
    const id = "e" + i++;
    (nodes.get(lca(e.a, e.b)).edges ??= []).push({ id, sources: [String(e.a.id)], targets: [String(e.b.id)] });
    meta.set(id, e);
  }
  // Chains: invisible edges that pull each sequence left to right into one row.
  for (const c of chains) {
    for (let j = 1; j < c.length; j++) {
      const a = rep(c[j - 1]), b = rep(c[j]);
      if (a === b || isUnder(a, b) || isUnder(b, a)) continue;
      const id = "k" + i++;
      (nodes.get(lca(a, b)).edges ??= []).push({ id, sources: [String(a.id)], targets: [String(b.id)], layoutOptions: { "elk.layered.priority.straightness": "20", "elk.layered.priority.direction": "20" } });
    }
  }
  let r;
  try { r = await elk.layout(g); }
  catch (err) { console.error(err); $("busy").hidden = true; report({ layout_error: String(err?.message ?? err) }); return; }
  if (my !== laying) return;
  const boxes = new Map();
  (function walk(n, ox, oy) {
    for (const c of n.children ?? []) { const b = { x: ox + c.x, y: oy + c.y, w: c.width, h: c.height }; boxes.set(byId.get(+c.id), b); walk(c, b.x, b.y); }
  })(r, 0, 0);
  boxes.set(root, { x: 0, y: 0, w: r.width, h: r.height });
  const edges = [];
  (function walk(n) {
    const off = n.id === "0" ? { x: 0, y: 0 } : boxes.get(byId.get(+n.id));
    for (const e of n.edges ?? []) {
      if (!meta.has(e.id)) continue; // chains are not drawn
      for (const s of e.sections ?? []) edges.push({ ...meta.get(e.id), pts: [s.startPoint, ...(s.bendPoints ?? []), s.endPoint].map((p) => [p.x + off.x, p.y + off.y]) });
    }
    for (const c of n.children ?? []) walk(c);
  })(r);
  $("busy").hidden = true;
  const old = L;
  L = { boxes, edges };
  startMorph(old, anchor);
}

// ---------- camera and morph ----------
const cv = $("cv");
const ctx = cv.getContext("2d");
let W = 0, Hh = 0, dpr = 1;
const cam = { x: 0, y: 0, k: 1 };
const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
const dur = () => (reduced ? 0 : P.anim);
const ease = (t) => (t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2);
let morph = null, camAnim = null, camAfter = null, afterSettle = null;
const toScreen = (x, y) => [(x - cam.x) * cam.k + W / 2, (y - cam.y) * cam.k + Hh / 2];
const toWorld = (sx, sy) => [(sx - W / 2) / cam.k + cam.x, (sy - Hh / 2) / cam.k + cam.y];
// Framing leaves room for the bar floating over the canvas's bottom edge.
const BAR_H = 44;
function fit(b, fill = 0.9) {
  const k = Math.min((W * fill) / b.w, ((Hh - BAR_H) * fill) / b.h, 6);
  return { x: b.x + b.w / 2, y: b.y + b.h / 2 + BAR_H / 2 / k, k };
}
function union(bs) {
  const x0 = Math.min(...bs.map((b) => b.x)), y0 = Math.min(...bs.map((b) => b.y));
  const x1 = Math.max(...bs.map((b) => b.x + b.w)), y1 = Math.max(...bs.map((b) => b.y + b.h));
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}
function startMorph(old, anchor) {
  const from = new Map();
  for (const [n, b] of L.boxes) {
    let o = old.boxes.get(n);
    if (!o) for (let d = n.path.length - 2; d >= 0 && !o; d--) { const a = old.boxes.get(n.path[d]); if (a) o = { x: a.x + a.w / 2, y: a.y + a.h / 2, w: 0, h: 0 }; }
    from.set(n, o ?? b);
  }
  let target = camAfter ? camAfter(L) : null;
  camAfter = null;
  if (!target && anchor && old.boxes.get(anchor) && L.boxes.get(anchor)) {
    // Keep the box you acted on where it was on screen.
    const a0 = old.boxes.get(anchor), a1 = L.boxes.get(anchor);
    const [sx, sy] = toScreen(a0.x + a0.w / 2, a0.y + a0.h / 2);
    const k = cam.k * Math.min(a0.w / Math.max(a1.w, 1), 1.0) ** 0.5;
    target = { k, x: a1.x + a1.w / 2 - (sx - W / 2) / k, y: a1.y + a1.h / 2 - (sy - Hh / 2) / k };
  }
  if (afterSettle) { afterSettle(target ?? cam); afterSettle = null; }
  if (!dur() || !old.boxes.size) { morph = null; if (target) { camAnim = null; Object.assign(cam, target); } kick(); return; }
  morph = { from, t0: performance.now() };
  if (target) camAnim = { a: { ...cam }, b: target, t0: morph.t0 };
  kick();
}
function flyTo(t) { if (!dur()) { Object.assign(cam, t); kick(); return; } camAnim = { a: { ...cam }, b: t, t0: performance.now() }; kick(); }
function boxNow(n, p) {
  const b = L.boxes.get(n);
  if (!morph || p >= 1) return b;
  const a = morph.from.get(n) ?? b, e = ease(p);
  return { x: a.x + (b.x - a.x) * e, y: a.y + (b.y - a.y) * e, w: a.w + (b.w - a.w) * e, h: a.h + (b.h - a.h) * e };
}

// ---------- state ----------
let step = -1, selected = null, hovered = null;
let hoverEdge = null; // the line under the pointer: it and its two ends light up
let mapStop = null;   // { fn, stop }: a call under the pointer in a function's code
const steps = () => V?.steps ?? [];
const stepFocus = (i) => { const s = steps()[i]; return s?.focus != null ? byId.get(s.focus) : null; };

// ---------- theme ----------
const T = {};
const TOK = ["paper", "panel", "ink", "dim", "hair", "hair2", "hair3", "orange", "call", "data", "handoff", "lib", "code","k-package", "k-file", "k-type", "k-type-edge", "k-function", "k-block", "k-lib"];
function readTheme() { const cs = getComputedStyle(document.documentElement); for (const k of TOK) T[k] = cs.getPropertyValue("--" + k).trim(); hatch = null; kick(); }
matchMedia("(prefers-color-scheme: dark)").addEventListener("change", readTheme);
new MutationObserver(readTheme).observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
function rgba(hex, a) { const m = /^#([0-9a-f]{6})$/i.exec(hex); if (!m) return hex; const v = parseInt(m[1], 16); return `rgba(${v >> 16},${(v >> 8) & 255},${v & 255},${a})`; }
let hatch = null;
function hatchPattern() {
  if (hatch) return hatch;
  const c = document.createElement("canvas"); c.width = c.height = 8;
  const x = c.getContext("2d"); x.fillStyle = T["k-lib"]; x.fillRect(0, 0, 8, 8);
  x.strokeStyle = rgba(T.lib, 0.35); x.lineWidth = 1; x.beginPath(); x.moveTo(-2, 10); x.lineTo(10, -2); x.moveTo(6, 10); x.lineTo(10, 6); x.moveTo(-2, 2); x.lineTo(2, -2); x.stroke();
  return (hatch = ctx.createPattern(c, "repeat"));
}

// How each kind of box looks.
function look(n) {
  switch (n.k) {
    case "folder": return { fill: T["k-package"], stroke: T.ink, lw: 1, rule: true, text: T.ink, upper: true };
    case "file": case "module": return { fill: T["k-file"], stroke: T.hair3, lw: 1, text: T.ink, lang: n.l };
    case "type": return { fill: T["k-type"], stroke: T["k-type-edge"], lw: 1.25, text: T.ink };
    case "function": return { fill: T["k-function"], stroke: T.hair3, lw: 1, stripe: T.ink, text: T.ink };
    case "block": return { fill: T["k-block"], stroke: T.hair3, lw: 0.8, dash: [3, 3], text: T.dim };
    case "field": return { fill: rgba(T.data, 0.12), stroke: T.data, lw: 1, text: T.data };
    case "static": return { fill: rgba(T.data, 0.2), stroke: T.data, lw: 2, text: T.data };
    case "libroot": return { fill: hatchPattern(), stroke: T.lib, lw: 1, dash: [5, 3], text: T.dim, upper: true };
    case "lib": case "libtype": case "ext": return { fill: n.k === "lib" ? hatchPattern() : T["k-lib"], stroke: T.lib, lw: 0.8, dash: [4, 3], text: T.dim };
    default: return { fill: T["k-file"], stroke: T.hair3, lw: 1, text: T.ink };
  }
}
function lineLook(kind) {
  switch (kind) {
    case "call": return { color: T.call, a: 0.55, lw: 1 };
    case "possible": return { color: T.call, a: 0.55, lw: 1, dash: [5, 4] };
    case "handoff": return { color: T.handoff, a: 0.7, lw: 1.2, dash: [8, 3, 2, 3] };
    case "lib": return { color: T.lib, a: 0.45, lw: 0.9 };
    case "write": return { color: T.data, a: 0.85, lw: 2 };
    case "read": return { color: T.data, a: 0.55, lw: 1 };
    case "access": return { color: T.data, a: 0.6, lw: 1, dash: [2, 3] };
  }
}

// ---------- drawing ----------
let drawnBoxes = [];
function draw(p) {
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.fillStyle = T.paper; ctx.fillRect(0, 0, W, Hh);
  if (TR) { drawTrace(); return; }
  if (!L.boxes.size) return;
  const mono = monoFont();
  // The ends of the line under the pointer, or the targets of the call under it.
  const ends = new Set(hoverEdge ? [hoverEdge.a, hoverEdge.b] : mapStop ? mapStop.stop.items.map((it) => rep(it.t)) : []);
  // Emphasis: the view's highlights, plus the steps so far and their highlights.
  const past = new Set(), current = step >= 0 ? stepFocus(step) : null;
  for (let i = 0; i <= step; i++) { const f = stepFocus(i); if (f) past.add(f); }
  const stepLit = new Set(step >= 0 ? viewNodes(steps()[step].highlight) : []);
  const strongSet = new Set([...lit, ...past, ...stepLit]);
  const holds = (n) => { for (const f of strongSet) if (isUnder(f, n)) return true; return false; };
  const order = [...L.boxes.keys()].filter((n) => n !== root).sort((a, b) => a.depth - b.depth);
  drawnBoxes = [];
  const noteTags = [];
  for (const n of order) {
    const b = boxNow(n, p);
    const [x, y] = toScreen(b.x, b.y), w = b.w * cam.k, h = b.h * cam.k;
    if (x > W || y > Hh || x + w < 0 || y + h < 0 || w < 1.5) continue;
    drawnBoxes.push([n, x, y, w, h]);
    const s = look(n);
    ctx.fillStyle = s.fill; ctx.fillRect(x, y, w, h);
    if (holds(n)) { ctx.fillStyle = rgba(T.orange, n === current ? 0.14 : 0.06); ctx.fillRect(x, y, w, h); }
    if (s.stripe && w > 6) { ctx.fillStyle = s.stripe; ctx.fillRect(x, y, Math.max(2, Math.min(4, 3 * cam.k)), h); }
    if (s.rule) { ctx.fillStyle = s.stroke; ctx.fillRect(x, y, w, Math.max(1.5, 2.5 * Math.min(1, cam.k))); }
    const strong = n === current || strongSet.has(n);
    ctx.setLineDash(s.dash ?? []);
    ctx.lineWidth = n === current ? 2.5 : strong ? 2 : ends.has(n) ? 2.5 : n === selected ? 2 : n === hovered ? Math.max(1.5, s.lw) : s.lw;
    ctx.strokeStyle = strong ? T.orange : ends.has(n) ? T.call : n === selected ? T.ink : n === hovered ? T.ink : s.stroke;
    ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
    ctx.setLineDash([]);
    if (notesOn.has(n)) noteTags.push([n, x, y, w]);
    // Labels: open boxes at the top-left, closed boxes centered with their size.
    const name = prefix(n) + (s.upper ? short(n).toUpperCase() : short(n));
    if (expanded(n) || shows(n)) {
      if (w < 50) continue;
      ctx.font = `500 ${Math.min(12, Math.max(8, 11 * Math.sqrt(cam.k)))}px ${mono}`; ctx.textAlign = "left"; ctx.textBaseline = "top";
      ctx.fillStyle = strong ? T.orange : s.text;
      const used = text(name, x + 8, y + 8, w - 50);
      if (s.lang && w > 120) langTag(s.lang, x + 8 + used + 6, y + 7);
      if (isCode(n)) drawCode(n, x, y, w, h);
    } else if (w > 30 && h > 12) {
      const fs = Math.min(12, Math.max(7, 12 * Math.min(1, cam.k * 1.1)));
      ctx.font = `500 ${fs}px ${mono}`; ctx.textAlign = "center"; ctx.textBaseline = "middle";
      ctx.fillStyle = strong ? T.orange : s.text;
      const two = h > fs * 3;
      text(name, x + w / 2, two ? y + h / 2 - fs * 0.55 : y + h / 2, w - 10);
      if (two) { ctx.font = `${fs * 0.82}px ${mono}`; ctx.fillStyle = T.dim; text(sizeText(n), x + w / 2, y + h / 2 + fs * 0.75, w - 10); }
    }
  }
  if (!(morph && p < 1)) drawLines(current);
  // Claude's notes: authored tags pinned to the box's top edge (full text in the bar and inspector).
  for (const [n, x, y, w] of noteTags) {
    if (w < 40) continue;
    const t = notesOn.get(n).join(" · ");
    ctx.font = `italic 12px ${getComputedStyle(document.body).getPropertyValue("--sans")}`; ctx.textAlign = "left"; ctx.textBaseline = "middle";
    const tw = Math.min(ctx.measureText(t).width, Math.max(160, w * 1.2)) + 16;
    const ty = Math.max(2, y - 22);
    ctx.fillStyle = T.panel; ctx.fillRect(x, ty, tw, 20);
    ctx.fillStyle = T.orange; ctx.fillRect(x, ty, 3, 20);
    ctx.strokeStyle = T.orange; ctx.lineWidth = 1; ctx.strokeRect(x + 0.5, ty + 0.5, tw - 1, 19);
    ctx.fillStyle = T.ink; text(t, x + 9, ty + 10, tw - 14);
  }
}
let drawnLines = []; // [edge, screen points] for hovering lines
function drawLines(current) {
  // What lights up: the line under the pointer; else the call under the pointer in code;
  // else every line of the box under the pointer (or selected). The rest dims.
  const focus = hoverEdge || mapStop ? null : hovered ?? selected;
  const stopHot = mapStop ? new Set(mapStop.stop.items.map((it) => rep(it.t))) : null;
  const hot = (e) => (hoverEdge ? e === hoverEdge : stopHot ? e.a === rep(mapStop.fn) && stopHot.has(e.b) : focus && (isUnder(e.a, focus) || isUnder(e.b, focus)));
  const dimming = !!(focus || hoverEdge || mapStop);
  const routePairs = new Set();
  for (let i = 1; i <= step; i++) { const a = stepFocus(i - 1), b = stepFocus(i); if (a && b) routePairs.add(rep(a).id + ">" + rep(b).id); }
  const layers = [[], [], []];
  for (const e of L.edges) layers[e.kind !== "read" && routePairs.has(e.a.id + ">" + e.b.id) ? 2 : hot(e) ? 1 : 0].push(e);
  drawnLines = [];
  layers.forEach((list, li) => {
    for (const e of list) {
      const s = lineLook(e.kind);
      ctx.strokeStyle = li === 2 ? T.orange : rgba(s.color, li === 1 ? 1 : dimming ? s.a * 0.2 : s.a);
      ctx.lineWidth = (li === 2 ? 2.4 : li === 1 && (hoverEdge || mapStop) ? s.lw + 1.6 : s.lw) + 0.4 * Math.log2(e.c);
      ctx.setLineDash(li === 2 ? [] : s.dash ?? []);
      const pts = e.pts.map(([wx, wy]) => toScreen(wx, wy));
      drawnLines.push([e, pts]);
      ctx.beginPath();
      pts.forEach(([sx, sy], j) => (j ? ctx.lineTo(sx, sy) : ctx.moveTo(sx, sy)));
      ctx.stroke(); ctx.setLineDash([]);
      const [ax, ay] = toScreen(...e.pts[e.pts.length - 1]), [bx, by] = toScreen(...e.pts[e.pts.length - 2]);
      const ang = Math.atan2(ay - by, ax - bx), z = 6 + ctx.lineWidth;
      ctx.fillStyle = ctx.strokeStyle; ctx.beginPath(); ctx.moveTo(ax, ay);
      ctx.lineTo(ax - z * Math.cos(ang - 0.42), ay - z * Math.sin(ang - 0.42)); ctx.lineTo(ax - z * Math.cos(ang + 0.42), ay - z * Math.sin(ang + 0.42)); ctx.fill();
    }
  });
}
function sizeText(n) {
  if (n.k === "function") return lines(n) + " ln";
  if (STATEK.has(n.k)) return "";
  if (LIBK.has(n.k)) { const c = count(n, (x) => x.k === "ext"); return c ? c + " used" : ""; }
  const f = count(n, (x) => x.k === "function"), t = count(n, (x) => x.k === "type");
  return [lines(n) ? lines(n) + " ln" : "", f ? f + " ƒ" : "", t ? t + " ◆" : ""].filter(Boolean).join(" · ");
}
function langTag(l, x, y) {
  ctx.save(); ctx.font = `10px ${monoFont()}`; ctx.textAlign = "left"; ctx.textBaseline = "top";
  const w = ctx.measureText(l).width + 8;
  ctx.strokeStyle = T.hair3; ctx.lineWidth = 1; ctx.strokeRect(x + 0.5, y + 0.5, w, 13);
  ctx.fillStyle = T.dim; ctx.fillText(l, x + 4, y + 2); ctx.restore();
}
function text(s, x, y, maxW) {
  if (maxW < 8) return 0;
  let t = s;
  while (t.length > 2 && ctx.measureText(t).width > maxW) t = t.slice(0, -2);
  if (t !== s) t = t.slice(0, -1) + "…";
  ctx.fillText(t, x, y);
  return ctx.measureText(t).width;
}
// The deepest level: the function's code. Blocks (branches, loops) are bands in the gutter.
function drawCode(n, x, y, w, h) {
  const src = codeOf(n), lh = LH * cam.k, top = y + 30 * cam.k, left = x + 12 * cam.k;
  ctx.save(); ctx.beginPath(); ctx.rect(x, y + 24 * cam.k, w, h - 24 * cam.k); ctx.clip();
  (function bands(c, lvl) { for (const k of c.children) { if (k.k !== "block") continue; ctx.fillStyle = rgba(T.ink, 0.07 + 0.05 * lvl); ctx.fillRect(x + (5 + lvl * 3) * cam.k, top + (k.a - n.a) * lh, 2.2 * cam.k, (k.b - k.a + 1) * lh); bands(k, lvl + 1); } })(n, 0);
  // Every call and data use written here: a tick at the right edge (where its line
  // leaves the code) and the call itself underlined, both in its line's colour.
  for (const s of stopsOf(n)) {
    if (!s.line) continue;
    const sy = top + (s.line - n.a) * lh, col = lineLook(s.items[0].kind).color;
    const on = TR ? traceStopLit(s) : mapStop && mapStop.stop === s;
    if (on) { ctx.fillStyle = rgba(col, 0.2); ctx.fillRect(x, sy, w, lh); }
    ctx.fillStyle = rgba(col, on ? 1 : 0.75); ctx.fillRect(x + w - 5 * cam.k, sy + lh * 0.2, 3 * cam.k, lh * 0.6);
    if (lh >= 7 && s.endLine === s.line) { ctx.fillRect(left + (s.col - 1) * CW * cam.k, sy + lh * 0.92, Math.max(2, (s.endCol - s.col) * CW * cam.k), Math.max(1, cam.k * (on ? 2 : 1))); }
  }
  const i0 = Math.max(0, Math.floor((0 - top) / lh)), i1 = Math.min(src.length, Math.ceil((Hh - top) / lh));
  if (lh >= 7) {
    ctx.font = `${Math.min(lh * 0.8, 15)}px ${monoFont()}`; ctx.textAlign = "left"; ctx.textBaseline = "top"; ctx.fillStyle = T.code;
    for (let i = i0; i < i1; i++) ctx.fillText(src[i], left, top + i * lh);
  } else {
    ctx.fillStyle = rgba(T.code, 0.35);
    for (let i = i0; i < i1; i++) { const l = src[i], ind = l.length - l.trimStart().length, len = l.trim().length; if (len) ctx.fillRect(left + ind * CW * cam.k, top + i * lh + lh * 0.2, Math.min(len, 100 - ind) * CW * cam.k, Math.max(0.6, lh * 0.6)); }
  }
  ctx.restore();
}

// ---------- frame loop ----------
let ticking = false;
function kick() { if (!ticking) { ticking = true; requestAnimationFrame(frame); } }
function frame(now) {
  let p = 1, moving = false;
  if (morph) { p = Math.min(1, (now - morph.t0) / dur()); if (p >= 1) morph = null; else moving = true; }
  if (camAnim) {
    const q = Math.min(1, (now - camAnim.t0) / Math.max(dur(), 1)), e = ease(q), a = camAnim.a, b = camAnim.b;
    cam.k = Math.exp(Math.log(a.k) + (Math.log(b.k) - Math.log(a.k)) * e); cam.x = a.x + (b.x - a.x) * e; cam.y = a.y + (b.y - a.y) * e;
    if (q >= 1) camAnim = null; else moving = true;
  }
  draw(p);
  ticking = false;
  if (moving || traceFading()) kick();
}
function resize() { const r = cv.getBoundingClientRect(); dpr = window.devicePixelRatio || 1; W = r.width; Hh = r.height; cv.width = Math.round(W * dpr); cv.height = Math.round(Hh * dpr); kick(); }
new ResizeObserver(resize).observe(cv);

// ---------- actions ----------
function setOpen(set, anchor, then) { open = set; for (const n of [...zoomOpened.keys()]) if (!open.has(n)) zoomOpened.delete(n); if (then) camAfter = then; relayout(anchor); }
function canOpen(n) { return isCode(n) || n.children.length > 0; }
function closeBox(s, n) { for (const x of [...s]) if (x === n || isUnder(x, n)) s.delete(x); }
function toggle(n, byZoom) {
  if (!canOpen(n)) return;
  const s = new Set(open);
  if (s.has(n)) closeBox(s, n); else { s.add(n); if (byZoom) afterSettle = (c) => zoomOpened.set(n, c.k); }
  setOpen(s, n);
}
function select(n) {
  selected = n; inspect(n); report();
  if (n && P.reshape && n.k === "function") {
    const s = step >= 0 ? new Set(open) : viewOpen(), c = callsOf.get(n) ?? { out: new Set(), in: new Set() };
    openPathTo(s, n);
    for (const m of [...c.out, ...c.in]) openPathTo(s, m);
    setOpen(s, n, (lay) => fit(union([n, ...c.out, ...c.in].map((m) => lay.boxes.get(rep(m))).filter(Boolean)), 0.85));
  } else kick();
  bar();
}
function goStep(i) {
  step = i; selected = null;
  // A step can open the trace view on a call stack; a step without one leaves it.
  const tr = step >= 0 ? steps()[step].trace : null;
  if (tr) { traceFromView(tr); bar(); transport(); report(); return; }
  if (TR) exitTrace(false);
  if (step < 0) { inspect(null); setOpen(viewOpen(), null, (lay) => fitAll(lay, [...lit])); bar(); transport(); report(); return; }
  const s = viewOpen();
  for (let j = 0; j <= step; j++) {
    const st = steps()[j], f = stepFocus(j);
    if (f) openPathTo(s, f);
    for (const n of viewNodes(st.open)) { openPathTo(s, n); s.add(n); }
    for (const n of viewNodes(st.highlight)) if (j === step) openPathTo(s, n);
  }
  const cur = stepFocus(step);
  if (cur && cur.k === "function") s.add(cur);
  inspect(cur);
  // The camera frames this step (its focus and highlights) with room around it; the
  // route so far stays lit, and Back/Next fly between steps.
  const fitTo = [cur, ...viewNodes(steps()[step].highlight)].filter(Boolean);
  setOpen(s, null, (lay) => { const bs = (fitTo.length ? fitTo : [...lit]).map((m) => lay.boxes.get(rep(m))).filter(Boolean); return bs.length ? fit(union(bs), 0.6) : fit(lay.boxes.get(root), 0.96); });
  bar(); transport(); report();
}
function transport() {
  const n = steps().length;
  $("count").textContent = !n ? "no steps" : step < 0 ? `${n} steps` : `step ${step + 1} / ${n}`;
  $("back").disabled = step < 0; $("next").disabled = !n || step >= n - 1; $("play").disabled = !n;
}
// Play: each step held long enough to read its note; Back, Next and Pause always work (BQ-39).
let playTimer = null;
function startPlay() { $("play").setAttribute("aria-pressed", "true"); $("play").textContent = "Pause"; schedule(); }
function stopPlay() { clearTimeout(playTimer); playTimer = null; $("play").setAttribute("aria-pressed", "false"); $("play").textContent = "Play"; }
function schedule() {
  clearTimeout(playTimer);
  const note = steps()[step]?.note ?? "";
  playTimer = setTimeout(() => { if (step < steps().length - 1) { goStep(step + 1); schedule(); } else stopPlay(); }, 2500 + 45 * note.length);
}
$("play").onclick = () => { if (playTimer) stopPlay(); else { if (step < 0 || step >= steps().length - 1) goStep(0); startPlay(); } };

const esc = (s) => String(s).replace(/[&<>"]/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[ch]);
// Claude's text: escaped, with `code` spans.
const prose = (s) => esc(s).replace(/`([^`]+)`/g, "<code>$1</code>");
const loc = (n) => (n.f ? `${n.f}:${n.a}` : "");
const HOW = {
  click: "Double-click a box to open it. Hover a line to see where it goes. Select a function and press Enter to trace it.",
  zoom: "Zoom into a box and it opens; zoom out and it closes. Hover a line to see where it goes. Select a function and press Enter to trace it.",
};
function bar() {
  const b = $("bar");
  const note = (tag, html) => { b.classList.add("note"); $("bartag").textContent = tag; $("bartext").innerHTML = html; };
  if (step >= 0) {
    const st = steps()[step], cur = stepFocus(step);
    note(`step ${step + 1} · claude`, st.note ? prose(st.note) : cur ? `<code>${esc(cur.n)}</code> (${esc(loc(cur))})` : "");
    return;
  }
  b.classList.remove("note");
  if (TR) { traceBar(); return; }
  if (hoverEdge) {
    const e = hoverEdge, parts = [...new Map(e.under.map((u) => [u.src.n + "→" + u.tgt.n, u])).values()];
    $("bartag").textContent = LINE[e.kind].label;
    $("bartext").innerHTML = `<code>${esc(short(e.a))}</code> → <code>${esc(short(e.b))}</code> · ${e.c} time${e.c > 1 ? "s" : ""}` +
      (parts.length > 1 || parts[0].src !== e.a || parts[0].tgt !== e.b ? " · " + parts.slice(0, 4).map((u) => `<code>${esc(u.src.n)}</code> → <code>${esc(u.tgt.n)}</code>${u.sites[0] ? " (line " + u.sites[0][0] + ")" : ""}`).join(", ") + (parts.length > 4 ? ` and ${parts.length - 4} more` : "") : "") +
      " · click to see both ends";
    return;
  }
  if (mapStop) {
    $("bartag").textContent = "line " + mapStop.stop.line;
    $("bartext").innerHTML = mapStop.stop.items.map((it) => `${esc(LINE[it.kind].label)} <code>${esc(it.t.n)}</code>`).join(", ") + " · click to trace from here";
    return;
  }
  const h = hovered ?? selected;
  if (h && h !== root) {
    if (notesOn.has(h)) { note("note · claude", prose(notesOn.get(h).join("\n"))); return; }
    $("bartag").textContent = KIND[h.k] ?? h.k;
    const sz = sizeText(h);
    $("bartext").innerHTML = `<code>${esc(h.n)}</code>${h.f ? " · " + esc(loc(h)) : ""}${sz ? " · " + esc(sz) : ""}`;
    return;
  }
  if (notesOn.has(root)) { note("note · claude", prose(notesOn.get(root).join("\n"))); return; }
  $("bartag").textContent = "how to use";
  $("bartext").textContent = G ? HOW[P.openBy] + (steps().length ? " Next ▶ steps through Claude's view." : " Ask Claude to show you something, and its view appears here.") : "";
}
function inspect(n) {
  const p = $("insp");
  if (!n || n === root) { p.hidden = true; return; }
  const tally = (pred) => E.filter(pred).reduce((s, e) => s + e.c, 0);
  const inside = (x) => isUnder(x, n);
  const cells = [["file", n.f ?? (LIBK.has(n.k) ? "library" : n.n)], ["lines", n.a ? n.a + "–" + n.b : "—"]];
  if (STATEK.has(n.k)) cells.push(["written by", tally((e) => e.kind === "write" && e.b === n)], ["read by", tally((e) => e.kind === "read" && e.a === n)]);
  else if (LIBK.has(n.k)) cells.push(["called from your code", tally((e) => e.kind === "lib" && inside(e.b))]);
  else cells.push(["calls out", tally((e) => (e.kind === "call" || e.kind === "possible") && inside(e.a) && !inside(e.b))], ["called from outside", tally((e) => (e.kind === "call" || e.kind === "possible") && inside(e.b) && !inside(e.a))], ["library calls", tally((e) => e.kind === "lib" && inside(e.a))]);
  const notes = notesOn.get(n);
  p.innerHTML = `<header><span class="chip">${KIND[n.k] ?? n.k}</span><span class="name">${esc(n.n)}</span><button class="box-btn" id="close">Close</button></header>
    ${notes ? `<div class="notes">${prose(notes.join("\n"))}</div>` : ""}
    <div class="cells">${cells.map(([l, v]) => `<div class="cell"><div class="lbl">${l}</div><div class="v">${esc(v)}</div></div>`).join("")}</div>
    <div class="acts">${canTrace(n) ? `<button class="box-btn primary" id="trace" title="Its code with its callers and calls, stepped in the order written (Enter)">Trace ▸</button>` : ""}${canOpen(n) ? `<button class="box-btn" id="tog">${open.has(n) ? "Close box" : n.k === "function" ? "Show code" : "Open box"}</button>` : ""}<button class="box-btn" id="fitb">Zoom to it</button>
      <button class="box-btn" id="up" title="Earlier in its column">▲ Up</button><button class="box-btn" id="down" title="Later in its column">▼ Down</button></div>`;
  p.hidden = false; $("settings").hidden = true; $("settingsBtn").setAttribute("aria-pressed", "false");
  $("close").onclick = () => { selected = null; inspect(null); bar(); kick(); report(); };
  if ($("trace")) $("trace").onclick = () => enterTrace(n);
  if ($("tog")) $("tog").onclick = () => { toggle(n); setTimeout(() => inspect(n), 0); };
  $("fitb").onclick = () => { const b = L.boxes.get(rep(n)); if (b) flyTo(fit(b, 0.85)); };
  $("up").onclick = () => move(n, -1);
  $("down").onclick = () => move(n, 1);
}
// Moving a box within its column: the same order Claude sets with `layout.order`.
function move(n, dir) {
  const parent = n.parent, me = L.boxes.get(n);
  if (!parent || !me) return;
  const { list } = orderedChildren(parent);
  // Neighbours: siblings in the same column (overlapping x), by their y on screen.
  const col = list.filter((x) => { const b = L.boxes.get(x); return b && b.x < me.x + me.w && me.x < b.x + b.w; }).sort((a, b) => L.boxes.get(a).y - L.boxes.get(b).y);
  const i = col.indexOf(n), j = i + dir;
  if (i < 0 || j < 0 || j >= col.length) return;
  [col[i], col[j]] = [col[j], col[i]];
  // The column's new top-to-bottom order goes first; everything else keeps its place.
  userOrder.set(parent, [...col, ...list.filter((x) => !col.includes(x))]);
  relayout(n);
}
// What the user has selected and which step they're on, for Claude's `selection`.
function report(extra) {
  const s = selected && selected !== root ? { id: selected.id, kind: KIND[selected.k] ?? selected.k, name: selected.n, path: pathText(selected), file: selected.f ?? null, line: selected.a ?? null } : null;
  invoke("select", { selection: { selected: s, step: step >= 0 ? step + 1 : null, steps: steps().length, view: V?.title ?? null, trace: traceReport(), ...(extra ?? {}) } }).catch(() => {});
}

// ---------- legend (also the line filter) ----------
function swatchBox(k) {
  const st = {
    folder: "background:var(--k-package);border:1px solid var(--ink);border-top-width:3px",
    file: "background:var(--k-file)", type: "background:var(--k-type);border-color:var(--k-type-edge)",
    function: "background:var(--k-function);border-left:4px solid var(--ink)", block: "background:var(--k-block);border-style:dashed",
    field: "background:color-mix(in srgb, var(--data) 14%, transparent);border-color:var(--data)",
    lib: "background:repeating-linear-gradient(135deg, var(--k-lib) 0 4px, color-mix(in srgb, var(--lib) 40%, var(--k-lib)) 4px 5px);border:1px dashed var(--lib)",
  }[k];
  return `<span class="sw box" style="${st}"></span>`;
}
function swatchLine(kind) {
  const c = { call: "var(--call)", possible: "var(--call)", handoff: "var(--handoff)", lib: "var(--lib)", write: "var(--data)", read: "var(--data)", access: "var(--data)" }[kind];
  const d = { possible: "5 4", handoff: "8 3 2 3", access: "2 3" }[kind] ?? "";
  const w = { write: 2.4, handoff: 1.6 }[kind] ?? 1.4;
  return `<svg class="sw" viewBox="0 0 30 16" aria-hidden="true"><line x1="1" y1="8" x2="23" y2="8" stroke="${c}" stroke-width="${w}" stroke-dasharray="${d}"/><path d="M22 4 L29 8 L22 12 Z" fill="${c}"/></svg>`;
}
function legend() {
  const el = $("legend");
  const boxes = [["folder", "folder"], ["file", "file"], ["type", "◆ type (struct, class, enum…)"], ["function", "ƒ function"], ["block", "block (branch, loop)"], ["field", "field or static (data)"], ["lib", "library code"]];
  el.innerHTML = `<header><span class="lbl">legend</span><button class="box-btn" id="legendClose">Hide</button></header>
    <div class="sec"><div class="lbl">boxes</div>${boxes.map(([k, l]) => `<div class="row">${swatchBox(k)}<span>${l}</span></div>`).join("")}</div>
    <div class="sec"><div class="lbl">lines (tick to show)</div>${Object.entries(LINE).map(([k, v]) => `<div class="row"><label><input type="checkbox" id="ln-${k}" ${P.lines[k] ? "checked" : ""}>${swatchLine(k)}<span title="${v.note}">${v.label}</span></label><small>${(totals[k] ?? 0).toLocaleString()}</small></div>`).join("")}</div>
    <div class="sec"><div class="row"><span class="sw box" style="background:color-mix(in srgb, var(--orange) 14%, transparent);border:2px solid var(--orange)"></span><span>what Claude is showing you</span></div></div>`;
  for (const k of Object.keys(LINE)) $("ln-" + k).onchange = (e) => { P.lines[k] = e.target.checked; save(); stopCache.clear(); if (TR) traceRefresh(); relayout(null); };
  $("legendClose").onclick = () => showLegend(false);
}
function showLegend(on) { $("legend").hidden = !on; $("legendBtn").setAttribute("aria-pressed", String(on)); }
$("legendBtn").onclick = () => showLegend($("legend").hidden);

// ---------- settings ----------
function bindSettings() {
  const pct = (v) => v + "%";
  const ms = (v) => (v == 0 ? "off" : v + " ms");
  const px = (v) => v + " px";
  const relayoutSoon = (() => { let t = null; return () => { clearTimeout(t); t = setTimeout(() => relayout(null), 150); }; })();
  [["sOpen", "oOpen", "openAt", pct], ["sClose", "oClose", "closePast", pct], ["sAnim", "oAnim", "anim", ms], ["sGap", "oGap", "gap", px, true], ["sCol", "oCol", "col", px, true]].forEach(([i, o, key, f, lay]) => {
    $(i).value = P[key]; $(o).textContent = f(P[key]);
    $(i).oninput = (e) => { P[key] = +e.target.value; $(o).textContent = f(P[key]); save(); if (lay) relayoutSoon(); };
  });
  $("sReshape").checked = P.reshape;
  $("sReshape").onchange = (e) => { P.reshape = e.target.checked; save(); };
}
$("settingsBtn").onclick = () => { const s = $("settings"); s.hidden = !s.hidden; $("settingsBtn").setAttribute("aria-pressed", String(!s.hidden)); if (!s.hidden) $("insp").hidden = true; };
$("settingsClose").onclick = () => { $("settings").hidden = true; $("settingsBtn").setAttribute("aria-pressed", "false"); };

document.querySelectorAll("[data-open]").forEach((b) => b.addEventListener("click", () => { P.openBy = b.dataset.open; save(); markOpenBy(); bar(); }));
function markOpenBy() { document.querySelectorAll("[data-open]").forEach((x) => x.setAttribute("aria-pressed", String(x.dataset.open === P.openBy))); }
$("next").onclick = () => { if (step < steps().length - 1) { goStep(step + 1); if (playTimer) schedule(); } };
$("back").onclick = () => { if (step >= 0) { goStep(step - 1); if (playTimer) schedule(); } };
$("reset").onclick = () => { if (TR) exitTrace(false); hovered = null; zoomOpened.clear(); stopPlay(); step = -1; selected = null; inspect(null); setOpen(new Set(HOME), null, (lay) => fit(lay.boxes.get(root), 0.96)); bar(); transport(); report(); };
addEventListener("keydown", (e) => {
  if (e.target.closest?.("input, select")) return;
  if (e.key === "ArrowRight") $("next").click();
  else if (e.key === "ArrowLeft") $("back").click();
  else if (e.key === " ") { e.preventDefault(); $("play").click(); }
  else if (e.key === "Escape") { selected = null; inspect(null); bar(); kick(); report(); }
  else if (e.key === "Enter" && selected && canTrace(selected)) enterTrace(selected);
});

// ---------- pointer ----------
function hit(sx, sy) { for (let i = drawnBoxes.length - 1; i >= 0; i--) { const [n, x, y, w, h] = drawnBoxes[i]; if (sx >= x && sy >= y && sx <= x + w && sy <= y + h) return n; } return null; }
const ptrs = new Map();
let drag = null, pinch = null;
cv.addEventListener("pointerdown", (e) => {
  cv.setPointerCapture(e.pointerId); ptrs.set(e.pointerId, [e.offsetX, e.offsetY]);
  if (ptrs.size === 1) drag = { x: e.offsetX, y: e.offsetY, moved: false };
  if (ptrs.size === 2) { const [a, b] = [...ptrs.values()]; pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]), k: cam.k }; drag = null; }
});
cv.addEventListener("pointermove", (e) => {
  if (ptrs.has(e.pointerId)) ptrs.set(e.pointerId, [e.offsetX, e.offsetY]);
  if (pinch && ptrs.size === 2) { const [a, b] = [...ptrs.values()]; zoomAt((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (pinch.k * Math.hypot(a[0] - b[0], a[1] - b[1])) / pinch.d / cam.k); return; }
  if (drag) {
    const dx = e.offsetX - drag.x, dy = e.offsetY - drag.y;
    if (drag.moved || Math.hypot(dx, dy) > 3) { drag.moved = true; cv.classList.add("dragging"); camAnim = null; cam.x -= dx / cam.k; cam.y -= dy / cam.k; drag.x = e.offsetX; drag.y = e.offsetY; kick(); }
    return;
  }
  if (TR) { traceMove(e.offsetX, e.offsetY); return; }
  // A line within a few pixels wins over the box beneath it; inside a function's code,
  // the call on the line under the pointer.
  const le = lineHit(e.offsetX, e.offsetY);
  const h = le ? null : hit(e.offsetX, e.offsetY);
  const ms = h && isCode(h) && shows(h) ? codeStopAt(h, e.offsetX, e.offsetY) : null;
  if (h !== hovered || le !== hoverEdge || ms?.stop !== mapStop?.stop) { hovered = h; hoverEdge = le; mapStop = ms; cv.style.cursor = le || ms ? "pointer" : ""; bar(); kick(); }
});
function segDist(px, py, [ax, ay], [bx, by]) {
  const dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy;
  const t = l2 ? Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / l2)) : 0;
  return Math.hypot(px - ax - t * dx, py - ay - t * dy);
}
function lineHit(sx, sy) {
  let best = null, bd = 5;
  for (const [e, pts] of drawnLines) for (let i = 1; i < pts.length; i++) { const d = segDist(sx, sy, pts[i - 1], pts[i]); if (d < bd) { bd = d; best = e; } }
  return best;
}
// The call written on the code line under the pointer (the nearest one by column).
function codeStopAt(n, sx, sy) {
  const b = drawnBoxes.find((d) => d[0] === n);
  if (!b) return null;
  const [, x, y] = b, lh = LH * cam.k, top = y + 30 * cam.k, left = x + 12 * cam.k;
  const line = n.a + Math.floor((sy - top) / lh), col = (sx - left) / (CW * cam.k) + 1;
  const here = stopsOf(n).filter((s) => s.line === line);
  if (!here.length) return null;
  here.sort((a, b) => Math.abs((a.col + a.endCol) / 2 - col) - Math.abs((b.col + b.endCol) / 2 - col));
  return { fn: n, stop: here[0] };
}
function up(e) {
  ptrs.delete(e.pointerId); if (ptrs.size < 2) pinch = null;
  cv.classList.remove("dragging");
  if (drag && !drag.moved && e.type === "pointerup") {
    if (TR) traceClick(e.offsetX, e.offsetY);
    else if (hoverEdge) { const bs = [hoverEdge.a, hoverEdge.b].map((n) => L.boxes.get(n)).filter(Boolean); if (bs.length) flyTo(fit(union(bs), 0.8)); }
    else if (mapStop) enterTrace(mapStop.fn, mapStop.stop.line);
    else { const n = hit(e.offsetX, e.offsetY); if (n !== selected) select(n); }
  }
  drag = null;
}
cv.addEventListener("pointerup", up); cv.addEventListener("pointercancel", up);
cv.addEventListener("pointerleave", () => { if (!drag) { hovered = hoverEdge = mapStop = null; if (TR) traceHover = null; bar(); kick(); } });
cv.addEventListener("dblclick", (e) => { if (TR) { traceDouble(e.offsetX, e.offsetY); return; } const n = hit(e.offsetX, e.offsetY); if (n && !mapStop) toggle(n); });
function zoomAt(sx, sy, f) {
  camAnim = null;
  const [wx, wy] = toWorld(sx, sy);
  cam.k = Math.max(0.03, Math.min(cam.k * f, 12));
  cam.x = wx - (sx - W / 2) / cam.k; cam.y = wy - (sy - Hh / 2) / cam.k;
  kick(); if (P.openBy === "zoom" && !TR) zoomOpenSoon(sx, sy);
}
// Zoom-to-open with hysteresis: a closed box under the cursor opens once it fills `openAt`
// of the screen. A box opened this way closes once you zoom out `closePast` beyond the scale
// it settled at, so open and close never fight over one zoom level.
let zt = null;
function zoomOpenSoon(sx, sy) {
  clearTimeout(zt);
  zt = setTimeout(() => {
    if (morph) return;
    const closing = [...zoomOpened].filter(([, k]) => cam.k < k * (1 - P.closePast / 100)).map(([n]) => n);
    if (closing.length) {
      const deepest = closing.sort((a, b) => b.depth - a.depth)[0];
      const s = new Set(open); closeBox(s, deepest); setOpen(s, deepest); return;
    }
    const n = hit(sx, sy);
    if (!n || open.has(n) || !canOpen(n)) return;
    const b = L.boxes.get(n), frac = Math.max((b.w * cam.k) / W, (b.h * cam.k) / Hh);
    if (frac >= P.openAt / 100) toggle(n, true);
  }, 120);
}
cv.addEventListener("wheel", (e) => { e.preventDefault(); zoomAt(e.offsetX, e.offsetY, Math.exp(-e.deltaY * (e.ctrlKey ? 0.01 : 0.0015))); }, { passive: false });

// ---------- projects, loading, and Claude's events ----------
async function fillPicker() {
  const { projects, loaded } = await invoke("projects");
  const pick = $("pick");
  pick.innerHTML = `<option value="">${projects.length ? "choose a snapshot…" : "no projects (insight init)"}</option>` +
    projects.map((p) => `<optgroup label="${esc(p.name)}">${p.snapshots.map((s) => `<option value="${esc(JSON.stringify([p.name, s.file]))}">${esc(p.name)} · ${esc(s.repo)} @ ${esc(s.commit.slice(0, 10))}${s.built ? " · built " + esc(new Date(s.built * 1000).toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" })) : ""}${s.current === false ? " · older format, rebuild" : ""}</option>`).join("")}</optgroup>`).join("");
  if (loaded) pick.value = JSON.stringify([loaded.project, loaded.snapshot]);
  return loaded;
}
$("pick").onchange = async (e) => {
  if (!e.target.value) return;
  const [project, snapshot] = JSON.parse(e.target.value);
  $("empty").hidden = false; $("empty").textContent = `loading ${project}…`;
  try { await invoke("load", { project, snapshot }); } // the backend then sends `loaded`
  catch (err) { $("empty").textContent = String(err); }
};
async function loadGraph() {
  $("empty").hidden = false; $("empty").textContent = "reading the snapshot…";
  try { setGraph(await invoke("graph")); }
  catch (err) { $("empty").textContent = String(err); return; }
  files.clear();
  $("empty").hidden = true; $("title").hidden = true;
  legend(); inspect(null); transport(); bar(); report();
  const v = await invoke("current_view");
  if (v) applyView(v);
  else { camAfter = (lay) => fit(lay.boxes.get(root), 0.96); relayout(null); }
}
listen("loaded", async () => { await fillPicker(); await loadGraph(); });
listen("view", (e) => { if (G) applyView(e.payload); });
listen("step", (e) => { if (G && e.payload < steps().length) { stopPlay(); goStep(e.payload); } });

// ---------- start: the whole system (ADR-0021) ----------
readTheme(); legend(); bindSettings(); markOpenBy(); transport();
if (innerWidth < 640) showLegend(false);
document.fonts.ready.then(async () => {
  resize();
  const loaded = await fillPicker().catch((e) => { $("empty").hidden = false; $("empty").textContent = String(e); });
  if (loaded) await loadGraph();
  else { $("empty").hidden = false; $("empty").textContent = "Choose a snapshot above, or ask Claude about a project and it loads one here."; bar(); }
});
