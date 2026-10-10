# Build questions (backend)

The decisions needed before building the backend (ADR-0030). They were drafted by three agents (solver, MCP tool API, architecture/stack/testing) on 2026-10-09 and merged and deduplicated here, **most-blocking first**. They're asked one at a time; each answer is recorded (ADR or ruling) and the status updated here.

Real code references are to Fletcher @ `cd5d6ff`. **Rec** = the drafting agent's recommendation, not a decision.

Status key: **OPEN** · **ANSWERED → ADR-XXXX / ruling**

---

## Round 1: Shape of the system

### BQ-01: Process layout and how Claude Code connects — ANSWERED → ADR-0031
**Ruling:** a standalone app (double-click `.exe`) that owns the graphs and the window. You or Claude load repos. Claude drives 100% of it via MCP. The backend crates stay UI-free (ADR-0030).

### BQ-01b: How Claude Code reaches the running app — ANSWERED (b), recorded in ADR-0031
**Ruling:** a tiny `insight mcp` stdio bridge, registered once (`claude mcp add insight -- insight mcp`). It forwards to the running app and launches the app if it isn't running.
- Options:
  - (a) The app hosts a local MCP endpoint over HTTP on 127.0.0.1 (`claude mcp add --transport http insight http://127.0.0.1:<port>/mcp`). Claude can only connect while the app is running.
  - (b) A tiny `insight mcp` stdio bridge, registered once with `claude mcp add insight -- insight mcp`. It forwards to the running app, and **launches the app if it isn't running**.
  - (c) Both.
- Rec: (b). It's the default transport every agent supports (Claude Code and Codex), and "Claude opens the app for me" falls out for free. (a) can be added later if useful.

*(Original options, kept for history:)*
Is the backend a headless `insight` program, separate from the window? And how does Claude Code reach it?
- Case: `claude` in Fletcher calls `focus(graftNode)`. If the backend lives inside the Tauri app, calls fail unless the window is open. Two sessions (Claude Code + Codex) must not each build the graph.
- Options:
  - (a) One Tauri app: solver + MCP + UI.
  - (b) A headless `insight` binary that Claude Code launches over stdio (`claude mcp add insight -- insight mcp`), with the window as a separate client.
  - (c) (b), but the stdio process is a thin shim that attaches to one long-lived backend daemon.
  - (d) The backend hosts an HTTP MCP endpoint directly.
- Rec: (b) now (zero config; snapshots are cached on disk, so a cold start is cheap), moving to (c) if two simultaneous sessions become real. The architecture agent recommended (b); the tool-API agent recommended (c).

### BQ-02: The first runnable milestone — ANSWERED (ruling)
**Ruling:** M0 → M1 → M2 → app. The user flagged that `--to-effect write` puts semantics in the solver (ADR-0023), so **M1 uses an explicit target with pure reachability**: `insight path "HistoryTree › nodeUp" --to "fsx::write_atomic"`. It passes when every receipt in steps 1–12 of the drag-graft example is on the printed path. Semantic targets come via tags (BQ-02b).

### BQ-02b: Tags: declared meaning, parsed statically — ANSWERED → ADR-0032
**Ruling:** tags are "the compiler". Claude tags inline by default (`@insight tag: …` above the symbol); the sidecar `.insight/tags.toml` is for dependency symbols, with simple `*` globs; plain names only for now.

### BQ-02c: Making sure Claude remembers to tag — ANSWERED (a + b + c)
**Tagging modes (user, follow-up):** tagging is opt-in per repo, not everywhere. A setting in `.insight/config.toml`:
- `tagging = "always"`: Claude tags as it writes code;
- `"on-request"`: only when you ask, e.g. `/insight-tag` or "tag this";
- `"never"`: no inline tags; the sidecar can still be used by hand.

