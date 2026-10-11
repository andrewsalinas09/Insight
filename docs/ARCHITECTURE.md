# Architecture (current state)

> **Derived document.** The ADRs, `docs/BUILD-QUESTIONS.md` rulings and `docs/TESTBED.md` are normative. On any conflict with this summary, they win.

## How it's used (ADR-0025, ADR-0031, ADR-0045)

- **A personal desktop tool.**
  - `insight setup` runs once per machine. It registers Insight with your agents and installs the after-build hook.
  - `insight init` runs once per **project** (a named set of one or more repos). Only projects you init'ed are ever built (R3-01).
  - Then `insight build <project> [--commit <rev>]` (R3-09).
- **You ask in the terminal** (Claude Code, Codex). The agent answers in text and calls Insight's MCP tools; the window follows along as a visual guide (R2-07). Insight calls no model itself.
- **Two-way parity (R2-09):** anything the agent can do through MCP, you can do in the window directly, and vice versa. The terminal is the default way.
- **Two binaries, one backend (R2-13):**
  - `insight.exe`: the CLI, the MCP stdio bridge and headless mode.
  - `insight-app.exe`: the window.

  They're connected over a local named pipe. CLI commands go through the running app when it's open, and otherwise run the same backend against the same cache. CLI output goes to log files; the terminal shows only the log path and the exit code (R2-20). "Ask first" is an exit code plus `--yes` (R3-10).
- **Project data** lives in `%APPDATA%\Insight\projects\<name>\`.
  - **All settings** are TOML ```` ```insight ```` blocks in the project's `INSIGHT.md`, with a `[repo."<name>"]` table per repo (R3-07). Malformed settings refuse with the parse error (R3-08).
  - The only Insight file inside a repo is `.insight/tags.toml`. Insight writes nothing else into repos, except the optional tagging line that `insight init` adds to CLAUDE.md (R2-16, ADR-0044).

## Trust and safety (R3-01…R3-03, R3-G)

- **Build commands come only from the project folder.** Each one is declared `install` or `check`. Insight stores a **fingerprint** of the approved command list; if it changes, Insight refuses over MCP and shows the diff until you approve in the window or with `insight approve <project>` (R3-01).
- **Your working copy is never modified.**
  - Every Insight command runs against Insight's own **frozen copy**.
  - Install-type commands run only where Insight owns the dependencies.
  - A **tripwire** stops if any repo file changes while an Insight command runs (R3-02).
- **Secrets:**
  - `source` serves only files in the snapshot's input set (it reads the snapshot's `files` table, never the disk).
  - Project, repo and snapshot names from MCP are validated, so nothing can leave Insight's folders. Over MCP, nothing can create or approve a project.
  - The agent's file deny rules are mirrored.
  - `insight purge <project>` wipes cached snapshots (R3-03).
