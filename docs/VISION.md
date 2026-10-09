# Vision

A new kind of IDE: instead of reading code file by file, you see the **execution graph** of what a piece of software does, starting from anything you can trigger, and you learn it by exploring that graph with an AI.

**In one line: insight into vibe-coded programs, so you can learn from them as much as possible. Everything is graphed and connected.**

**The thesis:** when AI writes the code, we no longer inspect line by line. We inspect *system by system*, and we have to verify that the system does what we want. There are no good tools for this yet: no IDE and no debugger built for AI-first code. Insight is that tool. Its two purposes (ADR-0012) are **learning** (~60%), understanding code you didn't write, and **oversight** (~40%), seeing what's actually being built and checking it at the system level.

The prime directive (ADR-0002): **the graph may be incomplete, but never wrong.**

**The first thing it must do well (ADR-0019):** load the software, ask about an algorithm, and get back how it works, the data structures it uses and where they live, and *how we know it's correct*.

## North-star goals

1. **Start from what you can see.** Clicking a UI element, running a CLI command, picking a function, or typing a prompt ("what happens when I hit Save?") is a starting point. Insight works out which code that trigger reaches.
2. **Every branch of execution.** It finds all the paths, not only the one you happened to run. `valid` vs `!valid` are two edges. Error handling, early returns, retries, and fallbacks are visible and named, because that's where vibe-coded software hides its surprises.
3. **Hierarchical bubbles.** A function is a bubble with its inputs and outputs. Open it and you see the nested structure inside: the calls and branches it contains, down as far as you want to go. Zoom out and the whole flow fits on one screen.
4. **Labeled for humans.** Code analysis builds the structure and an LLM names it. An edge reads "error handling", a cluster reads "auth check", and a bubble gets a one-line summary. You read the meaning first and the code second.
5. **Learn iteratively, in conversation.** The graph is the shared context between you and the AI. Select a bubble and ask "why does this exist?", "what breaks if I remove it?", or "walk me through the failure path". Understanding builds up over many sessions.
6. **Your code first.** Analysis stays within the code you added to the project. Dependencies and the standard library are leaves by default, and you can expand them when you want to.
7. **Extendable.** You can add more to the graph and the codebase from the graph. (Exactly what "add" means is open, see Q-10.)
8. **The map and the journey.** Static analysis draws every branch that *could* run; a recorded run lights up the path that *did* (ADR-0003).
9. **Documentation that can't lie.** Annotations follow a clearly defined style and are generated automatically, like Doxygen but far better. A compiler checks them against the code, so hallucinations are removed before you see them (Q-15).
10. **Meets you where you are.** Write notes on what you know ("React yes, Rust no; threading, but only Rust's"), and the LLM decides what to skip and what to teach (ADR-0004).
11. **During development and after the fact.** It sits beside your editor and shows code structure instantly as you work, and it's also a way to understand a finished vibe-coded app. Both modes are goals (Q-18).
12. **Ask for the view you need.** "Show me how clicking Start reaches the core state, and where this function gets its members." The AI composes a custom view with exactly that hierarchy and the relevant code snippets (Q-19), across both control flow and data (Q-20).
13. **Explain it, then explain it differently.** "What does this do?" always has an answer. If it doesn't land, Insight tries another angle: an example, a simpler version, the counterfactual, or a bridge from what you know. When you don't know what to ask, it walks you through one action and suggests the next question (Q-13, Q-22, Q-23).
14. **A learning journal that stays true.** Every question you ask is kept in your own words with its answer, its corrections, and where it lives in the code. When the code changes, outdated explanations get flagged (Q-22).
15. **Show, then tell.** Ask "why not just make it atomic?" and the graph lights up the check and the claim, then steps two workers through the race, while the prose spells it out. Every sentence is linked to what it's about. The graph is a companion that answers by pointing (ADR-0010).
16. **Step through it like a debugger, without setting one up.** Ask "walk me through what happens when the input is empty". The AI picks the inputs, runs the code to check it really goes where it should, then hands you a step-through. You click Next and Back, watch real values change, and see the graph move with you. It's built for learning first, and also handy for reproducing bugs and reviewing changes (ADR-0011).
17. **Review changes as a system diff, not a line diff.** After an AI session, see what changed in *behavior*: new paths, removed error handling, a new network call on the save path, data now shared across threads. Renames and moves that change nothing are quiet (Q-25).
18. **Say what you want; Insight keeps checking it.** "Saving never touches the network." "Every error reaches the UI." Plain-language expectations are compiled into deterministic checks on the graph and re-run on every change (Q-26).
19. **The oversight loop.** Prompt the agent, then see what it did: "it goes in here, comes out here, the error bubbles up to here… wait, it never reaches the logger." Click in and ask "where else could this happen?" Tell the agent to fix it *by design*, with the graph as context. Watch the system diff confirm the fix, and save the question as an expectation so it never comes back (Q-25, Q-26, Q-27; `docs/examples/oversight-loop-worked-example.md`).
20. **Across the wire, across devices.** Load the firmware, the radio protocol, and the frontend together. Ask "show me the trace that takes the data from the sensor to the microcontroller, over Wi-Fi/BLE, to my frontend's display, and focus on the algorithms that process it." You get one graph across three codebases, following the *value* itself, with plumbing collapsed and every place the data is transformed (or rescaled, or filtered twice) in view (ADR-0014).
21. **Time you can see.** Waiting, spawning, "later", polling, and crossing a wire each look different, and threads, windows, and devices are lanes in a sequence view. You never have to guess whether something blocks (ADR-0018).
22. **"How do we know it's right?"** For any value or algorithm: where it's validated, where it silently falls back to a default, which tests check it, and what nothing checks (ADR-0019, Q-31).
23. **Works with your agent, not instead of it.** Claude Code, Codex, or whatever comes next does the coding. Insight is the tool it calls and the window you watch it through (ADR-0017).

## Why now

Two things changed. First, a lot of software is now vibe coded: people ship systems they never read line by line, and the bottleneck has moved from writing code to understanding it. Second, LLMs can now read code well enough to name and explain its structure, and provider access is opening up (API credits, and possibly signing in with your own Anthropic or OpenAI account inside third-party software; research is pending, see Q-06). This tool is LLM-powered through and through, and it couldn't have been built this way before.

## What this is not

To be settled during discovery. The candidate list is in OPEN-QUESTIONS (Q-10, Q-11).
