# ADR-0054: Claude cites code in its notes; hovering a citation lights it up

- **Status:** Accepted
- **Date:** 2026-10-10
- **Source:** user, reading a walkthrough step that listed "int, float, tiny, group, small, vector": "you could hover them an dit could light up the thing on screen. Would be top tier feature. We need to make it so claude can 'cite' stuff. Same for line numbers etc..."

## Context
Claude's notes named functions, fields and line numbers as plain text. The reader had to find each one on screen themselves.

## Decision
- In any note text (a view's notes, a step's note), Claude writes **`[label](ref)`**. `ref` is:
  - any code reference a view takes (a node id or path suffix);
  - `file:line` or `file:start-end`, lines of a snapshot file;
  - `:line` or `:start-end`, lines of what the note is about: the note's `on`, else the step's trace (its last entry) or focus, else the view's.
- **Every citation must resolve**, like every code reference in a view (ADR-0040): unknown names, lines outside what's shown, and files the deny rules hide are refused with the reason. The resolved citations travel with the view as `cites`; the note keeps `[label](#cN)`.
- In the window, a citation reads as underlined orange text. **Hovering** lights the thing up wherever it's drawn: its box on the map (outlined, filled), its box in a trace, its lines in any code shown (banded). **Clicking** goes there: the node (as a link in the details does), or the function holding the lines, at that line.

## Why
- Orange already means "what Claude is showing you" (ADR-0027 palette), and a citation is exactly that, at the word.
- Checked citations keep Claude's text tied to the snapshot (never wrong, only incomplete: ADR-0002). A citation that doesn't resolve is caught before the user sees it. In the first real use it caught one.
- Markdown link syntax is what Claude writes naturally.

## Consequences
- A hovered citation whose thing is off screen lights nothing visible. Clicking still goes there.
- Square brackets followed by `(` in note text are read as citations, so they must resolve.
