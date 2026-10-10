# ADR-0027: Insight's visual language follows Fletcher's "Console"

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8, after the first clickable mockup ("This is fine, but the UI should be very very different. I think it should share similar design language as Fletcher actually")

## Context
The first mockup used a generic split "terminal | window" layout with rounded cards. The user already has a design language they chose for Fletcher across five iterations (Fletcher ADR-0010, "Console").

## Decision
Insight speaks the same language, adapted to code:
- **Palette and type:** paper `#f2efe9`, panel `#f9f7f2`, ink `#14171a`, dim `#8b8578`, hairlines `#e5dfd1` / `#c9c2b0` / `#b3ac9c`; IBM Plex Sans (UI) + IBM Plex Mono (code, labels, tabs). Light instrument aesthetic; dark surfaces only for selection states.
- **Anatomy:**
  - a single-row header (wordmark · mono tabs, the active one inverted · repo chip), with a 2px ink rule;
  - **a docked bottom bar** (Fletcher's A/B bar slot), which is where the agent's narration and step transport live;
  - content in hairline-divided strips and labeled cells, never card grids;
  - square corners, with hard offset shadows only for floating panels.
- **Insight's semantic colors** (the equivalent of Fletcher's boost/cut law): **ink = real code facts**, **orange `#c85a13` = what the model drew or highlighted** (authored, example), **blue `#3f6d9e` = state that changes in this step**.
- **Inspector:** labeled cells (tiny mono field labels over values), like Fletcher's history inspector.

## Why
It's the user's own approved taste, and it already reads as an instrument rather than a dashboard. Matching Fletcher also makes the dogfood case (Insight explaining Fletcher) look like one family of tools.

> **Update (same day):** the user reviewed a mockup built on this anatomy ("this isn't even close to what I want, but UI doesn't really matter right now"). The user then added: "but the style is good." So the **style** stands as approved: palette, type, hairlines, square edges, mono labels, and the overall look of the mockup. The **layout and content** (which panels exist, what goes where, header/strips/inspector/docked narration bar, the semantic color roles) are **unvalidated** and must not be treated as a design. UI work is deferred until the backend and tools exist.

## Consequences
- Mockups and the eventual UI start from Fletcher's `App.css` tokens and `design/` artboards.
- The terminal isn't drawn inside Insight. The agent speaks into the bottom bar, mirroring what it says in the terminal (ADR-0025).
- Changing the language later means superseding this ADR.
- **UI direction (user, R2-10):** "it should be very much out of the way. I like menus and popups and stuff very much. Multiple windows etc." The model is Fletcher's (and the user's other apps'): menus, popups, pop-out satellite windows. Study Fletcher's patterns when UI work starts; take only generic patterns from proprietary apps.
