# ADR-0056: A project glossary, shown when you hover a name

- **Status:** Accepted
- **Date:** 2026-10-11
- **Source:** user, confused by a codebase's own vocabulary ("what is a timeline?"): "I'd be cool if there way a 'crash course' to all the important things and words". Then: "insight doesn't know anything other than what's in the INSIGHT.md in terms of 'intelligence' we should totally add a glosary for the codebase that way you can get intelligence in everyday viewing outside what claude puts … And then the hover over thing is linked."

## Context
Claude explains a codebase's words in the terminal and in views, but that understanding vanished with the session, and everyday reading in the window had none of it.

## Decision
- **The glossary** is a list of terms per project: a name, a plain definition, and the code it names. It lives in the project's `INSIGHT.md`, in its own ```` ```insight-glossary ```` TOML block, next to the project's other notes (ADR-0044). The settings parser never reads it.
- **Claude keeps it** with a new `glossary` tool: no arguments lists it; `set` adds or replaces terms; `remove` deletes them. Every code name must resolve (as every reference in a view, ADR-0040). Names are stored as node paths and resolved against whatever snapshot is loaded; one that no longer resolves is shown as stale, never dropped.
- **Hover cards.** Hovering any identifier in any code on screen shows a card: what it is (kind, place), its glossary terms, its declaration and the lines written above it. Clicking the card goes there. The name is matched mechanically: something used with that name on that line (receipts), else the nearest declaration with that name.
- **Term citations.** Notes can cite a term as `[label](term:name)`; it must exist. Hovering shows the term's card and lights its code.
- The details panel shows a thing's glossary terms first.

## Why
- It's the user's own idea: the window gets Claude's understanding in everyday reading, not only during a walkthrough, and it survives the session.
- The split stays as before (ADR-0035): what a name *is* (declaration, kind, place, comment) is mechanical; what it *means* is Claude's, written down, labelled as such (orange rule) and tied to real code.

## Consequences
- Type names in code resolve by name only until `has-type` edges exist (SCHEMA, Planned).
- The glossary is private to the user's machine, with the project, never in a repo.
