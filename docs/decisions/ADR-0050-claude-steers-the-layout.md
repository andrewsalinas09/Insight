# ADR-0050: Claude steers the layout of a view: chains, order, and layout options

- **Status:** Accepted
- **Date:** 2026-10-10
- **Source:** user, frontend session ("claude should be able to control the order a little bit. Maybe an algo it can tweak or do it manually. Like the order of the hierarchy or something in order to show a certain path for example.")

## Context
The window lays the code out automatically: nested boxes, left to right (ELK layered, ADR-0051). Automatic placement doesn't know what Claude is explaining. A route Claude walks through can end up zig-zagging across the screen, and the boxes it talks about can sit far from each other.

## Decision
A view (`show`, ADR-0040) carries a **`layout`** section with three levers. They work alone or together, and the user has the same ones by hand (R2-09):
- **`chain`:** sequences of code references (or a `path` search, which stands for one shortest route). Each is laid out left to right in that order, kept on one row where it can be. The window adds invisible, high-priority layout edges between consecutive members, at any nesting depth.
- **`order`:** boxes that come first within their parent box, in this order (top to bottom in a column). By hand: ▲ Up / ▼ Down in the inspector.
- **`options`:** ELK layered options passed to the layout engine (spacing, node placement, layering, and so on). The algorithm and hierarchy handling are fixed by the window and refused. By hand: the spacing sliders in Settings.

## Why
- Covers both ways the user named. "Manually" is `chain` and `order`. "An algo it can tweak" is `options`.
- **Layout is a view parameter, not a tenth element kind.** It changes where real things are drawn and adds nothing to the picture, so ADR-0040's fixed vocabulary is unchanged.
- **Tested before deciding.** Invisible ordering edges put a sequence left to right at every nesting depth, even against the direction of the code's own calls. With network-simplex placement they keep it on one row.
  - ELK's partitioning (forced columns) works only at the top level, so it was not used.
  - Model order works per parent box, which is what `order` uses.
- Rejected:
  - Absolute positions from Claude. They break as soon as a box opens or closes.
  - A fixed set of named presets. These are runtime choices, so they are Claude's, as parameters (ADR-0047).

## Consequences
- References in `chain` and `order` must resolve, like every code reference in a view.
- The window turns on network-simplex node placement when a view has chains, unless `options` says otherwise.
- `layout.options` is ELK's vocabulary. If the layout library ever changes, this lever changes with it (ADR-0051).
