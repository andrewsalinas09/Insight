# ADR-0051: The window draws nested boxes, left to right, laid out by ELK and drawn on a canvas

- **Status:** Accepted
- **Date:** 2026-10-10
- **Source:** user, frontend session. The user tried prototypes v1 and v2 ("This is actually really really cool"), then said "I think you should build this into the app. There's some changes of course but I've found iterating with you works best."

## Context
Q-08 left the rendering and layout library open until the frontend session. The user described the picture they wanted: bubbles inside bubbles that zoom, a graph that reshapes around what's selected, highlighting, and stepping. Then they clarified "left to right" and that Fletcher means the theme and style only. Two clickable prototypes over Insight's own snapshot followed. The first used circle packing and was replaced. The second used ELK layered on a canvas, and is the one the user asked to build into the app.

## Decision
- **The picture:** the code's own containers (folders from file paths, then ADR-0008's levels) as nested boxes. Each open box lays out its contents left to right, callers to the left of what they call. A box opens and closes; the layout is recomputed and animated, and the box acted on stays where it was on screen. Lines are lifted to the deepest box on screen.
- **Layout:** ELK layered (elkjs, vendored in the app), with hierarchy handling `INCLUDE_CHILDREN` and orthogonal edges.
- **Drawing:** one HTML canvas, in the Tauri window (ADR-0031). No UI framework.
- **Style:** Fletcher's Console theme (ADR-0027), with the visual encoding proposed in FQ-06, which is still being judged.

## Why
- The user judged the working prototype, not a description of one (they work by trying things).
- ELK is the layered layout engine that handles nesting and edges across levels. It also has the levers ADR-0050 needs: model order per parent and edge priorities.
- **Speed is enough.** About 300 boxes and 400 lines lay out in about half a second, and a canvas draws thousands of boxes smoothly.
- Rejected:
  - Circle packing (v1): it has no direction, and the user wanted left to right.
  - React Flow and Cytoscape: they would need their own nesting and level-crossing layout, which ELK already does.
  - WebGL: not needed at current sizes.

## Consequences
- Resolves the open part of Q-08: the rendering and layout library.
- The open frontend questions (FQ-01 to FQ-06) are now changes to the app, iterated with the user on the real thing.
- The window's contract types are plain JSON from `insight-mcp` for now. BQ-44 (generated TypeScript types) is not done yet, and the window has no build step.
