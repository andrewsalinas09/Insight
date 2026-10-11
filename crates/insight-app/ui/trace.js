// Trace: one thing at a time, like stepping in a debugger (ADR-0052, ADR-0053).
// - A function: its code in the middle; the functions that call it on the left;
//   everything it calls, reads or writes on the right, each level with the line that
//   does it, in the order written. ↓/↑ walk those; Enter steps in; Backspace steps out.
// - Data (a field, a static, or a whole type): its declaration in the middle (inside
//   its type's code); the functions that write it on the left; those that read it on
//   the right. Stepping in opens that function at the line that touches it.
// All of it is mechanical: where each call, read and write is written is a fact in the
// snapshot (ADR-0035). Claude opens it with `show({view: {trace}})`, also per step.
// Every move is remembered, so ↶ (Alt+←, the mouse's back button) undoes it.
// Loaded before app.js; everything here runs only after the graph has loaded.
"use strict";
let TR = null;         // { S, mode, lay, cur, pick, stack: [{ S, cur, pick }], entry, t0, msg }
let traceHover = null; // { stop } | { target } | { caller }
let tHit = { tags: [], code: null, curves: [] };
const stopCache = new Map();
const TGAP = 170;

function traceReset() { TR = null; traceHover = null; stopCache.clear(); HIST.length = 0; histButton(); const t = document.getElementById("trail"); if (t) t.hidden = true; }
const isData = (n) => !!n && (STATEK.has(n.k) || n.k === "type");
const canTrace = (n) => !!n && !!n.f && !!n.a && (n.k === "function" || n.k === "span" || isData(n));
// An excerpt: lines of a file that belong to nothing smaller (a comment block, say).
let excerptN = 0;
function excerptOf(f, a, b) {
  const s = { id: "x" + excerptN++, k: "span", n: `${f.split("/").pop()}:${a}${b > a ? "–" + b : ""}`, f, a, b, children: [], parent: null };
  s.path = [root, s]; s.depth = 1;
  return s;
}
const traceFading = () => !!TR && performance.now() - TR.t0 < 320;

// The calls and data uses written inside a function (its closures included), grouped
// by where they're written, in source order. Only the line kinds ticked in the legend.
function stopsOf(F) {
  if (!F?.path) return [];
  if (stopCache.has(F)) return stopCache.get(F);
  const by = new Map();
  for (const e of E) {
    if (!P.lines[e.kind] || !e.src || !e.tgt || !isUnder(e.src, F) || isUnder(e.tgt, F)) continue;
    for (const [l, c, l1, c1] of e.sites.length ? e.sites : [[0, 0, 0, 0]]) {
      if (l && (l < F.a || l > F.b)) continue;
      const key = l + ":" + c;
      const s = by.get(key) ?? by.set(key, { line: l, col: c, endLine: l1, endCol: c1, items: [] }).get(key);
      if (!s.items.some((x) => x.t === e.tgt)) s.items.push({ t: e.tgt, kind: e.kind });
    }
  }
  const list = [...by.values()].sort((a, b) => (a.line || 1e9) - (b.line || 1e9) || a.col - b.col);
  stopCache.set(F, list);
  return list;
}
function callersOf(F) {
  const by = new Map();
  for (const e of E) {
    if (!["call", "possible", "handoff"].includes(e.kind) || e.tgt !== F) continue;
    const f = fnOf(e.src);
    if (!f || f === F || isUnder(f, F)) continue;
    const x = by.get(f) ?? by.set(f, { n: f, kind: e.kind, lines: [] }).get(f);
    for (const s of e.sites) x.lines.push(s[0]);
  }
  return [...by.values()].sort((a, b) => pathText(a.n).localeCompare(pathText(b.n)));
}
// The functions that write (or take a reference to) and read a piece of data, or
// anything inside it, with the lines that do it. Every kind counts here, whatever the
// legend shows.
function dataUses(S) {
  const w = new Map(), r = new Map();
  for (const e of E) {
    if (!["read", "write", "access"].includes(e.kind) || !e.tgt || !(e.tgt === S || isUnder(e.tgt, S))) continue;
    const f = fnOf(e.src);
    if (!f) continue;
    const m = e.kind === "read" ? r : w;
    const x = m.get(f) ?? m.set(f, { n: f, kind: e.kind, lines: [], what: new Set() }).get(f);
    for (const s of e.sites) x.lines.push(s[0]);
    x.what.add(e.tgt);
  }
  const done = (m) => [...m.values()].map((x) => ({ ...x, lines: [...new Set(x.lines)].sort((a, b) => a - b) })).sort((a, b) => pathText(a.n).localeCompare(pathText(b.n)));
  return { writers: done(w), readers: done(r) };
}
// Where something lives: its two nearest boxes.
const ctxOf = (n) => n.path.slice(1, -1).filter((x) => x.k !== "libroot").slice(-2).map(short).join(" › ");
function tagSize(n) {
  measure.font = "500 12px " + monoFont();
  const a = measure.measureText(prefix(n) + short(n)).width;
  measure.font = "10.5px " + monoFont();
  const b = measure.measureText(ctxOf(n)).width;
  return { w: Math.min(Math.max(a, b, 60) + 40, 340), h: 42 };
}
// The code shown for data: its type's code when that's not huge, else a few lines around it.
function dataCode(S) {
  if (S.k === "type") return S;
  const p = S.parent;
  if (p && p.k === "type" && p.f === S.f && p.a && lines(p) <= 160) return p;
  return { k: "span", n: S.n, f: S.f, a: Math.max(1, S.a - 3), b: S.b + 3, children: [] };
}

