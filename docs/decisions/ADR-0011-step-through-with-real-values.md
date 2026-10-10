# ADR-0011: The step-through: a debugger-like walkthrough with concrete values, set up and verified by the AI

- **Status:** Accepted; amended by ADR-0037 (any value may be shown. Execution and verification are tools the model uses to check itself, not a gate on what it may display)
- **Date:** 2026-10-09
- **Source:** user, discovery session 5 ("step through like a static analysis debugger. The AI can set it up, test it and then give it to you. And you can click next as though it was a debugger with specific values. Useful beyond learning, but learning is the primary purpose")

## Context
ADR-0010 lets answers step through sequences on the graph. Learners understand code best when they see *specific values* flowing through it ("sensor id 1 → `start([1])` → …", from the learning log), which is exactly what a debugger shows. But a real debugger needs the app running and the right breakpoints, and it shows only what happened, not why.

## Decision
A **step-through** is a first-class artifact: an ordered sequence of steps over the graph, each with the current location, the variable values, and the prose. The user clicks Next and Back like a debugger, and every step drives the graph (ADR-0010).

**The AI sets it up, tests it, then hands it over:**
1. **Set up**: the AI chooses the scenario: entry point, inputs, initial state, and (for concurrency) an interleaving.
2. **Test**: the scenario is *executed or evaluated deterministically* to produce the values (Q-24 for how). The AI checks that it actually reaches what it was meant to show, for example the `!valid` branch, and adjusts the inputs if not.
3. **Hand over**: only a verified step-through reaches the user.

**Value provenance (ADR-0002):** every value shown is tagged *executed* (real run), *evaluated* (deterministic interpreter), or *assumed* (a stubbed boundary such as network or hardware, where the stub's return value is stated as an assumption). **A value the LLM merely predicted is never displayed as a value.**

## Why
Concrete values turn abstract flow into something you can follow, and Next/Back puts the pace in the learner's hands. Verifying before handover is what makes it trustworthy: an LLM narrating "x is now 5" from imagination is exactly the confident wrongness ADR-0002 forbids. Rejected alternatives: LLM-narrated walkthroughs with predicted values (unverifiable), and only real-debugger sessions (they need the live app and expertise, and can't show alternatives).

## Consequences
- Step-throughs are recorded runs with chosen inputs (ADR-0003). They upgrade edges to *observed* like any run, and give Q-17 an AI-driven way to capture runs without the user clicking through the app.
- Hypothetical scenarios (ADR-0010, TB-065) can become *real*: the AI writes the proposed alternative in a scratch copy and runs it under a deterministic scheduler, so the race is demonstrated rather than described.
- They're saved in the journal and shareable (Q-22).
- Secondary uses (not primary, per the user): bug reproduction ("step through the failing input"), PR review ("step through what this change does with these values"), and documenting tricky paths.
- Mismatches are valuable. If the learner (or the AI) predicted a different value than the run produced, showing both is a teaching moment (TB-068).
