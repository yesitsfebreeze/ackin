"""Bounded ASP projections, independent of terminal widgets and provider types."""
import json
import math
import shlex
import copy
from collections import Counter


def terms(query, schemes=()):
    try:
        words = shlex.split(query)
    except ValueError:
        words = query.split()
    kinds = {word[5:].casefold() for word in words if word.startswith("type:")}
    aliases = {kind.casefold(): kind for kind in schemes}
    aliases.update({kind.casefold() + "s": kind for kind in schemes})
    start = 0
    while start < len(words) and words[start].casefold() in ("show", "me", "all", "the", "find"):
        start += 1
    if start < len(words) and words[start].casefold().rstrip(",") in aliases:
        kinds.add(aliases[words[start].casefold().rstrip(",")])
        words = words[start + 1:]
        while words and words[0].casefold() in ("from", "about", "containing", "with"):
            words.pop(0)
    return kinds, " ".join(word for word in words if not word.startswith("type:"))


class Context:
    def __init__(self):
        self.nodes, self.edges, self.records = {}, [], []
        self.types, self.scores, self.errors = {}, {}, {}
        self.generation = None
        self.revision = 0
        self.partial = False
        self._search_text = {}
        self._edge_keys = set()
        self._item_cache = {}
        self._cache_state = None

    def merge(self, graph):
        changed = False
        for node in graph.get("nodes", []):
            identity = node.get("id")
            if not isinstance(identity, str):
                continue
            old = self.nodes.get(identity, {})
            merged = dict(old, **node)
            merged["attributes"] = {**old.get("attributes", {}), **node.get("attributes", {})}
            if merged != old:
                self.nodes[identity] = merged
                self._search_text.pop(identity, None)
                changed = True
        for edge in graph.get("edges", []):
            key = json.dumps(edge, sort_keys=True)
            if key not in self._edge_keys:
                self.edges.append(edge)
                self._edge_keys.add(key)
                changed = True
            for endpoint in (edge.get("from"), edge.get("to")):
                if isinstance(endpoint, str):
                    if endpoint not in self.nodes:
                        self.nodes[endpoint] = {"id": endpoint}
                        changed = True
        if len(self.edges) > 8192:
            self.edges = self.edges[-8192:]
            self._edge_keys = {json.dumps(edge, sort_keys=True) for edge in self.edges}
        while len(self.nodes) > 10000:
            identity = next(iter(self.nodes))
            del self.nodes[identity]
            self._search_text.pop(identity, None)
            self.partial = True
        self.revision += changed
        self.partial |= bool(graph.get("truncated"))
        for source in graph.get("sources", []):
            key = source.get("contributor", "provider")
            if source.get("state") != "available":
                self.errors[key] = source.get("error") or source.get("state", "unavailable")
            else:
                self.errors.pop(key, None)

    def activity(self, payload):
        generation = payload.get("generation")
        records = payload.get("records", [])[-1024:]
        self.revision += generation != self.generation or records != self.records
        self.generation = generation
        self.records = records
        placeholders = []
        for record in self.records:
            for identity in record.get("entities", []):
                if identity not in self.nodes:
                    placeholders.append({"id": identity, "name": identity.split(":", 1)[-1]})
        self.merge(dict(payload, nodes=placeholders + payload.get("nodes", [])))

    def search(self, payload):
        self.scores = {hit["node"]["id"]: hit.get("score", 0) for hit in payload.get("hits", [])}
        self.merge(dict(payload, nodes=[hit["node"] for hit in payload.get("hits", [])]))

    def items(self, query="", used_only=False):
        state = (self.revision, self.types, self.scores)
        if self._cache_state != state:
            self._cache_state = copy.deepcopy(state)
            self._item_cache.clear()
        cache_key = (query, used_only)
        if cache_key in self._item_cache:
            return self._item_cache[cache_key]
        kinds, text = terms(query, self.types.get("schemes", {}))
        words = text.casefold().split()
        uses = Counter(identity for record in self.records for identity in set(record.get("entities", [])))
        result = []
        for identity, node in self.nodes.items():
            kind = identity.split(":", 1)[0]
            if kinds and kind not in kinds or used_only and not uses[identity]:
                continue
            if words and identity not in self.scores:
                if identity not in self._search_text:
                    self._search_text[identity] = json.dumps(node, ensure_ascii=False, default=str).casefold()
                if not all(word in self._search_text[identity] for word in words):
                    continue
            owner = self.types.get("schemes", {}).get(kind, {}).get("owner") or ""
            custom = node.get("attributes", {}).get(owner + ".scope", {})
            summary = custom.get("summary", "") if isinstance(custom, dict) else ""
            result.append(dict(node, type=kind, uses=uses[identity], summary=str(summary),
                               rank=float(self.scores.get(identity, 0)) + math.log1p(uses[identity])))
        totals = Counter()
        for item in result:
            totals[item["type"]] += item["rank"]
        result.sort(key=lambda item: (-totals[item["type"]], item["type"], -item["rank"], item["id"]))
        if len(self._item_cache) >= 8:
            self._item_cache.pop(next(iter(self._item_cache)))
        self._item_cache[cache_key] = result
        return result

    def observations(self, identities):
        return sorted((dict(record, key=f"{self.generation}:{record.get('seq', index)}")
                       for index, record in enumerate(self.records)
                       if identities.intersection(record.get("entities", []))),
                      key=lambda row: (row.get("ts_ms") is None, row.get("ts_ms") or 0, row["key"]))

    def detail_event(self, identity):
        kind = identity.split(":", 1)[0]
        event = "scope.detail." + kind
        owner = self.types.get("schemes", {}).get(kind, {}).get("owner")
        declaration = self.types.get("events", {}).get(event, {})
        return event if owner and declaration.get("owner") == owner else None

    def related(self, identity):
        return [{"relationship": edge.get("kind", "related"), "$entity": edge["to"] if edge["from"] == identity else edge["from"]}
                for edge in self.edges if identity in (edge.get("from"), edge.get("to"))]