// ---------- layout (world units; the code's top-left at ox, oy) ----------
function stackTags(tags, x, y0, right) {
  let y = y0;
  for (const t of tags) { const s = tagSize(t.n); t.box = { x: right ? x : x - s.w, y, w: s.w, h: s.h }; y += s.h + 8; }
}
function traceLayout(S, ox, oy) {
  const mode = S.k === "function" ? "code" : "data";
  const node = mode === "code" ? S : dataCode(S);
  const sz = codeSize(node);
  const code = { x: ox, y: oy, w: sz.width, h: sz.height, node };
  const lineY = (l) => code.y + 30 + (l - node.a) * LH + LH / 2;
  const right = new Map(), links = [];
  let left, stops, hl = null;
  if (mode === "code") {
    stops = stopsOf(S);
    stops.forEach((s, i) => {
      for (const it of s.items) {
        const t = right.get(it.t) ?? right.set(it.t, { n: it.t, kind: it.kind, stops: [] }).get(it.t);
        if (!t.stops.includes(i)) t.stops.push(i);
      }
    });
    // Each target sits level with the first line that uses it, pushed down when they crowd.
    let y = code.y;
    for (const t of right.values()) {
      const s = tagSize(t.n), first = stops[t.stops[0]];
      y = Math.max(first.line ? lineY(first.line) - s.h / 2 : y, y);
      t.box = { x: code.x + code.w + TGAP, y, w: s.w, h: s.h };
      y += s.h + 8;
    }
    left = callersOf(S).map((c) => ({ ...c, role: "caller" }));
    stackTags(left, code.x - TGAP, code.y, false);
    stops.forEach((s, i) => s.items.forEach((it) => links.push({ from: [code.x + code.w, s.line ? lineY(s.line) : code.y + code.h - 10], to: right.get(it.t), kind: it.kind, stop: i, t: it.t, into: false })));
    for (const c of left) links.push({ from: null, to: c, kind: c.kind, stop: -1, t: c.n, into: [code.x, code.y + 15] });
  } else {
    // Data: writers on the left, readers on the right; each one is a stop.
    const { writers, readers } = dataUses(S);
    hl = [S.a, S.b];
    const y0 = lineY(S.a);
    left = writers.map((x) => ({ ...x, role: "writer" }));
    const rs = readers.map((x) => ({ ...x, role: "reader" }));
    const hTot = (k) => k * 50;
    // Both columns centred on the data's line, so the lines fan in from above and below.
    stackTags(left, code.x - TGAP, y0 - hTot(left.length) / 2, false);
    stackTags(rs, code.x + code.w + TGAP, y0 - hTot(rs.length) / 2, true);
    for (const r of rs) right.set(r.n, r);
    stops = [...left, ...rs].map((t) => ({ line: t.lines[0] ?? 0, items: [{ t: t.n, kind: t.kind }], tag: t }));
    stops.forEach((s, i) => (s.tag.stops = [i]));
    for (const t of left) links.push({ from: null, to: t, kind: t.kind, stop: t.stops[0], t: t.n, into: [code.x, y0] });
    for (const t of rs) links.push({ from: [code.x + code.w, y0], to: t, kind: t.kind, stop: t.stops[0], t: t.n, into: false });
  }
  return { S, mode, node, code, lineY, stops, left, right, links, hl };
}
const tagOf = (lay, n) => lay.right.get(n) ?? lay.left.find((t) => t.n === n);
function traceBounds(lay) {
  // The right column is at least as wide as its heading.
  const head = lay.right.size ? [{ x: lay.code.x + lay.code.w + TGAP, y: lay.code.y, w: 340, h: 1 }] : [];
  return union([lay.code, ...head, ...[...lay.right.values()].map((t) => t.box), ...lay.left.map((c) => c.box)].map((b) => ({ x: b.x, y: b.y, w: b.w, h: b.h })));
}
// The camera: the three columns across the screen when they fit, else the code from the
// left edge; vertically, the current stop and what it touches.
function traceCam(lay, keepK) {
  const b = traceBounds(lay);
  const k = keepK ?? Math.max(0.45, Math.min(1.2, (W * 0.96) / b.w));
  const x = b.w * k <= W ? b.x + b.w / 2 : lay.code.x - (lay.left.length ? 60 : 30) + W / 2 / k;
  const sp = bottomSpace(), viewH = (Hh - sp) / k;
  let y;
  if (b.h <= viewH * 0.9) y = b.y + b.h / 2;
  else {
    const s = lay.stops[TR?.cur ?? 0], t = s && tagOf(lay, s.items[0].t);
    const ly = lay.mode === "data" ? lay.lineY(lay.S.a) : s?.line ? lay.lineY(s.line) : lay.code.y;
    // Both the line and what it touches when they fit on screen together, else what it touches.
    const ty = t ? t.box.y + t.box.h / 2 : ly;
    const want = Math.abs(ty - ly) < viewH * 0.7 ? (ly + ty) / 2 : ty;
    y = Math.min(Math.max(want, b.y - 30 + viewH / 2), b.y + b.h + 30 - viewH / 2);
  }
  return { x, y: y + sp / 2 / k, k };
}
// Keep the current stop on screen without jumping around when it already is.
function traceFollow() {
  const lay = TR.lay, s = lay.stops[TR.cur];
  if (!s) return;
  const t = tagOf(lay, TR.pick ?? s.items[0].t);
  const ly = lay.mode === "data" ? lay.lineY(lay.S.a) : s.line ? lay.lineY(s.line) : null;
  const ys = [ly, t ? t.box.y + t.box.h / 2 : null].filter((v) => v != null).map((v) => toScreen(0, v)[1]);
  if (ys.every((v) => v > Hh * 0.12 && v < Hh - bottomSpace() - 20)) return;
  flyTo({ ...traceCam(lay, cam.k), x: cam.x });
}

