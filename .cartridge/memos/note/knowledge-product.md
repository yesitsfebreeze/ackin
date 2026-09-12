---
kind: note
description: The canonical Product shape (id, name, price). Pull when implementing anything that touches
  a product line item.
uses:
- usage: '[[read-usage]]'
  when:
  - The canonical Product shape (id, name, price). Pull when implementing anything that touches a product
    line item.
  tags:
  - catalog
  - pricing
  - schema
---

# Product

The shared product schema; one source of truth, linked via `@knowledge/product#Product`.
See [[knowledge-orders]] for a consumer that nests `Product` inside an order line.

```psaido
!sc Product
- id: number
- name: string
- price: number
```
