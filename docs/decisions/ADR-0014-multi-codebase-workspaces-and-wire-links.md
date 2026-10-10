# ADR-0014: A workspace spans multiple codebases; wire links connect them

- **Status:** Accepted; wire links refined by R2-38 (exact same-literal facts plus model-declared pairs); "inferred" tier retired by R2-03; workspaces are projects (ADR-0045); amended by ADR-0048 (programming concepts only; the generic `names` link replaces framework links; `names` hops are order-unknown)
- **Date:** 2026-10-09
- **Source:** user, discovery session 7 ("we can load multiple things in and get communication across a wire too… 'Show me the trace that takes the data from the sensor to the uC to wifi/bluetooth to my front end to display, and focus on the algorithms that do data processing of XYZ'")

## Context
Real systems aren't one repo. A sensor product is firmware on a microcontroller, a radio link (Wi-Fi/BLE), and a host app or frontend, often in different repos and languages. The interesting behavior, *where my data comes from and what happens to it*, crosses all of them. Until now Insight's graph stopped at one codebase. Links (Q-20) covered IPC inside one app, not separate programs on separate devices.

## Decision
An Insight **workspace** can load several codebases. The graph connects them with **wire links**: edges between a *send* effect in one codebase and the matching *receive* in another, over a protocol (JSON over Wi-Fi/WebSocket/HTTP, BLE characteristics, serial, MQTT, …). Wire links are plugins, like language plugins and the Tauri bridge (ADR-0007). They obey the same truth rules (ADR-0002, ADR-0013):
- **proven**: both sides reference the same definition (a shared header, schema file, UUID constant, or generated code);
- **inferred**: matched by convention, such as the same JSON keys, endpoint path, or characteristic UUID literal, and drawn distinctly;
- **observed**: seen in a captured run on both sides (logs, serial output, packet capture), correlated by message (ADR-0003).

## Why
"From the sensor to the display" is the question people actually have about embedded and IoT systems, and no tool answers it end to end: each IDE stops at its own repo. Treating the wire as just another link keeps the model uniform. A send/receive pair is an edge with a receipt, like an IPC call. Rejected alternative: one workspace per repo with manual cross-references. That breaks every cross-device question, the system diff (Q-25), and expectations (Q-26).

## Consequences
- **Field-level data lineage becomes necessary** (Q-29). "Take the data from the sensor to the display" follows a *value* (a reading, a JSON field) through transformations, not just calls.
- **Embedded languages** (C/C++ on microcontrollers) join the plugin list (Q-03). Runtime tracing on a device is harder: observed data comes from logs, serial, or captures rather than an in-process tracer (Q-17, Q-24).
- **Wire mismatches become oversight findings**, e.g. the firmware sends `ax` but the frontend reads `accelX`. The link is missing, and that's shown, not guessed (TB-088).
- Step-throughs across devices need stubs or captured messages at the wire (ADR-0011, *assumed* values).
- Focus requests like "focus on the algorithms" are lenses over this larger graph (Q-19).
