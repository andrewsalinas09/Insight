# ADR-0012: Two purposes: learning (~60%) and system-level oversight of AI-written code (~40%)

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 5 ("Actually 40% code development, to have insight into what's actually being vibe coded. The idea is that we no longer inspect line by line. We inspect system by system… There's no good tools for 'debugging' and 'IDE' for AI-first written code")

## Context
Earlier sessions treated development uses (bug reproduction, PR review) as side benefits of a learning tool. The user corrected this: development oversight is a core purpose. When AI writes the code, humans stop reviewing line by line, so they need to review and verify **system by system**: does the system still do what we want? No current tool does this. IDEs and debuggers are built for humans who read and write every line.

## Decision
Insight has two purposes sharing one engine:
1. **Learning (~60%)**: understand a codebase you didn't write (ADR-0004, ADR-0010, ADR-0011).
2. **Oversight (~40%)**: the IDE and debugger for AI-first development. Inspect what's being built at the system level, and verify it does what you want without reading every line.

Both use the same fact layer, the same compiler, the same never-wrong rule, and the same views. Design decisions must serve both, and the learning purpose wins ties.

## Why
The two purposes need the same thing: a trustworthy, navigable, system-level picture of code a human didn't write line by line. A learner asks "what does this do?"; an overseer asks "what did this change do, and is it still what I want?" Building them separately would duplicate the hard part (the deterministic graph plus verification).

## Consequences
- New first-class needs for oversight: a **system-level diff** (Q-25) and **system-level expectations** that are checked on every change (Q-26).
- Live mode next to the editor (Q-18) and integration with coding agents become more important. The tool API (ADR-0006) doubles as the agent-facing interface: an agent can ask Insight what it's about to change and what it did change.
- Q-11 (who is it for) is partly answered: learners *and* developers supervising AI agents.
- Q-10 (IDE or not) leans toward IDE: "debugging and IDE for AI-first code" is the user's own framing.
