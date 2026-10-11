// The popped-out details window: shows what the main window sends, and sends clicked
// names back so the main window goes there.
"use strict";
const { emit, listen } = window.__TAURI__.event;
const el = document.getElementById("detail");
listen("detail", (e) => {
  const d = e.payload;
  el.querySelector(".dkind").textContent = d.kind;
  el.querySelector(".dname").textContent = d.name;
  el.querySelector(".dwhere").textContent = d.where;
  el.querySelector(".dbody").innerHTML = d.html;
});
el.addEventListener("click", (e) => {
  const a = e.target.closest("a[data-go]");
  if (!a) return;
  e.preventDefault();
  emit("detail-go", { id: +a.dataset.go, line: a.dataset.line ? +a.dataset.line : null });
});
addEventListener("beforeunload", () => { emit("detail-closed"); });
emit("detail-ready");
