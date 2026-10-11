# ADR-0052: The trace view steps through a function's calls like a debugger; lines are hoverable and tied to the code

- **Status:** Accepted
- **Date:** 2026-10-10
- **Source:** user, frontend session. On the running app: "If I zoom in i can't see where anything connects. Id' be nice if there was a way to see like hover over a line or something and see where it goes. The ordering is not clear. … it's missing something magic to help anyone understand this." And on the code: "I love the idea of having the code so do'nt get rid of that but maybe a way to connect the lines to the graph." Claude proposed the points below; the user: "I say yes build everything".

## Context
The map (ADR-0051) draws the whole system as nested boxes with every line at once. Inside a big function the lines cross, their ends go off screen when you zoom in, boxes are placed to keep lines short rather than in the order the code runs, and the code view and the graph sit side by side without being linked. Advanced readers can work with that. Someone learning a codebase can't tell what they're looking at.

## Decision
1. **Lines and boxes are coloured apart.** Calls get their own colour (teal), separate from the ink used for boxes. Data, handoffs and library code keep theirs.
2. **Hover a line** on the map: it lights up with both ends, the rest dims, and the bar names both ends with the call lines behind it. Click it and the camera frames both ends. Hovering a box lights only its lines, as before.
3. **The code is tied to the graph.** Every call and data use in a function's code is marked: a tick at the right edge (where its line leaves) and the call underlined, in its line's colour. Hovering a call lights what it calls; clicking it opens the trace there.
4. **The trace view** (select a function and press Enter, or Trace ▸ in the inspector, or click a call in the code):
   - The function's code in the middle; its callers on the left; on the right, everything it calls, reads and writes, each level with the line that does it, **in the order written**.
   - ↓/↑ (or ▼/▲) step to the next or previous call. Enter or → steps in to the callee, which opens where the callee's box was. Backspace or ← steps out, laid out so the function you left sits where its code was. The trail at the top (`main › run › rel`) jumps back to any frame. Tab picks among the targets of a dynamic call. Esc goes back to the map, opened at the function you ended on.
   - Library code and data can't be stepped into, and the window says so.
5. **Claude can open it.** A view's `trace` is `{stack, line}`: the call stack (outermost first; a `path` search stands for one route) and the line to start at. A step can carry its own `trace`, so Claude can walk a call path with a note per step. `selection` reports where the user is in a trace.

## Why
- **The order is a fact, not an interpretation.** Each call's place in the code is a receipt in the snapshot (BQ-12). Stepping through calls in the order written is mechanical, so it's the tool's job (ADR-0035). Why the code does it is still Claude's to explain.
- **One function and its neighbours** answers "what am I looking at" better than any layout of everything. The map stays for the whole system; the trace view is for following one path.
- **A debugger is a model people already have**: step in, step out, a call stack. It works the same in every language, because it only uses calls, reads and writes (ADR-0048).

## Consequences
- The window's graph carries each line's sites (`[line, col, end line, end col]`, from the first receipt of each occurrence).
- The order is the order *written*, not the order *run*: branches and loops aren't evaluated. The code (with its block bands) shows which branch a call is in.
- Only the line kinds ticked in the legend are stopped at. Library calls start unticked, so a trace skips them unless the user ticks them.
