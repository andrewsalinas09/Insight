# Worked example: the oversight loop. Prompt → what it did → spot the gap → fix it by design

Validates ADR-0012 (oversight) end to end. Source: the user's own description (session 6):

> "You give it a prompt and then you can see what it did. No one looks at 500 lines. They look at 'ok this goes in here and comes out here, the error bubbles up and goes here, but oh wait! you forgot to feed the error to our logger'. And then you go there and see that this could happen a lot, so you then tell the agent to refactor and make it automatically go to both places by design."

Same invented notes app as `save-note-worked-example.md`. The views are illustrative ASCII.

---

## 1. The prompt

> **You → agent:** "Add export: a button that writes all notes to a Markdown file the user picks."

The agent works for a few minutes. The line diff is +480 / −35 across 7 files. Nobody reads it.

## 2. What it did, as a system diff scoped to that prompt (Q-25)

```
 SYSTEM DIFF — prompt "Add export…"                         [line diff: 515 lines, 7 files — hidden]
 NEW TRIGGER   [Export button]
 NEW PATH      Export ─► exportNotes ─link(IPC "export_notes")─► export_notes
                 └─► pick_path (EFFECT dialog) ─► render_markdown ─► EFFECT fs.write
 ERRORS        export_notes Err ──► exportNotes catch ──► UI toast       ✓ reaches user
                                                     ╳ logger            ← not connected
 UNCHANGED     save path, store, worker pool (no behavioral change)
```

You read it the way you described: *it goes in here, comes out here, the error bubbles up and goes here.* It takes seconds, not 500 lines.

## 3. "Oh wait, you forgot the logger"

You noticed an **absence**. Insight can make absences visible without guessing, because the fact is deterministic: compare this path to its siblings.

```
 PATTERN CHECK (deterministic: counted paths, not an opinion)
   backend errors reaching UI:      5 / 5 commands
   backend errors reaching logger:  4 / 5 commands   ← export_notes is the exception
```

The *suggestion* ("the export path is probably missing logging") is the LLM's, and it's labeled as one. The *count* is a fact (ADR-0002).

## 4. "This could happen a lot"

You click into `exportNotes`'s catch and see why. Every command's frontend wrapper decides on its own to call the logger. Logging only happens if whoever wrote the wrapper remembered. You ask:

> **You → Insight:** "Show me every error path that could skip the logger."

Insight runs that as a query over the fact graph and highlights all error edges whose path to the UI doesn't pass through `log`:

```
 QUERY  error edges ─► UI  that do NOT pass through  logger
   export_notes ─► exportNotes ─► toast           (new, from this prompt)
   (+ 0 others today — but 5 handwritten catch blocks each repeat the toast+log pair)
```

There's only one gap today, but the *structure* makes the next gap likely. That's a design problem, not a bug.

## 5. Tell the agent to fix it by design

You direct the agent **from the graph** (Q-27). You select the five catch blocks, and Insight attaches them, the query result, and the paths as context:

> **You → agent:** "Refactor so every backend error goes to both the UI and the logger by design. One error sink, no per-command catch blocks."

## 6. Verify the fix as a system diff, and lock it in

```
 SYSTEM DIFF — prompt "Refactor error sink…"
 NEW NODE      reportError(e)  ─► UI toast
                               ─► logger
 CHANGED       5 command wrappers: catch blocks removed ─► all route through reportError
 ERRORS        backend errors reaching UI:     6 / 6 ✓
               backend errors reaching logger: 6 / 6 ✓
```

Then you turn the query from step 4 into a standing **expectation** (Q-26):

> "Every backend error reaches both the UI and the logger."

It's saved, committed next to the code (ADR-0005), and re-checked after every future prompt. The next agent that adds a command and hand-writes its own `catch` gets a red result on its own system diff before you ever look.

---

## What this example taught the design

1. **The unit of review is a prompt.** The system diff should be scoped to an agent turn or session and linked to the prompt that caused it. The prompt is also the *intent* ("why") for everything in that diff, which answers the earlier question of where the "why" comes from.
2. **Absence is the hardest thing to see, and deterministic sibling comparison makes it visible** ("4 of 5 do X, this one doesn't"). The count is a fact; the "probably a bug" is labeled LLM suggestion.
3. **Ad hoc queries graduate into expectations.** The question you asked once ("what skips the logger?") becomes the check that runs forever. The same query language serves both (Q-26).
4. **Fix the class, not the instance.** The graph shows when a gap is *structural* (every caller must remember), which prompts a by-design fix rather than a patch.
5. **The loop closes on the graph**: prompt → system diff → spot → query → direct the agent with graph context → system diff → expectation.
