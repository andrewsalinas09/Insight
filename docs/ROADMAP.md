# Roadmap

This file always states the **current focus**. Update it when the focus changes. Phases after the MVP are provisional and are reshaped by real use (BQ-45).

## Current focus
**Next: data-flow edges** (state, read/write, binds, function values through variables and fields), validated on the user's private C++ firmware pair (a sensor and its gateway; their code is never committed here). The C/C++ extractor is built (WORKLOG 2026-10-10). M2 is built (ADR-0030; BQ-02). All four backend question rounds (BQ, R2, R3, R4) are complete (`docs/BUILD-QUESTIONS.md`); frontend questions are deferred to their own session. Build order:
1. **M0 ✅ (2026-10-09):** `insight build Fletcher --commit cd5d6ff` → 1,936 functions in 24 files, 0 mismatches against the independent count; Insight on itself passes too. `docs/SCHEMA.md` written. Not yet built from the M0 list: `insight-mcp` (comes with M2), the "ask first" size estimate with `--yes` (R3-10), and node_modules reuse (R2-25). Spec: `insight init` then `insight build Fletcher --commit cd5d6ff` (R3-06, R3-09); commit export from git is part of M0. Output goes to a log file, and the terminal shows only the log's path (R2-20). Pass = the function count matches an independent tree-sitter count exactly (R3-11). **Crates (BQ-03 as amended by R4-C):** `insight-model`, `insight-solver` (pipeline plus the generic same-literal and `names` link passes), `insight-lang-ts` and `insight-lang-rust` (thin launchers for the out-of-process extractors), `insight-engine`, `insight-mcp`, `insight-cli`. No framework link crates (ADR-0048).
2. **M1 ✅ (2026-10-10):** both extractors emit calls (TS checker with one-hop value flow; rust-analyzer through macro expansions), handoffs, returns and string literals; the solver adds the generic `link/literal` and `link/names` passes; `insight path` (SCHEMA.md §10). The gate passes: `crates/insight-solver/tests/m1_gate.rs` finds both windows' routes from `nodeUp` to `graftNode` and the cross-language route to `fsx::write_atomic`. Still planned from SCHEMA: state and read/write/access, `binds` as stored edges, Rust one-hop value flow and `dyn` dispatch (`implements`), exits, timing. Spec: `insight path <from> --to <to> --follow …`: algorithmic reachability with receipts, where the caller chooses the hop kinds and cycles and recursion are reported (R2-12, ADR-0047). Gate = the required-hop list for the drag walkthrough, with the Tauri step as a `names` hop (R3-12, ADR-0048).
3. **M2 ✅ built (2026-10-10), awaiting the user's first run inside Claude Code:** `insight mcp` (crate `insight-mcp`) serves `status`, `load`, `build`, `sql`, `path` and `source` over stdio. Security review first; its findings are fixed and recorded in WORKLOG. SQL hardening is per R3-G #7. `path` takes set-valued and open ends. Deny rules are mirrored, and the frozen-source folder is written read-only. `docs/MCP-INSTRUCTIONS.md` is final and compiled in. Until the app exists, the bridge runs the backend itself (R2-13). Not yet: `reach()`/`nid()`, `compare`, `show`, `selection`, `tag_coverage`, and `insight init` registering the MCP server.
4. **The app and window,** after the frontend session.

## Phase 0: Discovery ✅ (2026-10-09)
Design and all three backend question rounds are complete (ADR-0001…ADR-0048, `docs/BUILD-QUESTIONS.md`).

## Phase 1: Build the backend headless (M0–M2)
- Scaffold the Cargo workspace (BQ-03), the extractors (TS compiler API, and a separate rust-analyzer helper; ADR-0034, R2-27), the build check (ADR-0036), and SQLite snapshot output (ADR-0038, R2-30).
- ✅ `SCHEMA.md` (normative), written at M0. Each planned concept is marked with the milestone that brings it. Finalize `docs/MCP-INSTRUCTIONS.md` at M2.
- The `path` tool (R2-12), then the MCP bridge (ADR-0031, R2-13/14/33).
- *Only if real use demands it:* claim checks / the compiler mechanism (ADR-0024).
- Still open: the graph rendering and layout library (Q-08).

## Phase 2: Tracer bullet = the MVP (ADR-0019)
Shape (ADR-0025): you ask Claude Code in the terminal, and it answers while driving the Insight window through MCP tools. Done when Fletcher answers **"How does dragging a node onto another node work?"** (TB-100), matching the hand-traced key in `docs/examples/fletcher-history-graft-worked-example.md`.
- **In scope:** the TS + Rust static graph in the generic concept vocabulary, with the Tauri hop found by the generic `names` link (ADR-0048); build → SQL → `show` → window, with Claude answering in the terminal; all nine view elements, including steps (ADR-0040); receipts; the `path` and `compare` tools (R2-12, ADR-0041). Effects, timing meaning and the assurance trace are the model's output over the graph (ADR-0023).
- **Out of scope:** executed/verified step-throughs, oversight (system diff, expectations), wire links, the journal.
- The sensor "is the frequency correct?" question (TB-101) is the post-MVP cross-device target. Fletcher is the user's chosen test repo, not a dependency (BQ-45).

## Phase 3+: To be defined
Breadth (more languages, oversight, multi-repo projects), shaped by real use and the wish list in `INSIGHT.md` (ADR-0044).

## Build rules (user, 2026-10-09)
- **Spec-drift rule:** code follows the ADRs and BUILD-QUESTIONS rulings. Any place the implementation must deviate (an API doesn't provide something, a ruling turns out impossible or too slow) is reported to the user immediately and recorded (a new ADR or ruling) before the code diverges. Never silently.
- **Dogfooding:** once Insight has code, Insight analyzes **itself** as a second test target (Rust + the TypeScript extractor), alongside Fletcher at `cd5d6ff`.
