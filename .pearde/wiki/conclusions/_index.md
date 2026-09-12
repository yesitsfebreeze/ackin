---
type: index
scope: conclusions
---

# Conclusions Index

Synthesized answers built from sources — the KB the agent queries.

## Hub conclusions (most linked)

```dataview
TABLE WITHOUT ID file.link AS "Conclusion", length(file.inlinks) AS "Refs", length(file.outlinks) AS "Cites", date
FROM "pearde/wiki/conclusions"
SORT length(file.inlinks) DESC
LIMIT 25
```

## Recently updated

```dataview
TABLE WITHOUT ID file.link AS "Conclusion", date, file.mtime AS "Modified"
FROM "pearde/wiki/conclusions"
SORT file.mtime DESC
LIMIT 25
```

## Thin conclusions (few sources)

Under two sources: research before trusting.

```dataviewjs
const pages = dv.pages('"pearde/wiki/conclusions"')
  .map(p => ({ p, n: (p.sources ?? []).length }))
  .where(x => x.n < 2)
  .sort(x => x.n, "asc");
dv.table(["Conclusion", "Sources", "Date"],
  pages.map(x => [x.p.file.link, x.n, x.p.date]));
```

## Orphan conclusions

Linked from nowhere — check discoverability.

```dataview
LIST
FROM "pearde/wiki/conclusions"
WHERE length(file.inlinks) = 0
```

## By tag

```dataview
TABLE WITHOUT ID rows.file.link AS "Conclusions", length(rows) AS "Count"
FROM "pearde/wiki/conclusions"
FLATTEN tags AS tag
WHERE tag
GROUP BY tag
SORT length(rows) DESC
```
