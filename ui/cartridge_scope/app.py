"""Textual shell for the shared ASP context and its three coordinated panes."""
import asyncio
import copy
import time

from textual import on, work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.containers import Container
from textual.widgets import Input, Static, Footer, DataTable, Tree, RichLog, TextArea
from rich.text import Text

from .client import Client
from .detail import DetailTree, document, fallback
from .model import Context, terms
from .tables import ItemTable, WaterfallTable, sync_table, timeline, timestamp
from .conversation import AgentSession
from .commands import Commands
from .finder_input import Finder, FinderInput
from .search_engine import SearchEngine
from .editor import Editing
from .leader import Leader


class Scope(Leader, Editing, Finder, Commands, App):
    TITLE = "Scope · ASP"
    CSS_PATH = "scope.tcss"
    ENABLE_COMMAND_PALETTE = False
    BINDINGS = [Binding("ctrl+space", "leader", "Commands", priority=True),
                Binding("ctrl+@", "leader", "Commands", show=False, priority=True),
                Binding("escape", "clear", "Back")]

    def __init__(self, project, client=None, context=None, live=True):
        super().__init__(ansi_color=True)
        self.client = client or Client(project)
        self.context = context or Context()
        self.live = live
        self.filter_query = ""
        self.used_only = False
        self.selected = None
        self.field = "name"
        self.observation = None
        self.history = []
        self.frozen = None
        self.items, self.occurrences = [], []
        self.span, self.end = 600000, None
        self.auto_span = True
        self.detail_token = 0
        self.last_detail = None
        self.dimensions = (0, 0)
        self.zoomed = None
        self.polling = False
        self.loading_roots = False
        self.search_revision = 0
        self._rendered_items = None
        self._items_by_id = {}
        self.agent = AgentSession(self.client)
        self.agent_mode = False
        self.agent_draft = ""
        self.result_ids = None
        self.window_start = 0
        self.item_indices = {}
        self.init_editor()
        self.waterfall_preview = False
        self.init_finder(SearchEngine(project, self.client, self.context))

    @property
    def shown(self):
        return self.frozen if self.frozen is not None else self.context

    def compose(self) -> ComposeResult:
        yield Static("Connecting to ASP…", id="status", markup=False)
        with Container(id="preview"):
            with Container(id="waterfall-pane", classes="pane"):
                yield Static("WATERFALL · observed usage", id="time-label", classes="title", markup=False)
                yield WaterfallTable(id="waterfall")
            with Container(id="detail-pane", classes="pane"):
                yield Static("PREVIEW", classes="title", markup=False)
                yield DetailTree(id="detail")
            yield RichLog(id="conversation", wrap=True, markup=False, max_lines=4000)
            yield TextArea(id="editor", show_line_numbers=True, soft_wrap=True)
        yield Static("ASP · Files · Dirs · Grep · All — Tab completes or chains", id="chain", markup=False)
        yield FinderInput(placeholder="memo jev · Files src > Grep TODO · /agent", id="search")
        with Container(id="panes"):
            with Container(id="list-pane", classes="pane"):
                yield Static("RESULTS", classes="title", markup=False)
                yield ItemTable(id="items")
        yield Footer()

    def on_mount(self):
        self.query_one("#search").focus()
        self.refresh_projection()
        if self.live:
            self.bootstrap()
            self.set_interval(1, self.poll)
            self.set_interval(10, self.bootstrap)

    def on_resize(self, event):
        if self.is_mounted:
            self.call_after_refresh(self.refresh_detail)

    @work(exclusive=True, group="bootstrap")
    async def bootstrap(self):
        try:
            types, graph = await asyncio.gather(self.client.asp("types"), self.client.asp("expand", entity="asp:root", depth=1, limit=256))
            self.context.types = types
            self.context.merge(graph)
            self.context.errors.pop("graph", None)
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            self.context.errors["graph"] = str(error)
        self.refresh_projection()
        if self.live:
            self.load_roots()

    @work(group="roots")
    async def load_roots(self):
        if self.loading_roots:
            return
        self.loading_roots = True
        semaphore = asyncio.Semaphore(4)
        async def load(identity):
            async with semaphore:
                try:
                    graph = await self.client.asp("expand", entity=identity, depth=1, limit=128)
                    self.context.merge(graph)
                    self.context.errors.pop(identity, None)
                except (ValueError, OSError, asyncio.TimeoutError) as error:
                    self.context.errors[identity] = str(error) or "Provider timed out"
                self.refresh_projection()
        roots = {edge["to"] for edge in self.context.edges if edge.get("from") == "asp:root"}
        try:
            await asyncio.gather(*(load(identity) for identity in sorted(roots)))
        finally:
            self.loading_roots = False

    @work(group="poll")
    async def poll(self):
        if self.polling:
            return
        self.polling = True
        try:
            payload = await self.client.asp("activity")
            self.context.activity(payload)
            self.context.errors.pop("activity", None)
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            self.context.errors["activity"] = str(error)
        finally:
            self.polling = False
        self.refresh_projection()

    @on(Input.Changed, "#search")
    def search_changed(self, event):
        if self.agent_mode:
            self.agent_draft = event.value
            return
        if event.value.startswith("/"):
            return
        self.filter_query = event.value
        self.result_ids = None
        self.search_revision += 1
        if self.finder_active(event.value):
            self.run_finder(event.value, self.search_revision)
            return
        self.workers.cancel_group(self, "finder")
        self.finder_rows = None
        self.context.errors.pop("finder", None)
        self.query_one("#chain", Static).update("ASP · Files · Dirs · Grep · All — Tab completes or chains")
        self.context.scores = {}
        if self.frozen:
            self.frozen.scores = {}
        self.refresh_projection()
        if self.live:
            self.search_remote(self.filter_query, self.search_revision)

    @work(exclusive=True, group="search")
    async def search_remote(self, query, revision):
        await asyncio.sleep(.25)
        _, text = terms(query, self.context.types.get("schemes", {}))
        if not text.strip():
            self.context.errors.pop("search", None)
            return
        try:
            payload = await self.client.asp("search", query=text, limit=256)
            if revision != self.search_revision:
                return
            self.context.search(payload)
            self.context.errors.pop("search", None)
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            self.context.errors["search"] = str(error)
        self.refresh_projection()

    def refresh_projection(self):
        if not self.is_mounted:
            return
        model = self.shown
        self.items = self.finder_rows if self.finder_rows is not None else model.items("" if self.result_ids is not None else self.filter_query, self.used_only)
        if self.result_ids is not None:
            self.items = [item for item in self.items if item["id"] in self.result_ids]
        identities = {item["id"] for item in self.items}
        if self.selected not in identities:
            self.selected = self.items[0]["id"] if self.items else None
            self.observation = None
        table = self.query_one("#items", ItemTable)
        with table.prevent(DataTable.CellHighlighted):
            if self.items is not self._rendered_items:
                self._rendered_items = self.items
                self._items_by_id = {item["id"]: item for item in self.items}
                self.item_indices = {item["id"]: index for index, item in enumerate(self.items)}
                self.render_window()
            if self.selected and self.field in table.columns:
                table.move_cursor(column=table.get_column_index(self.field), animate=False)
        self.occurrences = model.observations(identities)
        end = self.end if self.end is not None else time.time() * 1000
        if self.auto_span and self.end is None:
            stamps = [row["ts_ms"] for row in self.occurrences if isinstance(row.get("ts_ms"), (float, int))]
            self.span = max(60000, (end - min(stamps)) * 1.05) if stamps else 600000
        water = self.query_one("#waterfall", WaterfallTable)
        selected_key = self.observation.get("key") if self.observation else None
        if selected_key and not any(row["key"] == selected_key for row in self.occurrences):
            self.observation = None
        with water.prevent(DataTable.RowHighlighted):
            sync_table(water, [(row["key"], (timestamp(row.get("ts_ms")), ("▶ " if self.selected in row.get("entities", []) else "") + ", ".join(model.nodes.get(identity, {}).get("name") or identity for identity in row.get("entities", [])), timeline(row, end, self.span))) for row in self.occurrences], selected_key)
            if self.end is None and not self.observation and not water.has_focus and self.occurrences:
                water.move_cursor(row=len(self.occurrences) - 1, animate=False)
        self.query_one("#time-label", Static).update(f"WATERFALL · {timestamp(end-self.span)} → {timestamp(end)} · ● call ━ duration")
        errors = " · ".join(f"{key}: {value}" for key, value in model.errors.items())
        label = f"{'PAUSED' if self.frozen else 'LIVE'} · {len(self.items)} loaded matches · {len(self.occurrences)} uses · {'Used only' if self.used_only else 'All'}"
        label += f" · rows {self.window_start + 1 if self.items else 0}–{min(len(self.items), self.window_start + 300)} · ASP search"
        if self.result_ids is not None:
            label += " · Agent selection"
        if model.partial:
            label += " · Partial graph"
        self.query_one("#status", Static).update(label + (" · " + errors if errors else ""))
        self.refresh_detail()

    def render_window(self):
        table = self.query_one("#items", ItemTable)
        previous_start, previous_scroll = self.window_start, table.scroll_y
        index = self.item_indices.get(self.selected, 0)
        self.window_start = max(0, (index // 100 - 1) * 100)
        visible = self.items[self.window_start:self.window_start + 300]
        with table.prevent(DataTable.CellHighlighted):
            sync_table(table, [(item["id"], (item.get("name") or item["id"], item["summary"], item["type"], f"{item['rank']:.2f}", item["uses"])) for item in visible], self.selected)
            if previous_start != self.window_start:
                table.scroll_to(y=max(0, previous_scroll + previous_start - self.window_start), animate=False, force=True)
                table.scroll_to_region(table._get_cell_region(table.cursor_coordinate), animate=False)
        table.window_start = self.window_start
        table.total_items = len(self.items)

    @on(DataTable.CellHighlighted, "#items")
    def item_highlighted(self, event):
        self.finder_selected = True
        self.choose(event.cell_key.row_key.value, event.cell_key.column_key.value)

    @on(DataTable.CellSelected, "#items")
    def item_selected(self, event):
        self.choose(event.cell_key.row_key.value, event.cell_key.column_key.value)
        self.query_one("#detail").focus()

    def choose(self, identity, field="name", observation=None):
        if identity not in self.shown.nodes and identity not in self._items_by_id:
            return
        previous = self.selected
        self.selected, self.field, self.observation = identity, field, observation
        self.last_detail = None
        index = self.item_indices.get(identity, 0)
        if max(0, (index // 100 - 1) * 100) != self.window_start or identity not in self.query_one("#items", ItemTable).rows:
            self.render_window()
        table = self.query_one("#items", ItemTable)
        with table.prevent(DataTable.CellHighlighted):
            if identity in table.rows:
                table.move_cursor(row=table.get_row_index(identity), column=table.get_column_index(field), animate=False)
        water = self.query_one("#waterfall", WaterfallTable)
        for row in self.occurrences:
            entities = row.get("entities", [])
            if previous != identity and (previous in entities or identity in entities):
                label = ("▶ " if identity in entities else "") + ", ".join(self.shown.nodes.get(key, {}).get("name") or key for key in entities)
                water.update_cell(row["key"], "item", Text(label))
                water._scope_values.pop(row["key"], None)
        self.refresh_detail()
        if self.frozen is None:
            row = self._items_by_id.get(identity, {})
            if "scope.disk" in row.get("attributes", {}):
                self.preview_disk(row)
            elif self.live:
                self.expand_selected(identity)

    @on(DataTable.RowHighlighted, "#waterfall")
    @on(DataTable.RowSelected, "#waterfall")
    def occurrence_highlighted(self, event):
        row = next((row for row in self.occurrences if row["key"] == event.row_key.value), None)
        if row:
            visible = {item["id"] for item in self.items}
            identity = next((identity for identity in row.get("entities", []) if identity in visible), None)
            if identity:
                self.choose(identity, observation=row)

    def on_descendant_focus(self, event):
        if isinstance(event.widget, WaterfallTable) and self.observation is None and self.occurrences:
            row = self.occurrences[min(event.widget.cursor_row, len(self.occurrences) - 1)]
            identity = next((key for key in row.get("entities", []) if key in self._items_by_id), None)
            if identity:
                self.choose(identity, observation=row)

    def move_item(self, index):
        if self.items:
            self.choose(self.items[max(0, min(len(self.items) - 1, index))]["id"], self.field)

    @work(exclusive=True, group="expand")
    async def expand_selected(self, identity):
        await asyncio.sleep(.12)
        try:
            graph = await self.client.asp("expand", entity=identity, depth=1, limit=128)
            self.context.merge(graph)
            self.context.errors.pop("expand", None)
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            self.context.errors["expand"] = str(error)
        self.refresh_projection()

    def refresh_detail(self):
        tree = self.query_one("#detail", DetailTree)
        if not self.selected:
            self.last_detail = None
            self.detail_token += 1
            tree.show_document({"title": "No matching items", "sections": [{"label": "Search", "value": "Clear the filter or wait for ASP."}]}, None)
            return
        item = self._items_by_id.get(self.selected) or self.shown.nodes[self.selected]
        size = {"width": max(1, tree.size.width), "height": max(1, tree.size.height)}
        key = (item, self.field, self.observation, size, self.shown.related(self.selected))
        if key == self.last_detail:
            return
        self.last_detail = copy.deepcopy(key)
        self.detail_token += 1
        payload = fallback(item, key[-1], self.observation, self.field)
        event = self.shown.detail_event(self.selected)
        if not event or not tree.last_document or tree.last_document[0] != self.selected:
            tree.show_document(payload, self.selected)
        if event and self.live and self.frozen is None:
            self.load_detail(event, item, size, self.detail_token, payload)

    @work(exclusive=True, group="detail")
    async def load_detail(self, event, item, size, token, generic):
        try:
            request = {"op": "describe", "version": 1, "item": item, "selection": {"field": self.field, "observation": self.observation}, "viewport": size}
            payload = document(await self.client.call(event, request, timeout=2))
            payload["sections"] += [section for section in generic["sections"] if section["label"] in ("Relationships", "Selected usage") or section["label"].startswith("Selected field:")]
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            payload = dict(generic, sections=[{"label": "Renderer unavailable", "value": str(error) or "Timed out after 2 seconds"}] + generic["sections"])
        if token == self.detail_token and self.frozen is None:
            self.query_one("#detail", DetailTree).show_document(payload, item["id"])

    @on(Tree.NodeSelected, "#detail")
    def follow_relationship(self, event):
        if not event.node.data:
            return
        path, value = event.node.data
        identity = value.get("$entity") if isinstance(value, dict) else value if path[-1:] == ("$entity",) else None
        if not isinstance(identity, str) or ":" not in identity:
            return
        self.history.append((self.filter_query, self.selected, self.field, self.observation))
        self.history = self.history[-100:]
        if identity not in self.context.nodes:
            self.context.merge({"nodes": [{"id": identity}]})
        self.query_one("#search", Input).value = ""
        self.filter_query = ""
        self.refresh_projection()
        self.choose(identity)

    def action_search(self):
        self.query_one("#search").focus()

    def action_clear(self):
        if self.editing:
            self.close_editor()
        elif self.agent_mode:
            self.query_one("#search", Input).value = ""
            self.query_one("#search").focus()
        elif self.zoomed:
            self.action_zoom_pane()
        else:
            self.query_one("#search", Input).value = ""
            self.query_one("#items").focus()

    def action_pause(self):
        self.frozen = copy.deepcopy(self.context) if self.frozen is None else None
        self.detail_token += 1
        self.last_detail = None
        if self.frozen:
            self.end = time.time() * 1000
        else:
            self.end = None
        self.refresh_projection()

    def action_used(self):
        self.used_only = not self.used_only
        self.refresh_projection()

    def action_refresh(self):
        self.last_detail = None
        self.engine.disk.scanned = 0
        self.engine.cache.clear(); self.engine.expanded.clear()
        pipeline = self.finder_active(self.filter_query)
        if pipeline:
            self.run_finder(self.filter_query, self.search_revision)
        if self.live:
            self.bootstrap()
            self.poll()
            if not pipeline:
                self.search_remote(self.filter_query, self.search_revision)
            if self.selected:
                self.choose(self.selected, self.field, self.observation)

    def action_pan(self, direction):
        self.end = (self.end or time.time() * 1000) + direction * self.span / 4
        self.refresh_projection()

    def action_scale(self, factor):
        self.auto_span = False
        self.span = min(86400000, max(1000, self.span * factor))
        self.refresh_projection()

    def action_follow(self):
        if self.frozen is None:
            self.end = None
            self.auto_span = True
        self.refresh_projection()

    def action_back(self):
        if self.history:
            query, identity, field, observation = self.history.pop()
            self.query_one("#search", Input).value = query
            self.filter_query = query
            self.refresh_projection()
            self.choose(identity, field, observation)

    def action_zoom_pane(self):
        if self.editing:
            return
        if self.zoomed:
            self.zoomed = None
        elif self.focused:
            self.zoomed = next((node.id for node in self.focused.ancestors if node.has_class("pane")), None)
        self.query_one("#preview").display = self.zoomed != "list-pane"
        self.query_one("#panes").display = self.zoomed in (None, "list-pane")
        self.query_one("#detail-pane").display = self.zoomed == "detail-pane" or (not self.zoomed and not self.waterfall_preview and not self.agent_mode)
        self.query_one("#waterfall-pane").display = self.zoomed == "waterfall-pane" or (not self.zoomed and self.waterfall_preview and not self.agent_mode)

    def action_preview_mode(self):
        if self.editing:
            self.close_editor()
        self.waterfall_preview = not self.waterfall_preview
        self.query_one("#waterfall-pane").display = self.waterfall_preview and not self.agent_mode
        self.query_one("#detail-pane").display = not self.waterfall_preview and not self.agent_mode
