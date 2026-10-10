# ADR-0046: Updates follow the user's proven desktop-app pattern

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, R2-24 ("follow what [the user's other desktop app] does here"). The pattern was read from the user's own proprietary Tauri app and is recorded here only in generic form.

## Decision
**Released updates (in-app):**
- Signed updates from Insight's GitHub releases (Tauri updater; public key embedded, **signing key never in the repo, never printed or logged**).
- Checked shortly after launch and once a day, **silently**: offline is normal, not an error. A small indicator in the menu shows an available version, and installing is a menu action.
- **The install guard:** installing is refused while Insight is busy (a build or snapshot in progress); state is flushed to disk first; the installer closes and reopens the app. Development builds never install updates.

**Local installs** (a wish approved → Claude builds → install):
- An install script builds to a **separate executable**, asks the running app to close normally and verifies it's idle, then **stages the files and replaces them atomically**, retrying transient Windows sharing violations. **Never force-kill** to clear a lock.
- It launches once, then **verifies the bridge↔app connection actually works**. On failure it reports, without retry loops or watchdogs.
- Output goes to log files (consistent with R2-20). Development/QA instances stay isolated from the installed app (R2-33).

**Releases:** versions change only through one version-bump script; user-facing notes go in `CHANGELOG.md`; **Claude never pushes a release tag unless the user asks.**

**Insight-specific addition:** the `insight mcp` bridge and the app exchange version numbers on connect. If an old bridge reaches a new app, Claude gets a clear "Insight was updated: reconnect with `/mcp`" instead of silently using outdated instructions.

## Why
The user has already run this pattern in production on their own app. It's safe (signed, idle-guarded, atomic, no force-kills) and quiet (silent checks, offline is fine). Reusing it follows "don't solve solved problems" (ADR-0035).

## Consequences
- Answers R2-24. Insight needs a releases pipeline (GitHub releases + signing) before the first public build. A diagnostics log (rotating file, no secrets) comes with it.
