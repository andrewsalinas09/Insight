# ADR-0053: A details panel (docked or popped out), data traces, and a Back history

- **Status:** Accepted
- **Date:** 2026-10-10
- **Source:** user, on the trace view with their own project. "it says '… is data there is no code of it in the snapshot to step into'. I feel like if i clicked it, it should show me the … type or something." "I feel like navigation is unintivive slightly. Like I can close something and it jumps out somewhere random." "I feel like the bottom thing should have a popout button (like I love so much) so that I can see much more detail about the left and right. I feel like we can add a tonnn of useful ifnormation here." Claude proposed the three below; the user: "Yeah do that."

## Context
The trace view (ADR-0052) showed how functions connect, but almost nothing about each thing: a field was a dead end, a function was just a name, and moving around had no way back.

## Decision
1. **Details panel.** Docked at the bottom (D or the Details button), or popped out to its own window (R2-10). It follows the trace's current stop (or what's hovered there), or the selection on the map. Facts only, every name a link that goes there:
   - **Everything:** what's written directly above it (comments, attributes, annotations: the lines up to the first blank line or the previous thing in the same box) and how it starts or is declared.
   - **Function:** who calls it, grouped by top box, with lines; what it calls in the order written; what it writes and reads; its library calls.
   - **Type:** what it holds (each field with its declaration line); functions inside; who writes and reads it.
   - **Field or static:** its declaration, what it's inside, who writes and who reads it.
   - **Library symbol:** where your code calls it.
2. **Data traces.** Stepping into a field, static or type no longer stops: the middle shows its declaration inside its type's code (or a few lines around it when the type is huge), the functions that write it (or take a reference) on the left and those that read it on the right. ↓/↑ walk them; Enter opens that function at the line that touches the data. Step out from data goes to its type. Data traces count every read and write, whatever the legend shows.
3. **Back history.** Every move (step in, step out, open or close a box, reshape, trace in and out, a link in the details) is remembered. ↶ Back (Alt+←, or the mouse's back button) returns to exactly the view before, camera included. Esc from a trace returns to the map as it was when the trace began. Claude's step buttons are now ◀ Prev / Next ▶, so "Back" means one thing.

## Why
- Everything shown is mechanical: spans, receipts and edges already in the snapshot (ADR-0035). "What's written above it" is positional, so it works in every language without knowing its comment syntax (ADR-0048).
- Data is half of understanding code; a field that leads nowhere broke the debugger model the user liked.
- Predictable navigation: a way back from anything is what makes exploring safe.

## Consequences
- A field's own type is read from its declaration line. Linking a field to its type's box needs `has-type` edges (SCHEMA, Planned), which no extractor emits yet.
- In C++, methods defined outside their class (`Type::method` in a .cpp) are not inside the type's box; the type's details list only what's written inside it.
- The popped-out window talks to the main one by events (`detail`, `detail-go`, `detail-ready`, `detail-closed`).
