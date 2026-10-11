// Trace: one function at a time, like stepping in a debugger (FQ-08). Its code in the
// middle; the functions that call it on the left; everything it calls, reads or writes
// on the right, each placed at the line that does it, in the order written. Next and
// Prev walk those calls; Step in opens the callee with the trail of where you came from;
// Step out goes back. All of it is mechanical: where each call is written is a fact in
// the snapshot (ADR-0035). Claude opens it with `show({view: {trace}})`, also per step.
// Loaded before app.js; everything here runs only after the graph has loaded.
"use strict";
let TR = null;         // { fn, lay, cur, pick, stack: [{ fn, cur, pick }], t0, msg }
let traceHover = null; // { stop } | { target } | { caller }
let tHit = { tags: [], code: null, curves: [] };
const stopCache = new Map();
const TGAP = 170;

function traceReset() { TR = null; traceHover = null; stopCache.clear(); const t = document.getElementById("trail"); if (t) t.hidden = true; }
const canTrace = (n) => !!n && n.k === "function" && !!n.f && !!n.a;
const traceFading = () => !!TR && performance.now() - TR.t0 < 320;

// The calls and data uses written inside a function (its closures included), grouped
// by where they're written, in source order. Only the line kinds ticked in the legend.
function stopsOf(F) {
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
    const x = by.get(f) ?? by.set(f, { f, kind: e.kind, lines: [] }).get(f);
    for (const s of e.sites) x.lines.push(s[0]);
  }
  return [...by.values()].sort((a, b) => pathText(a.f).localeCompare(pathText(b.f)));
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

// ---------- layout (world units; the code's top-left at ox, oy) ----------
function traceLayout(F, ox, oy) {
  const sz = codeSize(F);
  const code = { x: ox, y: oy, w: sz.width, h: sz.height };
  const lineY = (l) => code.y + 30 + (l - F.a) * LH + LH / 2;
  const stops = stopsOf(F);
  const targets = new Map();
  stops.forEach((s, i) => {
    for (const it of s.items) {
      const t = targets.get(it.t) ?? targets.set(it.t, { n: it.t, kind: it.kind, stops: [] }).get(it.t);
      if (!t.stops.includes(i)) t.stops.push(i);
    }
  });
  // Each target sits level with the first line that uses it, pushed down when they crowd.
  let y = code.y;
  const rx = code.x + code.w + TGAP;
  for (const t of targets.values()) {
    const s = tagSize(t.n), first = stops[t.stops[0]];
    y = Math.max(first.line ? lineY(first.line) - s.h / 2 : y, y);
    t.box = { x: rx, y, w: s.w, h: s.h };
    y += s.h + 8;
  }
  const callers = callersOf(F);
  let cy = code.y;
  for (const c of callers) { const s = tagSize(c.f); c.box = { x: code.x - TGAP - s.w, y: cy, w: s.w, h: s.h }; cy += s.h + 8; }
  return { F, code, lineY, stops, targets, callers };
}
function traceBounds(lay) {
  // The right column is at least as wide as its heading.
  const head = lay.targets.size ? [{ x: lay.code.x + lay.code.w + TGAP, y: lay.code.y, w: 340, h: 1 }] : [];
  return union([lay.code, ...head, ...[...lay.targets.values()].map((t) => t.box), ...lay.callers.map((c) => c.box)].map((b) => ({ x: b.x, y: b.y, w: b.w, h: b.h })));
}
// The camera: the three columns across the screen when they fit, else the code from the
// left edge; vertically, the current call and what it calls.
function traceCam(lay, keepK) {
  const b = traceBounds(lay);
  const k = keepK ?? Math.max(0.45, Math.min(1.2, (W * 0.96) / b.w));
  const x = b.w * k <= W ? b.x + b.w / 2 : lay.code.x - (lay.callers.length ? 60 : 30) + W / 2 / k;
  const viewH = Hh / k;
  let y;
  if (b.h <= viewH * 0.9) y = b.y + b.h / 2;
  else {
    const s = lay.stops[TR?.cur ?? 0], t = s && lay.targets.get(s.items[0].t);
    const want = s?.line ? (lay.lineY(s.line) + (t ? t.box.y + t.box.h / 2 : lay.lineY(s.line))) / 2 : lay.code.y;
    y = Math.min(Math.max(want, b.y - 30 + viewH / 2), b.y + b.h + 30 - viewH / 2);
  }
  return { x, y, k };
}
// Keep the current call on screen without jumping around when it already is.
function traceFollow() {
  const lay = TR.lay, s = lay.stops[TR.cur];
  if (!s?.line) return;
  const t = lay.targets.get(TR.pick ?? s.items[0].t);
  const ys = [toScreen(0, lay.lineY(s.line))[1], t ? toScreen(0, t.box.y + t.box.h / 2)[1] : null].filter((v) => v != null);
  if (ys.every((v) => v > Hh * 0.12 && v < Hh * 0.88)) return;
  flyTo({ ...traceCam(lay, cam.k), x: cam.x });
}