- **Hardening:**
  - the named pipe serves only clients that present the token from a run file only the current user can read, rejects remote clients, and is created as the first instance of its random name (process verification is not built);
  - `sql` is limited to an authorizer allowlist (no ATTACH, PRAGMA or VACUUM), an immutable read-only open, no extensions, one statement, a 5 s deadline and a heap cap (R3-G #6/#7).

## The pipeline

```
project (repos) ─► trigger (after-build hook, Snapshot now, on demand)
   ─► freeze source copy (R3-05) ─► project check in Insight's copy (ADR-0036, R3-04)
   ─► extractors (ADR-0034, NDJSON; files-read hashes) ─► link passes (same-literal BQ-22, names ADR-0048)
   ─► snapshot: one read-only SQLite file per repo; cross-repo links computed at query time (ADR-0038, BQ-24, R2-30)
   ─► agent: sql / path / compare / source ──► show(view) ──► window ──► selection ──► agent
```

1. **When snapshots happen.** The agent-agnostic after-build hook (`insight snapshot --after-build`) is **only a trigger**. There's also **Snapshot now** in the menu, and builds on demand (R2-10/10b). Triggers are **coalesced**: one snapshot at a time per project, only the newest pending, a 10 s quiet period, and skipped if the content hash is unchanged (R3-04).
2. **Freeze.** The snapshot's source files are copied into a content-addressed store, so the snapshot's text never changes. A snapshot is **exactly the files the extractors read**, each hashed: unrelated edits don't discard it, and a mid-read change is caught (R3-05, refining R2-17/R2-25).
3. **Build check (ADR-0036).** Insight runs the project's own check, warm, in its own copy, using the toolchain that built the program (R2-15). It refuses on a failed check, failed dependencies, or a misbehaving extractor. A language whose toolchain is missing (and can't be fetched) is skipped with a loud notice; Claude announces a fix, then applies it (R2-05). In a multi-repo project, a repo that fails keeps its last good snapshot (R3-20).
4. **Extractors (ADR-0033/0034/0048).** Each language is analyzed by its own native toolchain: the TS compiler API, rust-analyzer as a separate helper (R2-27), and for C/C++ libclang on the compile commands the project's own build wrote (`compile_commands.json`; gcc-style cross compilers are asked for their include paths and predefined macros, so clang sees the configuration that was built). Each translates its language into the **programming-concept vocabulary** and emits versioned NDJSON (R2-28). Memory is sized from free RAM, with one process per project part; out-of-memory is a distinct refusal (R3-15).
5. **Snapshot storage.** One read-only SQLite file per codebase, cached in `%LOCALAPPDATA%\Insight\snapshots\`.
   - The key covers: content, extractor versions, resolved toolchains, target/features, and fact-affecting settings (R2-28, R3-08, R3-G #10).
   - Writes are atomic, with a single-writer lock (R3-G #9).
   - Receipts store spans; the bytes live in a `files` table (R3-G #8).
   - There's one private Rust target per (project, toolchain), and the last N working-copy snapshots are kept (R3-16).
   - A schema mismatch after an update refuses until rebuilt (R3-19).
6. **Agent tools (ADR-0039 + amendments, R4-B).** Parameters, never relevance defaults (ADR-0047). Output is lean by default, and pedantic detail is one switch away (ADR-0049).
   - **`status`:**
     - the loaded snapshots and their SQL aliases;
     - the schema DDL, and enum values with counts;
     - the exact definition of every `follow` token as a SQL predicate;
     - the shipped views (`callers`, `writers`, `receipt_text`, `race_candidates`);
     - the frozen-source folder path;
     - the list of pedantic switches.
   - `load`/`unload`, `build`.
   - **`sql(query, max_rows, timeout_ms)`:** Claude sets the limits under hard ceilings, and every result reports totals.
     - The table functions `reach(from, follow, dir, max_depth)` and `nid(name)` are registered in-process (not loadable extensions).
     - Hardening is per R3-G #7.
   - **`path(from?, to?, follow=[…], …)`:** deterministic search. **Claude chooses every hop kind**; there is no implicit set.
     - Either end may be open (forward or reverse reachability, with frontier external leaves) or a set.
     - `follow` entries take constraints (`max_occurrences`, `target_kind`, `avoid`, `max_depth`), plus an `edges_sql` escape hatch.
     - The result is a compact DAG of nodes and labeled hops with ids, and `receipts="expr|statement|none"`.
     - Cycles, recursion, `possible` candidates and "may call nothing" (absent optional function values) are reported.
     - Both routes into a visited node are kept (R2-12, R3-13, ADR-0047).
   - **`compare`:** an exact diff that normalizes line endings and reports EOL-only changes separately (ADR-0041, R3-18).
   - **`source(refs=[…])`:** batched reads from the frozen snapshot. A read-only, path-shaped **frozen-source folder** also lets the agent's own Grep and Read hit exactly the snapshot bytes.
   - **`show(view)`:** a view element can embed a `path` spec instead of listing ids. A view's `layout` steers placement: `chain` (sequences laid out left to right in a row), `order` (first within their box) and ELK `options` (ADR-0050). `show(step=N)` jumps to a step. `trace: {stack, line}` opens the trace view, a debugger-like walk through one function's calls in the order written (ADR-0052), also per step.
   - `selection`.
   - **`tag_coverage(scope, kinds)`:** untagged symbols in a scope, with counts. Claude picks.

   Insight quietly reminds Claude when it references code it hasn't fully read (ADR-0041). The snapshot a session queries is **pinned** until Claude switches explicitly (R3-17). The up-front instructions are drafted in `docs/MCP-INSTRUCTIONS.md`.
7. **Views (ADR-0040).** Nine elements: nodes/edges, groups, collapse, highlight/focus, notes, steps, code excerpts, abstraction nodes, example data.
   - The window styles them by kind (real vs. Claude's interpretation).
   - Only elements that stand in for code must resolve to real code (R2-02).
   - Steps are driven by you, or played as a walkthrough (BQ-39).

## Division of labor (ADR-0023, ADR-0033, ADR-0035, ADR-0043, ADR-0047, ADR-0048)

| Insight (deterministic) | The model |
|---|---|
| Language semantics via each toolchain, translated into **programming concepts**: types, resolution, mutability, `await`, closures, handoffs | Library and domain meaning: "writes a file", "a 400 ms debounce", "the logger". **No framework or library knowledge in Insight's analysis** (Insight's own Tauri shell is unaffected) |
| Connecting every call and every read/write of every piece of state; every literal; every `names` match | Which connections matter; which hop kinds to follow; grouping; explaining |
| Parsing `@insight <keyword>: <value>` (tags first; unknown keywords → diagnostics) | Reading free-text comments and docs |
| Pathfinding, counting, diffing, reference resolution, fingerprints | Choosing the question and its parameters; judging race candidates and `names` matches; reconciling renames (ADR-0028) |

Tests: *if a compiler could do it, it's allowed in the tool* (ADR-0043). *If it's cheap and makes it more correct, do it* (ADR-0035). *Relevance is a parameter, never a default* (ADR-0047). *Lean by default, pedantic on request* (ADR-0049).

## Graph model (BQ-09…BQ-22, R2-30, R3-G, ADR-0048)

- **The vocabulary is programming concepts**, not any language or framework: module, type, interface/implementation, function, closure, call, handoff, await/suspend, state, read/write/access, binding, branch, loop, exit, literal, import… If any supported language has a concept, the vocabulary covers it. Language knowledge lives only in extractors (ADR-0048). Anything with its own compiler (Vue, Svelte, `.ino`) is a language (R4-02).
- **Nodes:**
  - `container{file|module|type|function|block|macro|init}`: every function, anonymous ones too (ADR-0022); compiler-synthesized members have receipt role `synthesized`; locals and parameters are state (R3-G #3);
  - `state{var|field|element|static|cell}`;
  - `literal`;
  - `external`: one leaf per referenced dependency or standard-library symbol, carrying `package`, `module`, `version` and **`origin = project|dependency|stdlib`** (BQ-20, ADR-0049); built-ins stay external leaves (R4-01, strict);
  - `unresolved`.
- **Edges:**
  - `contains`;
  - `call` (exact/possible), with subkind `implicit` plus a `via` value for compiler-inserted calls: operator, accessor, conversion, deref, index, iteration, destructure, compare, destructor, implicit constructor (P3);
  - `expand`, from a macro use to a project macro (P9);
  - `read`, `write`, and `access` (may write), with `via_eid` pointing at the method call that caused it;
  - `binds`: every argument→parameter, with a `mode` of value/copy/move/ref/mut-ref/out/inout/pointer, plus closure `captures` (P6). JSX props bind to the props type's fields via the TS checker's contextual typing;
  - `value-flow`/**handoff**: a function or value given to external code (JSX is a handoff to the external JSX factory; there is no `render` call);
  - `implements`, with subkinds extends / implements / conforms (structural) / embeds / overrides / provides-default, and an open-world candidate (P7);
  - `instantiates` (type parameter → concrete type) at generic call sites, and `static-bound` dispatch (P8);
  - `annotates` (metadata → target), with executing decorators as calls with timing `load` (P10);
  - `imports` (static/dynamic/type-only/include), with `init-order` where the language defines it (P11);
  - `has-type` (state or return → type, including type arguments; inline types get `<type>#n` ids);
  - `exit` with subkinds return/propagate/throw/break/continue/goto/diverge and a `to` target, with catch blocks carrying their caught types. Functions carry `throws`, `nothrow` and `diverges` (P5);
  - suspension rows `suspend{await|yield|delegate}` plus `resume` edges for generators and coroutines (P2).