// ---------- history (↶) ----------
const HIST = [];
function snapshot() {
  if (TR?.lay) return { k: "trace", S: TR.S, cur: TR.cur, pick: TR.pick, stack: TR.stack.map((f) => ({ ...f })), entry: TR.entry, ox: TR.lay.code.x, oy: TR.lay.code.y, cam: { ...cam } };
  return { k: "map", open: new Set(open), cam: { ...cam }, selected, showAll, step };
}
function remember() { if (!G) return; HIST.push(snapshot()); if (HIST.length > 200) HIST.shift(); histButton(); }
function histButton() { const b = document.getElementById("hist"); if (b) b.disabled = !HIST.length; }
function goBack() {
  const s = HIST.pop();
  histButton();
  if (!s) return;
  if (s.k === "view") {
    const i = asides.findIndex((a) => a.V === s.V);
    if (i >= 0) asides.length = i;
    restoreView(s.V, s.step);
    return;
  }
  if (s.k === "map") {
    if (TR) { TR = null; traceHover = null; $("trail").hidden = true; }
    if (s.showAll !== undefined && s.showAll !== showAll) { showAll = s.showAll; computeSub(); }
    if (s.step !== undefined) step = s.step;
    selected = s.selected; inspect(selected);
    setOpen(new Set(s.open), null, () => s.cam);
    bar(); transport(); report(); resetButton();
  } else {
    TR = { ...(TR ?? {}), entry: s.entry };
    setTrace(s.S, s.cur, s.ox, s.oy, { stack: s.stack, cam: s.cam, pick: s.pick });
  }
}

