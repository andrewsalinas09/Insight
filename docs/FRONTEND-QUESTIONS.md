# Frontend questions (the window)

The decisions needed before building the window. They are asked one at a time, most-blocking first, in the same way as `docs/BUILD-QUESTIONS.md`. Each answer is recorded as an ADR or a ruling, and its status is updated here. **Rec** = Claude's recommendation, not a decision.

Status key: **OPEN** · **ANSWERED → ADR-XXXX / ruling** · **LATER**

---

## What is already settled

These constrain every question below. Changing one means a superseding ADR.

- **Bubbles are the code's own containers** (ADR-0008): package › file › type › function › block. Every level always exists, and a container's size is visible.
- **Claude's hierarchy sits on top** (ADR-0040): groups, collapse, abstraction nodes. Each must resolve to real code.
- **The view vocabulary is fixed** (ADR-0040). It has nine element kinds: nodes and edges, groups, collapse, highlight/focus, notes, steps, code excerpts, abstraction nodes, example data. Real things look real; Claude's interpretation looks authored. The window does that styling, not Claude.
- **Claude drives, the user can do everything too** (ADR-0031, R2-09). Every view change Claude makes over MCP, the user can make by hand, and the reverse.
- **The home screen is the whole system** at its top level, with sizes (ADR-0021).
- **Style is Fletcher's "Console"** (ADR-0027): the palette, IBM Plex, hairlines, square corners. Only the style is approved; the layout is not. The UI should be "very much out of the way… menus and popups… multiple windows" (R2-10).
- **Stack:** Tauri 2 app, separate from the backend (ADR-0030, ADR-0031). The rendering library is open (Q-08).

## The user's picture (2026-10-10)

> "each 'bubble' can be infinitely zoomed into … as hierarchy. … a nice neat graph that automatically can reshape based on what's selected. I can zoom in and out. Things can be highlighted. The step forward and back to see how it actually interacts with the code."

Two clarifications from the user, the same day:
- **Fletcher means theme and style only**, not its layout: "In terms of fletcher I just meant the theme and style and stuff like that … not necessarily the layout." This matches ADR-0027, where only the style is approved.
- **The graph runs left to right:** "also I imagine the graph from left to right."

**Claude's reading (proposal, not confirmed):** one canvas of **nested boxes laid out left to right** (a layered layout: callers to the left of what they call), with every box holding its own left-to-right flow.
- **Open and close:** a closed box shows its name and size. An open box makes room and lays out its insides. An open function shows its code, with its blocks (branches, loops) marked in the gutter. That is the bottom of the zoom.
- **Reshape:** opening, selecting or stepping changes what is open. The layout is recomputed and animated, and the box you acted on stays where it was on screen.
- **Edges across levels:** a call connects the deepest boxes on screen at each end. Several calls between the same two boxes become one thicker line.
- **Steps:** each step opens the boxes along the route so far, shows the current function's code, and moves the camera to it. Next/Back animates between them.

**Prototype (private artifact):** https://claude.ai/artifact/9Lc1Q2CpEcHrH9HbqJL88Q. It shows Insight's own snapshot (2,362 containers, calls, source) and a real six-function route from `main` to `SnapshotWriter::finish`. Layout is ELK layered, drawn on a canvas. An earlier circle-packing version (nested bubbles, no direction) was replaced when the user asked for left to right.

## Questions

### FQ-01: What opens a box — OPEN
Zooming in a left-to-right graph moves the camera. Opening a box is what reshapes the layout. The question is what triggers an opening.
- Options:
  - (a) **An explicit action** (double-click, or a button). Zooming only moves the camera.
  - (b) **Zooming:** a box opens when it fills enough of the screen, and closes again when you zoom out. This is "infinite zoom".
  - (c) **Focus:** selecting a function opens the boxes holding what it calls and what calls it, and closes the rest. Steps do the same along a route.
  - (d) (a) and (c) together, with (b) as a setting.
- Rec: (d). Explicit opening never moves things unexpectedly. Focus is what makes the graph "reshape based on what's selected". Zoom-to-open is the most magical, but it relayouts while you are moving, so it should be tried before it becomes the default. All three are switchable in the prototype.

**User feedback on the prototype (2026-10-10):** "This is actually really really cool." On zoom-to-open: "I think should switch it a little sooner and when you zoom out it should also be sooner. (with hysteresis since it overlaps in my mind…) We can put these little things in settings too."
- Prototype v2 opens at 15% of the screen (was 30%).
- A box opened by zooming closes once you zoom out 20% past the scale it opened at. Both numbers and the animation speed are in a Settings panel.
- FQ-01 is not closed by this: the user tuned zoom-to-open but didn't choose between the options.

### FQ-06: The visual encoding — OPEN
User: "we need to figure out how we should actually show things … Lots of color coding … library code, user code, different edge types, structs vs functions … useful just on glance. If you use this a lot the colors and structure can mean something in your head."
- **Proposal (prototype v2):**
  - **One meaning per color:** ink for your code's structure and control flow; blue for data (fields, statics, reads, writes); violet for handoffs (a function passed on to run later); grey hatching for library code. Orange is reserved for what Claude is showing you.
  - **One look per kind of box:**
    - package: a heavy top rule;
    - file: plain, with a language tag;
    - type: a green tint, marked ◆;
    - function: an ink stripe on the left, marked ƒ;
    - block: dashed;
    - field or static: a blue chip;
    - library: hatched, with a dashed border.
  - **Lines by kind:** calls are solid ink; possible calls are dashed; handoffs are violet dash-dot; library calls are grey; writes are thick blue; reads are thin blue and drawn from the field to the reader; references taken are dotted blue.
  - **A legend that is also the filter:** tick a line kind to show it, with its total count. Reads and references start hidden.
  - **Closed boxes show their size:** lines, functions (ƒ) and types (◆).
- To be judged on the prototype.

### FQ-02: Edges between levels — OPEN
How a call looks when one end is inside a closed box: merged into one line to the box (the prototype, thicker for more calls), ports on the box's sides, or hidden until opened.

### FQ-03: What the deepest zoom shows — OPEN
Below a function: its code inside its box with blocks marked in the gutter (the prototype), its blocks as boxes first and then the code, or the code in a popup or pop-out window.

### FQ-04: Where text lives — OPEN
Notes, code excerpts, the inspector and step narration: on the canvas, in popups, in pop-out windows (R2-10), or in a docked bar (ADR-0027 anatomy, not validated).

### FQ-05: Rendering and layout library (Q-08) — OPEN
Decided after FQ-01 to FQ-03, because the zoom model sets the requirements (nested layout, thousands of nodes, animation).

### Deferred here from the backend rounds
- R2-11: is "+N hidden" automatic?
- R2-21: the user clicking while Claude updates the view.
- R2-22: restoring the view and step the next day.
- R2-23: two Claude sessions on one window.
- R2-35: one view showing two snapshots (diff styling).
- Q-04 (remainder): how bubble inputs and outputs are drawn.
- Q-39: presenting a cross-device flow.