- **Literal links:**
  - **same-literal:** every literal occurrence is linked whatever its position, and position and occurrence counts are facts (BQ-22, ADR-0047);
  - **`names`:** a literal exactly equals a declared name, e.g. `invoke("set_fletcher_chain")` → `fn set_fletcher_chain`. It targets every named declaration, with the target kind and match count as facts, and qualified names and module paths also match. This replaces the Tauri link (ADR-0048).
- **Edge columns:**
  - `call_eid` and `arg_index` on edges inside a call's argument list;
  - `base_nid` on state accesses (the root binding they go through), so that `dragStart.current` and `hist.current` stay distinct;
  - `generated=1` for macro-expanded facts;
  - `timing` = inline / eager-start / deferred-create / concurrent / scope-exit (with `order_at`) / load / build / unsequenced group (P1).
- **Inactive code:** inactive `cfg`/`#if` regions are `block{inactive}`, with the active configuration in `meta` (P9).
- **Build-time tables:** `entries` (functions entered across an order break) and `order_breaks`, plus the shipped `race_candidates` view.
- **Follow tokens:** each one is an exact SQL predicate over `edges`, listed by `status` (e.g. `names` = function → literal → declaration).
- **Later:** P12 (concurrency syntax: Go channels/`select`, `synchronized`, actors, atomics), P13 (SQL tables, and embedded SQL; ask the user when SQL is built), P14 (linkage/FFI, entry points, memory-mapped registers, unions).
- **Resolution:** `exact` / `possible` / `unresolved`, with *observed* added later from runs. There's no model-added "inferred" tier (R2-03).
- **Branches** are `block` containers with their condition text; `return` / `?` / `throw` are `exit` edges (BQ-18).
- **IDs** are location paths plus a `body_hash` per node. Any reasonable name form resolves, and ambiguity returns candidates (BQ-10, R2-18, R3-17).
- **Receipts** are exact spans into the frozen snapshot. Macro-generated facts point at the macro call site (BQ-12, R3-05, R3-G #4).
- **Causal order (ADR-0042, R3-14, P1):** code is ordered by position, and where the real moment differs from the position, the `timing` column states it (lazy futures, scope-exit and Drop/`defer`/`finally`, load-time code, unsequenced operands). These are **order-unknown** ("may interleave"):
  - `await`/suspension;
  - handoffs to external code;
  - `names` hops;
  - un-awaited async calls.

  Race candidates are the `race_candidates` view, which also pairs a region with itself. Handoffs to the **standard library** (`map`, `then`…) are pedantic detail, off by default (ADR-0049).

## Snapshots and comparison (ADR-0029, ADR-0036, ADR-0028, R3-21)

- A snapshot is a commit or the working copy that just built (frozen), possibly spanning several repos of one project.
- Worktrees are separate working copies of the same repo, identified by `git-common-dir`.
- Snapshots are immutable and cached.
- Comparing means two snapshots attached in SQL (R2-32), or `compare`. The model reconciles renames and moves.
- Views and explanations are pinned to their snapshot.
- M0 and M1 test against Fletcher `--commit cd5d6ff` (R3-06).

## Post-MVP (designed, not built)

- Oversight: system diff (Q-25), expectations as saved SQL (Q-26), the prompt baseline (R2-34).
- Recorded runs, probes, and verified step-throughs as self-check tools for the model (ADR-0011, ADR-0015, ADR-0037).
- Multi-repo wire links (ADR-0014, Q-28, R2-38) and firmware builds (R2-37). Python/JS "builds" means the language's own syntax/import check (R3-22).
- Learning journal (Q-22).
- Looking inside dependencies: Claude reads their source for now (R3-23).

## Stack (decided)

- Cargo workspace crates (BQ-03, as amended by R4-C):
  - `insight-model`;
  - `insight-solver` (pipeline plus the generic link passes: same-literal and `names`);
  - `insight-lang-*` (thin extractor launchers);
  - `insight-engine`;
  - `insight-mcp`;
  - `insight-cli`.

  No framework link crates.
- Extractors built on each toolchain (ADR-0034).
- SQLite per codebase snapshot (ADR-0038).
- MCP via the `insight mcp` stdio bridge over a local named pipe to the running app (ADR-0031, BQ-42/43, R2-33, R3-G #6). The bridge answers the handshake itself and, on the first tool call, connects to the app or starts it (through Explorer, so the app holds none of the agent's pipes). With no app (`INSIGHT_NO_APP=1`, or it can't start), it runs the backend itself against the snapshot cache (R2-13's "app closed" mode).
- Tauri 2 window (`crates/insight-app`): the backend's `Server` lives in the app; the window is plain HTML/JS drawing on a canvas, laid out by ELK layered (ADR-0051). TS types generated by `ts-rs` (BQ-44) are not built yet: the window reads plain JSON.
- Updates follow the user's proven pattern: signed, idle-guarded, atomic local installs (ADR-0046), with versioned install folders behind a stable launcher (R3-24).

**Visual language:** Fletcher's "Console" style (palette, IBM Plex, hairlines, square corners). The layout is unvalidated, and all frontend questions are deferred to their own session (ADR-0027). Rendering and layout: nested boxes, left to right, ELK layered on a canvas (ADR-0051). Two ways to read it: the map (the whole system, with hoverable lines and calls marked in the code) and the trace view (one function, its callers and its calls in the order written; step in and out; ADR-0052). The open frontend questions are in `docs/FRONTEND-QUESTIONS.md`.