// ---------- moving around ----------
async function setTrace(S, cur, ox, oy, o = {}) {
  await ensureFiles([S]); await ensureFile(S.f);
  const lay = traceLayout(S, ox, oy);
  TR = { ...(TR ?? {}), stack: o.stack ?? TR?.stack ?? [], S, mode: lay.mode, lay, cur: Math.max(0, Math.min(cur, lay.stops.length - 1)), msg: null, t0: performance.now() };
  TR.pick = o.pick ?? lay.stops[TR.cur]?.items[0]?.t ?? null;
  traceHover = null;
  if (o.cam) flyTo(o.cam); else if (o.fly !== false) flyTo(traceCam(lay)); else kick();
  trail(); bar(); report();
}
function enterTrace(S, line, stack = []) {
  if (!canTrace(S)) return;
  remember();
  const entry = TR ? TR.entry : snapshot();
  const b = TR ? { x: TR.lay.code.x, y: TR.lay.code.y } : L.boxes.get(rep(S)) ?? { x: cam.x, y: cam.y };
  selected = null; hovered = hoverEdge = mapStop = null; inspect(null);
  $("settings").hidden = true; $("settingsBtn").setAttribute("aria-pressed", "false");
  TR = { ...(TR ?? {}), entry };
  const stops = S.k === "function" ? stopsOf(S) : [];
  setTrace(S, line ? Math.max(0, stops.findIndex((s) => s.line >= line)) : 0, b.x, b.y, { stack });
}
// Claude's `trace`: a call stack of node ids (outermost first) and the line to start at.
function traceFromView(tr) {
  const ns = (tr.stack ?? []).map((id) => byId.get(id)).filter(canTrace);
  if (!ns.length) return;
  const frames = ns.slice(0, -1).map((f, i) => {
    const c = Math.max(0, stopsOf(f).findIndex((s) => s.items.some((it) => it.t === ns[i + 1])));
    return { S: f, cur: c, pick: ns[i + 1] };
  });
  enterTrace(ns[ns.length - 1], tr.line ?? null, frames);
}
// Back to the map as it was when the trace began (camera, open boxes, selection). A trace
// Claude opened has no map to go back to: the map then opens at the thing you ended on.
// Steps and views that close the trace pass `false`, as they lay the map out themselves.
function exitTrace(restore = true) {
  const S = TR?.S, entry = TR?.entry;
  if (restore) remember();
  TR = null; traceHover = null; $("trail").hidden = true; cv.style.cursor = "";
  if (restore && entry && entry.k === "map" && G && L.boxes.size) {
    selected = entry.selected; inspect(selected);
    setOpen(new Set(entry.open), null, () => entry.cam);
  } else if (restore && S) {
    const s = new Set(open); openPathTo(s, S);
    selected = S; inspect(S);
    setOpen(s, null, (lay) => { const b = lay.boxes.get(rep(S)); return b ? fit(b, 0.7) : fit(lay.boxes.get(root), 0.96); });
  }
  bar(); report(); kick();
}
function traceRefresh() { if (TR?.lay) setTrace(TR.S, TR.cur, TR.lay.code.x, TR.lay.code.y, { fly: false, pick: TR.pick }); }
function moveCur(d) {
  const n = TR.lay.stops.length;
  if (!n) return;
  TR.cur = Math.max(0, Math.min(n - 1, TR.cur + d));
  TR.pick = TR.lay.stops[TR.cur].items[0].t; TR.msg = null;
  traceFollow(); trail(); bar(); report(); kick();
}
function say(msg) { TR.msg = msg; bar(); }
// Into a function (at the line that touches what you came from), a field or a type.
function stepIn(n = TR.pick, line = null) {
  if (!n) return;
  if (!canTrace(n)) { say(`<code>${esc(n.n)}</code> is library code: there is no code of it in the snapshot to step into.`); return; }
  remember();
  const t = tagOf(TR.lay, n);
  if (line == null && TR.mode === "data") line = t?.lines?.[0] ?? null;
  TR.stack.push({ S: TR.S, cur: TR.cur, pick: n });
  const stops = n.k === "function" ? stopsOf(n) : [];
  const cur = line ? Math.max(0, stops.findIndex((s) => s.line >= line)) : 0;
  setTrace(n, cur, t ? t.box.x : TR.lay.code.x, t ? t.box.y : TR.lay.code.y);
}
// Back to a caller, laid out so the thing you were in sits where its code was.
function placeAround(outer, child, cur) {
  const lay0 = traceLayout(outer, 0, 0), t = tagOf(lay0, child), c = TR.lay.code;
  setTrace(outer, cur, t ? c.x - t.box.x : c.x, t ? c.y - t.box.y : c.y);
}
function stepOut() {
  const fr = TR.stack.pop();
  if (fr) { remember(); placeAround(fr.S, TR.S, fr.cur); return; }
  if (TR.mode === "data") {
    const p = TR.S.parent;
    if (p && canTrace(p) && p.k === "type") { remember(); placeAround(p, TR.S, 0); }
    else say("This is where the trace began. Esc goes back to the map.");
    return;
  }
  const cs = TR.lay.left;
  if (cs.length === 1) goCaller(cs[0].n);
  else say(cs.length ? "Several functions call this one: click one on the left to step out to it." : "Nothing in the snapshot calls this function.");
}
function goCaller(f) {
  const top = TR.stack[TR.stack.length - 1];
  if (top && top.S === f) { stepOut(); return; }
  remember();
  const from = TR.S;
  TR.stack = [];
  placeAround(f, from, Math.max(0, stopsOf(f).findIndex((s) => s.items.some((it) => it.t === from))));
}
function toFrame(i) {
  remember();
  const fr = TR.stack[i], child = TR.stack[i + 1]?.S ?? TR.S;
  TR.stack = TR.stack.slice(0, i);
  placeAround(fr.S, child, fr.cur);
}
// From the details panel: go to anything, keeping the trail.
function jumpTo(n, line = null) {
  if (!n) return;
  if (!TR) {
    if (canTrace(n)) { enterTrace(n, line); return; }
    remember();
    const s = new Set(open); openPathTo(s, n);
    selected = n; inspect(n);
    setOpen(s, n, (lay) => { const b = lay.boxes.get(rep(n)); return b ? fit(b, 0.6) : null; });
    bar(); report();
    return;
  }
  if (n === TR.S) return;
  stepIn(n, line);
}

