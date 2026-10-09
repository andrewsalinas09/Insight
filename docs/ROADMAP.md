# Roadmap

This file always states the **current focus**. Update it when the focus changes. Phases after 0 are provisional and will be reshaped by discovery.

## Current focus
**Phase 0: Discovery, with the MVP now defined (ADR-0019).** Remaining before Phase 1: the Q-20 v1 data-edge cut (scoped to the MVP), a first SCHEMA draft (with node identity, Q-35), and the hand-traced key for TB-100. This is the long conversation. The goal is to get the whole idea out of the user's head and into VISION, ADRs, OPEN-QUESTIONS, and TESTBED. Settled so far: never wrong (ADR-0002), the static map with dynamic overlay (ADR-0003), the free-form knowledge profile (ADR-0004), annotations as committed repo files (ADR-0005), and the deterministic graph with the LLM tool API (ADR-0006). Also settled: language plugins (ADR-0007), containers + control edges (ADR-0008), and inline-first annotations (ADR-0009). The learning loop now has evidence (`docs/research/2026-10-learning-session-patterns.md`) and two new questions: the explanation layer and journal (Q-22), and the explain-differently ladder (Q-23). Settled: show, then tell (ADR-0010) and step-throughs with real values (ADR-0011; mechanism in Q-24). Purpose reframed (ADR-0012): learning ~60%, oversight of AI-written code ~40%, which adds the system diff (Q-25), expectations (Q-26), and directing agents from the graph (Q-27). The oversight loop is validated in `docs/examples/oversight-loop-worked-example.md`. Tracing strategy accepted (ADR-0015: static first, LLM where unsure, log probes as the runtime tracer); language set and build order accepted (ADR-0016: TS + Rust first; then Python, C/C++, Go, SQL; then Kotlin/Java, Swift, C#; definition and glue languages as wire links need them). Multi-codebase workspaces with wire links accepted (ADR-0014; matching in Q-28, value lineage in Q-29). Q-21 accepted as ADR-0013 (supersedes ADR-0006). The user notes it's hand-wavy, so the mechanisms go to a Phase 1 compiler spike. Next: (how much power the LLM has, which would supersede ADR-0006); data and boundary edges (Q-20); the annotation format and claim vocabulary (Q-15); and the editor relationship (Q-10/Q-18).

## Phase 0: Discovery (started 2026-10-09)
- Map the full idea into docs: every decision becomes an ADR, every debate a Q-ID, every edge case a TB-ID.
- Exit when Q-04, Q-05, Q-11, Q-15, and Q-18 are resolved. (Q-01 and Q-02 are already resolved.)

## Phase 1: Research spikes
- **Compiler spike (ADR-0013 is a rule set, not yet a mechanism).** On a small real codebase: quote-receipt matching, 3–5 claim kinds with deterministic checks, a chain receipt, a broken-receipt → re-anchor cycle, and redundant-claim linting. Rules that can't be built as stated get superseded.
- LLM provider access and auth options (Q-06).
- Analysis tooling: tree-sitter vs. LSP / rust-analyzer vs. SCIP, and runtime instrumentation (Q-02, Q-03).
- Graph rendering and layout at scale (Q-08).
- Output: reports in `docs/research/`, ADRs for the stack, and a frozen `SCHEMA.md` for the graph model.

## Phase 2: Tracer bullet = the MVP (ADR-0019)
Done when Fletcher answers: **"How does dragging a node onto another node work?"** (TB-100), matching the hand-traced key in `docs/examples/fletcher-history-graft-worked-example.md`. (The sensor "is the frequency correct?" question, TB-101, is the post-MVP cross-device target.) In scope: TS + Rust + the Tauri bridge static graph, effects, data-structure facts, timing kinds (ADR-0018), show-then-tell, the assurance trace (Q-31), and receipts. Out of scope: step-throughs, oversight, wire links, journal, live mode.
One trigger in one small real codebase, end to end: trace → graph → LLM labels → zoomable view → chat about a bubble. **Lead candidate: Fletcher** (the user's open-source Tauri app: real, vibe-coded monolith, has its own docs for the "why"). The target question is already traced by hand in `docs/examples/fletcher-history-graft-worked-example.md`, so the tracer bullet has a known-correct answer to match. Fallback: Insight itself once it has code (TB-007).

## Phase 3+: To be defined
Breadth (more languages and trigger kinds), freshness, and learning UX. Shaped by what Phase 2 teaches.
