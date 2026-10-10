# Insight

A personal desktop tool for seeing and checking code. Insight builds a complete, deterministic graph of a project (one or more repos) at a commit or at the working copy that just built successfully (ADR-0029, ADR-0036, ADR-0045). You ask your terminal agent (Claude Code, Codex), which queries the graph in SQL and drives the Insight window over MCP to show, group, annotate and step through the real code while it explains (ADR-0025, ADR-0031, ADR-0038–0040). Built for a world where most software is vibe coded and the person responsible for it didn't write it line by line. Two purposes on one engine (ADR-0012): **learning** (~60%) and **system-level oversight of AI-written code** (~40%).

## The two principles every proposal must pass

1. **Never wrong, only incomplete** (ADR-0002), for Insight's own facts. What Claude says is interpretation, and checkable claims get tools (ADR-0041).
2. **The intelligence principle** (ADR-0035): the tool does what's trivial (deterministic, by algorithm); the second there's a question, it's the model's. "Tracing a call stack is not intelligent. Explaining how it works is." Practical test: if a compiler could do it, it's allowed (ADR-0043).

## Current status

**Build phase, starting M0.** The design and both backend question rounds are complete (`docs/BUILD-QUESTIONS.md`); frontend questions are deferred to their own session. Stack:
- a Rust Cargo workspace (BQ-03);
- per-language extractors on each project's own native toolchain (ADR-0034, R2-15);
- one read-only SQLite file per codebase snapshot (ADR-0038);
- `insight.exe` (CLI and MCP bridge) plus `insight-app.exe` (the window), connected over a local named pipe (ADR-0031, R2-13);
- Tauri 2 as the window shell.

The graph rendering library is still open (Q-08).

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
| `docs/BUILD-QUESTIONS.md` | Backend decisions (BQ- and R2-IDs), asked one at a time | Before any backend design or code |
| `docs/ROADMAP.md` | Phases and **current focus** | Starting any session |
| `docs/WORKLOG.md` | Append-only session journal | Resuming after a gap |
| `docs/archive/` | Original raw notes, verbatim | Almost never (history only) |

`docs/SCHEMA.md` (the graph model) does not exist yet. It is created at M0 from BQ-09 (node and edge kinds) and the SQL schema (ADR-0038, R2-30), and then becomes normative.

## Rules for every agent session

1. **Orient first.** Read this file, `docs/ROADMAP.md` (current focus), and the last 2–3 entries of `docs/WORKLOG.md` before doing anything.
2. **Never re-litigate an Accepted ADR.** If new information genuinely contradicts one, don't argue in chat — write a new ADR that supersedes it (and mark the old one `Superseded by ADR-XXXX`). The user decides.
3. **Every settled design decision becomes an ADR.** If a conversation resolves a debate, capture it in `docs/decisions/` before the session ends, using `docs/decisions/ADR-0000-template.md`. A decision that lives only in chat history is lost.
4. **Every unresolved debate goes to OPEN-QUESTIONS.** If a problem comes up and isn't settled, add it with a Q-ID. When it's later settled, mark it `Resolved → ADR-XXXX`.
4b. **Every edge case goes to TESTBED.md the moment it's raised** — before it's solved. Every design change must be checked against every TB case.
5. **End every session with a WORKLOG entry.** What changed, what was learned, what's next. Use the template at the top of `docs/WORKLOG.md`.
6. **Living docs describe *now*.** `ARCHITECTURE.md`, `GLOSSARY.md`, `ROADMAP.md` are always current-state — update them in place. History belongs in ADRs, the worklog, and git.
7. **Graph-model changes touch SCHEMA.md** (normative, once it exists) and, if a decision was involved, an ADR; then sync the derived summaries.
8. **Document authority:** ADRs, `TESTBED.md`, and (later) `SCHEMA.md` are normative; `ARCHITECTURE.md` and `README.md` are derived — on conflict, normative wins.