// ---------- moving around ----------
async function setTrace(F, cur, ox, oy, fly = true, stack = null) {
  await ensureFiles([F]);
  const lay = traceLayout(F, ox, oy);
  TR = { ...(TR ?? {}), stack: stack ?? TR?.stack ?? [], fn: F, lay, cur: Math.max(0, Math.min(cur, lay.stops.length - 1)), msg: null, t0: performance.now() };
  TR.pick = lay.stops[TR.cur]?.items[0]?.t ?? null;
  traceHover = null;
  if (fly) flyTo(traceCam(lay)); else kick();
  trail(); bar(); report();
}
function enterTrace(F, line, stack = []) {
  if (!canTrace(F)) return;
  const b = L.boxes.get(rep(F)) ?? { x: cam.x, y: cam.y };
  selected = null; hovered = hoverEdge = mapStop = null; inspect(null);
  $("settings").hidden = true; $("settingsBtn").setAttribute("aria-pressed", "false");
  const stops = stopsOf(F);
  setTrace(F, line ? Math.max(0, stops.findIndex((s) => s.line >= line)) : 0, b.x, b.y, true, stack);
}
// Claude's `trace`: a call stack of node ids (outermost first) and the line to start at.
function traceFromView(tr) {
  const ns = (tr.stack ?? []).map((id) => byId.get(id)).filter(canTrace);
  if (!ns.length) return;
  const frames = ns.slice(0, -1).map((f, i) => {
    const c = Math.max(0, stopsOf(f).findIndex((s) => s.items.some((it) => it.t === ns[i + 1])));
    return { fn: f, cur: c, pick: ns[i + 1] };
  });
  enterTrace(ns[ns.length - 1], tr.line ?? null, frames);
}
// Back to the map, which shows where the function you ended on lives (its boxes open,
// its code shown, selected). Steps and views that close the trace pass `false`, as
// they lay the map out themselves.
function exitTrace(restore = true) {
  const F = TR?.fn;
  TR = null; traceHover = null; $("trail").hidden = true; cv.style.cursor = "";
  if (restore && F) {
    const s = new Set(open); openPathTo(s, F); s.add(F);
    selected = F; inspect(F);
    setOpen(s, null, (lay) => { const b = lay.boxes.get(rep(F)); return b ? fit(b, 0.7) : fit(lay.boxes.get(root), 0.96); });
  }
  bar(); report(); kick();
}
function traceRefresh() { if (TR) setTrace(TR.fn, TR.cur, TR.lay.code.x, TR.lay.code.y, false); }
function moveCur(d) {
  const n = TR.lay.stops.length;
  if (!n) return;
  TR.cur = Math.max(0, Math.min(n - 1, TR.cur + d));
  TR.pick = TR.lay.stops[TR.cur].items[0].t; TR.msg = null;
  traceFollow(); trail(); bar(); report(); kick();
}
function say(msg) { TR.msg = msg; bar(); }
function stepIn(n = TR.pick) {
  if (!n) return;
  if (!canTrace(n)) { say(`<code>${esc(n.n)}</code> is ${LIBK.has(n.k) ? "library code" : "data"}: there is no code of it in the snapshot to step into.`); return; }
  const t = TR.lay.targets.get(n);
  TR.stack.push({ fn: TR.fn, cur: TR.cur, pick: n });
  setTrace(n, 0, t ? t.box.x : TR.lay.code.x, t ? t.box.y : TR.lay.code.y);
}
// Back to a caller, laid out so the function you were in sits where its code was.
function placeAround(caller, child, cur) {
  const lay0 = traceLayout(caller, 0, 0), t = lay0.targets.get(child), c = TR.lay.code;
  setTrace(caller, cur, t ? c.x - t.box.x : c.x, t ? c.y - t.box.y : c.y);
}
function stepOut() {
  const fr = TR.stack.pop();
  if (fr) { placeAround(fr.fn, TR.fn, fr.cur); return; }
  const cs = TR.lay.callers;
  if (cs.length === 1) goCaller(cs[0].f);
  else say(cs.length ? "Several functions call this one: click one on the left to step out to it." : "Nothing in the snapshot calls this function.");
}
function goCaller(f) {
  const top = TR.stack[TR.stack.length - 1];
  if (top && top.fn === f) { stepOut(); return; }
  const from = TR.fn;
  TR.stack = [];
  placeAround(f, from, Math.max(0, stopsOf(f).findIndex((s) => s.items.some((it) => it.t === from))));
}
function toFrame(i) {
  const fr = TR.stack[i], child = TR.stack[i + 1]?.fn ?? TR.fn;
  TR.stack = TR.stack.slice(0, i);
  placeAround(fr.fn, child, fr.cur);
}

