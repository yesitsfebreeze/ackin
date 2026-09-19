"""Stable table identities and truthful timeline glyphs for every ASP type."""
from datetime import datetime
from rich.text import Text
from rich.style import Style
from textual.widgets import DataTable


def sync_table(table, rows, selected=None):
    previous = getattr(table, "_scope_values", {})
    next_values = dict(rows)
    keys = {key for key, _ in rows}
    for key in list(table.rows):
        if key.value not in keys:
            table.remove_row(key)
    order = {}
    for index, (key, values) in enumerate(rows):
        order[key] = index
        if previous.get(key) == values:
            continue
        if key not in table.rows:
            table.add_row(*(Text(str(value), style=Style(meta={"scope_key": key})) for value in values), key=key)
        else:
            for index, (column, value) in enumerate(zip(table.columns, values)):
                if previous.get(key, ())[index:index+1] != (value,):
                    table.update_cell(key, column, Text(str(value), style=Style(meta={"scope_key": key})))
    table._scope_values = next_values
    if rows:
        current = [row.key.value for row in table.ordered_rows]
        wanted = [key for key, _ in rows]
        if current != wanted:
            table.sort(*list(table.columns)[:1], key=lambda value: order[value.style.meta["scope_key"]])
        if selected in table.rows:
            table.move_cursor(row=table.get_row_index(selected), column=table.cursor_column, animate=False)


class ItemTable(DataTable):
    def __init__(self, **kwargs):
        super().__init__(cursor_type="cell", **kwargs)
        self.add_column("Item", key="name", width=18)
        self.add_column("Summary", key="summary", width=24)
        self.add_column("Type", key="type", width=10)
        self.add_column("Rank", key="rank", width=7)
        self.add_column("Uses", key="uses", width=6)
        self.fixed_columns = 1
        self.window_start = 0
        self.total_items = 0

    def action_cursor_down(self):
        self.app.move_item(self.window_start + self.cursor_row + 1)

    def action_cursor_up(self):
        self.app.move_item(self.window_start + self.cursor_row - 1)

    def action_page_down(self):
        self.app.move_item(self.window_start + self.cursor_row + 100)

    def action_page_up(self):
        self.app.move_item(self.window_start + self.cursor_row - 100)

    def action_scroll_top(self):
        self.app.move_item(0)

    def action_scroll_bottom(self):
        self.app.move_item(self.total_items - 1)

    def _on_mouse_scroll_down(self, event):
        event.stop()
        event.prevent_default()
        self.app.move_item(self.window_start + self.cursor_row + 3)

    def _on_mouse_scroll_up(self, event):
        event.stop()
        event.prevent_default()
        self.app.move_item(self.window_start + self.cursor_row - 3)


class WaterfallTable(DataTable):
    def __init__(self, **kwargs):
        super().__init__(cursor_type="row", **kwargs)
        self.add_column("Occurrence", key="key", width=15)
        self.add_column("Item", key="item", width=20)
        self.add_column("Time →", key="time", width=32)
        self.fixed_columns = 1


def timeline(record, end, span, width=32):
    stamp = record.get("ts_ms")
    if not isinstance(stamp, (int, float)):
        return "Time unavailable"
    left = end - span
    if stamp < left or stamp > end:
        return "← before window" if stamp < left else "after window →"
    position = min(width - 1, max(0, int((stamp - left) / span * width)))
    cells = ["·"] * width
    finish = record.get("end_ms")
    if isinstance(finish, (int, float)) and finish >= stamp:
        stop = min(width, max(position + 1, int((finish - left) / span * width)))
        cells[position:stop] = ["━"] * (stop - position)
    else:
        cells[position] = "●"
    return "".join(cells)


def timestamp(stamp):
    return datetime.fromtimestamp(stamp / 1000).strftime("%H:%M:%S") if isinstance(stamp, (int, float)) else "unknown"
