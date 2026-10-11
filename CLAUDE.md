# Insight

A personal desktop tool for seeing and checking code. Insight builds a complete, deterministic graph of a project (one or more repos) at a commit or at the working copy that just built successfully (ADR-0029, ADR-0036, ADR-0045). You ask your terminal agent (Claude Code, Codex), which queries the graph in SQL and drives the Insight window over MCP to show, group, annotate and step through the real code while it explains (ADR-0025, ADR-0031, ADR-0038–0040). Built for a world where most software is vibe coded and the person responsible for it didn't write it line by line. Two purposes on one engine (ADR-0012): **learning** (~60%) and **system-level oversight of AI-written code** (~40%).

## The three principles every proposal must pass

1. **Never wrong, only incomplete** (ADR-0002), for Insight's own facts. What Claude says is interpretation, and checkable claims get tools (ADR-0041).
2. **The intelligence principle** (ADR-0035): the tool does what's trivial (deterministic, by algorithm); the second there's a question, it's the model's. "Tracing a call stack is not intelligent. Explaining how it works is." Practical test: if a compiler could do it, it's allowed (ADR-0043).
3. **Programming concepts only** (ADR-0048): zero framework or library knowledge anywhere. Language knowledge lives only in each language's extractor; everything else is in generic programming-concept terms. User: "0 framework anywhere besides the language itself." Runtime relevance choices are Claude's, as tool parameters (ADR-0047). Lean by default, pedantic on request (ADR-0049).

## Current status

**Build phase: M0, M1, M2, the C/C++ extractor, data flow with call-site matching (all languages), multi-repo projects and `insight install` built; validated on private projects. The window app (`crates/insight-app`) is started and iterated with the user** (ADR-0050, ADR-0051; open questions in `docs/FRONTEND-QUESTIONS.md`). The design and all four backend question rounds (BQ, R2, R3, R4) are complete (`docs/BUILD-QUESTIONS.md`). The solver speaks only generic programming concepts, with no framework knowledge (ADR-0048), and query tools take parameters Claude chooses, never defaults (ADR-0047). Stack:
- a Rust Cargo workspace (BQ-03);
- per-language extractors on each project's own native toolchain (ADR-0034, R2-15), run against a frozen source copy (R3-05);
- one read-only SQLite file per codebase snapshot (ADR-0038);
- `insight.exe` (CLI and MCP bridge) plus `insight-app.exe` (the window), connected over a local named pipe (ADR-0031, R2-13);
- Tauri 2 as the window shell.

The window draws nested boxes laid out by ELK layered on a canvas (ADR-0051).

## Doc map — where everything lives

| File | Contains | Read when |
|---|---|---|
| `docs/VISION.md` | Why this exists, north-star goals | Starting any session |
| `docs/ARCHITECTURE.md` | Current-state design: pipeline, systems | Touching design |
| `docs/GLOSSARY.md` | Domain terms (snapshot, receipt, tag, view element…) | A term is unfamiliar |
| `docs/decisions/` | One ADR per settled decision, with the *why* | Before proposing any design change |
| `docs/OPEN-QUESTIONS.md` | Unresolved debates, each with an ID and status | Before designing anything new |
| `docs/TESTBED.md` | Every known edge case (TB-IDs); the design acceptance suite | Before AND after any design change |
| `docs/examples/` | Worked examples tracing real code through the design | When applying or testing a pattern |
| `docs/research/` | Research reports (tooling, LLM providers, rendering) | When a question needs evidence |
| `docs/MCP-INSTRUCTIONS.md` | Draft of the up-front instructions agents receive | Designing or changing MCP tools |
| `docs/SCHEMA.md` | The graph model (normative) | Touching extractors, the engine or SQL |
| `docs/BUILD-QUESTIONS.md` | Backend decisions (BQ-, R2- and R3-IDs), asked one at a time | Before any backend design or code |
| `docs/FRONTEND-QUESTIONS.md` | Window decisions (FQ-IDs), judged on the running app | Before any window design or code |
| `docs/ROADMAP.md` | Phases and **current focus** | Starting any session |
| `docs/WORKLOG.md` | Append-only session journal | Resuming after a gap |
| `docs/archive/` | Original raw notes, verbatim | Almost never (history only) |

`docs/SCHEMA.md` is the graph model (normative): node and edge kinds, IDs, spans, tables, counting rules, extractor protocol. Each concept is marked Built or Planned (Mn).

## Rules for every agent session

1. **Orient first.** Read this file, `docs/ROADMAP.md` (current focus), and the last 2–3 entries of `docs/WORKLOG.md` before doing anything.
2. **Never re-litigate an Accepted ADR.** If new information genuinely contradicts one, don't argue in chat — write a new ADR that supersedes it (and mark the old one `Superseded by ADR-XXXX`). The user decides.
3. **Every settled design decision becomes an ADR.** If a conversation resolves a debate, capture it in `docs/decisions/` before the session ends, using `docs/decisions/ADR-0000-template.md`. A decision that lives only in chat history is lost.
4. **Every unresolved debate goes to OPEN-QUESTIONS.** If a problem comes up and isn't settled, add it with a Q-ID. When it's later settled, mark it `Resolved → ADR-XXXX`.
4b. **Every edge case goes to TESTBED.md the moment it's raised** — before it's solved. Every design change must be checked against every TB case.
5. **End every session with a WORKLOG entry.** What changed, what was learned, what's next. Use the template at the top of `docs/WORKLOG.md`.
6. **Living docs describe *now*.** `ARCHITECTURE.md`, `GLOSSARY.md`, `ROADMAP.md` are always current-state — update them in place. History belongs in ADRs, the worklog, and git.
7. **Graph-model changes touch SCHEMA.md** (normative) and, if a decision was involved, an ADR; then sync the derived summaries.
8. **Document authority:** ADRs, `TESTBED.md`, and `SCHEMA.md` are normative; `ARCHITECTURE.md` and `README.md` are derived — on conflict, normative wins.
