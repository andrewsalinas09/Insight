# Architecture (current state)

> **Derived document.** The ADRs, `docs/BUILD-QUESTIONS.md` rulings and `docs/TESTBED.md` are normative. On any conflict with this summary, they win.

## How it's used (ADR-0025, ADR-0031, ADR-0045)

- **A personal desktop tool.** `insight setup` once per machine registers Insight with your agents and installs the after-build hook. `insight init` once per **project** (a named set of one or more repos).
- **You ask in the terminal** (Claude Code, Codex). The agent answers in text and calls Insight's MCP tools, and the window follows along as a visual guide (R2-07). Insight calls no model itself.
- **Two-way parity (R2-09):** anything the agent can do through MCP, you can do in the window directly, and vice versa. The terminal is the default way.
- **Two binaries, one backend (R2-13):** `insight.exe` (CLI, MCP stdio bridge, headless mode) and `insight-app.exe` (the window), connected over a local named pipe. CLI commands go through the running app when it's open, and otherwise run the same backend against the same cache. The bridge launches the app when needed.
- **Project data** (`INSIGHT.md`, build settings) lives in `%APPDATA%\Insight\projects\<name>\`. Insight writes nothing into your repos, except the optional tagging line `insight init` adds to CLAUDE.md (R2-16, ADR-0044).

## The pipeline

```
project (repos) ─► build check (ADR-0036) ─► extractors (ADR-0034, NDJSON) ─► link passes (Tauri BQ-21, same-literal BQ-22)
                                                                                    │
                                    snapshot: one read-only SQLite file per repo + a link layer (ADR-0038, BQ-24, R2-30)
                                                                                    │
                       agent: sql / path / compare / source ──► show(view) ──► window ──► selection ──► agent
```

1. **When snapshots happen.** After the project's own successful build, via an agent-agnostic after-build hook (`insight snapshot --after-build`), plus **Snapshot now** in the menu, plus on demand (R2-10/10b). A snapshot is exactly the code that just built: tracked plus untracked-not-ignored files. If anything changes mid-snapshot, it's discarded (R2-17).
2. **Build check (ADR-0036).** The project's own check (`cargo check`, `tsc -b --noEmit`, or settings from the project folder) using the toolchain that built the program (R2-15). Insight refuses on a failed check, failed dependencies, or a misbehaving extractor. A language whose toolchain is missing entirely is skipped with a loud notice. Claude announces a fix, then applies it (R2-05).
3. **Extractors (ADR-0033/0034).** Each language is analyzed by its own native toolchain: the TS compiler API, and rust-analyzer as a separate helper (R2-27). They emit the standard representation as versioned NDJSON (R2-28).
4. **Snapshot storage.** One read-only SQLite file per codebase, cached in `%LOCALAPPDATA%\Insight\snapshots\`, keyed by content and extractor versions (BQ-24, R2-28), with an LRU cap and an ask-first rule for never-seen dependency sets (BQ-07, R2-19).
5. **Agent tools (ADR-0039 + amendments).** `status`, `load`/`unload`, `build`, `sql` (read-only, 200-row cap), `path` (algorithmic search that reports cycles and recursion, R2-12), `compare` (exact diff, ADR-0041), `source`, `show`, `selection`, `tag_coverage`. Insight quietly reminds Claude when it references code it hasn't fully read (ADR-0041).
6. **Views (ADR-0040).** Nine elements: nodes/edges, groups, collapse, highlight/focus, notes, steps, code excerpts, abstraction nodes, example data. The window styles them by kind (real vs. Claude's interpretation). Only elements that stand in for code must resolve to real code (R2-02). Steps are driven by you, or played as a walkthrough (BQ-39).

## Division of labor (ADR-0023, ADR-0033, ADR-0035, ADR-0043)

| Insight (deterministic) | The model |
|---|---|
| Language semantics via each toolchain: types, resolution, mutability, `await`, JSX desugaring | Library and domain meaning: "writes a file", "a 400 ms debounce", "the logger" |
| Connecting every call and every read/write of every piece of state | Which connections matter; grouping; explaining |
| Parsing `@insight <keyword>: <value>` (tags first) | Reading free-text comments and docs |
| Pathfinding, counting, diffing, reference resolution | Choosing the question, judging race candidates, reconciling renames (ADR-0028) |

Test: *if a compiler could do it, it's allowed in the tool.*

## Graph model (BQ-09…BQ-22, R2-30)

- **Nodes:** `container{file|module|type|function|block}` (every function, anonymous ones too, ADR-0022), `state{var|field|element|static|cell}`, `external` (one leaf per referenced dependency symbol, BQ-20), `unresolved`.
- **Edges:** `contains`, `call`, `read`, `write`, `access` (may write), `binds`, `value-flow`, `implements`, `link`, `exit`.
- **Resolution:** `exact` / `possible` / `unresolved`, with *observed* added later from runs. There's no model-added "inferred" tier (R2-03).
- **Branches** are `block` containers carrying their condition text; `return` / `?` / `throw` are `exit` edges (BQ-18).
- **IDs** are location paths. Any reasonable name form resolves, and ambiguity returns candidates (BQ-10, R2-18).
- **Receipts:** exact spans within an immutable snapshot (BQ-12, ADR-0029).
- **Causal order (ADR-0042):** code within a function is ordered by position. `await`, and functions handed to external code, are order breaks (*may interleave*). Race candidates are a SQL query.

## Snapshots and comparison (ADR-0029, ADR-0036, ADR-0028)

A snapshot is a commit or the working copy that just built, possibly spanning several repos of one project. Snapshots are immutable and cached. Comparing means two snapshots attached in SQL (R2-32); the model reconciles renames and moves. Views and explanations are pinned to their snapshot.

## Post-MVP (designed, not built)

- Oversight: system diff (Q-25), expectations as saved SQL (Q-26), the prompt baseline (R2-34).
- Recorded runs, probes, and verified step-throughs as self-check tools for the model (ADR-0011, ADR-0015, ADR-0037).
- Multi-repo wire links (ADR-0014, Q-28, R2-38) and firmware builds (R2-37).
- Learning journal (Q-22).

## Stack (decided)

- Cargo workspace crates (BQ-03).
- Extractors in each toolchain's language (ADR-0034).
- SQLite per codebase snapshot (ADR-0038).
- MCP via the `insight mcp` stdio bridge over a local named pipe (ADR-0031, BQ-42/43, R2-33).
- Tauri 2 window, with TS types generated by `ts-rs` (BQ-44).
- Updates follow the user's proven pattern: signed, idle-guarded, atomic local installs (ADR-0046).

**Visual language:** Fletcher's "Console" style (palette, IBM Plex, hairlines, square corners). The layout is unvalidated, and all frontend questions are deferred to their own session (ADR-0027). **Still open:** the rendering and layout library (Q-08).