// ---------- the trail: where you are and how you got here ----------
function trail() {
  const el = $("trail");
  if (!TR?.lay) { el.hidden = true; return; }
  const n = TR.lay.stops.length, data = TR.mode === "data";
  el.innerHTML = `<span class="lbl">${TR.S.k === "span" ? "excerpt" : data ? "data" : "trace"}</span>
    <span class="crumbs">${TR.stack.map((f, i) => `<button class="crumb" data-i="${i}" title="${esc(pathText(f.S))}">${esc(f.S.n)}</button><span class="sep">›</span>`).join("")}<span class="here" title="${esc(pathText(TR.S))}">${esc(TR.S.n)}</span></span>
    <span class="count">${n ? `${data ? "use" : "call"} ${TR.cur + 1} / ${n}` : TR.S.k === "span" ? "" : data ? "no uses" : "no calls"}</span>
    <button class="box-btn" id="tPrev" title="Previous (↑)">▲</button><button class="box-btn" id="tNext" title="Next (↓)">▼</button>
    <button class="box-btn" id="tIn" title="Open it (Enter)">Step in ▸</button>
    <button class="box-btn" id="tOut" title="Back out (Backspace)">◂ Step out</button>
    <button class="box-btn" id="tMap" title="Back to the map (Esc)">Map</button>`;
  el.hidden = false;
  el.querySelectorAll(".crumb").forEach((b) => (b.onclick = () => toFrame(+b.dataset.i)));
  $("tPrev").onclick = () => moveCur(-1); $("tNext").onclick = () => moveCur(1);
  $("tIn").onclick = () => stepIn(); $("tOut").onclick = () => stepOut(); $("tMap").onclick = () => exitTrace();
  $("tPrev").disabled = TR.cur <= 0; $("tNext").disabled = TR.cur >= n - 1;
  $("tIn").disabled = !canTrace(TR.pick);
  $("tOut").disabled = !TR.stack.length && !TR.lay.left.length && !(data && TR.S.parent?.k === "type");
}
addEventListener("keydown", (e) => {
  if (e.target.closest?.("input, select")) return;
  if (e.altKey && e.key === "ArrowLeft") { e.preventDefault(); e.stopImmediatePropagation(); goBack(); return; }
  if (!TR?.lay) return;
  const claudeSteps = steps().length > 0 && step >= 0; // ← → stay with Claude's steps then
  const act = {
    ArrowDown: () => moveCur(1), j: () => moveCur(1), ArrowUp: () => moveCur(-1), k: () => moveCur(-1),
    Enter: () => stepIn(), Backspace: () => stepOut(), Escape: () => exitTrace(),
    Tab: () => { const s = TR.lay.stops[TR.cur]; if (s && s.items.length > 1) { const i = s.items.findIndex((it) => it.t === TR.pick); TR.pick = s.items[(i + 1) % s.items.length].t; trail(); bar(); kick(); } },
    ...(claudeSteps ? {} : { ArrowRight: () => stepIn(), ArrowLeft: () => stepOut() }),
  }[e.key];
  if (!act) return;
  e.preventDefault(); e.stopImmediatePropagation(); act();
}, true);

