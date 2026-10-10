# ADR-0031: Insight is a standalone desktop app; Claude drives all of it through MCP

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-01 ("I like the idea of double-clicking an .exe, having Claude load something in or I load in the UI. Like I want a standalone app that Claude has 100% access to drive via probably an MCP server")

## Context
BQ-01 asked whether the backend is a headless program launched per Claude session, a daemon, or one app. The user wants an app they launch themselves.

## Decision
- **Insight is a standalone desktop app** (`Insight.exe`, double-click to run). The running app owns the graphs (per-commit snapshots, ADR-0029) and the window.
- **Either side can load things.** You load a repo or commit in the UI, or Claude loads it through MCP. Both act on the same app state.
- **Claude has 100% access:** anything you can do in the UI, Claude can do through MCP tools (load a repo, switch commit, build, query, show and step views, read what you've selected). UI-only actions are a bug.
- **ADR-0030 still holds, as code separation:** the backend is its own crates with no UI dependency, tested headless. The app is the backend plus the window, packaged together.

## Why
It matches how the user works: an instrument that's open on screen, which both the human and the agent use. One running app means one graph and one window, even with Claude Code and Codex connected together, which avoids BQ-01's duplicate-backend problem.

## Consequences
- **How Claude Code reaches the app (BQ-01b, user chose (b)):** a tiny `insight mcp` stdio bridge, registered once with `claude mcp add insight -- insight mcp`. It forwards each call to the running app and launches `Insight.exe` if it isn't running. It works for any agent that speaks stdio MCP (Claude Code, Codex).
- Tauri 2 (Q-08) is the natural shell, but backend crates never depend on it.
- MCP tool parity with the UI becomes a standing requirement for every feature.
- **Parity runs both ways (R2-09, user):** everything Claude can do through MCP (load, build, query, compare, show, step, select) you can also do directly in the window. "The terminal is simply a way to get Claude to do it on your behalf (the default way you use this tool)." The question box is the terminal.
- **Binaries (R2-13):** the window is `insight-app.exe` (Start-menu shortcut "Insight"), and the CLI/bridge is `insight.exe`, both in `%LOCALAPPDATA%\Insight\bin`. Where this ADR says `Insight.exe`, read `insight-app.exe`.
