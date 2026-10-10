# ADR-0040: The fixed view vocabulary: nine element kinds, all in the MVP

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, BQ-33/34 ("yeah that seems really good tbh. All of those including abstraction and example data")

## Decision
A view (`show(view)`) is composed from exactly these elements. Claude composes them freely per question (ADR-0039), and each one maps to real graph nodes:

| Element | What it is |
|---|---|
| **Nodes and edges** | Any subset of the snapshot's graph |
| **Groups** | Named, nestable boxes over real nodes: Claude's own hierarchy for this view |
| **Collapse** | A group or container folded to "+N hidden", expandable |
| **Highlight / focus** | Emphasize a set of nodes, edges or a path; zoom to it; dim the rest |
| **Notes** | Text on a node, edge or group |
| **Steps** | An ordered list of view states (highlight + focus + note), driven by Next/Back |
| **Code excerpts** | Real lines from the snapshot, optionally trimmed (omissions marked) |
| **Abstraction nodes** | One box standing for a set of real nodes, with Claude's pseudocode; expands to the real nodes |
| **Example data** | A table or small diagram of illustrative values (e.g. `hist.nodes` drawn as a tree), linked to the fields it illustrates |

All nine are in the MVP. Adding a tenth kind is a feature request through the user (ADR-0039).

## Why
This is the smallest set that covers every move in the user's learning log and both Fletcher walkthroughs: regrouping, re-hierarchy, pseudocode, simplified code, before/after values, step-throughs. It's fixed so the tool stays stable, and expressive so composition stays free.

## Consequences
- Supersedes the open parts of Q-19/ADR-0026 on *what* a view can hold. Per ADR-0037 nothing about content is policed; only references to real nodes must resolve.
- Expands ADR-0019's MVP scope with the full teaching-view vocabulary.
- **Styling and mapping (R2-02, user: "yeah"):** the window styles elements by kind automatically. Real nodes, edges and code excerpts look *real*; abstraction nodes, example data, notes and simplified code look like *Claude's interpretation*. Claude marks nothing by hand. Groups, collapses, abstractions and excerpts must resolve to real code (that's the only check). Notes and example data may point at nothing.