// ---------- the trail: where you are and how you got here ----------
function trail() {
  const el = $("trail");
  if (!TR) { el.hidden = true; return; }
  const n = TR.lay.stops.length;
  el.innerHTML = `<span class="lbl">trace</span>
    <span class="crumbs">${TR.stack.map((f, i) => `<button class="crumb" data-i="${i}" title="${esc(pathText(f.fn))}">${esc(f.fn.n)}</button><span class="sep">›</span>`).join("")}<span class="here" title="${esc(pathText(TR.fn))}">${esc(TR.fn.n)}</span></span>
    <span class="count">${n ? `call ${TR.cur + 1} / ${n}` : "no calls"}</span>
    <button class="box-btn" id="tPrev" title="Previous call (↑)">▲</button><button class="box-btn" id="tNext" title="Next call (↓)">▼</button>
    <button class="box-btn" id="tIn" title="Open the function this line calls (Enter)">Step in ▸</button>
    <button class="box-btn" id="tOut" title="Back to the caller (Backspace)">◂ Step out</button>
    <button class="box-btn" id="tMap" title="Back to the whole system (Esc)">Map</button>`;
  el.hidden = false;
  el.querySelectorAll(".crumb").forEach((b) => (b.onclick = () => toFrame(+b.dataset.i)));
  $("tPrev").onclick = () => moveCur(-1); $("tNext").onclick = () => moveCur(1);
  $("tIn").onclick = () => stepIn(); $("tOut").onclick = () => stepOut(); $("tMap").onclick = () => exitTrace();
  $("tPrev").disabled = TR.cur <= 0; $("tNext").disabled = TR.cur >= n - 1;
  $("tIn").disabled = !canTrace(TR.pick);
  $("tOut").disabled = !TR.stack.length && !TR.lay.callers.length;
}
addEventListener("keydown", (e) => {
  if (!TR || e.target.closest?.("input, select")) return;
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
function traceBar() {
  const lay = TR.lay;
  if (TR.msg) { $("bartag").textContent = "trace"; $("bartext").innerHTML = TR.msg; return; }
  if (traceHover?.caller) {
    const c = lay.callers.find((x) => x.f === traceHover.caller);
    $("bartag").textContent = "called by"; $("bartext").innerHTML = `<code>${esc(c.f.n)}</code> (${esc(ctxOf(c.f))})${c.lines.length ? ", line " + c.lines.join(", ") : ""} · click to step out to it`;
    return;
  }
  if (traceHover?.target) {
    const t = lay.targets.get(traceHover.target);
    $("bartag").textContent = LINE[t.kind].label;
    $("bartext").innerHTML = `<code>${esc(t.n.n)}</code> (${esc(ctxOf(t.n))}) · line ${t.stops.map((i) => lay.stops[i].line).join(", ")}${canTrace(t.n) ? " · double-click to step in" : ""}`;
    return;
  }
  const s = traceHover?.stop ?? lay.stops[TR.cur];
  if (!s) { $("bartag").textContent = "trace"; $("bartext").innerHTML = `<code>${esc(TR.fn.n)}</code> makes no calls of the kinds ticked in the legend.`; return; }
  $("bartag").textContent = s.line ? "line " + s.line : "trace";
  $("bartext").innerHTML = s.items.map(itemText).join(", ") +
    (s === lay.stops[TR.cur] ? ` · ${canTrace(TR.pick) ? "Enter steps in, " : ""}↓ next call${s.items.length > 1 ? ", Tab picks which" : ""}` : " · click to go to this call");
}
function traceReport() {
  if (!TR?.lay) return null;
  const s = TR.lay.stops[TR.cur];
  return { function: pathText(TR.fn), stack: TR.stack.map((f) => pathText(f.fn)), line: s?.line ?? null, calls: s ? s.items.map((it) => pathText(it.t)) : [] };
}
function traceStopLit(s) {
  if (!TR?.lay) return false;
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
  const hovT = traceHover?.target, hovC = traceHover?.caller;
  const litT = new Set([...(cur ? cur.items.map((it) => it.t) : []), ...(hovT ? [hovT] : []), ...(traceHover?.stop ? traceHover.stop.items.map((it) => it.t) : [])]);
  // Column headings.
  ctx.font = `10px ${mono}`; ctx.fillStyle = T.dim; ctx.textBaseline = "bottom"; ctx.textAlign = "left";
  const head = (label, wx, wy) => { const [sx, sy] = toScreen(wx, wy); if (sy > 14) ctx.fillText(label.toUpperCase(), sx, sy - 6); };
  if (lay.callers.length) head("called by", lay.callers[0].box.x, lay.code.y);
  if (lay.targets.size) head("calls, reads and writes, in the order written", lay.code.x + lay.code.w + TGAP, lay.code.y);
  // Callers → this function.
  const [inX, inY] = toScreen(lay.code.x, lay.code.y + 15);
  for (const c of lay.callers) {
    const [ax, ay] = toScreen(c.box.x + c.box.w, c.box.y + c.box.h / 2);
    const L0 = lineLook(c.kind);
    curve(ax, ay, inX, inY, L0.color, hovC === c.f ? 1 : 0.45, hovC === c.f ? 2.2 : 1, L0.dash);
  }
  // Each call → what it calls: dim ones first, the current and hovered ones on top.
  const lit = (s, it) => (s === cur && (!hovT || it.t === hovT)) || s === traceHover?.stop || it.t === hovT;
  const all = [];
  lay.stops.forEach((s, i) => s.items.forEach((it) => all.push([s, i, it])));
  all.sort((p, q) => lit(p[0], p[2]) - lit(q[0], q[2]));
  for (const [s, i, it] of all) {
    const t = lay.targets.get(it.t);
    const [ax, ay] = toScreen(lay.code.x + lay.code.w, s.line ? lay.lineY(s.line) : lay.code.y + lay.code.h - 10);
    const [bx, by] = toScreen(t.box.x, t.box.y + t.box.h / 2);
    const L0 = lineLook(it.kind), on = lit(s, it);
    curve(ax, ay, bx, by, L0.color, on ? 1 : i < TR.cur ? 0.4 : 0.2, on ? 2.2 : 1, L0.dash);
    tHit.curves.push([s, it.t, ax, ay, bx, by]);
  }
  // The function's code.
  const c = lay.code, [cx, cy] = toScreen(c.x, c.y), cw = c.w * k, ch = c.h * k;
  tHit.code = [cx, cy, cw, ch];
  const s = look(lay.F);
  ctx.fillStyle = s.fill; ctx.fillRect(cx, cy, cw, ch);
  ctx.fillStyle = s.stripe; ctx.fillRect(cx, cy, Math.max(2, 3 * k), ch);
  ctx.lineWidth = 1.5; ctx.strokeStyle = T.ink; ctx.strokeRect(cx + 0.5, cy + 0.5, cw - 1, ch - 1);
  ctx.font = `500 ${Math.min(13, Math.max(8, 12 * k))}px ${mono}`; ctx.textAlign = "left"; ctx.textBaseline = "top"; ctx.fillStyle = T.ink;
  const used = text("ƒ " + lay.F.n, cx + 10 * k, cy + 8 * k, cw - 20 * k);
  ctx.font = `${Math.min(11, Math.max(7, 10.5 * k))}px ${mono}`; ctx.fillStyle = T.dim;
  text(`${ctxOf(lay.F)} · ${lay.F.f}:${lay.F.a}`, cx + 10 * k + used + 12, cy + 9 * k, cw - used - 40 * k);
  drawCode(lay.F, cx, cy, cw, ch);
  // The current call: a marker in the gutter.
  if (cur?.line) {
    const ly = toScreen(0, lay.lineY(cur.line))[1];
    ctx.fillStyle = T.call; ctx.beginPath(); ctx.moveTo(cx - 12, ly - 6); ctx.lineTo(cx - 2, ly); ctx.lineTo(cx - 12, ly + 6); ctx.fill();
  }
  // Callers and targets.
  for (const cl of lay.callers) drawTag(cl.f, cl.box, { role: "caller", hov: hovC === cl.f, lit: hovC === cl.f, color: T.call, sub: `${ctxOf(cl.f)}${cl.lines.length ? " · line " + cl.lines[0] : ""}` });
  for (const t of lay.targets.values()) {
    const on = litT.has(t.n);
    drawTag(t.n, t.box, { role: "target", lit: on, hov: hovT === t.n, color: lineLook(t.kind).color, step: canTrace(t.n) ? (TR.pick === t.n && cur?.items.some((it) => it.t === t.n) ? "⏎" : "▸") : null });
  }
  ctx.restore();
}

// ---------- pointer ----------
function traceAt(sx, sy) {
  for (const [role, n, x, y, w, h] of tHit.tags) if (sx >= x && sy >= y && sx <= x + w && sy <= y + h) return role === "caller" ? { caller: n } : { target: n };
  const c = tHit.code;
  if (c && sx >= c[0] && sx <= c[0] + c[2] && sy >= c[1] && sy <= c[1] + c[3]) {
    const F = TR.lay.F, line = F.a + Math.floor((sy - c[1] - 30 * cam.k) / (LH * cam.k));
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
  const i = h.stop ? lay.stops.indexOf(h.stop) : lay.targets.get(h.target).stops[0];
  TR.cur = i; TR.pick = h.target ?? lay.stops[i].items[0].t; TR.msg = null;
  traceFollow(); trail(); bar(); report(); kick();
}
function traceDouble(sx, sy) {
  if (performance.now() - TR.t0 < 500) return; // the first click already moved somewhere new
  const h = traceAt(sx, sy);
  if (h?.target) stepIn(h.target);
  else if (h?.stop) stepIn(h.stop.items[0].t);
}
