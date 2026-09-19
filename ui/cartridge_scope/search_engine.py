"""Evaluate typed ASP/disk pipelines without blocking the terminal thread."""
import asyncio
import time
from .disk_search import DiskSearch
from .filter_chain import fuzzy, grep_rows, literal, parse
from .model import terms


class SearchEngine:
    def __init__(self, project, client, context):
        self.disk = DiskSearch(project)
        self.client, self.context = client, context
        self.cache = {}
        self.expanded = {}
        self.options = {"case": False, "word": False, "regex": False}
        self.errors = []

    def project(self, nodes):
        result = []
        for node in nodes:
            kind = node["id"].split(":", 1)[0]
            owner = self.context.types.get("schemes", {}).get(kind, {}).get("owner") or ""
            summary = node.get("attributes", {}).get(owner + ".scope", {})
            result.append(dict(node, type=kind, summary=summary.get("summary", "") if isinstance(summary, dict) else "", rank=node.get("rank", 0), uses=node.get("uses", 0)))
        return result

    async def asp(self, query):
        kinds, text = terms(query, self.context.types.get("schemes", {}))
        local = self.context.items(" ".join("type:" + kind for kind in kinds))
        local = await asyncio.to_thread(fuzzy, local, text, self.options["case"])
        if not text:
            return local
        try:
            answer = await self.client.asp("search", query=literal(text), limit=256)
            nodes = [dict(hit["node"], rank=hit.get("score", 0)) for hit in answer.get("hits", [])]
            remote = self.project(nodes)
            remote = [row for row in remote if not kinds or row["type"] in kinds]
            known = {row["id"] for row in remote}
            self.errors.extend(str(source.get("error") or source.get("state")) for source in answer.get("sources", []) if source.get("state") != "available")
            return remote + [row for row in local if row["id"] not in known]
        except (OSError, ValueError, asyncio.TimeoutError) as error:
            self.errors.append("ASP: " + (str(error) or "timeout"))
            return local

    async def expand(self, rows):
        semaphore = asyncio.Semaphore(4)
        async def one(row):
            key = (row["id"], str(row.get("revision", "")), str(row.get("contributors", "")))
            cached = self.expanded.get(key)
            if cached and time.monotonic() - cached[0] < 10:
                return cached[1]
            async with semaphore:
                answer = await self.client.asp("expand", entity=row["id"], depth=1, limit=128)
                self.expanded[key] = (time.monotonic(), answer)
                if len(self.expanded) > 512: self.expanded.pop(next(iter(self.expanded)))
                return answer
        answers = await asyncio.gather(*(one(row) for row in rows[:256]), return_exceptions=True)
        result = {}
        for answer in answers:
            if isinstance(answer, Exception): self.errors.append(str(answer)); continue
            result.update({row["id"]: row for row in self.project(answer.get("nodes", []))})
        if len(rows) > 256: self.errors.append("Expansion limited to 256 input entities")
        return list(result.values())

    async def evaluate(self, expression, scopes=None):
        stages = parse(expression)
        rows = None
        self.errors = []
        self.disk.partial = False
        for index, stage in enumerate(stages):
            if scopes and index in scopes:
                rows = scopes[index]
            key = (stage, self.context.revision, tuple(sorted(self.options.items())),
                   None if rows is None else tuple((row["id"], str(row.get("revision", "")), row.get("content", "")) for row in rows))
            cached = self.cache.get(key)
            if cached and time.monotonic() - cached[0] < 2 and stage.picker not in ("Grep", "Expand"):
                rows = cached[1]; self.errors.extend(cached[2]); continue
            if stage.picker == "ASP": rows = await self.asp(stage.query)
            elif stage.picker == "All":
                asp, disk = await asyncio.gather(self.asp(stage.query), self.disk.files(stage.query, case=self.options["case"]))
                merged = {row["id"]: row for row in disk}
                for row in asp:
                    previous = merged.get(row["id"], {})
                    merged[row["id"]] = dict(previous, **row)
                    merged[row["id"]]["attributes"] = {**previous.get("attributes", {}), **row.get("attributes", {})}
                rows = list(merged.values())
            elif stage.picker in ("Files", "Dirs"):
                rows = await self.disk.files(stage.query, rows, stage.picker == "Dirs", self.options["case"])
            elif stage.picker == "Fuzzy": rows = await asyncio.to_thread(fuzzy, rows, stage.query, self.options["case"])
            elif stage.picker == "Type":
                kinds = set(stage.query.casefold().replace(",", " ").split())
                rows = [row for row in rows if row["type"].casefold() in kinds]
            elif stage.picker == "Expand":
                rows = await self.expand(rows)
                rows = await asyncio.to_thread(fuzzy, rows, stage.query, self.options["case"])
            elif stage.picker == "Grep":
                if rows is None or any(row["type"] in ("file", "directory") for row in rows):
                    disk = await self.disk.grep(stage.query, rows, **self.options)
                    entities = [row for row in rows or [] if row["type"] not in ("file", "directory")]
                    rows = disk + await asyncio.to_thread(grep_rows, entities, stage.query, **self.options)
                else:
                    if rows and rows[0]["type"] != "disk-line":
                        expanded = await self.expand(rows)
                        ids = {row["id"] for row in rows}
                        rows = [row for row in expanded if row["id"] in ids]
                    rows = await asyncio.to_thread(grep_rows, rows, stage.query, **self.options)
            self.cache[key] = (time.monotonic(), rows, list(self.errors))
            if len(self.cache) > 32: self.cache.pop(next(iter(self.cache)))
        return rows or []
