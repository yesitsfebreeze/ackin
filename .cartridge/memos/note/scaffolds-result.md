---
kind: note
description: The Result type — success or a typed error, never both. Pull when implementing fallible operations,
  error mapping, or retry boundaries.
uses:
- usage: '[[read-usage]]'
  when:
  - The Result type — success or a typed error, never both. Pull when implementing fallible operations,
    error mapping, or retry boundaries.
  tags:
  - errors
  - result
  - retry
  - convention
---

# Result

`Ok`/`Err` construct the cases; `map` transforms the success value; `recover`
turns an error into a fallback.

```psaido
!sc Result
- ok: boolean
- value: any
- error: string

!fn Ok > Result
- value: any
< Result{ ok: true, value: value, error: "" }

!fn Err > Result
- message: string
< Result{ ok: false, value: null, error: message }

!fn map > Result
- input: Result
- transform: any
  if input.ok == false then
    < input
  newValue = transform(input.value)
< Ok(newValue)

!fn recover > Result
- input: Result
- fallback: any
  if input.ok == true then
    < input
< Ok(fallback)
```

## Notes

- `transform` / `fallback` are functions passed as values — translate to the host's first-class function / closure type.
- Never read `value` or `error` before checking `ok`.
- If the host has a built-in `Result`/`Either`/`Outcome`, prefer it — the convention matters more than the shape.
