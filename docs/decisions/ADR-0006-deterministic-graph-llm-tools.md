# ADR-0006: The graph is built deterministically; the LLM works on it through tools

- **Status:** Superseded by ADR-0013 (the LLM may add, with receipts; the tool-API idea carries over)
- **Date:** 2026-10-09
- **Source:** user, discovery session 2 ("you build the graph deterministically, and then the LLM has tools to put in hierarchy, collapse, expand, annotate etc.")

## Context
ADR-0002 forbids the LLM from making structure up, but the raw graph of a real codebase is unreadable without organization. The LLM's value is in organizing and explaining it. We need a boundary between the two.

## Decision
Code analysis builds the graph, which is the fact layer. The LLM is an agent with a **tool API** over that graph, with operations such as: group nodes into a hierarchy level, collapse, expand, annotate (claims + prose, Q-15), and compose a custom view (Q-19). The LLM can never create or delete a structural node or edge. It can only organize, filter, and describe what already exists.

## Why
This makes ADR-0002 hold by construction rather than by trusting the LLM: the tool API simply has no "add edge" operation. Tool calls are also inspectable, replayable, and can be committed as data (ADR-0005), whereas free-form LLM output can't. Rejected alternative: the LLM emits a graph directly, such as JSON or a diagram. That's impossible to verify, and wrong without anyone noticing.

## Consequences
- The tool API is a core interface. It's probably exposed as MCP so any agent (Claude Code, etc.) can drive Insight, not just the built-in chat.
- Groupings and hierarchy levels are *organization*, not facts. Their names are prose.
- Every view is reproducible: the same graph plus the same tool calls gives the same view.
