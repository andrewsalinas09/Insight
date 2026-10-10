# View proposal: showing a cross-device data flow (Claude's idea, not confirmed)

**Status: unconfirmed.** Claude wrote this on 2026-10-10 after tracing a real two-repo question, roughly "how does a sensor's measurement get to the gateway, and where is it stored". The user asked for it to be jotted down to look at later. **The user has not confirmed any of it.** It is input for the frontend session (see `docs/ROADMAP.md`), not a decision. Q-39 tracks it.

The example below is generic. A sensor firmware samples a device and sends batches over a radio. A gateway firmware receives them and stores them to flash for upload.

## The answer as data (what `path` / `sql` / `source` produced)

About 12 main steps across two repos:
- **Interrupt → tasks:** an interrupt handler, then three tasks connected by queues.
- **The radio write.**
- **On the gateway:**
  - a stack callback;
  - two indirections, a virtual call and a callable-object field;
  - a per-characteristic callback;
  - a reassembly buffer.
- **Deferred handling:** the data is processed on disconnect, then stored to the file system and an uploader is notified.

## Proposed presentation

1. **Lay it out by where the code runs.** Columns per device (sensor | radio | gateway). Inside each, one lane per thread or task, because the queue hand-offs are thread changes.
2. **The main route as one highlighted spine.** Everything else collapses into labeled groups that can be opened (for example "radio state machine, N functions", "logging", "buffer pool").
3. **Edges styled by certainty.**
   - **Solid:** a compiler fact.
   - **Dashed, labeled with the queue variable:** the same queue is passed to both the send and the receive.
   - **Amber "to review":** equality links such as matching characteristic IDs across the radio.
   - **Notes where timing changes,** for example "stored on disconnect, not on receipt".
4. **The data's shape at each stage** (example-data elements): raw samples, packet struct, batch format, length-prefixed chunks, upload wire format, file path pattern.
5. **Steps.** "Play" walks the spine one step at a time, with each step's code next to it; the user can also step through it manually.

## Focusing on something in between (for example a processing algorithm in the middle)

- **The selected part expands in place.** Its neighbors shrink to stubs, and a **mini-map of the whole journey** stays visible with the current position marked.
- **The focused part is shown at a finer level:** its own call tree and its data flow (state, reads, writes).
- **Runtime configuration:** if the middle is configured at runtime (for example a processing graph loaded from a file), the code alone gives only possible candidates. Claude reads the actual configuration and marks what really runs, drawn as interpretation.
- **Narrowing the question:** a sub-segment becomes a new `path` query between its two ends, so every zoom level stays backed by receipted hops.
- **Zooming back out** collapses the segment again; the main route stays where it was.

## What this would need beyond what's built

- groups by thread or task;
- edge styles by certainty (including "to review");
- a mini-map;
- expand-in-place.

All of these map onto the nine view elements (ADR-0040) and would come with the window.
