# Roadmap

This file always states the **current focus**. Update it when the focus changes. Phases after the MVP are provisional and are reshaped by real use (BQ-45).

## Current focus
**Build M0** (ADR-0030; BQ-02). Both backend question rounds are complete (`docs/BUILD-QUESTIONS.md`); frontend questions are deferred to their own session. Build order:
1. **M0:** `insight build` on a chosen repo. Output goes to a log file, and the terminal shows only the log's path (R2-20). Pass = the function count matches an independent count exactly.
2. **M1:** `insight path … --to …`: algorithmic reachability with receipts, with cycles and recursion reported (R2-12). Gate = drag steps 3 and 5–11 from `nodeUp`, plus steps 1, 2 and 4 from `nodeDown`.
3. **M2:** the same over MCP in Claude Code.
4. **The app and window,** after the frontend session.

## Phase 0: Discovery ✅ (2026-10-09)
Design and both backend question rounds are complete (ADR-0001…ADR-0046, `docs/BUILD-QUESTIONS.md`).

## Phase 1: Build the backend headless (M0–M2)
- Scaffold the Cargo workspace (BQ-03), the extractors (TS compiler API, and a separate rust-analyzer helper; ADR-0034, R2-27), the build check (ADR-0036), and SQLite snapshot output (ADR-0038, R2-30).
- Write `SCHEMA.md` from BQ-09 and the ADR-0038 / R2-30 tables at M0.
- The `path` tool (R2-12), then the MCP bridge (ADR-0031, R2-13/14/33).
- *Only if real use demands it:* claim checks / the compiler mechanism (ADR-0024).
- Still open: the graph rendering and layout library (Q-08).

## Phase 2: Tracer bullet = the MVP (ADR-0019)
Shape (ADR-0025): you ask Claude Code in the terminal, and it answers while driving the Insight window through MCP tools. Done when Fletcher answers **"How does dragging a node onto another node work?"** (TB-100), matching the hand-traced key in `docs/examples/fletcher-history-graft-worked-example.md`.
- **In scope:** the TS + Rust + Tauri-bridge static graph; build → SQL → `show` → window, with Claude answering in the terminal; all nine view elements, including steps (ADR-0040); receipts; the `path` and `compare` tools (R2-12, ADR-0041). Effects, timing meaning and the assurance trace are the model's output over the graph (ADR-0023).
- **Out of scope:** executed/verified step-throughs, oversight (system diff, expectations), wire links, the journal.
- The sensor "is the frequency correct?" question (TB-101) is the post-MVP cross-device target. Fletcher is the user's chosen test repo, not a dependency (BQ-45).

## Phase 3+: To be defined
Breadth (more languages, oversight, multi-repo projects), shaped by real use and the wish list in `INSIGHT.md` (ADR-0044).
