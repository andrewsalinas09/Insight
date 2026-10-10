# ADR-0030: Backend and frontend are separate; questions are settled one by one before building the backend

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("I think we should have a bunch of questions (as we do) and you ask, I answer and we can go one by one. We can even spawn some agents to come up with questions that may arise and get them all down now so we can build the backend (backend and frontend should largely be separate)")

## Context
The design is now concrete enough to build the backend (solver + snapshots + MCP tools). The UI is deliberately deferred (ADR-0027 update). The user wants all open backend decisions surfaced up front and answered one at a time.

## Decision
- **Backend and frontend are largely separate.** The backend (solver, per-commit graphs, query layer, MCP tools) works and is tested with no window at all. The frontend (the window) is a client of a defined contract and can be rebuilt freely.
- **Process:** agents draft every backend decision question (solver, MCP tool API, architecture/stack/storage/testing). They're merged into `docs/BUILD-QUESTIONS.md`, ordered by what blocks the most, and asked one at a time. Each answer becomes an ADR or a recorded ruling before the next question.

## Why
Separating the two lets the backend be built and verified against Fletcher now, while UI taste (which has missed twice) is settled later against something real. Answering questions up front, one at a time, matches how the user works and avoids building on unasked assumptions.

## Consequences
- The backend↔frontend contract becomes an explicit artifact (designed in the question round).
- The first runnable milestone needs no UI (e.g. a CLI or MCP answer for the Fletcher drag-graft path).
