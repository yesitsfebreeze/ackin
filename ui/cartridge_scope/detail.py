"""The declarative detail boundary and a generic, navigable value tree."""
import json
import textwrap
from rich.text import Text
from textual.widgets import Tree


def document(payload):
    if not isinstance(payload, dict) or payload.get("version") != 1:
        raise ValueError("Detail renderer must return version 1")
    if not isinstance(payload.get("title"), str) or not isinstance(payload.get("sections"), list):
        raise ValueError("Detail renderer must return title and sections")
    if len(payload["sections"]) > 64 or len(json.dumps(payload)) > 262144:
        raise ValueError("Detail renderer exceeded the document limit")
    for section in payload["sections"]:
        if not isinstance(section, dict) or not isinstance(section.get("label"), str) or "value" not in section:
            raise ValueError("Every detail section needs label and value")
    edit = payload.get("edit")
    if edit is not None:
        if not isinstance(edit, dict) or not isinstance(edit.get("tool"), str) or not all(isinstance(edit.get(key), dict) for key in ("read", "write")):
            raise ValueError("An edit action needs a tool and read/write requests")
        for key in ("read_text_key", "write_text_key"):
            if key in edit and not isinstance(edit[key], str):
                raise ValueError("Edit text keys must be strings")
    return payload


def fallback(item, relationships, observation=None, field=None, error=None):
    sections = []
    if error:
        sections.append({"label": "Renderer unavailable", "value": error})
    if field:
        sections.append({"label": "Selected field: " + field, "value": item.get(field)})
    sections.extend([{"label": "Meaning", "value": item.get("description", "No description published")},
                     {"label": "Published attributes", "value": item.get("attributes", {})},
                     {"label": "Identity and provenance", "value": {k: v for k, v in item.items() if k != "attributes"}},
                     {"label": "Relationships", "value": relationships}])
    if observation:
        sections.insert(0, {"label": "Selected usage", "value": observation})
    return {"version": 1, "title": item.get("name") or item["id"], "sections": sections}


class DetailTree(Tree):
    """Build values lazily, keeping expanded paths stable across refreshes."""
    def __init__(self, **kwargs):
        super().__init__("Select an item", **kwargs)
        self.show_root = True
        self.last_document = None

    def show_document(self, payload, identity):
        signature = (identity, payload)
        if signature == self.last_document:
            return
        same_item = self.last_document and self.last_document[0] == identity
        expanded = set()
        cursor_path = self.cursor_node.data[0] if self.cursor_node and self.cursor_node.data else None
        def remember(node):
            if node.is_expanded and node.data:
                expanded.add(node.data[0])
            for child in node.children:
                remember(child)
        if same_item:
            remember(self.root)
        self.clear()
        self.root.set_label(Text(payload["title"]))
        self.root.expand()
        self.last_document = signature
        self._restore_cursor = cursor_path if same_item else None
        self._expanded = expanded
        for index, section in enumerate(payload["sections"]):
            self.add_value(self.root, section["label"], section["value"], (index,))

    def add_value(self, parent, label, value, path):
        long_text = isinstance(value, str) and ("\n" in value or len(value) > 80)
        branch = isinstance(value, (dict, list)) and bool(value) or long_text
        display = f"{label} ({len(value)}{' characters' if long_text else ''})" if branch else f"{label}: {value}"
        node = parent.add(Text(display), data=(path, value), allow_expand=branch)
        if path == self._restore_cursor:
            self.select_node(node)
        if path in self._expanded:
            self.populate(node)
            node.expand()

    def populate(self, node):
        if node.children or not node.data:
            return
        path, value = node.data
        if isinstance(value, str):
            width = max(12, self.size.width - 4 - len(path) * 2)
            count = 0
            for line in value.splitlines():
                for part in textwrap.wrap(line, width=width, replace_whitespace=False, drop_whitespace=False) or [""]:
                    if count >= 1000:
                        node.add_leaf(Text("Further text omitted (1000 line limit)"))
                        return
                    node.add_leaf(Text(part))
                    count += 1
            return
        children = value.items() if isinstance(value, dict) else enumerate(value) if isinstance(value, list) else []
        for index, (label, child) in enumerate(children):
            if index >= 1000:
                node.add_leaf(Text("Further values omitted (1000 child limit)"))
                break
            self.add_value(node, str(label), child, path + (str(label),))

    def on_tree_node_expanded(self, event):
        self.populate(event.node)
