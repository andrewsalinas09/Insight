# ADR-0017: Coding agents live outside; Insight is their tool and your window onto their work

- **Status:** Accepted
- **Date:** 2026-10-09
- **Source:** user, discovery session 8 (Q-27: "just running Claude Code or Codex outside of this app and it's mainly done there and this tool is simply a tool for it. Integrating it inside could be nice too, especially since Codex can use 'api' stuff with your own usage, but having it separate would be nice")

## Context
The oversight loop (ADR-0012, Q-27) needs a coding agent. Insight could host one, or work alongside the agents people already use.

## Decision
**Primary: separate.** The coding happens in an external agent (Claude Code, Codex, …). Insight is (a) a **tool the agent uses**, exposing the graph, queries, expectations, and the system diff over MCP (ADR-0006/0013 tool API), and (b) **the human's window** onto what the agent did (system diff, show-then-tell, step-throughs).
**Optional later: embedded.** An in-app agent panel may come later. It's attractive where an agent can run on the user's own API usage or subscription (Q-06), but it's never required.

## Why
People already have the agent they like, and agents change fast. Competing with them is a losing race. Being the best *instrument* for any agent compounds instead: every agent improvement makes Insight more useful, not obsolete. Keeping them separate also keeps a clean trust boundary. The agent writes code; Insight checks and shows it deterministically, and doesn't grade its own work.

## Consequences
- The MCP surface is a first-class product, not an afterthought. "What paths will my change affect?" and "did I break expectation X?" become agent-callable.
- The system diff scopes to an agent's prompt or session by watching the repo (commits, working-tree changes) and, where available, the agent's transcript. Reading the transcript gives the *why* (Q-25), without hosting the agent.
- Directing the agent from the graph (Q-27) becomes "copy this selection as context" or an MCP resource the agent pulls, rather than an in-app chat.
