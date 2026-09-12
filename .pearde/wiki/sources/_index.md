---
type: index
scope: sources
---

# Sources Index

Raw findings from research runs — one file, one topic.

## All sources by date

```dataview
TABLE WITHOUT ID file.link AS "Source", tags, date
FROM "pearde/wiki/sources"
WHERE type = "source"
SORT date DESC
```

## Untagged sources

```dataview
LIST
FROM "pearde/wiki/sources"
WHERE type = "source" AND (!tags OR length(tags) = 0)
```

## Sources with no inbound conclusion link

Sources no conclusion references — synthesize or prune.

```dataview
LIST
FROM "pearde/wiki/sources"
WHERE length(file.inlinks) = 0
```

## By tag

```dataview
TABLE WITHOUT ID rows.file.link AS "Sources", length(rows) AS "Count"
FROM "pearde/wiki/sources"
FLATTEN tags AS tag
WHERE tag
GROUP BY tag
SORT length(rows) DESC
```
