# ADR-0055: Aside views: a detour the user returns from to the exact view and step

- **Status:** Accepted
- **Date:** 2026-10-11
- **Source:** user, on step 4 of a walkthrough, asking a side question: "I think we need a way to pause what we have, you can show the race condition, and then we can go back to step 4 nothing changed."

## Context
Each `show` replaced the view on screen. A question in the middle of a walkthrough meant losing the walkthrough, or Claude re-sending it and jumping back to the step by hand.

## Decision
- A view can be **`aside: true`**: a detour on top of the current view. The server attaches the view underneath and the step the user was on (`under: {view, step}`). It tracks the step from `show(step=N)` and from what the window reports.
- The window shows **"◂ Back to *title* · step N"** while an aside is on screen. One click restores that view at that step, exactly as it was, and tells the server it is current again (`restore_view`), so `show(step=N)` counts its steps. Asides can nest.
- Every view change is also one **↶ Back** away (ADR-0053), whether or not it was an aside.

## Why
- Explaining is a conversation. Questions come mid-walkthrough, and the place the user was is worth keeping.
- The link lives on the server, so it holds even when the window wasn't there for the first view: in the first test, the window started fresh and missed the walkthrough arriving.

## Consequences
- `aside` is a view parameter, like `layout` (ADR-0050): it adds no element kind to ADR-0040.
