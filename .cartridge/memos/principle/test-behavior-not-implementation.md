---
kind: principle
description: Apply when you write, change, or keep a test. Call the code the way its users do and assert the observed result against a literal expected value; if the test would still pass with every dependency returning nothing, rewrite or delete it.
group: verification
---

# Test behaviour, not implementation

A test calls the code the way its users do and asserts the result they observe
against a literal expected value. A test that asserts which calls the code
made, or restates a constant the code contains, does neither.

**The check:** before keeping a test, ask whether it would still pass if every
function it calls returned nothing. If yes, it observes no behaviour and
cannot fail for a defect. Rewrite the assertion or delete the test.

**Why:** A test that cannot fail for a defect costs check time and review
attention and catches nothing. A constant pin also fails when someone edits
the constant it restates, so it prevents that edit.

**Five shapes that still pass when every dependency returns nothing:**

- **Weak or no assertion.** No assertion at all, or only "is defined", "is
  truthy", "does not panic", "is an instance of", "is greater than zero".
- **Mock or absence only.** Only "was called", "was not called", "is none",
  "equals the empty list", "has length zero", "is not the wrong value".
- **Self-referential.** The expected value comes from the code under test:
  `assert_eq!(f(a), f(a))`, or comparing a parsed field against the same
  builder that produced it.
- **Constant pin.** The assertion restates a hand-maintained constant, config
  default, table row or prompt string.
- **Fixture asserts fixture.** The assertion reads data the test built or a
  value computed in setup, and the subject never runs in the body.

**The fix:** call the subject in the test body with one concrete input and
assert the literal output or the observable effect — `slugify("Hello,
World!")` is `"hello-world"`. For an absence, assert the presence on the other
input in the same test. For a constant, test the mechanism that reads it. For
a mock, assert the payload it received or the state after the call. When no
such assertion exists, delete the test.

**Keep** a test of a relation across a table's rows — a key present in two
tables, a parent that exists — and a compile-time type check.

A rule that must hold from now on is a test, not a note. That is
[[encode-lessons-in-structure]].
