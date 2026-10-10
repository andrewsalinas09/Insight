# ADR-0032: Tags are the "compiler": meaning is declared (by Claude, inline first) and parsed statically

- **Status:** Accepted; reframed by ADR-0033 (tags are *extras*, declared shortcuts for efficiency and annotation; the main semantics come from language plugins and the model); settings location settled by R3-07 (all settings in the project's INSIGHT.md TOML blocks; `.insight/config.toml` retired; only `.insight/tags.toml` lives in repos)
- **Date:** 2026-10-09
- **Source:** user, BQ-02b ("exactly, this is like the 'compiler / interpreter' I envisioned. Claude simply tags it inline or in a file and that's what generates the logic for the compiler / interpreter to know how to build… I feel like having it inline is good")

## Context
The solver only connects (ADR-0023), but queries need meaning ("every path to a disk write"). Hard-coding that meaning would need infinite semantics for infinite languages. The user's original "compiler" idea (Q-15, ADR-0013) was a defined documentation style that the system builds from. Tags are that idea in its simplest form.

## Decision
- A **tag** is any label (`disk-write`, `ui-gesture`, `logger`, `sensor-sample`, …) attached to a symbol. It is **declared, never inferred**.
- **Inline is the default:** an `@insight tag: <name>` doc comment directly above the symbol, in the source file, written by Claude as it writes or edits code (or by the user).
- **Sidecar for what can't be annotated:** a committed `.insight/tags.toml` (`[[tag]] name = "disk-write"; match = ["std::fs::*", …]`) for std, npm and other dependency symbols. `match` takes exact symbol paths with simple `*` globs.
- **The solver parses tags as data**: from the comment directly above a symbol, or by exact or glob symbol match. Queries filter by tag (`insight path … --to-tag disk-write`).
- **Extendable without code changes:** any tag name, any language. Plain names only for now; values (`logger(level=error)`) are deferred.

## Why
This keeps the solver purely mechanical while letting meaning grow without limit. Claude already understands the code, so it supplies the semantics once, in the code, where they're versioned with everything else (ADR-0005, ADR-0029). Inline tags sit with the code they describe and move with it.

## Consequences
- This is the concrete form of the "compiler" from Q-15 / ADR-0013: annotations that drive the build. Claim verification stays optional (ADR-0024).
- **The main risk is Claude forgetting to tag.** Making tagging effortless and reminded is BQ-02c.
- **Tagging is opt-in per repo** (user): `.insight/config.toml` `tagging = "always" | "on-request" | "never"`, with `on-request` as the default. Claude tags everywhere only if you ask for that.
- Globs and plain names were Claude's proposed defaults, not explicitly confirmed. The user can override them.
