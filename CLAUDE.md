# Insight

An LLM-powered tool for understanding any codebase visually and interactively. You point it at a codebase and a trigger (a UI click, a CLI command, a function, or a plain-language prompt); it finds every branch of execution that trigger can reach, builds a hierarchical graph of it, has an LLM label the graph in human terms ("validation", "error handling", "retry"), and lets you explore and learn from it in conversation with an AI. Built for a world where most software is vibe coded and the person responsible for it didn't write it line by line. Two purposes on one engine (ADR-0012): **learning** (~60%) and **system-level oversight of AI-written code** (~40%), meaning the IDE and debugger for AI-first development.

## The two principles every proposal must pass

1. **Never wrong, only incomplete** (ADR-0002).
2. **The intelligence principle** (ADR-0035): the tool does what's trivial (deterministic, by algorithm); the second there's a question, it's the model's. "Tracing a call stack is not intelligent. Explaining how it works is."

## Current status

**Discovery phase.** There is no code and no committed stack (Tauri 2 is proposed — see Q-08). The work right now is the long design conversation: getting the whole idea out of the user's head into docs, decisions, and open questions before building.

## Doc map — where everything lives

| File | Contains | Read when |
|---|---|---|
| `docs/VISION.md` | Why this exists, north-star goals | Starting any session |
| `docs/ARCHITECTURE.md` | Current-state design: pipeline, systems | Touching design |
| `docs/GLOSSARY.md` | Domain terms (trigger, branch edge, bubble, label…) | A term is unfamiliar |
| `docs/decisions/` | One ADR per settled decision, with the *why* | Before proposing any design change |
| `docs/OPEN-QUESTIONS.md` | Unresolved debates, each with an ID and status | Before designing anything new |
| `docs/TESTBED.md` | Every known edge case (TB-IDs); the design acceptance suite | Before AND after any design change |
| `docs/examples/` | Worked examples tracing real code through the design | When applying or testing a pattern |
| `docs/research/` | Research reports (tooling, LLM providers, rendering) | When a question needs evidence |
| `docs/BUILD-QUESTIONS.md` | Backend decisions (BQ-IDs), asked one at a time | Before any backend design or code |
| `docs/ROADMAP.md` | Phases and **current focus** | Starting any session |
| `docs/WORKLOG.md` | Append-only session journal | Resuming after a gap |
| `docs/archive/` | Original raw notes, verbatim | Almost never (history only) |

`docs/SCHEMA.md` (the frozen graph model) does not exist yet; it is created when Phase 1 freezes the model, and becomes normative.

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
