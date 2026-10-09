# ADR-0015: Tracing is static-first, with the LLM in the loop where unsure, and LLM-placed log probes as the universal runtime tracer

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 7 ("this is one of those things that can be traced with LLM in the loop. The tracer can come from logs (which the LLM can add) but also static analysis is good for like 99% of the time. The calls are direct, and whenever it's unsure LLM can again be in the loop")

## Context
ADR-0003 settled *hybrid* tracing (static map + dynamic overlay) but not *how* each half works across very different targets: desktop apps, servers, scripts, and microcontrollers with no room for a tracer (TB-090). Building a native runtime tracer per language and platform is expensive, and impossible on some devices.

## Decision
1. **Static analysis first.** The language plugin resolves what it can deterministically. In most code most calls are direct, so this does most of the work.
2. **LLM in the loop where static is unsure.** Unresolved dispatch, callbacks, string-keyed routing, and fuzzy wire matches go to the LLM. Its answers enter as receipted claims at the *inferred* tier (ADR-0013), never as proven.
3. **Log probes are the universal runtime tracer.** To observe a run, the LLM inserts structured log statements ("probes") at the points it needs: branch arms, call sites, sends and receives, values on a lineage path. The code then runs and the logs are parsed back onto the graph. This works anywhere code can print: a desktop app, a server, or a microcontroller over serial. Probe output is *observed* (ADR-0003). Probes can also settle the LLM's inferred edges, upgrading them to observed or refuting them.

**Rules for probes:**
- Probes are added in a scratch copy or branch, never silently in the user's working tree.
- Each probe carries a machine-readable tag mapping it to its graph node, so the parsing is deterministic and the LLM doesn't interpret the log text.
- Probes are removed after the run unless the user chooses to keep them.

## Why
Static analysis is cheap, complete over branches, and needs nothing running. The LLM covers the dynamic remainder without pretending to proof. Log probes reuse the one runtime facility every platform already has (printing) instead of a tracer per language and per target. That makes runtime observation available on embedded targets from day one, which is critical for the user's cross-device workspaces (ADR-0014). Rejected alternative: native tracers or debuggers per language first. They're higher fidelity, but slower to build, platform-limited, and unavailable on microcontrollers. They may still come later as optional plugins.

## Consequences
- **The "99%" is the user's estimate, to be measured, not assumed.** It's plausible for Rust, Go, and C with direct calls. It's likely lower for TypeScript/React (props, events, hooks), Python (duck typing), and DI-heavy code. Each plugin reports its own resolution rate (ADR-0007 consequences), so this becomes a number per codebase (TB-097).
- **Probes perturb what they measure.** Logging changes timing, which matters for races and real-time firmware. Concurrency observations made with probes must say so (TB-096).
- Q-17 (capturing runs) and Q-24 (step-through values) gain probes as their default mechanism. Native tracers and debuggers become optional upgrades.
- Firmware probes mean rebuild and reflash, which needs the user's hardware in the loop. The step-through flow must be able to pause for "flash this and press the button".
