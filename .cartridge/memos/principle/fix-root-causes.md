---
kind: principle
description: Apply when debugging. Trace each symptom to its root cause and fix it there; reproduce first, ask why until you reach it, and refuse guards that only silence a crash.
group: verification
---

# Fix root causes

When debugging, do not fix symptoms. Trace every problem to its root cause and
fix it there.

**Why:** Symptom fixes accumulate. Each workaround makes the system harder to
reason about and the real bug remains. Root-cause fixes are slower up front
and reduce total debugging time.

**The pattern:**

- Reproduce first.
- Ask "why" until you hit the root cause.
- Do not add guards. A nil check that silences a crash is a symptom fix.
- If a workaround needs a paragraph-long comment to justify it, the code is
  wrong. Fix the code, not the comment.
- Check for the pattern, not just the instance. Grep for the same shape and
  fix every instance — one guard in the shared function is a smaller diff than
  a guard in every caller.
- When stuck, instrument. Do not guess. Add logging, read the actual error.

**Restart bugs: suspect state before code.** When something fails after a
restart, suspect stale persistent state first — config files, caches, lock
files, serialized state. If clearing a state file restores behaviour, the fix
is state validation, and the design fix is
[[make-operations-idempotent]].

When several fixes have failed the same gate, stop fixing and switch to
[[attack-the-premise]].
