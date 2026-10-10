# ADR-0045: Install once, init per project. A project is a named set of one or more repos

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, R2-14b ("I think it's install once, init per repo, however 'per repo' is more like 'per project', since a single project can have multiple repos")

## Decision
- **`insight setup`, once per machine:** registers Insight with the agents present (Claude Code at user scope with an absolute path; Codex as supported; R2-14) and installs the after-build hook (R2-10b).
- **`insight init`, once per project**, like `git init`:
  - run inside a repo, it creates a **project** named after the folder;
  - `insight init --project <name>` in another repo adds that repo to an existing project.
- **A project is Insight's unit:** a named set of one or more repos. Opening a project (in the window, or Claude via `load`) loads all its repos together as one multi-codebase snapshot (ADR-0036, BQ-24). Cross-repo link passes (same-literal, and wire links later) run **only within a project**, so unrelated repos never connect.
- **Where a project lives (proposed, confirmed in R2-16):** `%APPDATA%\Insight\projects\<name>\`, holding the project's `INSIGHT.md` (ADR-0044) and its per-repo build settings. That's outside the repos, so it works across several repos, for old commits, and for proprietary code. `insight init` may also add the tagging snippet to each repo's CLAUDE.md/AGENTS.md (BQ-02c).

## Why
The user's real systems span repos (firmware + desktop app), so the natural unit is the project, not the repo. Git's mental model (install the tool once, init where you work) is already familiar.

## Consequences
- Answers R2-14b and R2-36 (named workspaces = projects).
- ADR-0044's "per-repo additions" become per-project.
- `status` and the window list projects, and loading by project is the default.
