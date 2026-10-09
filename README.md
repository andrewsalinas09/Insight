# Insight

See what your code actually does. Pick a trigger — click a button, run a command, name a function, or just ask "what happens when a user logs in?" — and Insight finds every branch of execution it can reach, draws it as a graph you can zoom into, and labels it in plain language. Then you learn it by talking to it.

When AI writes the code, nobody inspects it line by line any more. You have to inspect it system by system, and there's no IDE or debugger built for that. Insight is one: about 60% a tool for learning code you didn't write, and 40% a tool for overseeing what your AI agents are actually building.

What it's for:
- **Understand vibe-coded software** — the code exists, it works (mostly), and nobody holds the whole thing in their head. Insight gives you the map.
- **Every branch, not just the happy path** — `valid` and `!valid` are two edges. Error handling, early returns, and fallbacks are first-class, labeled, and visible.
- **Hierarchical, not a hairball** — each function is a bubble with its inputs and outputs; open it to see the structure nested inside.
- **Your code first** — tracing stays inside the code you added; libraries are leaves unless you choose to expand them.
- **Review AI changes as a system diff**: new effects, lost error handling, and changed paths, instead of a wall of line changes. State expectations in plain language and have them checked on every change.
- **LLM-powered throughout** — labeling, summarizing, mapping prompts to entry points, and the conversation you learn through.

## Status

Discovery phase, started 2026-10-09. No code yet. The idea is being mapped out into docs before anything is built.

## Where everything is

Start with **[CLAUDE.md](CLAUDE.md)**, which has the map and working rules. It's written for AI agent sessions but works for humans too.

| | |
|---|---|
| [docs/VISION.md](docs/VISION.md) | Why this exists |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Current design |
| [docs/decisions/](docs/decisions/) | Architecture decision records: the settled "why"s (normative) |
| [docs/TESTBED.md](docs/TESTBED.md) | Edge-case acceptance tests (normative) |
| [docs/OPEN-QUESTIONS.md](docs/OPEN-QUESTIONS.md) | Open design questions |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phases and current focus |
| [docs/GLOSSARY.md](docs/GLOSSARY.md) | Domain vocabulary |
| [docs/WORKLOG.md](docs/WORKLOG.md) | Session journal |
| [docs/archive/](docs/archive/) | Original pitch, verbatim |

## License

[Apache 2.0](LICENSE)
