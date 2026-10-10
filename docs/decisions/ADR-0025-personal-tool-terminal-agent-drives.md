# ADR-0025: A personal tool. You ask in the terminal; your agent drives the Insight desktop window

- **Status:** Accepted; binaries named in R2-13 (insight.exe + insight-app.exe); two-way parity (R2-09)
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 ("I would like for it to be in terminal if possible… it's single person on single thing so I think terminal / desktop makes the most sense. We're not really integrated, this isn't an application for other people"). On cost, the user noted that Claude's API credit allowances aren't much for Opus-class usage, while subscription agents (Claude Code, Codex) run against plan usage. These are the user's statements, not independently verified.

## Context
For the MVP we had to decide where the question is asked and who runs the model: Insight's own chat calling an API, or the user's existing terminal agent.

## Decision
- **You ask in the terminal**, in Claude Code or Codex. The agent answers in text, as in the user's learning log, and **calls Insight's tools** (local MCP) to focus, highlight, and step on the graph while it talks.
- **Insight is a desktop window** (Tauri 2 proposed, Q-08) showing the graph. It has **no chat and no model calls of its own** in the MVP.
- **Built for one person on one machine**: the author, on their own codebases. It's open source (ADR-0020), but not designed as a product for other users.

## Why
- The agent already knows the code and the conversation. Duplicating that inside Insight adds nothing.
- Running on the agent's subscription avoids paying per token through API credits for heavy Opus-class use.
- A single user removes whole categories of work: accounts, sharing, onboarding, team features, pricing.

## Consequences
- **The MVP's critical path is the tool API plus the window**, i.e. what the agent can call and how the window reacts (ADR-0024).
- **ADR-0021's "question box" is the terminal.** The window shows the whole system and reacts; it doesn't take questions in the MVP.
- **Closed or deprioritized:**
  - Q-14 (business model): moot.
  - Q-06 (LLM provider access): no in-app model, so no API integration needed.
  - Q-07 (privacy): simpler, since the agent already sees the code.
  - Sharing explanations (Q-22): low priority.
  - Team and onboarding framing in Q-11: out of scope.
- ADR-0017's optional embedded agent stays optional and unplanned.
