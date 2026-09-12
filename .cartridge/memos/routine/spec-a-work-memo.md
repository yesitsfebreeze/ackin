---
kind: routine
description: Turn one work memo's outcome into an approach and acceptance checks by attempting it
uses:
  - usage: "[[run-usage]]"
    when: [a work memo has an Outcome but no Approach, planning how a bounded outcome will be delivered]
    tags: [planning, work]
---

## Inputs

One work memo in `.cartridge/memos/work/` or a cartridge's record, given by name. Its
`## Outcome` is the contract. Nothing else is assigned; do not touch another
memo's outcome.

## Do

Read the memo's Outcome, its Check, and the memos it links. Read the `work` type
declaration for the sections a memo may carry.

Do not write an approach from reading alone. Open `just lane <memo-name>` from
the workspace root, `cd` into it, and attempt the outcome. Build until it works
or until it reaches something the Outcome does not settle. Commit the attempt on
the lane branch: it is the first pass, and whoever implements the memo continues
it rather than starting over.

Then write one of three results into the memo through the memo tool:

- **Specced** — the attempt went through, or far enough that only defined work
  remains. Write `## Approach`: the files it touches, the steps in order, the
  command or routine that runs each. Say what the attempt already did and what
  is left. Sharpen `## Check` so every box is a behaviour a command can fail —
  never effort, never "commit". A box that names work instead of an observable
  behaviour is not a check: rephrase it or split the memo. Keep the estimate to two days of work or less;
  if it cannot be, split instead.
- **Split** — the attempt hit a piece large enough to be its own contract, or
  the Outcome holds more than one. Write one child work memo per unit, each with
  an Outcome and a Check and no Approach, and name them on this memo's
  `subwork:`. This memo's Check becomes that every child is done. Siblings must
  own disjoint files so they can run at once; use `needs:` only where one
  consumes what another makes. A chain of steps is one memo, not a split.
- **Question** — the attempt hit a fork it cannot pick and cannot build around.
  Only a fork actually reached, never a hedge and never a fact: a fact is found
  by building. Write a `kind: question` memo stating the fork in two sentences
  with three prepared answers, one recommended, and add it to this memo's
  `needs:`.

## Check

The memo carries exactly one of those three results, saved through a validated
memo write. On Specced, `## Approach` names real files and runnable steps and
every Check box can fail. On Split, the children exist, are named on `subwork:`,
and own disjoint files. On Question, the question memo exists and is on `needs:`.
The lane holds the attempt as commits and is not landed.

## Failure

If the attempt cannot start at all — the tree will not build, a required service
is missing — that is a Question, not a silent stop: write what blocked it. Never
invent an approach for work that was not attempted. Never edit another memo's
Outcome, never tick a Check box, never land the lane.