The standing instruction (a) reads this setting, and `tag_coverage` (c) only nags in `always` mode. **Default: `on-request`** (Claude's pick, not yet confirmed).
**Ruling:** (a) a standing instruction (MCP server instructions plus an `insight init` snippet in CLAUDE.md/AGENTS.md); (b) a `/insight-tag` command that tags the current uncommitted diff; (c) a mechanical `tag_coverage` tool listing untagged candidates for Claude to judge. (d) hooks only if needed. Also clarified: tags are extras. Language semantics live in plugins (ADR-0033).

*(Original BQ-02c text, kept for history:)*
User: "My main worry is Claude won't remember to do this. Or it needs to be _very_ easy to say 'hey Claude, don't forget to annotate this with insight'."
- Options (they can be combined):
  - (a) **Standing instruction:** Insight's MCP server instructions plus a snippet that `insight init` adds to the repo's CLAUDE.md/AGENTS.md: "when you add or change code that touches the outside world or matters for understanding, add `@insight tag:` comments."
  - (b) **One-word command:** a Claude Code skill/slash command `/insight-tag` that tags everything in the current uncommitted diff. Saying "tag it" is all you do.
  - (c) **A mechanical coverage tool:** `tag_coverage` lists, deterministically, what in the current diff calls into a dependency symbol or is a new function with no tag nearby. These are candidates, not judgments, and Claude decides.
  - (d) **An automatic reminder:** a Claude Code hook that runs `tag_coverage` when Claude finishes a turn that changed code and, if something's untagged, reminds it once.
- Rec: (a) + (b) + (c) now, and (d) if (a)–(c) still miss things in practice.

*(Original BQ-02b text, kept for history:)*
User: "maybe this is when the doc comes into play… you simply define what you want in the code file itself (and that gets parsed) or a standalone file… It needs to be extendable."
- Proposal:
  - A **tag** is any label (`disk-write`, `ui-gesture`, `logger`…) attached to a symbol.
  - It's declared **inline** (`/// @insight tag: disk-write` directly above a function), or in a committed **sidecar** `.insight/tags.toml` (`[[tag]] name = "disk-write"; match = ["std::fs::write", …]`) for code you can't annotate (std, npm).
  - The solver parses tags as data, matching by exact symbol path or by the comment directly above.
  - Queries can filter by tag (`--to-tag disk-write`). Claude writes most tags; you can too.
  - No code change is needed for new tags, in any language.
- Open details:
  - whether `match` allows simple globs (`std::fs::*`);
  - whether tags can carry values (`tag: logger(level=error)`);
  - whether Claude may write tags into your source files or only into the sidecar.

*(Original BQ-02 options, kept for history:)*
What's the first thing that runs, and what's its pass condition?
- Options:
  - M0: `insight build <fletcher> cd5d6ff` prints stats (files, functions including anonymous ones, edges, time).
  - M1: `insight path "HistoryTree › nodeUp" --to-effect write` prints the drag chain through both `onGraft` wirings → `applySnap` → `invoke("set_fletcher_chain")` → `fsx::write_atomic`, each hop with its receipt.
  - M2: M1 as an MCP tool inside Claude Code, text only.
- Rec: M0 → M1 (pass: every receipt in steps 1–12 of the drag-graft example is on the path) → M2. UI after that.

### BQ-03: Cargo workspace layout — DECIDED BY CLAUDE (user delegated: "I don't program in Rust. I want you to follow best practices")
**Ruling:** one Cargo workspace (`resolver = "3"`, edition 2024, shared `[workspace.dependencies]` and `[workspace.lints]`), with crates under `crates/`:
- `insight-model`: the standard representation (nodes, edges, receipts, IDs). No I/O.
- `insight-solver`: the build pipeline plus the `LanguagePlugin` / `LinkPlugin` traits.
- `insight-lang-ts`, `insight-lang-rust`, `insight-link-tauri`: one crate per plugin, which is the hard boundary of ADR-0033.
- `insight-engine`: snapshot storage plus queries.
- `insight-mcp`: the MCP tools and the stdio bridge.
- `insight-cli`: the `insight` binary.

`insight-contract` (window types) is added when the window work starts. The Tauri app lives in `app/` and is excluded from `default-members`, so `cargo test` never builds the UI. Start with this split rather than one crate, because the plugin boundary is architectural, not a refactor for later.

### BQ-03 (original question)
- Options:
  - (a) Many crates: `insight-model`, `insight-contract`, `insight-solver`, `insight-lang-ts`, `insight-lang-rust`, `insight-link-tauri`, `insight-engine` (store + query), `insight-mcp`, and the `insight` binary. The window lives in `app/`, outside `default-members`, so backend tests never compile WebView2.
  - (b) Three crates: core / plugins / bin.
  - (c) One crate until it hurts.
- Rec: (a).

### BQ-04: What a plugin is physically — SUPERSEDED → ADR-0034
**Ruling (user, via BQ-05):** each language is an extractor written in that language, using its official toolchain, emitting the standard representation as versioned NDJSON. The Rust core never parses languages. The Rust extractor runs in-process; link passes stay in Rust.

*(Earlier ruling, superseded:)* DECIDED BY CLAUDE (same delegation as BQ-03)
**Ruling:** (a) compile-time Rust crates implementing `LanguagePlugin` / `LinkPlugin`, registered in the binary. Revisit only if someone else ever writes plugins. A plugin *may* internally drive an external engine (e.g. the TS checker, BQ-05) without changing this.

### BQ-04 (original question)
- Options:
  - (a) Compile-time Rust crates implementing a `LanguagePlugin` / `LinkPlugin` trait.
  - (b) WASM loaded at runtime.
  - (c) Separate executables speaking a fact protocol.
- Rec: (a). One user, no third-party plugins (ADR-0025), and link passes need TS and Rust facts in one address space.

## Round 2: Solver front ends

### BQ-05: The TypeScript engine — ANSWERED → ADR-0034
**Ruling:** the TS compiler API in a Node extractor written in TypeScript (`extractors/ts/`), with `tsgo` swapped in later. The general rule: every language uses its own toolchain.

*(Original question:)*
- Case: connecting `onto.children.push(node.id)` (`App.tsx:4966`) to `HistNode.children` needs type inference through `Map.get`. Scope analysis alone can't do it, and field-level state (Q-20 ruling) requires it.
- Options:
  - (a) The real TypeScript checker (compiler API) in a Node sidecar, emitting facts.
  - (b) `oxc` in Rust: fast, one binary, scopes and read/write flags, but **no type checker**.
  - (c) tree-sitter plus our own resolver.
  - (d) The `scip-typescript` index.
  - (e) The native TS port (`tsgo`), when its API is usable.
- Rec: the solver agent says (a), because field-level state is impossible without types. The architecture agent says (b), for one binary and no Node or PATH surprises. Needs your call.

### BQ-06: The Rust engine and macro expansion — ANSWERED (c)
**Ruling:** rust-analyzer (ADR-0034), with proc macros and build scripts **on**. User: "if my program builds it should be possible". `cargo build` already runs the same dependency code. *(The expansion-off fallback was dropped by ADR-0036: snapshots must build, so expansion always works.)*

*(Original question:)*
- Case: `#[tauri::command] fn set_fletcher_chain` is registered only inside `tauri::generate_handler![…]` (`lib.rs:5029`). `#[derive(Serialize)]` generates the field reads. Neither is visible without expanding macros, which runs dependency code.
- Options:
  - (a) rust-analyzer as a library (`ra_ap_*`), with proc macros and build scripts **on**.
  - (b) The same, with them **off**.
  - (c) A rustc driver (nightly, full compile).
  - (d) tree-sitter or `syn` plus our own resolver.
- Rec: (a), falling back to (b) when expansion fails.

### BQ-07: Where a commit's bytes and dependencies come from — ANSWERED (a), with the user's constraints
**Ruling:** sources are read exactly from git (`gix`); dependencies are materialized per lockfile, falling back to "no dependencies" (unresolved externals) if that fails. User constraints: "build as needed / have the LLM build as needed… efficient… cache without bloat… if it requires something crazy like downloading 20 GB it should say first."
**Efficiency rules (decided by Claude, delegated):**
1. **Reuse when it's provably identical.** If the commit's lockfiles (`package-lock.json`, `Cargo.lock`) match the working tree's byte for byte, use your existing `node_modules`/`target` directly. That's the common case (asking about HEAD), and it costs nothing extra.
2. **Otherwise share by lockfile hash:** one dependency install per distinct lockfile, reused by every commit that has it. Use the global package caches (npm cache, `~/.cargo/registry`) so nothing is downloaded twice.
3. **Size cap with LRU eviction:** a total cache budget (default 10 GB, configurable), with the least recently used installs deleted first. Snapshot graphs are small and kept longer.
4. **Ask first when it's big:** before any install or build estimated over 1 GB to download, or over ~3 minutes, Insight returns a "needs confirmation" result with the estimate, instead of doing it. Claude relays it to you.
5. **Builds are on demand only:** triggered by you in the UI or by Claude through MCP, never in the background on every commit.

*(Original question:)*
- Case: Fletcher's working tree is dirty right now and `core.autocrlf=true`. Reading from disk would label uncommitted edits as `cd5d6ff`. Types for `useState` and `invoke` need `node_modules`, and rust-analyzer needs crates pinned by `Cargo.lock`.
- Options:
  - (a) Read sources from git's object store (`gix`) for exact bytes, and materialize dependencies in a cached worktree (`npm ci --ignore-scripts`, `cargo fetch`, keyed by lockfile hash).
  - (b) Reuse the live checkout's `node_modules` and `target`.
  - (c) No dependencies: external symbols stay unresolved.
- Rec: (a), falling back to (c).

### BQ-08: Code that doesn't parse or type-check — SUPERSEDED → ADR-0036
**Ruling:** refuse. A snapshot must build (`cargo check`, `tsc --noEmit`, …). If it doesn't, Insight says "build first". *(Earlier ruling "build anyway with diagnostics" reversed.)*

*(Original question:)*
- Options:
  - (a) Fail the build.
  - (b) Build anyway, with per-file diagnostics, `unresolved` stubs, and nodes in error ranges flagged `degraded`.
  - (c) Skip broken files.
- Rec: (b), which is "incomplete, never wrong" (TB-029).

## Round 3: The fact schema (the solver's output)

**All of Round 3 was DECIDED BY CLAUDE (user delegated: "decide those yourself, I really don't know"), within ADR-0033/0034.** These are internals of the standard representation and the extractors. The individual questions below are kept for history.

| BQ | Ruling |
|---|---|
| 09 | A closed set. **Nodes:** `container{file\|module\|type\|function\|block}`, `state{var\|field\|element\|static\|cell}`, `external`, `unresolved`. **Edges:** `contains`, `call`, `read`, `write`, `access` (may write), `binds`, `value-flow`, `implements`, `link`, `exit`. Each edge carries {id, kind, from, to, resolution: exact\|possible\|unresolved, receipt}. |
| 10 | IDs are location paths with context names and per-name ordinals (`App.tsx › MainApp › useEffect#14`, `HistoryTree › nodeDown › =>`), scoped internally by (repo, snapshot). The agent may type any unique suffix; ambiguity returns candidates. |
| 11 | Repo-relative, forward-slash, case-exact git paths. Spans are UTF-8 byte ranges plus 1-based lines, converted to UTF-16 only at the window boundary. |
| 12 | A receipt = {snapshot, file, byte range, line/col range, minimal expression text, enclosing statement text, innermost container ID}. Spans are exact (immutable snapshots), so there's no quote matching. |
| 13 | State identity = the declaration symbol the language's checker resolves to (`h.current` ≡ `hist.current.current`). Collection contents are a pseudo-field (`nodes[*]`). Instances of one type merge, marked `possible-instance`. |
| 14 | Writes come from language semantics: Rust `&mut self` or a mutable deref is a `write`; interior mutability and TS method calls on state (no mutability info in TS types) are `access`. *Visible effect: TS `children.push(…)` reads as "may write".* |
| 15 | One-hop value flow: an indirect call gets `possible` edges to every function value written to that symbol (assignments, fields, arguments, JSX props), plus an "absent" candidate. Returned function literals are followed one level, which handles curried handlers. *Visible effect: `onGraft?.(…)` shows main / pop-out / none.* |
| 16 | JSX is language syntax (TSX desugaring), so `<C p={v}/>` is a `render` call to `C` and binds `v` to `p`. Callbacks handed to external code get a `value-flow` into the external leaf. Who calls them later is for the model. |
| 17 | Names destructured from one call share one `cell` (`[histVersion, setHistVersion]`, `(tx, rx)`). Calling a function sibling is `access`; reading another sibling is `read`. |
| 18 | Every `if`/`else`/`switch` case/`match` arm/ternary/`&&` JSX guard/loop/`try`/`catch` is a `block` container with its condition text. `return`/`?`/`throw` are `exit` edges (ADR-0008). |
| 19 | Dynamic dispatch = one edge to the trait or interface method plus `implements` edges, expanded at query time. Narrowing comes later. |
| 20 | Analyze all git-tracked source in the TS projects and Cargo members, tests included. One external leaf per referenced symbol; external globals are state. `std` counts as a library (ADR-0033). |
| 21 | The Tauri link is command level: the `invoke` string = the `#[tauri::command]` fn name, flagged if it's missing from `generate_handler!`. Argument-name mapping (camelCase) is for the model. |
| 22 | Same-literal links between identical strings in argument, property or comparison positions, workspace-wide. Literals with more than 25 occurrences, or shorter than 3 characters, are recorded as counts, not edges. *Visible effect: `"hist-cmd"` links, `"main"` doesn't.* |

### BQ-09: The closed set of node and edge kinds — OPEN
- Options:
  - (a) Nodes: `container{file|module|type|function|block}`, `state{var|field|element|static|cell}`, `external`, `unresolved`. Edges: `contains`, `call`, `read`, `write`, `access` (may write), `binds`, `value-flow`, `implements`, `link`, `exit`. Each edge carries {id, kind, from, to, resolution: exact|possible|unresolved, receipt}.
  - (b) An open, plugin-defined vocabulary.
- Rec: (a), closed, so the query layer stays language-agnostic.

### BQ-10: The node ID grammar, anonymous naming, namespacing, and what the agent may type — OPEN
- Case: dozens of `useEffect(() => …)` in `MainApp`; the curried `nodeDown = (id) => (e) => …` (`:737`) whose *inner* arrow is the mounted handler; the field `hist.current.current`.
- Options:
  - (a) Path with context names and per-name ordinals (`App.tsx › MainApp › useEffect#14`, `HistoryTree › nodeDown › =>`), scoped by `(repo, snapshot)` internally from day one. The agent may pass a unique suffix (`graftNode`), and ambiguity returns an error with candidates.
  - (b) Pure ordinals (`λ7`).
  - (c) `file:byte-offset`.
  - (d) Opaque handles per query.
- Rec: (a).

### BQ-11: Path and span encoding — OPEN
- Case: `App.tsx` has 486 non-ASCII lines, so UTF-8 bytes, UTF-16 offsets in the webview, and RustRover columns all disagree. Windows paths are case-insensitive; git paths aren't.
- Options:
  - (a) Repo-relative, forward-slash, case-exact git paths. Spans are UTF-8 byte ranges plus 1-based lines, converted to UTF-16 only at the window boundary.
  - (b) LSP-style UTF-16 line/col.
  - (c) Absolute Windows paths.
- Rec: (a).

### BQ-12: What a receipt contains — OPEN
- Case: `id: h.next++,` (`:4958`) is both a read and a write. `invoke("history_save", …).catch(() => {})` (`:5126`) is a call, a read, a link and an anonymous function on one line.
- Options:
  - (a) {commit, file, byte range, line/col, the minimal expression text}.
  - (b) Full source lines.
  - (c) (a) plus the enclosing statement and the innermost container ID.
- Rec: (c). Spans are exact because snapshots are immutable (ADR-0029), so quote-matching is unnecessary.

### BQ-13: State identity and aliasing — OPEN
- Case: `const h = hist.current; h.current = node.id;` (`:4942/4968`); `h.nodes.get(c)?.parent`; Rust `Arc` clones sharing one `AtomicBool` (`lib.rs:918–922`).
- Options:
  - (a) Declaration-symbol identity: `h.current` and `hist.current.current` are the same field symbol, collection contents are a pseudo-field `nodes[*]`, and a typed object literal writes every field it sets. Instances of a type merge, marked `possible-instance`.
  - (b) Access paths with intraprocedural alias tracking.
  - (c) Full points-to analysis.
- Rec: (a).

### BQ-14: When a method call counts as a write — OPEN
- Case: `onto.children.push(…)` and `h.nodes.set(…)` look like plain calls in TS; Rust `stop.store(true)` mutates through `&self`.
- Options:
  - (a) Signature-based: Rust `&mut self` or a mutable deref is a write; interior-mutability types and every TS method call on state are recorded as `access` (may write).
  - (b) A built-in list of mutating methods (`push`, `set`, `store`…).
  - (c) Only `call-on-state`, and the model decides.
- Rec: (a), since (b) is a catalogue (ADR-0023). Revisit if the mutation matrix is too noisy.

### BQ-15: Function values and indirect calls — OPEN
- Case: `onGraft?.(id, target)` (`:774`), where `HistoryTree` is mounted three times: `onGraft={graftNode}` (`:8345`), an `emitTo` arrow (`:1405`), and once with none (`:6351`). `busRef.current.cmd(...)` is assigned in two places. Rust `(self.make)(fs)`.
- Options:
  - (a) No resolution: the call targets the state node, and the model follows its writes.
  - (b) One-hop value flow: callees are every function written to that symbol (assignments, object fields, arguments, JSX props), each marked `possible`, plus an "absent" candidate. Returned function literals are followed one level, which handles curried handlers.
  - (c) Full points-to analysis.
- Rec: (b), keeping the (a) edges as well.

### BQ-16: Callers that aren't our code (React, Tauri, std) and JSX — OPEN
- Case: `useEffect(() => …)`, `listen("hist-cmd", (e) => …)`, `setTimeout(…, 400)`, `std::thread::spawn(move || …)`, `<HistoryTree onGraft={graftNode}/>`.
- Options:
  - (a) A `value-flow` edge from the callback into the external leaf only.
  - (b) (a) plus a synthetic `call` back from the external leaf.
  - (c) (a) for externals, plus JSX as syntax: `<C p={v}/>` is a `render` call to `C` and binds `v` to prop `p`. That's plain desugaring, not a React rule.
- Rec: (c). Without it there's no component tree and no prop flow.

### BQ-17: Same-source binding, precisely — OPEN
- Case: `const [histVersion, setHistVersion] = useState(0)` (about 120 in `App.tsx`); Rust `let (tx, rx) = mpsc::channel()`.
- Options:
  - (a) All names destructured from one call share one `cell` node: calling a function sibling is `access`, reading any other sibling is `read`.
  - (b) (a) plus direction by position.
  - (c) Per-hook plugins (rejected by ADR-0023).
- Rec: (a).

### BQ-18: Branches as block containers — OPEN
- Case: `if (!h || !src || !onto || id === ontoId) return;` (`:4946`); bus arm `else if (p.type === "graft" …) graftNode(…)` (`:5235`); Rust `?`.
- Options:
  - (a) No blocks.
  - (b) A block for every `if`/`else`/`switch` case/`match` arm/ternary/`&&` JSX guard/loop/`try`/`catch`, with its condition text, plus `exit` edges for `return`/`?`/`throw`.
  - (c) Statement-level only.
- Rec: (b), which ADR-0008 already commits to.

### BQ-19: Trait and interface dispatch — OPEN
- Case: `source.fill(&mut fbuf)` with `source: &mut dyn Source` has three impls across two crates (`playback.rs`, `engine.rs`, `profile.rs`).
- Options:
  - (a) Fan out to every impl at build time.
  - (b) One edge to `Source::fill` plus `implements` edges, expanded at query time.
  - (c) (b) plus narrowing by value flow.
- Rec: (b) now, (c) later.

### BQ-20: What gets analyzed, and external leaves — OPEN
- Case: tests, `#[cfg(test)]`, `vite.config.ts`, `build.rs`; externals like `@tauri-apps/api/core.invoke`, dynamic `import("@tauri-apps/plugin-dialog")`, `window.location.search` (which decides `IS_HISTORY_WINDOW`), `std::fs`.
- Options:
  - (a) All git-tracked source in the TS projects and Cargo members, tests included, with one external leaf per referenced symbol and external globals as state.
  - (b) One leaf per package.
  - (c) Index dependency source too.
- Rec: (a). Tests are needed for "what reaches X" (Q-31).

### BQ-21: The Tauri link — OPEN
- Case: `invoke<EqState>("set_fletcher_chain", { filters: own })` (`App.tsx:4724`) ↔ `fn set_fletcher_chain(filters: Vec<FilterIn>)` (`lib.rs:272`); `rename_all = "camelCase"` maps `fc_hz` to `fcHz`.
- Options:
  - (a) Tauri-bridge plugin: `invoke` string = `#[tauri::command]` fn name, at command level, flagging fns missing from `generate_handler!`.
  - (b) Generic, via macro expansion and identical strings.
  - (c) (a) plus argument-field links through `rename_all`.
- Rec: (a). camelCase mapping is a transform, so it's the model's job.

### BQ-22: Generic same-string links (events, bus dispatch) — OPEN
- Case: `"hist-cmd"` in TS `emitTo`, TS `listen` and Rust `emit_to`; `{ type: "graft" }` ↔ `p.type === "graft"`; `"main"` appears everywhere (noise).
- Options:
  - (a) None: plugin rules only.
  - (b) Undirected `same-literal` edges between literals in argument, property or comparison positions, workspace-wide.
  - (c) (b), but literals above N occurrences, or very short ones, are recorded as counts instead of edges.
- Rec: (c). This is mechanical and resolves the pop-out wiring with no framework rules.

## Round 4: Snapshots

### BQ-23: Uncommitted code — ANSWERED → ADR-0036
**Ruling:** (b): the working copy can be its own snapshot (no commit needed), but **every snapshot must build**, or Insight refuses.

*(Original question:)*
- Case: Claude Code just edited `App.tsx` without committing, and you ask "how does graft work now?"
- Options:
  - (a) Commits only, with every response stamped `snapshot: cd5d6ff (working tree differs in 2 files)`.
  - (b) An automatic `commit+dirty` snapshot (HEAD plus modified files, keyed by a content hash).
  - (c) Refuse until you commit.
- Rec: (a) for the first milestone, then (b). The key format allows a dirty suffix from day one.

### BQ-24: Snapshot storage and cache key — ANSWERED (a)
**Ruling:** built graphs are kept on disk under `%LOCALAPPDATA%\Insight\snapshots\`, loaded into memory when used, and evicted by LRU under the shared cache cap (BQ-07). **User note: an Insight snapshot can span multiple codebases** (ADR-0014). So: each codebase's graph is cached separately, keyed by (repo, commit or working-copy hash, Insight version), and the cross-repo link layer (wire links) is a small separate file keyed by the combination of its member keys. Changing one repo rebuilds only that repo plus the link layer. Including the Insight version in every key means a bug fix never serves a stale graph.

*(Original question:)*
- Options:
  - (a) In-memory graph, persisted as one immutable binary file per key under `%LOCALAPPDATA%\Insight\snapshots\…`, keyed by (commit, dirty hash, solver version, plugin versions).
  - (b) SQLite as storage and query engine.
  - (c) No persistence.
- Rec: (a). Including the solver version means a solver bug fix never serves a stale graph. SQLite later only for mutable things (saved views).

### BQ-25: Repo loading, build trigger and time budget — ANSWERED (explicit loading)
**Ruling (user):** you load codebases in the Insight app, then open Claude and ask, and Claude sees what's loaded. Or you tell Claude to load them and it calls the load tool. **No default from Claude's working folder:** that guesses intent (ADR-0035). The repo for a loaded folder is decided by git's own rule (walk up to `.git`, i.e. `git rev-parse --show-toplevel`), which is mechanical. A folder outside any git repo is refused. With nothing loaded, tools return "nothing loaded" and Claude asks you or loads it. Builds stay on demand (BQ-07), and a cold build runs in the background with a progress/ETA status Claude can poll.

*(Original question:)*
- Case: `claude` started in `Fletcher\app` (a subfolder); a cold build might exceed an MCP call timeout.
- Options:
  - (a) Default to the cwd's repo at HEAD, start building eagerly in the background at MCP startup, override with an `open(repo, rev)` tool, plus `insight build` on the CLI.
  - (b) Explicit load only.
  - (c) A per-repo `insight.toml`.
- Rec: (a). Targets: Fletcher cold build < 60 s, cached load < 1 s.

## Round 5: Queries and the MCP tool API

### BQ-26: The query layer (Q-34 for the MVP) — ANSWERED → ADR-0038
**Ruling:** a query language. Claude writes SQL against a documented schema (nodes, edges, receipts, tags); each codebase's snapshot is a read-only SQLite file. This refines BQ-24's storage format. It also shrinks BQ-27/28/29.

*(Original question:)*
- Case: "every path from `nodeUp` to a file write"; "all writers of `hist.*`" (the mutation matrix).
- Options:
  - (a) Hand-written traversal primitives as fixed tools: `related(node, relation, direction, depth)`, `paths(from, to, kinds)`, `matrix(rows, cols, relation)`, with facts stored as plain relations so Datalog can be added later.
  - (b) Embedded Datalog now.
  - (c) SQL recursive CTEs.
- Rec: (a).

### BQ-27: The MVP tool list — ANSWERED → ADR-0039
**Ruling:** `status`, `load`/`unload`, `build`, `sql`, `source`, `show`, `selection`, `tag_coverage`. A fixed tool and style, free composition inside views, and capability wishes go to the user (no mods).

*(Original question:)*
- Options:
  - (a) About 9 coarse tools: `overview`, `find`, `resolve`, `inspect`, `related`, `matrix`, `source`, `show` (a whole view document), `ui_state`. ADR-0026's verbs become element kinds inside `show`.
  - (b) About 20 imperative verbs (`focus`, `highlight`, `group`…).
  - (c) Two tools: `query` + `render`.
- Rec: (a).

### BQ-28: Result size caps and row format — ANSWERED (ADR-0038/0039): `sql` is capped at 200 rows with "truncated, N total".

*(Original question:)*
- Case: "everything in `MainApp`" means about 1,100 functions.
- Options:
  - (a) A hard cap (e.g. 50 rows), plus `total`/`truncated`/`cursor` and per-container rollups (`HistoryTree: +212`), in compact one-line text rows (`path | kind | L4960 | "h.current = node.id;"`).
  - (b) A caller-given token budget.
  - (c) No cap.
- Rec: (a).

### BQ-29: How the agent finds things — ANSWERED (ADR-0038): SQL over `nodes` and `receipts`.

*(Original question:)*
- Options:
  - (a) `find(name pattern)` plus `resolve(file, line|quote)` → the innermost node. Text search stays with the agent's grep; semantic search is the model's job.
  - (b) Also full-text/regex.
  - (c) Also semantic search.
- Rec: (a).

### BQ-30: Reading source through Insight — ANSWERED (ADR-0039): `source` returns snapshot text.

*(Original question:)*
- Case: you have uncommitted edits, and the agent quotes working-tree text that doesn't exist in snapshot `cd5d6ff`.
- Options:
  - (a) No source tool.
  - (b) `source(ref, context)` returns snapshot text with line numbers, and responses warn when the working tree differs.
  - (c) Source embedded in every result.
- Rec: (b).

### BQ-31: The commit stamp in every response — ANSWERED (ADR-0039): `status` shows the loaded snapshots, and every result names its snapshot.

*(Original question:)*
- Options:
  - (a) An active snapshot, with every response starting `@cd5d6ff (dirty: no)`, plus an async `build(rev)` tool.
  - (b) An explicit `rev` on every call.
  - (c) Auto-follow HEAD (breaks ADR-0029's pinning).
- Rec: (a).

### BQ-32: What the agent is told up front — ANSWERED (ADR-0039): the MCP instructions document the SQL schema, the workflow, and the wish rule. `status` returns the live schema.

*(Original question:)*
- Options:
  - (a) Tool descriptions only.
  - (b) Short MCP `instructions` (workflow + reference grammar, about 300 words) plus an `overview(depth)` tool (container tree with sizes, counts, unresolved rate).
  - (c) A skill or AGENTS.md snippet in your repo.
- Rec: (b).

## Round 6: Views and the backend↔window contract

**Internal plumbing DECIDED BY CLAUDE (user explicitly delegated: "you can decide").** The individual questions below are kept for history.

| BQ | Ruling |
|---|---|
| 33 | Each `show` sends a **complete view** with an id. For small follow-ups, Claude may send `base: <view id>` plus a short patch (add/remove elements, change highlight, go to step). The app owns the current view state; the window just renders it, which keeps views replayable and testable. |
| 35 | A reference is a node ID (or a unique suffix, BQ-10), optionally narrowed to `lines` within it. Abstraction pseudocode and code excerpts can map **each line** to its real lines, so "expand to real code" lands on the exact spot. |
| 37 | Example data is either `{table: columns, rows}` or `{diagram: objects[{id, label, fields{name: value or ->objectId}}]}`, each linked to the real fields it illustrates. Before/after is a list of frames; the window shows what changed. Values are free (ADR-0037). |
| 42/43 | One app (ADR-0031). The `insight mcp` bridge talks to the running app over a **local named pipe** (Windows-native, no network port, no firewall prompt), speaking JSON-RPC. If the pipe isn't there, the bridge launches `Insight.exe` and waits for it. Inside the app, the core pushes view updates to the window with Tauri events. A headless mode (`insight serve --headless`) runs the same core with no window, for tests. |
| 44 | Shared types are defined once in Rust (serde). TypeScript types for the window are **generated** (`ts-rs`, committed, CI fails if stale), and JSON Schema for extractors is generated (`schemars`, ADR-0034). That avoids the Fletcher `HistNode`/`HistTreeNode` copy bug (TB-082). |
| 46 | Three test layers: (1) a **conformance suite**, small fixtures per rule that every language extractor must pass the same way; (2) **targeted Fletcher assertions** taken from the hand-traced keys (e.g. `graftNode` writes `children/nodes/current/next`; `invoke("history_save")` ↔ `fn history_save`); (3) an **oracle report** comparing against each toolchain's own "find all references", which prints the static resolution rate (TB-097). It's informative and doesn't block. No whole-graph golden files (too brittle). |

### BQ-33: Views: whole documents or edits to a live view — OPEN
- Options:
  - (a) A complete view document every call.
  - (b) Imperative edits to a server-held current view.
  - (c) Complete documents with ids, plus optional `base: <view_id>` and a small patch.
- Rec: (c). The backend owns view state and the window is a pure renderer of versioned "scenes", which keeps replay and tests deterministic.

### BQ-34: The view schema — ANSWERED → ADR-0040
**Ruling:** nine element kinds, all in the MVP: nodes/edges, groups, collapse, highlight/focus, notes, steps, code excerpts, abstraction nodes, example data.

*(Original question:)*
- Options:
  - (a) One `elements[]` list: `{id, kind: group|abstraction|annotation|excerpt|table|instance|timeline, label, body, maps_to[], provenance: real|simplified|rewritten|example|pseudocode}`, plus `focus[]`, plus `steps[]`.
  - (b) A typed object per kind, each with its own tool.
  - (c) Free-form markdown/mermaid plus refs.
- Rec: (a). `provenance` enforces "visibly authored" mechanically.

### BQ-35: Anchors below node level — OPEN
- Case: pseudocode "walk up parents, count steps" should map to the `for (let steps = 1; c != null; steps++)` loop; a highlight on the `< 6` check in `nodeMove`.
- Options:
  - (a) Node-level only.
  - (b) `{node, quote}` anchors within a node, plus per-line `maps_to` on code and pseudocode.
  - (c) Raw `file:line` ranges.
- Rec: (b).

### BQ-36: Bad references in a view — ANSWERED (ADR-0037): only reference resolution is checked. Valid elements render; unresolvable ones come back as errors with candidates.

*(Original question:)*
- Case: the typo `MainApp › graftNodes`, or quoting `let best = 48;` but mapping it to `nodeUp` instead of `dropTarget`.
- Options:
  - (a) Reject the whole call.
  - (b) Render the valid elements, drop the bad ones, and return per-element errors with candidates.
  - (c) Render everything marked "unmapped" (forbidden by ADR-0026).
- Rec: (b).

### BQ-37: Instance diagrams and before/after frames — OPEN
- Case: `hist.nodes` holding `#0→#4→#5→#6→#7`; `graftNode(7, 4)` adds `#12` and `current` moves.
- Options:
  - (a) A typed object graph `objects[{id, type_ref, fields{name: value | ->objId}}]` with each field mapped to its field node. A frame is a full instance, and Insight computes the diff.
  - (b) The agent sends diffs.
  - (c) Tables only.
- Rec: (a).

### BQ-38: Example values in steps — ANSWERED → ADR-0037
**Ruling:** any value is fine. Insight defers to the model and gives it tools to verify itself (e.g. confirm a race actually races before showing it).

*(Original question:)*
- ADR-0011 says an LLM-predicted value is never displayed; ADR-0019 puts valued step-throughs out of the MVP; ADR-0026 allows *example* values.
- Case: the graft walk's "`d = 3`, `node.id` e.g. 12, label '3 edits in one'".
- Options:
  - (a) Allow values tagged `provenance: example`, rendered as authored, and amend ADR-0011.
  - (b) Steps carry location, highlights and prose only.
  - (c) No steps in the MVP.
- Rec: (a). It's what both Fletcher answers needed.

### BQ-39: Step-throughs: all at once or one call per step — ANSWERED (both, mainly you)
**Ruling (user):** the whole `steps[]` arrives in one view. By default **you drive** with Next/Back at your own pace. When you don't want to think, say "show me the walkthrough": Claude sends the view in **play mode** and the window auto-advances, each step held long enough to read its note, with pause/back/next always available. Claude can also jump the window to a step ("look at step 7").

*(Original question:)*
- Options:
  - (a) The whole `steps[]` in one view (each step {focus, highlight[], frame?, values?, text}); the window owns Next/Back, and the agent can jump to a step.
  - (b) One tool call per step, timed to the narration.
  - (c) Both.
- Rec: (a).

### BQ-40: Linking prose to graph elements — ANSWERED (a)
**Ruling:** text in notes and steps can contain links to nodes (e.g. `[graftNode](MainApp › graftNode)`), which are clickable in the window and light up the node. The terminal copy stays plain text.

*(Original question:)*
- Options:
  - (a) Inline markup in any text field (`[nodeUp](HistoryTree › nodeUp)`), validated like `maps_to`, with the terminal copy left plain.
  - (b) A separate `say(text, refs[])` tool.
  - (c) None in the MVP.
- Rec: (a).

### BQ-41: Window → agent ("what am I looking at?") — ANSWERED (ADR-0039): the `selection` tool.

*(Original question:)*
- Case: you click `ghost` in the window and ask "why is *this* useState?"
- Options:
  - (a) A `ui_state()` tool returning {view_id, step, selection[], snapshot}.
  - (b) An MCP resource subscription.
  - (c) None.
- Rec: (a).

### BQ-42: No window open — OPEN
- Options:
  - (a) Validate, store as the latest view, and return `window: not connected`.
  - (b) Error.
  - (c) Auto-launch the window.
- Rec: (a), with auto-launch as an opt-in later. Headless is what makes it testable.

### BQ-43: The push channel to the window — OPEN
- Options:
  - (a) The backend hosts a WebSocket on an ephemeral 127.0.0.1 port with a token, and writes `%LOCALAPPDATA%\Insight\run\session.json` for the window to find.
  - (b) A named pipe relayed by Tauri events.
  - (c) A watched file.
- Rec: (a). It works for Tauri, a browser tab and a test client alike.

### BQ-44: Where the contract types are defined — OPEN
- Case: adding a `timing` field must not silently drop out of the window. That's the same bug class as Fletcher's `HistNode` → `HistTreeNode` copy (TB-082).
- Options:
  - (a) Rust serde types in `insight-contract`, with TS generated (`ts-rs` or `specta`), committed, and checked in CI.
  - (b) JSON Schema with codegen for both sides.
  - (c) Protobuf.
- Rec: (a).

## Round 7: Testing

### BQ-45: How tests reach Fletcher — OPEN
- Options:
  - (a) A git submodule `testdata/fletcher` pinned to `cd5d6ff`, read via `gix`, plus tiny synthetic fixtures per language rule.
  - (b) Your local clone at a pinned hash (local only).
  - (c) Vendored files.
- Rec: (a). It works in public CI (Insight is Apache-2.0).

### BQ-46: The solver correctness suite — OPEN
- Options:
  - (a) Synthetic fixtures per rule plus curated assertions on Fletcher (e.g. `graftNode` writes `children/nodes/current/next`; `setHistVersion` ↔ the autosave deps; `invoke("history_save")` ↔ `fn history_save` → `fsx::write_atomic`).
  - (b) (a) plus whole-graph snapshot goldens.
  - (c) (b) plus a differential oracle against tsc/rust-analyzer "find all references", reporting the TB-097 resolution rate each run.
- Rec: the solver agent says (c); the architecture agent warns whole-graph goldens are brittle (assert targeted facts only). Needs your call.

### BQ-47: Tool-API replay, eval tiers and session logging — ANSWERED → ADR-0037
**Ruling:** no `claude -p`. Claude tests Insight from a development session by calling its MCP tools directly, plus deterministic automated tests.

*(Original question:)*
- Options:
  - (a) A deterministic golden replay (scripted tool calls for the drag-graft and history-tree answers → snapshot-tested responses and scenes) in CI.
  - (b) (a) plus a manual `insight eval` running headless `claude -p` with the MCP attached, scored against the hand key.
  - (c) (b) plus an LLM judge.
- Rec: (b), with JSONL session logging always on (`%LOCALAPPDATA%\Insight\sessions\`). Recorded sessions also let the window be built without a live agent.
