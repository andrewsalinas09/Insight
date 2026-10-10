# Insight

See what your code actually does. Ask your coding agent in the terminal ("what happens when a user logs in?", "how does dragging a node work?"), and it answers while driving the Insight window: highlighting, grouping and stepping through the real code as it explains.

When AI writes the code, nobody inspects it line by line any more. You have to inspect it system by system, and there's no IDE or debugger built for that. Insight is one: about 60% a tool for learning code you didn't write, and 40% a tool for overseeing what your AI agents are actually building.

What it's for:
- **Understand vibe-coded software:** the code exists, it works (mostly), and nobody holds the whole thing in their head. Insight gives you the map.
- **Every branch, not just the happy path:** every function and every branch arm is in the graph, so error handling, early returns, and fallbacks can always be shown.
- **Hierarchical, not a hairball:** files, types, functions and blocks nest. Your agent groups and collapses them into the view your question needs.
- **Your code first:** libraries are leaves, one per symbol your code uses.
- **Review AI changes at the system level** *(planned)*: compare two snapshots, and save expectations as queries that are re-checked on each new snapshot.
- **Model-driven:** Insight calls no model itself. Your agent (Claude Code, Codex) does all the understanding and uses Insight as its visual instrument.

## Status

Design complete; building milestone M0 (see [docs/ROADMAP.md](docs/ROADMAP.md)).

## Where everything is

Start with **[CLAUDE.md](CLAUDE.md)**, which has the map and working rules. It's written for AI agent sessions but works for humans too.

| | |
|---|---|
| [docs/VISION.md](docs/VISION.md) | Why this exists |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Current design |
| [docs/decisions/](docs/decisions/) | Architecture decision records: the settled "why"s (normative) |
| [docs/BUILD-QUESTIONS.md](docs/BUILD-QUESTIONS.md) | Backend build decisions, asked one at a time |
| [docs/TESTBED.md](docs/TESTBED.md) | Edge-case acceptance tests (normative) |
| [docs/OPEN-QUESTIONS.md](docs/OPEN-QUESTIONS.md) | Open design questions |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phases and current focus |
| [docs/GLOSSARY.md](docs/GLOSSARY.md) | Domain vocabulary |
| [docs/WORKLOG.md](docs/WORKLOG.md) | Session journal |
| [docs/archive/](docs/archive/) | Original pitch, verbatim |

## License

[Apache 2.0](LICENSE)
