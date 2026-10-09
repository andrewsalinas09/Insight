# Worked example: "Save note", end to end

Validates the Q-21 proposal (code is truth · taking away is free · adding needs a receipt · graph shape follows code shape · drift becomes a broken receipt) against a small, typical vibe-coded Tauri app. The app is invented for this example. **The annotation syntax is illustrative only.** The real format is Q-15.

---

## The code

A notes app: a React editor and a Rust backend.

```tsx
// src/components/NoteEditor.tsx
export function NoteEditor() {
  const [text, setText] = useState("");
  async function handleSave() {
    await saveNote(text);
  }
  return (<>
    <textarea value={text} onChange={e => setText(e.target.value)} />
    <button onClick={handleSave}>Save</button>
  </>);
}

// src/api.ts
export async function saveNote(text: string) {
  return persist(text);
}
async function persist(text: string) {
  try {
    await invoke("save_note", { text });
    toast("Saved");
  } catch (e) {
    toast(`Failed: ${e}`);
  }
}
```

```rust
// src-tauri/src/notes.rs
pub struct AppState {
    pub store: Mutex<Store>,
    pub on_saved: Box<dyn Fn(&Note) + Send + Sync>,   // set in main.rs: Box::new(|n| index::reindex(n))
}

#[tauri::command]
pub fn save_note(state: State<AppState>, text: String) -> Result<(), String> {
    let note = Note::parse(&text)?;
    if note.body.trim().is_empty() {
        return Err("Note is empty".into());
    }
    if note.body.len() > 10_000 {
        return Err("Note too long".into());
    }
    let mut store = state.store.lock().unwrap();
    store.notes.push(note.clone());
    store.dirty = true;
    std::fs::write(&store.path, serde_json::to_string(&store.notes).unwrap())
        .map_err(|e| e.to_string())?;
    (state.on_saved)(&note);
    Ok(())
}
```

---

## Step 1: The deterministic graph (fact layer). No LLM involved.

The TS plugin, the Rust plugin, and the Tauri-bridge plugin (ADR-0007) produce:

```
[Save button] ─onClick─► handleSave ─call─► saveNote ─call─► persist
                                                              │ try
                                                              ├─link (Tauri IPC "save_note")─► save_note ─────────┐
                                                              ├─ ok:  call toast                                   │
                                                              └─ catch: call toast                                 │
                                                                                                                   ▼
save_note (inputs: State<AppState>, text: String → Result<(), String>)
 ├─ call Note::parse ──?──► Err  (return)                                  branch 1
 ├─ branch body empty      ──► return Err("Note is empty")                 branch 2
 ├─ branch len > 10_000    ──► return Err("Note too long")                 branch 3
 ├─ lock state.store                                                       data: lock
 ├─ write store.notes, write store.dirty                                   data: field writes
 ├─ EFFECT fs::write(store.path) ──?──► Err (return)                       effect + branch 4
 ├─ call (state.on_saved)  ──► ??? unresolved: dyn Fn                      possible / unresolved
 └─ return Ok
```

Everything here is **proven**, except `on_saved`. Static analysis sees a `dyn Fn` field and can't prove what it holds, so it shows an explicit *unresolved* stub rather than a guess (ADR-0002).

## Step 2: The LLM annotates (inline, ADR-0009)

The LLM is asked to document `save_note`. Through the tool API it writes this doc comment:

