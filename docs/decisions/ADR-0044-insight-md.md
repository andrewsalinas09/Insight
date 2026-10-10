# ADR-0044: INSIGHT.md: one Markdown file that shapes both Claude (prose) and Insight (parsed settings)

- **Status:** Accepted; per-repo additions become per-project (ADR-0045); R4-03: the global INSIGHT.md holds prose only; settings are per project; the BQ-22 limit example was removed by ADR-0047
- **Date:** 2026-10-09
- **Source:** user, R2-08 ("I think this could be a doc you write like an md file that's Insight specific that you can modify / tell Claude to modify on your behalf, and it's read in and Claude uses it to change how the compiler behaves / solver solves")
- **Amends:** ADR-0004 (the knowledge profile now lives here) and ADR-0039 (wishes are recorded here)

## Decision
- **`INSIGHT.md`** is Insight's equivalent of `CLAUDE.md`. You edit it, or tell Claude to edit it on your behalf.
- **Two parts, by the compiler test (ADR-0043):**
  1. **Prose, for Claude:** your knowledge notes (ADR-0004), how you like explanations, and a **Wishes** section that Claude appends to when it wishes Insight could do something (ADR-0039). Insight hands the prose to Claude at the start of each session (MCP instructions / `status`). Claude interprets it; Insight never does.
  2. **Fenced ```` ```insight ```` blocks, parsed by Insight:** settings for the solver and builds, such as tagging mode (ADR-0032), build-check commands (ADR-0036), and the same-literal limit (BQ-22). Plain key/value parsing, so they change how the solver runs deterministically. Unknown keys are reported to Claude, not silently ignored.
- **Locations (proposed):**
  - a global `INSIGHT.md` in `%APPDATA%\Insight\`, for you and every repo;
  - optional per-repo additions, either in the repo or in app data per repo. Which one is decided with R2-16.

  Per-repo settings override global ones.

## Why
One human-readable file, editable by you or by Claude, holds everything personal and configurable. That matches how the user already works with CLAUDE.md. Splitting it by the compiler test keeps the line clean: prose is the model's, parsed blocks are the tool's.

## Consequences
- No new MCP tools: Claude edits the file like any file, and Insight re-reads it on change.
- R2-08 answered. The wish list has a durable home.