// ---------- text ----------
const itemText = (it) => `${esc(LINE[it.kind].label)} <code>${esc(it.t.n)}</code>${ctxOf(it.t) ? ` <span class="dim">(${esc(ctxOf(it.t))})</span>` : ""}`;
const ROLE = { caller: "called by", writer: "written by", reader: "read by" };
function traceBar() {
  const lay = TR.lay;
  if (!lay) return;
  if (TR.msg) { $("bartag").textContent = TR.mode === "data" ? "data" : "trace"; $("bartext").innerHTML = TR.msg; return; }
  const hovTag = traceHover?.caller ? lay.left.find((x) => x.n === traceHover.caller) : traceHover?.target ? tagOf(lay, traceHover.target) : null;
  if (hovTag) {
    $("bartag").textContent = ROLE[hovTag.role] ?? LINE[hovTag.kind]?.label ?? "";
    const ls = hovTag.lines ?? (hovTag.stops ?? []).map((i) => lay.stops[i].line);
    $("bartext").innerHTML = `<code>${esc(hovTag.n.n)}</code> (${esc(ctxOf(hovTag.n))})${ls.length ? " · line " + ls.slice(0, 6).join(", ") + (ls.length > 6 ? "…" : "") : ""}${hovTag.role === "caller" ? " · click to step out to it" : canTrace(hovTag.n) ? " · double-click to step in" : ""}`;
    return;
  }
  if (TR.S.k === "span") { $("bartag").textContent = "excerpt"; $("bartext").innerHTML = `<code>${esc(TR.S.f)}</code>, lines ${TR.S.a}${TR.S.b > TR.S.a ? "–" + TR.S.b : ""} · Esc or ↶ Back to return`; return; }
  const s = traceHover?.stop ?? lay.stops[TR.cur];
  if (!s) { $("bartag").textContent = "trace"; $("bartext").innerHTML = TR.mode === "data" ? `Nothing in the snapshot reads or writes <code>${esc(TR.S.n)}</code>.` : `<code>${esc(TR.S.n)}</code> makes no calls of the kinds ticked in the legend.`; return; }
  if (TR.mode === "data") {
    $("bartag").textContent = ROLE[s.tag.role];
    $("bartext").innerHTML = `<code>${esc(s.tag.n.n)}</code> (${esc(ctxOf(s.tag.n))}), line ${s.tag.lines.slice(0, 6).join(", ")} · Enter opens it there, ↓ next`;
    return;
  }
  $("bartag").textContent = s.line ? "line " + s.line : "trace";
  $("bartext").innerHTML = s.items.map(itemText).join(", ") +
    (s === lay.stops[TR.cur] ? ` · ${canTrace(TR.pick) ? "Enter steps in, " : ""}↓ next call${s.items.length > 1 ? ", Tab picks which" : ""}` : " · click to go to this call");
}
function traceReport() {
  if (!TR?.lay) return null;
  const s = TR.lay.stops[TR.cur];
  return { [TR.mode === "data" ? "data" : "function"]: pathText(TR.S), stack: TR.stack.map((f) => pathText(f.S)), line: s?.line ?? null, at: s ? s.items.map((it) => pathText(it.t)) : [] };
}
function traceStopLit(s) {
  if (!TR?.lay || TR.mode !== "code") return false;
  return s === TR.lay.stops[TR.cur] || s === traceHover?.stop || (!!traceHover?.target && s.items.some((it) => it.t === traceHover.target));
}

