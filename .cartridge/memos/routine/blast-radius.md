---
kind: routine
description: Find what a change breaks somewhere else before it ships — find the one fact its safety depends on, prove that fact by running real code, and report the risks you confirmed against the ones you cleared.
uses:
  - usage: "[[run-usage]]"
    when: [asking what a change could break, reviewing a small diff you do not trust, sizing the reach of an edit beyond its own files]
---

Companion to [[explain-how]] and [[explain-why]]. That pair tells you what the
code does and why it is shaped that way. This tells you what it breaks
somewhere else.

Listing the callers is not the job — a grep finds those in a second. The job
is the breakage grep will not show you.

**Do not trust your own writeup.** A blast-radius writeup that sounds right is
worthless; it reads as convincing whether or not it is true. So do not hand
back the writeup. Find the one or two facts the whole thing depends on and
prove them by running code.

**How sure are you.** For each fact the change's safety depends on, get as far
down this list as is cheap, and say where it stopped:

1. You said so. Worthless on its own.
2. You pointed at the line. A real `file:line`, or the library's own source.
3. You showed the bad case cannot happen. You walked the failure step by step
   and it does not reach.
4. You ran it. A script or test that calls the real code and fails loudly if
   you are wrong.
5. You reproduced it in the running program.

Any safety fact you cannot get to step 4, say so. Do not write it up as
settled. Step 4 is usually one small script that calls the exact function you
are worried about.

1. Read the change

The diff, the symbols it adds, changes and deletes, and what it now does
differently — including the part the diff does not spell out. Use
[[explain-why]] step 2 to pull the commits and PR.

2. Find the one fact it is safe because of

Most changes that look risky are safe because of a single fact, such as "this
call only drops already-dead cache entries and does nothing else". Find that
fact. If it holds, most risky cases clear at once. Spend your time here, not
on a long list of maybes.

3. Look where grep stops

Read the source of the library you call, and check its pinned version and any
local patch. Work out when things run: task ordering, teardown, shutdown
paths. Follow what a symbol search misses — the JSON an API returns, a
database column, a wire format, another language reading the same bytes, a
feature flag, code three hops downstream.

4. Be honest about each risk

Give it a real chance of happening and a real cost if it does. Keep the risks
you confirmed. List the ones you checked and cleared separately. Cite a real
`file:line`. A search that finds nothing is still an answer. Never invent a
caller or an API.

5. Prove the one fact

Write the script or test that runs the real code, run it, and paste what
happened. That is [[build-the-lever]] and [[prove-it-works]] applied to a
review. If you cannot prove it cheaply, mark it unproven. Do not overstate.

6. For a big or wide change, run it as an [[arena]]

Ask several models the same question and merge the answers. Different models
catch different real bugs.

Check, and what to hand back:

- **What it does.** What changed, including the part that is not obvious.
- **The one fact it is safe because of.** State it, say which step you got it
  to, show the proof. If you could not prove it, write unproven.
- **Risks.** Only the real ones. Each names how it breaks, the `file:line`,
  how likely and how bad, and how to check. Paste the proof for the ones that
  matter.
- **Cleared.** What you checked and why it is fine.
- **Before you merge.** The cheapest test or repro that catches the real bug,
  including the script you wrote.

Write it through [[unslop]], cite real code, and strip anything private before
it goes anywhere public.

Failure: the one safety fact will not prove. That is the finding. Report it
unproven and name the cheapest experiment that would settle it, rather than
padding the writeup with cleared maybes.
