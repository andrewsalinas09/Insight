# ADR-0001: Use the Humanity Tech Tree documentation system

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, kickoff session ("look at the documentation standard for Humanity Tech Tree and copy it for this")

## Context
Insight starts the way Humanity Tech Tree did: a long design conversation before any code, carried across many agent sessions. Without structure, decisions get lost in chat history and get debated again.

## Decision
Adopt HTT's documentation system as-is: CLAUDE.md/AGENTS.md (map + rules), VISION, ARCHITECTURE, GLOSSARY, ADRs, OPEN-QUESTIONS (Q-IDs), TESTBED (TB-IDs), ROADMAP, an append-only WORKLOG, research/examples/archive folders, and the same document authority split (normative vs. derived).

## Why
The system has already proven itself in HTT across 65 ADRs and many sessions. Agents orient quickly, settled decisions stay settled, and edge cases get captured before they're solved. Reusing it means no time is spent designing process.

## Consequences
- Every session follows the CLAUDE.md rules: orient first, ADR every decision, Q-ID every debate, TB-ID every edge case, and end with a WORKLOG entry.
- `SCHEMA.md` (the graph model) joins the normative set once it exists.
