# How people actually learn a codebase: patterns from a real Q&A log

Written 2026-10-09 from a real multi-day Q&A log where the user learned an unfamiliar (proprietary, vibe-assisted) Tauri + Rust desktop app by asking an AI questions. **All specifics are generalized**; nothing from the source app is reproduced. This is evidence for Q-13, Q-22, and Q-23, not a decision.

## The headline

The user's verdict partway through: *"this software isn't complicated but I don't really know how it's implemented. I don't even know what questions to ask."* Overviews of subsystems didn't work. **Following one concrete action through the code in small steps, with the AI supplying the next question**, did work: "yes! this is what I want."

Explanations were also anything but sparse. One function's worth of code produced several follow-ups, each answered from a different angle. So Insight needs **lots** of explanation, not minimal docs. See "Where explanations live" below for how this fits Q-21 rule 6.

## 1. The kinds of questions asked (generalized)

| # | Question shape | Real example, generalized | What Insight needs to answer it |
|---|---|---|---|
| 1 | **What is this framework/tool, really?** | "Is the desktop framework just IPC between the two languages?" | External knowledge placed *relative to this app* (which parts of the app it provides) |
| 2 | **What represents a domain thing?** | "Is there an object for a device?" | Domain concept → types, including the several types one concept splits into (saved record vs. live handle vs. status) |
| 3 | **What is this field for?** | "What is the id for?" | Data edges: where it's assigned, validated, read, and which invariants hold (Q-20) |
| 4 | **How does a feature work end to end?** | "How does discovery work?" | A flow trace across threads, network, and external protocols (links + effects) |
| 5 | **Data path vs. control path** | "Go over the collection path with many devices" | Two overlaid views: who decides vs. where the bytes go |
| 6 | **How does a cross-cutting concern travel?** | "How does error handling work?" | Error edges through every layer, up to the UI, including surprising swallows |
| 7 | **I don't know what to ask** | (verbatim, above) | **Guided walkthrough mode**: pick an action, step through it, supply the next question |
| 8 | **I'm lost on these names** | "I'm very lost on the whole handle / state / container / table thing" | Step back to a map of the names and how they relate before going on |
| 9 | **Why is it structured this way?** | "Why does the container wrap the engine? Why not top level?" | Design rationale, alternatives, and the *reusable rule*: "so I would do it myself" |
| 10 | **Where does this name come from?** | "What is spawn? Clone? Where do they come from?" | **Origin of each symbol**: ours vs. standard library vs. framework. Two different `spawn`s on one path |
| 11 | **How would someone think to write this?** | "I understand the pieces but not how someone would think to write this" (an iterator chain) | Derive code from a requirement; an equivalent plain rewrite (chain → loop) |
| 12 | **Challenge / what if** | "Why not just lock the one flag? Isn't that atomic already?" | The counterfactual shown concretely (an interleaving where two workers both win) |
| 13 | **Bridge to what I know** | "So it has smart pointers? Why doesn't C++ have this?" | Map to known languages (knowledge profile, ADR-0004) and correct the premise kindly |
| 14 | **Check my restatement** | "So it's just an infinite loop polling the table?" | Confirm and refine ("yes, with two refinements…") |
| 15 | **Language semantics** | "What counts as failure for `?`? What is a crate?" | Precise rules, plus the expansion (`?` → `match`) |
| 16 | **Is this a mistake?** | "Why `crate::` and not a name? Seems like an oversight?" | Explain the language design; separate it from the code-organization choice |

## 2. Teaching moves that worked

1. **Follow one concrete action**, in small excerpts, instead of inventories of types.
2. **Supply the next question** at the end of each step.
3. **One concept at a time**, introduced only when the code needs it.
4. **A simplified teaching version first, then the real code**, with the omissions named ("styling and the stop branch omitted").
5. **Teaching expansions**: `?` → explicit `match`; a method chain → named variables; an iterator chain → a plain loop.
6. **Illustrative values, labeled as such** ("an example device with id 1").
7. **Explain why each container exists**, what problem the split solves, and when the alternative is reasonable.
8. **Pause and step back** when the user is lost; don't push forward.
9. **Show the failure** for a "why not X?" (a step-by-step interleaving).
10. **Bridge from what's known** (C++ `shared_ptr` ↔ Rust `Arc`).

## 3. Honesty rules the log kept (these map directly onto ADR-0002)

- **Inspect the code before making claims.** Docs lag the code. The log caught a README saying one worker count while the code said another, and treated the code as truth (code-first, Q-21).
- **Separate implemented / intended / inferred / verified.** "This traces source; it is not a fault-injection test."
- **"Makes sense" is not understanding.** The log explicitly refuses to treat a "makes sense" as mastery, and tracks what is still unclear.
- **Corrections are preserved, not overwritten.** A later step found the error path from an earlier answer was wrong (a wrapper swallowed the error). The earlier entry got a visible correction.
- **A partial diagram misled.** Showing only one field of a container made the container look like a pointless wrapper. The fix was to show its siblings. *Omission can create a false belief*, so omissions must be visible.

## 4. The record format the user chose

One entry per question: **Question** (in the user's words) · **The confusion** · **Answer** · **How we got there** (follow-ups and corrections, in order) · **Where it lives** (code paths, which act as receipts) · **Still open**. Entries link forward and back ("refines entry 6"). There's an index, plus a separate "leads worth testing" list.

## Implications for Insight (proposals)

- **Explanations are abundant; committed annotations stay minimal.** Q-21 rule 6 is about what goes in the *repo*. Explanations are a different, much larger layer (Q-22).
- **The learning journal is a first-class artifact** (Q-22): the format above, generated as you learn, attached to graph nodes, personal like the knowledge profile, and arguably *the source* the profile evolves from.
- **Explanations carry receipts too.** "Where it lives" = anchors. When the code changes, the explanation is flagged "may be outdated", which is the same drift machinery as annotations (Q-21).
- **"Explain it differently" needs an explicit ladder** of angles (Q-23).
- **Views must show omissions** ("+11 more fields hidden") so that subtraction doesn't mislead (TB-056).
- **Symbol origin** (ours / std / framework / dependency) is deterministic and should be visible everywhere (TB-059).
