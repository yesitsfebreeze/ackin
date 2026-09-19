"""Typed composable filter stages and deterministic fuzzy ranking."""
from dataclasses import dataclass
import json
import re
from rapidfuzz import process
from rapidfuzz.distance import LCSseq


@dataclass(frozen=True)
class Stage:
    picker: str
    query: str = ""


PICKERS = {
    "ASP": ({"none"}, "entity"),
    "All": ({"none"}, "mixed"),
    "Files": ({"none", "file", "directory", "entity", "line", "mixed"}, "file"),
    "Dirs": ({"none", "directory"}, "directory"),
    "Grep": ({"none", "file", "directory", "entity", "line", "mixed"}, "line"),
    "Fuzzy": ({"entity", "file", "directory", "line", "mixed"}, None),
    "Type": ({"entity", "file", "directory", "line", "mixed"}, None),
    "Expand": ({"entity", "file", "mixed"}, "entity"),
}


def segments(value):
    """A separator inside quotes or escaped with a backslash is literal."""
    result, current, quote, escaped = [], [], None, False
    for char in value:
        if escaped:
            current.extend(("\\", char)); escaped = False
        elif char == "\\":
            escaped = True
        elif char == quote:
            quote = None; current.append(char)
        elif char in "\"'" and quote is None:
            quote = char; current.append(char)
        elif char == ">" and quote is None:
            result.append("".join(current).strip()); current = []
        else:
            current.append(char)
    if escaped:
        current.append("\\")
    result.append("".join(current).strip())
    return result


def parse(value):
    result, kind = [], "none"
    for index, part in enumerate(segments(value)):
        name, _, query = part.partition(" ")
        matches = [key for key in PICKERS if key.casefold() == name.casefold()]
        if not part or not matches:
            if index == 0:
                result.append(Stage("ASP", part)); kind = "entity"; continue
            raise ValueError("Choose the next filter: " + ", ".join(compatible(kind)))
        name = matches[0]
        accepts, produces = PICKERS[name]
        if kind not in accepts:
            raise ValueError(f"{name} cannot follow {kind} results")
        result.append(Stage(name, query.strip()))
        kind = produces or kind
    return result


def compatible(kind):
    return [name for name, (accepts, _) in PICKERS.items() if kind in accepts]


def output_type(stages):
    kind = "none"
    for stage in stages:
        kind = PICKERS[stage.picker][1] or kind
    return kind


def literal(query):
    if len(query) >= 2 and query[0] == query[-1] and query[0] in "\"'":
        return query[1:-1]
    return query.replace("\\>", ">")


def fuzzy_score(query, text, case=False):
    if not case:
        query, text = query.casefold(), text.casefold()
    if not query:
        return 0.0
    at, score, previous = 0, 0.0, -2
    for char in query:
        index = text.find(char, at)
        if index < 0:
            return None
        score += 4 + (8 if index == previous + 1 else 0)
        score += 6 if index == 0 or text[index-1] in "/_.- :" else 0
        score -= (index - at) * .05
        previous, at = index, index + 1
    return score + (20 if query in text else 0) - len(text) * .005


def fuzzy(rows, query, case=False):
    words = literal(query).split()
    if not words:
        return rows
    choices = [str(row.get("name") or row["id"]) + (" " + row["content"] if row.get("content") else "") for row in rows]
    if not case:
        choices = [text.casefold() for text in choices]
        words = [word.casefold() for word in words]
    candidates = None
    for word in sorted(words, key=len, reverse=True):
        pool = choices if candidates is None else {index: choices[index] for index in candidates}
        candidates = [index for _, _, index in process.extract_iter(word, pool, scorer=LCSseq.similarity, score_cutoff=len(word))]
        if not candidates: return []
    ranked = []
    for index in candidates:
        text = choices[index]
        score = sum(fuzzy_score(word, text, True) for word in words)
        ranked.append((score, rows[index]))
    ranked.sort(key=lambda pair: (-pair[0], pair[1]["id"]))
    return [dict(row, rank=score) for score, row in ranked]


def grep_rows(rows, query, case=False, word=False, regex=False):
    query = literal(query)
    pattern = query if regex else re.escape(query)
    if word:
        pattern = r"\b(?:" + pattern + r")\b"
    try:
        matcher = re.compile(pattern, 0 if case else re.IGNORECASE)
    except re.error as error:
        raise ValueError(f"Invalid regular expression: {error}") from error
    return [row for row in rows if matcher.search(str(row.get("content", json.dumps(row, ensure_ascii=False))))]
