# ADR-0035: The intelligence principle: the tool does what's trivial, and the moment there's a question it defers to the model

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8, stating the philosophy behind the whole project

> "The idea is that frontier models are geniuses and we want the tooling to get out of the way but give it what it needs to do its job. This is basically a fancy UI for, say, Opus 5.5 to use when explaining things to me, or letting me see what / how it architected something / make sure it matches what I want in a fast way to keep up. (I would love to make sure it does exactly what I want but that's not always possible.) I'm also very visual. The tool also does the things it can easily do trivially. Trivial = deterministic by algorithms. The *second* there's a question, it's deferred to the actual intelligent thing here. We have for the first time a program that can make intelligent decisions. We don't need to write intelligence when it already exists. Tracing a call stack is not intelligent. Explaining how it works is."

## Decision
Every design choice in Insight is judged by one test:

- **Trivial → the tool.** If a deterministic algorithm can do it exactly (parse with a language's own toolchain, connect calls and reads and writes, trace a path, count, diff two commits, cache, render), Insight does it, fast and exactly.
- **A question → the model.** The moment something requires judgment (what this means, which matters, how to explain it, how to group it, whether a rename is "the same thing", what to tag), Insight doesn't try. It hands the model what it needs and gets out of the way.
- **The product is the space between the two:** a visual instrument (the user is very visual) through which a genius model shows its understanding, so the human can see how something was architected and check it matches what they want, fast enough to keep up.

## Why
For the first time there's a program that makes intelligent decisions. Writing intelligence by hand (rule catalogues, heuristics, framework semantics) duplicates what the model already does better, and it never ends. Doing the trivial part deterministically keeps the model's inputs exact and cheap. Deferring every question keeps Insight small and lets it improve automatically as models improve.

## Consequences
- This unifies and explains ADR-0023 (solver connects, model understands), ADR-0024 (Insight is the instrument), ADR-0028 (the model reconciles renames), ADR-0032 (tags are declared, not inferred), ADR-0033 (language semantics are the deterministic part) and ADR-0034 (use existing toolchains).
- It sits beside ADR-0002 ("incomplete, never wrong") as one of the two principles every proposal must pass: *Is this trivial? Then make it exact. Is it a question? Then it's the model's.*
- Stated aim: verifying that what was built is what you wanted, "in a fast way to keep up". Exact verification is a goal, not a promise ("that's not always possible").
- **Practical test (ADR-0043, user): "if a compiler could do it, it's allowed."** Parsing, keyword grammars, name resolution and structural checks belong in the tool; interpreting free text belongs to the model.
