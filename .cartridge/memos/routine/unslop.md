---
kind: routine
description: Cut the tells out of generated prose — the vocabulary, the punctuation crutches, the metaphor nouns and the sentences that name a feeling instead of a mechanism.
uses:
  - usage: "[[run-usage]]"
    when: [writing or editing prose that is written down — a memo, a commit message, a README, a report, a PR body]
---

Edit text to remove the patterns that mark it as generated. Applies to every
prose surface that is written down and read later. The reply in the terminal
takes the terse register instead, per [[the-register-is-chosen-by-surface]].

1. Scan for the patterns below.
2. Rewrite. Preserve meaning, match the intended tone.
3. Self-audit: what still makes this obviously generated? Fix what is left.

Rule numbers are stable identifiers other memos cite. A removed rule leaves a
gap rather than renumbering.

**Content**

3. **Superficial -ing phrases.** "highlighting", "ensuring", "reflecting",
   "showcasing", "fostering". Delete, or expand with a real source.
5. **Vague attributions.** "Experts believe", "reports suggest", "some argue".
   Name the source or delete it.

**Language**

7. **Generated vocabulary.** Additionally, crucial, delve, enduring, enhance,
   fostering, garner, interplay, intricate, landscape (abstract), pivotal,
   showcase, tapestry, testament, underscore, vibrant. Use plain words.
8. **Fancy ways to say "is".** "serves as", "stands as", "boasts",
   "features". Say "is" or "has".
9. **"Not just X, but Y."** State the point directly.
10. **Rule of three.** Forcing ideas into groups of three. Use the natural
    number.
11. **Synonym cycling.** Four names for one thing in one paragraph. Pick one
    and repeat it.
12. **False ranges.** "from X to Y" where X and Y are not on a scale. List the
    things.

**Style**

13. **Em dash overuse.** Use periods or commas. If a thought needs separation,
    end the sentence.
14. **Colon overuse.** Fine before a list or an example, not as a mid-sentence
    connector. Let the point stand without the comparison framing.
15. **Boldface overuse.** Do not bold every proper noun or acronym.
16. **Inline-header lists.** The tell is a bold label and colon restating the
    line: "**Performance:** Performance improved". Convert to prose. A bold
    lead-in that ends in a period, names the item and is followed by genuinely
    new detail is fine.
17. **Title case headings.** Use sentence case.
18. **Decorative emoji.** Remove from headings and bullets.
19. **Curly quotes.** Use straight quotes.

**Communication artifacts**

20. **Chatbot phrases.** "I hope this helps", "Let me know if", "Of course",
    "Certainly", "Found the smoking gun". Remove.
22. **Sycophancy.** "Great question", "You're absolutely right". Respond
    directly.

**Filler**

23. **Filler phrases.** "In order to" is "To". "Due to the fact that" is
    "Because". "It is important to note that" is deleted.
24. **Excessive hedging.** "could potentially possibly be argued that it
    might" is "may".
25. **Generic conclusions.** "The future looks bright." State a specific plan
    or fact.

**Jargon**

26. **Abstract metaphor nouns.** Substrate, wedge, vector, locus, vantage,
    nexus, primitive (as a noun), harness (as metaphor), surface (as in "API
    surface"), bedrock, scaffolding (as metaphor), modality, paradigm,
    gold-plating, ratchet, evacuate (for moving code), endgame, north star,
    flywheel. These read as technical and have a plainer concrete word.
    "Substrate" is "base". "Wedge in" is "add". "Vector" is "way".
    "Gold-plating" is "more than the job needs". "Evacuate" is "move out".
    "Endgame" is "the last phase".

**Plain speech**

27. **Say what it does, not how it feels.** "the database stays close at
    hand", "SQL you can read" name a feeling. The fix names the mechanism or a
    number: "`.to_sql()` returns the exact string sent to the database", "a
    column rename fails the build". Ask what the sentence tells the reader to
    do or know, then write that. If you cannot restate it as a concrete
    instruction, fact or number, cut it. One more check: a sentence that could
    appear unchanged in another project's docs says nothing about this one.
28. **Shorten or split dense sentences.** If the reader backtracks to parse
    it, break it in two. One idea per sentence.
29. **Active voice.** Catch "is/are/was/were" plus a past participle and name
    the actor. "queries are validated" is "the compiler validates queries".
    Passive is fine only when the actor is unknown or genuinely does not
    matter.
30. **Cut adverbs, or use a stronger verb.** "runs quickly" is "is fast" or
    the number. "significantly improves" is the measured delta. An adverb
    propping up a weak verb means the verb is wrong.
31. **Prefer the plain word.** "utilize" is "use", "leverage" is "use",
    "facilitate" is "help", "numerous" is "many", "in the event that" is "if".
32. **Mannered prose.** Metaphor or flourish where a literal phrase exists:
    aphorisms, rhetorical fragments for effect, personified code, figurative
    verbs, stock framing. "A dial worth turning" is "a parameter worth
    varying". Rule 26 covers the metaphor nouns.
33. **Over-compression.** Dropped articles, verbless fragments, symbol-speak
    and abbreviations that make the reader decode instead of read. Write whole
    sentences with their articles and verbs, and spell out arrows and
    abbreviations. This rule governs prose that is written down and read later.
    It does not govern the reply in the terminal, which takes the terse
    register on purpose — see [[the-register-is-chosen-by-surface]].

Check: reread the output against rules 27 and 33 last. A pass that fixed the
vocabulary but left sentences naming feelings has not run.

Failure: the edit changed the meaning. Meaning wins over every rule here.
Restore the claim and find a plainer way to say the same thing.