// ---------- drawing ----------
function curve(ax, ay, bx, by, color, alpha, lw, dash) {
  const dx = Math.max(40, (bx - ax) / 2);
  ctx.strokeStyle = rgba(color, alpha); ctx.lineWidth = lw; ctx.setLineDash(dash ?? []);
  ctx.beginPath(); ctx.moveTo(ax, ay); ctx.bezierCurveTo(ax + dx, ay, bx - dx, by, bx, by); ctx.stroke(); ctx.setLineDash([]);
  const z = 5 + lw * 1.5;
  ctx.fillStyle = rgba(color, alpha); ctx.beginPath(); ctx.moveTo(bx, by); ctx.lineTo(bx - z, by - z * 0.45); ctx.lineTo(bx - z, by + z * 0.45); ctx.fill();
}
function bez(ax, ay, bx, by, t) {
  const dx = Math.max(40, (bx - ax) / 2), u = 1 - t;
  return [u * u * u * ax + 3 * u * u * t * (ax + dx) + 3 * u * t * t * (bx - dx) + t * t * t * bx, u * u * u * ay + 3 * u * u * t * ay + 3 * u * t * t * by + t * t * t * by];
}
function drawTag(n, b, o) {
  const [x, y] = toScreen(b.x, b.y), w = b.w * cam.k, h = b.h * cam.k;
  tHit.tags.push([o.role, n, x, y, w, h]);
  if (x > W || y > Hh || x + w < 0 || y + h < 0) return;
  const s = look(n);
  ctx.fillStyle = s.fill; ctx.fillRect(x, y, w, h);
  if (s.stripe) { ctx.fillStyle = s.stripe; ctx.fillRect(x, y, Math.max(2, 3 * cam.k), h); }
  ctx.setLineDash(o.lit ? [] : s.dash ?? []);
  ctx.lineWidth = o.lit ? 2.5 : o.hov ? 2 : s.lw; ctx.strokeStyle = o.lit ? o.color : o.hov ? T.ink : s.stroke;
  ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1); ctx.setLineDash([]);
  const fs = 12 * cam.k;
  if (fs < 5) return;
  const mono = monoFont();
  ctx.textAlign = "left"; ctx.textBaseline = "top";
  ctx.font = `500 ${fs}px ${mono}`; ctx.fillStyle = s.text;
  text(prefix(n) + short(n), x + 10 * cam.k, y + 7 * cam.k, w - 30 * cam.k);
  ctx.font = `${10.5 * cam.k}px ${mono}`; ctx.fillStyle = T.dim;
  text(o.sub ?? ctxOf(n), x + 10 * cam.k, y + 24 * cam.k, w - 30 * cam.k);
  if (o.step) { ctx.font = `${13 * cam.k}px ${mono}`; ctx.textAlign = "right"; ctx.fillStyle = o.lit ? o.color : T.dim; ctx.fillText(o.step, x + w - 7 * cam.k, y + h / 2 - 7 * cam.k); }
}
function drawTrace() {
  const lay = TR?.lay;
  if (!lay) return;
  const k = cam.k, mono = monoFont(), cur = lay.stops[TR.cur];
  tHit = { tags: [], code: null, curves: [] };
  ctx.save();
  ctx.globalAlpha = Math.min(1, (performance.now() - TR.t0) / 300);
  const hovT = traceHover?.target ?? traceHover?.caller;
  // Column headings.
  ctx.font = `10px ${mono}`; ctx.fillStyle = T.dim; ctx.textBaseline = "bottom"; ctx.textAlign = "left";
  const head = (label, wx, wy) => { const [sx, sy] = toScreen(wx, wy); if (sy > 14) ctx.fillText(label.toUpperCase(), sx, sy - 6); };
  const lefts = lay.left.map((t) => t.box), rights = [...lay.right.values()].map((t) => t.box);
  if (lefts.length) head(lay.mode === "data" ? "written by" : "called by", Math.min(...lefts.map((b) => b.x)), Math.min(...lefts.map((b) => b.y)));
  if (rights.length) head(lay.mode === "data" ? "read by" : "calls, reads and writes, in the order written", lay.code.x + lay.code.w + TGAP, Math.min(...rights.map((b) => b.y)));
  // Lines: dim ones first, the current and hovered ones on top.
  const lit = (l) => l.t === hovT || (traceHover?.stop && lay.stops[l.stop] === traceHover.stop) || (l.stop === TR.cur && (!hovT || l.t === hovT));
  const order = [...lay.links].sort((p, q) => lit(p) - lit(q));
  for (const l of order) {
    const L0 = lineLook(l.kind), on = lit(l), b = l.to.box;
    let ax, ay, bx, by;
    if (l.into) { [ax, ay] = toScreen(b.x + b.w, b.y + b.h / 2); [bx, by] = toScreen(...l.into); }
    else { [ax, ay] = toScreen(...l.from); [bx, by] = toScreen(b.x, b.y + b.h / 2); }
    curve(ax, ay, bx, by, L0.color, on ? 1 : l.stop >= 0 && l.stop < TR.cur ? 0.4 : 0.22, on ? 2.2 : 1, L0.dash);
    if (l.stop >= 0) tHit.curves.push([lay.stops[l.stop], l.t, ax, ay, bx, by]);
  }
  // The code in the middle.
  const c = lay.code, [cx, cy] = toScreen(c.x, c.y), cw = c.w * k, ch = c.h * k;
  tHit.code = [cx, cy, cw, ch];
  const s = look(lay.node.k === "span" ? lay.S : lay.node);
  ctx.fillStyle = s.fill; ctx.fillRect(cx, cy, cw, ch);
  if (s.stripe) { ctx.fillStyle = s.stripe; ctx.fillRect(cx, cy, Math.max(2, 3 * k), ch); }
  const codeCited = citeTarget() === lay.S || citeTarget() === lay.node;
  ctx.lineWidth = codeCited ? 3 : 1.5; ctx.strokeStyle = codeCited ? T.orange : T.ink; ctx.strokeRect(cx + 0.5, cy + 0.5, cw - 1, ch - 1);
  // The data in question, banded.
  if (lay.hl) {
    const [a, b] = lay.hl, [, y0] = toScreen(0, lay.lineY(a) - LH / 2);
    ctx.fillStyle = rgba(T.data, 0.22); ctx.fillRect(cx, y0, cw, (b - a + 1) * LH * k);
    ctx.fillStyle = T.data; ctx.fillRect(cx, y0, Math.max(2, 3 * k), (b - a + 1) * LH * k);
  }
  ctx.font = `500 ${Math.min(13, Math.max(8, 12 * k))}px ${mono}`; ctx.textAlign = "left"; ctx.textBaseline = "top"; ctx.fillStyle = T.ink;
  const title = lay.node.k === "span" ? prefix(lay.S) + lay.S.n : prefix(lay.node) + lay.node.n;
  const used = text(title, cx + 10 * k, cy + 8 * k, cw - 20 * k);
  ctx.font = `${Math.min(11, Math.max(7, 10.5 * k))}px ${mono}`; ctx.fillStyle = T.dim;
  text(`${ctxOf(lay.S)} · ${lay.node.f}:${lay.node.a}`, cx + 10 * k + used + 12, cy + 9 * k, cw - used - 40 * k);
  drawCode(lay.node, cx, cy, cw, ch);
  // The current call: a marker in the gutter.
  if (lay.mode === "code" && cur?.line) {
    const ly = toScreen(0, lay.lineY(cur.line))[1];
    ctx.fillStyle = T.call; ctx.beginPath(); ctx.moveTo(cx - 12, ly - 6); ctx.lineTo(cx - 2, ly); ctx.lineTo(cx - 12, ly + 6); ctx.fill();
  }
  // The tags on both sides.
  const curT = new Set(cur ? cur.items.map((it) => it.t) : []);
  for (const t of [...lay.left, ...lay.right.values()]) {
    const isLeftCaller = t.role === "caller";
    const on = t.n === hovT || (!isLeftCaller && curT.has(t.n));
    const sub = t.lines?.length ? `${ctxOf(t.n)} · line ${t.lines[0]}` : undefined;
    const steppable = !isLeftCaller && canTrace(t.n);
    const isCited = citeTarget() === t.n;
    drawTag(t.n, t.box, { role: isLeftCaller ? "caller" : "target", lit: on || isCited, hov: t.n === hovT, color: isCited ? T.orange : lineLook(t.kind).color, sub, step: steppable ? (TR.pick === t.n && on ? "⏎" : "▸") : null });
  }
  ctx.restore();
}

