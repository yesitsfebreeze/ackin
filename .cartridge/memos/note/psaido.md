---
kind: note
description: The PSAIDO scaffold dialect used in ```psaido``` blocks — imports, schemas, functions, statements,
  control flow, and @-references. Read, not compiled — a model translates it into the project's real language.
  Use to author a scaffold memo or read one a note carries.
uses:
- usage: '[[read-usage]]'
  when:
  - write a psaido scaffold
  - what does !sc or !fn mean
  - scaffold dialect syntax
  - how to sketch logic in a memo
  - psaido imports and references
  - translate a scaffold
  tags:
  - psaido
  - scaffold
  - dialect
  - schema
  - agent
---

# psaido

The scaffold dialect that lives in ```psaido``` blocks. Unlike a ```just```
recipe (which a `just` runner executes), a scaffold is **read, not compiled** — a model
translates it into the project's real language. It is context, like prose:
describe *what* should happen and how the pieces connect, never *how* one
language implements it. Rough is fine; add a keyword only when its absence causes
real ambiguity.

## The three sigils

| Sigil | Form | Declares |
|-------|------|----------|
| `!im` | `!im @path [as alias]` | import another file's provided names |
| `!sc` | `!sc Name` + `- field: type` | a data schema (shape) |
| `!fn` | `!fn Name > return` + `- param: type`, `< expr` | a function |

```psaido
!im @db/users as udb

!sc User
- id: number
- name: string
- email: string

!fn lookup > udb.User
- id: number
< udb.findById(id)
```

Primitive types: `string`, `number`, `boolean`, `null`, `any`. Schemas nest and
hold arrays — `- items: [Product]`. A function body is indented; `< expr`
returns. The `provides:` frontmatter names the schemas/functions other files
reach via `#Name`.

## Statements and control flow

`=` binds a variable, `!` binds a constant.

```psaido
x ! 5                                 // constant
name = "hello"                        // variable
user = lookupUser(id)                 // call
first = items[0]                      // index
user = User{ id: 1, name: "Alice" }   // construct

if x > 10 then
  y = 1
else
  y = 0

for item in items do
  total = total + item.price

while x < 100 do
  x = x + 1
```

## References inside a scaffold

`@` links work inline and *are* the import — put one where a type or call
belongs (`@auth/user#User`, `@db/users#findByEmail(...)`), or alias a path once
with `!im` and use the short name below. A scaffold is plain reference text in a memo; a model
reads it and writes the real code.
Unlike prose, a scaffold link is translated into the target language's normal
import or call — never left as a literal `@`.
