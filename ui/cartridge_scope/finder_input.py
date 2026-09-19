"""Finder input keys: typed stages, compatible completion, scopes and backtracking."""
import asyncio
from textual import work
from textual.widgets import Input, Static
from .filter_chain import PICKERS, compatible, fuzzy_score, output_type, parse, segments


class FinderInput(Input):
    async def on_key(self, event):
        app = self.app
        if app.agent_mode or self.value.startswith("/"):
            return
        if event.key in ("tab", "shift+tab", "ctrl+n", "ctrl+k", "up", "down", "ctrl+t", "ctrl+w", "ctrl+x"):
            event.stop(); event.prevent_default()
            if event.key == "tab": app.finder_forward()
            elif event.key == "shift+tab": app.finder_back()
            elif event.key == "ctrl+t": app.finder_mark()
            elif event.key in ("ctrl+w", "ctrl+x"):
                key = "word" if event.key == "ctrl+w" else "regex"
                app.engine.options[key] = not app.engine.options[key]
                app.search_revision += 1
                app.run_finder(self.value, app.search_revision)
            else:
                app.finder_selected = True
                app.move_item(app.item_indices.get(app.selected, 0) + (1 if event.key in ("ctrl+n", "down") else -1))
        elif event.key == "backspace" and self.cursor_position == 0 and ">" in self.value:
            event.stop(); event.prevent_default(); app.finder_back()


class Finder:
    def init_finder(self, engine):
        self.engine = engine
        self.finder_rows = None
        self.finder_scopes = {}
        self.finder_prefixes = {}
        self.finder_selected = False
        self.finder_marks = set()

    def finder_active(self, value):
        return ">" in value or value.split(" ", 1)[0].casefold() in {name.casefold() for name in PICKERS}

    def finder_forward(self):
        entry = self.query_one("#search", Input)
        parts = segments(entry.value)
        tail = parts[-1]
        prior = " > ".join(parts[:-1])
        try:
            choices = compatible(output_type(parse(prior))) if prior else compatible("none")
        except ValueError:
            return
        matches = sorted(((-score, name) for name in choices if (score := fuzzy_score(tail, name)) is not None))
        if matches and (not tail or tail.casefold() not in {name.casefold() for name in PICKERS}):
            entry.value = (prior + " > " if prior else "") + matches[0][1] + " "
            entry.cursor_position = len(entry.value)
            return
        try: stages = parse(entry.value)
        except ValueError as error:
            self.query_one("#status", Static).update(str(error)); return
        rows = self.finder_rows if self.finder_rows is not None else self.items
        ids = self.finder_marks or ({self.selected} if self.finder_selected else set())
        if ids: rows = [row for row in rows if row["id"] in ids]
        index = len(stages)
        self.finder_scopes[index] = list(rows)
        self.finder_prefixes[index] = entry.value.strip()
        self.finder_marks.clear(); self.finder_selected = False
        entry.value = entry.value.rstrip() + " > "
        entry.cursor_position = len(entry.value)
        self.query_one("#chain", Static).update("Next: " + " · ".join(compatible(output_type(stages))))

    def finder_back(self):
        entry = self.query_one("#search", Input)
        parts = segments(entry.value)
        entry.value = " > ".join(parts[:-1]) if len(parts) > 1 else ""
        entry.cursor_position = len(entry.value)
        self.finder_marks.clear(); self.finder_selected = False

    def finder_mark(self):
        if self.selected:
            if self.selected in self.finder_marks: self.finder_marks.remove(self.selected)
            else: self.finder_marks.add(self.selected)
            self.query_one("#chain", Static).update(f"{len(self.finder_marks)} marked · Tab passes marked results")

    @work(exclusive=True, group="finder")
    async def run_finder(self, expression, revision):
        await asyncio.sleep(.06)
        parts = segments(expression)
        if len(parts) > 1 and not parts[-1]:
            try:
                self.query_one("#chain", Static).update("Next: " + " · ".join(compatible(output_type(parse(" > ".join(parts[:-1]))))))
            except ValueError as error:
                self.query_one("#chain", Static).update(str(error))
            return
        scopes = {index: rows for index, rows in self.finder_scopes.items()
                  if self.finder_prefixes[index] == " > ".join(parts[:index])}
        self.finder_scopes = scopes
        try:
            rows = await self.engine.evaluate(expression, scopes)
            if revision != self.search_revision: return
            self.finder_rows = rows
            self.context.errors.pop("finder", None)
            if self.engine.errors:
                self.context.errors["finder"] = "; ".join(dict.fromkeys(self.engine.errors))
            self.query_one("#chain", Static).update(expression + " · " + " ".join(key for key, enabled in self.engine.options.items() if enabled) + f" · {len(rows)} results" + (" · result limit reached" if self.engine.disk.partial else ""))
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            if revision != self.search_revision: return
            self.finder_rows = []
            self.context.errors["finder"] = str(error) or "Search timed out"
            self.query_one("#chain", Static).update(str(error))
        self.refresh_projection()
        if self.selected and self.finder_rows:
            self.choose(self.selected, self.field)

    @work(exclusive=True, group="expand")
    async def preview_disk(self, row):
        try:
            node = await self.engine.disk.preview(row)
            self.context.merge({"nodes": [node]})
            if self.selected == row["id"]:
                self._items_by_id[row["id"]] = dict(row, **node)
                self.last_detail = None; self.refresh_detail()
        except (OSError, ValueError) as error:
            self.context.errors["preview"] = str(error)
            self.query_one("#status", Static).update(str(error))
