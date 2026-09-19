"""An upper-pane text editor, with optional VISUAL/EDITOR handoff."""
import asyncio
import os
import shlex
import tempfile
from pathlib import Path
from textual import work
from textual.widgets import TextArea, Static
from .edit_session import EditSession


class Editing:
    def init_editor(self):
        self.editing = False
        self.edit_session = None
        self.edit_identity = None
        self.edit_drafts = {}

    @work(exclusive=True, group="editor")
    async def action_edit(self):
        if self.editing:
            self.query_one("#editor", TextArea).focus(); return
        row = self._items_by_id.get(self.selected)
        if not row: return
        descriptor = None
        document = self.query_one("#detail").last_document
        if document and document[0] == row["id"]:
            descriptor = document[1].get("edit")
        if descriptor:
            owner = self.context.types.get("schemes", {}).get(row["type"], {}).get("owner")
            tool_owner = self.context.types.get("events", {}).get("tool." + descriptor.get("tool", ""), {}).get("owner")
            if not owner or tool_owner != owner:
                self.query_one("#status", Static).update("Edit action is not declared by this item's owner"); return
        session = EditSession(self.client)
        try:
            if row["id"] in self.edit_drafts:
                session, text = self.edit_drafts[row["id"]]
            else:
                text = await session.open(row, descriptor)
            self.edit_identity = row["id"]
            self.edit_session = session
            self.editing = True
            self.query_one("#detail-pane").display = False
            self.query_one("#waterfall-pane").display = False
            self.query_one("#conversation").display = False
            area = self.query_one("#editor", TextArea)
            area.display = True; area.load_text(text); area.focus()
            self.query_one("#chain", Static).update("EDIT · Ctrl-Space: es save · ex external editor · ec keep draft and close")
        except (OSError, ValueError, KeyError, asyncio.TimeoutError) as error:
            self.query_one("#status", Static).update("Cannot edit: " + str(error))

    @work(exclusive=True, group="save")
    async def action_save(self):
        if not self.editing:
            self.query_one("#status", Static).update("Open an item for editing first: Ctrl-Space ee")
            return
        text = self.query_one("#editor", TextArea).text
        try:
            outcome = await self.edit_session.save(text)
            self.close_editor(saved=True)
            if isinstance(outcome, dict) and "request" in outcome:
                self.set_mode(True); self.ask_agent(outcome["request"])
            else:
                self.query_one("#status", Static).update("Unchanged; nothing written." if outcome == "Unchanged" else "Saved through the owner tool; update events published.")
                self.engine.cache.clear(); self.engine.expanded.clear(); self.engine.disk.scanned = 0
                if self.finder_active(self.filter_query): self.run_finder(self.filter_query, self.search_revision)
                elif self.live: self.action_refresh()
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            self.query_one("#status", Static).update("Save failed; your draft is retained: " + str(error))

    def close_editor(self, saved=False):
        if self.editing:
            text = self.query_one("#editor", TextArea).text
            if not saved and text != self.edit_session.original:
                self.edit_drafts[self.edit_identity] = (self.edit_session, text)
            else:
                self.edit_drafts.pop(self.edit_identity, None)
        self.editing = False
        self.query_one("#editor").display = False
        self.query_one("#detail-pane").display = not self.waterfall_preview and not self.agent_mode
        self.query_one("#waterfall-pane").display = self.waterfall_preview and not self.agent_mode
        self.query_one("#conversation").display = self.agent_mode
        self.query_one("#search").focus()

    @work(exclusive=True, group="external-editor")
    async def action_external_editor(self):
        if not self.editing: return
        command = os.environ.get("VISUAL") or os.environ.get("EDITOR") or "micro"
        area = self.query_one("#editor", TextArea)
        fd, filename = tempfile.mkstemp(prefix="scope-edit-", suffix=".txt")
        os.close(fd)
        path = Path(filename)
        try:
            path.write_text(area.text)
            with self.suspend():
                process = await asyncio.create_subprocess_exec(*shlex.split(command), filename)
                code = await process.wait()
            if code == 0:
                if path.stat().st_size > 65536: raise ValueError("Edited document exceeds 64 KiB")
                area.load_text(path.read_text())
                self.query_one("#chain", Static).update("Editor returned · Ctrl-Space es publishes through the owner tool")
        except (OSError, ValueError) as error:
            self.query_one("#status", Static).update("Editor unavailable: " + str(error))
        finally:
            path.unlink(missing_ok=True)