```rust
/// @insight
/// Saves the editor's text as a note. Rejects unparseable [c1], empty [c2], or
/// oversized [c3] notes, then adds it to the shared store [c4] and writes the
/// whole list to disk [c5]. Finally notifies the search index [c6].
///
/// c1: returns Err when parse fails  @ `Note::parse(&text)?`
/// c2: returns Err when body empty   @ `if note.body.trim().is_empty()`
/// c3: returns Err when body > 10k   @ `if note.body.len() > 10_000`
/// c4: writes store.notes            @ `store.notes.push(note.clone())`
/// c5: effect fs.write               @ `std::fs::write(&store.path`
/// c6: calls index::reindex          @ `(state.on_saved)(&note)`
/// c7: calls log::info               @ `log::info!("saved")`
#[tauri::command]
pub fn save_note(...)
```

**The receipt (`@ …`) is a quote of the code, not a line number.** The compiler searches for that fragment inside *this function*. Line numbers would break whenever anything above the function moved, which would be false drift (TB-051).

The prose cites claims (`[c1]`…). It's shown as interpretation, but it inherits each cited claim's status.

## Step 3: The compiler checks every receipt

| Claim | Check | Result |
|---|---|---|
| c1 | fragment found; it's a `?` on a call to `Note::parse` → an Err-return branch | ✅ **proven** |
| c2 | fragment found; it's an `if` whose arm returns `Err` | ✅ **proven** |
| c3 | same | ✅ **proven** |
| c4 | fragment found; it's a mutating call on field `store.notes` | ✅ **proven** |
| c5 | fragment found; it's a call to `std::fs::write`, which is in the effect catalogue as `fs.write` | ✅ **proven** |
| c6 | fragment found, and it is a call through `on_saved`. `index::reindex` exists. Whether the field holds *that* function needs a global proof (see the note below) | 〜 **inferred** with a basic plugin, drawn dashed and labelled "AI-inferred"; ✅ **proven** with a strong Rust plugin |
| c7 | fragment `log::info!("saved")` **not found** in `save_note` | ❌ **rejected**: never shown; the LLM is told why |

The hallucinated logging call (c7) is removed before you ever see it. The plausible callback guess (c6) is kept, but visibly marked as a guess.

### Why c6 is different: local vs. global proof
c1–c5 are **local**: the quoted code is in this function and means what the claim says. c6 needs three facts:
1. the call goes through field `on_saved` (local ✅);
2. the closure at the construction site, `on_saved: Box::new(|n| index::reindex(n))` in `main.rs`, calls `reindex` (local ✅);
3. **that construction is the *only* value the field can ever hold** (global: no other constructor, no reassignment).

Fact 3 is an exhaustiveness claim, and whether it can be proven depends on the **language plugin**, not the claim. In this Rust code it *can* be proven: `AppState` is built once, and after `manage()` commands only get shared read-only access through `State<>`, so the field can't be reassigned. A strong Rust plugin therefore upgrades c6 to **proven**. The same pattern in Python or JS (`self.on_saved = …`, reassignable anywhere) can't be proven, so it stays **inferred**. If there were two construction sites (prod → `reindex`, tests → no-op), the honest result is two *possible* edges (TB-002, TB-053).

So c6's receipt should be a **chain** with both quotes:
`c6: calls index::reindex @ (state.on_saved)(&note) → @ on_saved: Box::new(|n| index::reindex(n))`

### Step 3b: But that annotation was too long (user review)
Claims c1–c5 restate facts the graph **already proves**, which is pure duplication and a source of drift. Under Q-21 rule 6 ("never write down what the code already says"), the compiler lints them as redundant, and the committed annotation shrinks to what only the author knows plus the one gap:

```rust
/// @insight Saves the editor's text and persists the whole note list.
/// why `note.body.len() > 10_000`: keeps notes.json small enough to rewrite on every save
/// c6: calls index::reindex @ (state.on_saved)(&note) → @ on_saved: Box::new(|n| index::reindex(n))
```

Labels like "error handling" on the branches and explanations of `?` are generated live at view time, personalized (ADR-0004), and never committed. `saveNote`, `persist`, and `handleSave` get no annotation at all. Steps 1–3 above show the *checking mechanics*. This is what actually lands in the repo.

## Step 4: Taking away (free)

The LLM sees that `handleSave → saveNote → persist` is two layers of pure forwarding. It **collapses** them:

```
[Save button] ─► ⟦handleSave … persist⟧ (3 functions, collapsed) ─link─► save_note
```

This is allowed with no check, because nothing false is shown. One click expands it back to the three real functions (TB-049).

## Step 5: The LLM wants a "Validation" node

The LLM feels branches 2 and 3 are one conceptual step called "validation". It may **not** invent a node. Its options:

- **Group** the two existing branch blocks under a dashed outline labelled "validation". This is the organization layer: it groups real nodes, and the label is prose. ✅
- **Suggest a refactor**: "Extract into `fn validate(note: &Note) -> Result<(), String>`?" If you accept, the code changes, and `validate` appears in the graph as a real function, deterministically. ✅

A free-floating "Validation" node with no code behind it is ❌. There's no tool that creates one (TB-050).

## Step 6: A dynamic run (the overlay, ADR-0003)

You click Save with the text "hello". The recorded run lights up a path:

```
Save → … → save_note → parse ✓ → not empty ✓ → not too long ✓ → lock → write → fs.write ✓ → on_saved → index::reindex → Ok → toast("Saved")
```

- c6 is upgraded from **inferred** to **observed**: the callback really did reach `index::reindex`.
- The three error branches stay drawn but unlit. The map still shows the `!valid` paths you didn't take, which is the point of the static map.

## Step 7: Drift (a week later)

Someone (you or an AI) changes the code:
1. The 10k length check moves into `Note::parse`.
2. `std::fs::write(...)` is replaced with `write_atomic(&store.path, ...)`, a new helper that writes a temp file and renames it.

The compiler re-checks every receipt pointing into the changed function:

| Claim | Result | What happens |
|---|---|---|
| c3 `if note.body.len() > 10_000` | ❌ **broken**: fragment gone | The claim is hidden. The prose sentence citing [c3] is flagged *stale*. The LLM is asked to re-anchor, and it finds the check now lives in `Note::parse`, so it writes a new claim **there**. |
| c5 `std::fs::write(&store.path` | ❌ **broken** | Re-anchored as `c5: effect fs.write via write_atomic @ write_atomic(&store.path`. The compiler checks that `write_atomic` reaches an `fs.write` effect transitively → ✅ |
| c1, c2, c4, c6 | ✅ unchanged | Nothing happens. The quote anchors survived edits around them. |

Nothing went stale silently. The diff for this commit shows code changes **and** annotation changes side by side (ADR-0005), and CI can fail if any receipt is broken.

## Step 8: Two learners, same facts (ADR-0004)

The claims and graph are identical for everyone. Only the prose for a selected node changes.

**Your notes:** "React yes, Rust no. I know threading, but only Rust's."
→ On `state.store.lock()`: *"Same idea as Rust's `Mutex<T>` you know — the lock guards the store itself. Note `.unwrap()`: if another thread panicked while holding it, this panics too."*
→ On `?`: *"`?` means: if this is an `Err`, return it immediately. It's three hidden early-return branches here — Insight draws each one."*

**A colleague's notes:** "Senior Rust dev, never touched React."
→ Skips `?` and `Mutex` entirely, and explains `useState` and why `onClick={handleSave}` doesn't call it immediately.

---

## What this example taught the design

1. **Receipts should be code quotes, not line numbers.** They're readable by humans, they survive unrelated edits, and the compiler matches them structurally within the annotated function (proposed for Q-15/Q-21; TB-051).
2. **Prose cites claims, so prose inherits staleness.** Unverifiable text still can't drift silently.
3. **Grouping ≠ inventing.** A dashed group around real nodes is organization. A node with nothing behind it is fiction.
4. **Claims need a direct vs. transitive form** ("effect fs.write *via* write_atomic"), checked by reachability (TB-052).
5. **Three tiers of truth appear naturally:** proven (static), inferred (LLM, partly checked), observed (runtime). Rejected claims never appear.
6. **The tier belongs to the checker, not the claim.** The same claim can be proven by a strong plugin and only inferred by a weak one. The hard part is usually an exhaustiveness fact ("this is the *only* value"), not a local one.
7. **Receipts can be chains**: several quotes across files, each checked, with the plugin deciding whether the chain closes.
8. **Never write down what the code already says.** Committed annotations are intent/why plus gap-filling claims only. Structure is computed, and explanations are generated live. Most functions have no annotation.
