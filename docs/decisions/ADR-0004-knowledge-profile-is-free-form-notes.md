# ADR-0004: The knowledge profile is free-form notes the LLM interprets

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 2 (Q-16: "that's the power of LLM, it can choose what to do with your notes")

## Context
To learn as much as possible, explanations have to match what the user already knows. Knowledge is uneven and doesn't fit categories. The user's own example: "I know React, not Rust. I know multithreading, but not C++-specific threading, only Rust's."

## Decision
The user describes what they know in plain-language notes. There are no skill levels, checkboxes, or taxonomy. The LLM reads the notes and decides what to skip, what to explain, and how to frame it, for example bridging new ideas from concepts the user already knows.

## Why
A structured skill matrix can't express "I know the concept but not this language's version of it", and that is exactly the distinction that matters (Rust's `Mutex<T>` owns its data, which is different from a C++ mutex). Interpreting nuanced notes is something LLMs are good at. Rejected alternatives: beginner/intermediate/expert levels, which are too coarse, and per-technology checklists, which are rigid and never complete.

## Consequences
- The profile is personal and lives with the user (app-level), not in the repo. It's never committed with annotations (ADR-0005).
- Personalization only affects *explanation* (prose and chat). Claims and structure are identical for every user (ADR-0002).
- Notes can grow over time. Whether Insight proposes additions ("you seem comfortable with `?` now") is part of Q-13.
- "Abstraction levels" (zoom) is a separate topic and stays with Q-04 and Q-19.
