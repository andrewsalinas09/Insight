# Vision

A new kind of IDE: instead of reading code file by file, you see the **graph** of what a piece of software does, and you learn it by following your AI agent as it explains and points.

**In one line: insight into vibe-coded programs, so you can learn from them as much as possible. Everything is graphed and connected.**

**The thesis:** when AI writes the code, we no longer inspect line by line. We inspect *system by system*, and we have to verify that the system does what we want. There are no good tools for this yet: no IDE and no debugger built for AI-first code. Insight is that tool. Its two purposes (ADR-0012) are **learning** (~60%), understanding code you didn't write, and **oversight** (~40%), seeing what's actually being built and checking it at the system level.

The prime directive (ADR-0002, scoped by ADR-0041): **Insight's own facts may be incomplete, but never wrong.** What the model says is interpretation, and it's shown as such.

The intelligence principle (ADR-0035): **the tool does what's trivial (deterministic, by algorithm), and the second there's a question it defers to the model.** "We don't need to write intelligence when it already exists. Tracing a call stack is not intelligent. Explaining how it works is." Practical test: if a compiler could do it, it's allowed (ADR-0043). Insight is a fancy, visual UI for a frontier model to use when explaining code to you, showing how it architected something, and letting you check it matches what you want, fast enough to keep up.

**What Insight is (ADR-0024):** the model already understands the code (it wrote it) and explains it well in text. Insight is the *instrument* that lets you see it: a graph of the real code, a UI you can navigate and hold in your head, and tools the model uses to highlight, focus, and step through as it explains, so you can trace and follow what it says.

**The first thing it must do well (ADR-0019):** load the software, ask about an algorithm, and get back how it works, the data structures it uses and where they live, and *how we know it's correct*.

## North-star goals

1. **Start from what you can see.** A UI element, a CLI command, a function, or a prompt ("what happens when I hit Save?") is a starting point. Your agent maps it to code with a query (ADR-0038).
2. **Every branch of execution.** The graph has every function and every branch arm (BQ-09, BQ-18). `valid` vs `!valid` are separate blocks. Error handling, early returns, retries, and fallbacks are all there to be shown, because that's where vibe-coded software hides its surprises.
3. **Hierarchical bubbles.** Files, types, functions (anonymous ones too, ADR-0022) and blocks nest inside each other. Open one and you see what's inside.
4. **Labeled for humans.** The solver builds the structure and the model names it. An edge gets a note like "error handling", a group reads "auth check", and you read the meaning first and the code second.
5. **Learn iteratively, in conversation.** You ask in the terminal, and the agent answers while the window follows along (R2-07). "Why does this exist?", "what breaks if I remove it?", "walk me through the failure path".
6. **Your code first.** Analysis covers the code in your project. Dependencies and the standard library are leaves, one per referenced symbol (BQ-20).
7. **Extendable.** Your agent edits code; Insight reads it (ADR-0017). Missing capabilities are requested through the wish rule, recorded in `INSIGHT.md` (ADR-0039, ADR-0044).
8. **The map and the journey.** Static analysis draws every branch that *could* run. Recorded runs that light up the path that *did* come later (ADR-0003, ADR-0015).
9. **Declared meaning.** Claude tags code inline (`@insight tag: disk-write`), and Insight parses the tags as data (ADR-0032, ADR-0043). Free-text comments are code the model reads. Verification is a tool the model uses, not a gate (ADR-0037, ADR-0041).
10. **Meets you where you are.** Write notes on what you know in `INSIGHT.md` ("React yes, Rust no; threading, but only Rust's"), and the model decides what to skip and what to teach (ADR-0004, ADR-0044).
11. **During development and after the fact.** Insight snapshots the working copy every time your project builds successfully (R2-10), so it keeps up as you work, and it's also a way to understand a finished vibe-coded app.
12. **Ask for the view you need.** "Show me how clicking Start reaches the core state, and where this function gets its members." The agent composes a view with exactly that hierarchy and the relevant code (ADR-0026, ADR-0040), across calls and data (read/write edges, ADR-0023).
13. **Explain it, then explain it differently.** If an explanation doesn't land, the model tries another angle: an example, a simpler version, the counterfactual, or a bridge from what you know. When you don't know what to ask, it walks you through one action (ADR-0035, ADR-0040).
14. **A learning journal** *(post-MVP)*. Every question you ask is kept in your own words with its answer and where it lives in the code, each entry pinned to the commit it was made on (ADR-0029, Q-22).
15. **Show, then tell.** Ask "why not just make it atomic?" and the graph lights up the check and the claim, then steps two workers through the race, while the agent explains in the terminal. Notes and steps in the window can link to the nodes they mention (BQ-40). The graph is a companion that answers by pointing (ADR-0010).
16. **Step through it like a debugger, without setting one up.** Ask "walk me through what happens when the input is empty". The agent composes a step-through (ADR-0040) with any values, example or real. You click Next and Back, or say "show me the walkthrough" and it plays (BQ-39). Running the code to check itself is a tool the model can use (ADR-0037, post-MVP).
17. **Review changes as a system diff, not a line diff** *(post-MVP)*. After an AI session, see what changed in *behavior*: new paths, removed error handling, a new network call on the save path. A diff is two snapshots compared in SQL, and the model reconciles renames (ADR-0028, Q-25).
18. **Say what you want; Insight keeps checking it** *(post-MVP)*. "Saving never touches the network." "Every error reaches the UI." Expectations are saved SQL queries, re-run on each new snapshot (Q-26).
19. **The oversight loop.** Prompt the agent, then see what it did: "it goes in here, comes out here, the error bubbles up to here… wait, it never reaches the logger." Click in and ask "where else could this happen?" Tell the agent to fix it *by design*, with the graph as context. Watch the system diff confirm the fix, and save the question as an expectation so it never comes back (Q-25, Q-26; `docs/examples/oversight-loop-worked-example.md`).
20. **Across the wire, across devices** *(later)*. Load the firmware, the radio protocol, and the frontend as one project (ADR-0045). Ask "show me the trace that takes the data from the sensor to my frontend's display, and focus on the algorithms that process it." You get one graph across codebases, following the *value* itself (ADR-0014).
21. **Causal time you can see.** Ordered code reads as ordered; `await` and functions handed to outside code (callbacks, timers, spawns) are marked *may interleave*. Race candidates are one query away (ADR-0042).
22. **"How do we know it's right?"** For any value or algorithm, the solver returns everything connected to it, and the model sorts it into where it's validated, where it silently falls back to a default, which tests reach it, and what nothing checks (ADR-0019, Q-31).
23. **Works with your agent, not instead of it.** Claude Code, Codex, or whatever comes next does the coding. Insight is the tool it calls and the window you watch it through (ADR-0017). Anything the agent can do in Insight, you can do in the window directly (R2-09).

## Why now

Two things changed. First, a lot of software is now vibe coded: people ship systems they never read line by line, and the bottleneck has moved from writing code to understanding it. Second, frontier models can now read code well enough to explain its structure. Insight needs no provider access of its own: your subscription agent does the thinking, and Insight is its visual instrument (ADR-0025).

## What this is not

- Not a team or onboarding product. It's a personal tool for one user (ADR-0025).
- Not a chat client or a model host. Insight calls no model (ADR-0025).
- Not a coding agent. It reads code; the agent edits it (ADR-0017).
- Not self-modifying. New capabilities go through the user (ADR-0039).
- Not for code that doesn't build (ADR-0036).