// ---------- pointer ----------
function traceAt(sx, sy) {
  for (const [role, n, x, y, w, h] of tHit.tags) if (sx >= x && sy >= y && sx <= x + w && sy <= y + h) return role === "caller" ? { caller: n } : { target: n };
  const c = tHit.code;
  if (c && sx >= c[0] && sx <= c[0] + c[2] && sy >= c[1] && sy <= c[1] + c[3]) {
    if (TR.mode !== "code") return null;
    const F = TR.lay.node, line = F.a + Math.floor((sy - c[1] - 30 * cam.k) / (LH * cam.k));
    const col = (sx - c[0] - 12 * cam.k) / (CW * cam.k) + 1;
    const here = TR.lay.stops.filter((s) => s.line === line);
    if (here.length) { here.sort((a, b) => Math.abs((a.col + a.endCol) / 2 - col) - Math.abs((b.col + b.endCol) / 2 - col)); return { stop: here[0] }; }
    return null;
  }
  let best = null, bd = 6;
  for (const [s, t, ax, ay, bx, by] of tHit.curves) {
    let prev = [ax, ay];
    for (let i = 1; i <= 16; i++) { const p = bez(ax, ay, bx, by, i / 16), d = segDist(sx, sy, prev, p); if (d < bd) { bd = d; best = { stop: s, target: t }; } prev = p; }
  }
  return best;
}
function traceMove(sx, sy) {
  const h = traceAt(sx, sy);
  const same = (a, b) => a?.stop === b?.stop && a?.target === b?.target && a?.caller === b?.caller;
  if (same(h, traceHover)) return;
  traceHover = h; if (TR) TR.msg = null;
  cv.style.cursor = h ? "pointer" : "";
  bar(); kick();
}
function traceClick(sx, sy) {
  const h = traceAt(sx, sy);
  if (!h) return;
  if (h.caller) { goCaller(h.caller); return; }
  const lay = TR.lay;
  const i = h.stop ? lay.stops.indexOf(h.stop) : tagOf(lay, h.target)?.stops?.[0] ?? TR.cur;
  TR.cur = i; TR.pick = h.target ?? lay.stops[i].items[0].t; TR.msg = null;
  traceFollow(); trail(); bar(); report(); kick();
}
function traceDouble(sx, sy) {
  if (performance.now() - TR.t0 < 500) return; // the first click already moved somewhere new
  const h = traceAt(sx, sy);
  if (h?.target) stepIn(h.target);
  else if (h?.stop) stepIn(h.stop.items[0].t);
}
