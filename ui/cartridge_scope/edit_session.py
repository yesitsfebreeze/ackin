"""Edits use the same revision guards, tool events and provenance as agent writes."""
import asyncio
import hashlib
from pathlib import Path
import json
import uuid


class EditSession:
    def __init__(self, client):
        self.client = client
        self.context = None
        self.target = None
        self.original = None
        self.revision = None

    async def tool(self, name, args):
        context = dict(self.context, call=uuid.uuid4().hex)
        result = await self.client.call("tool." + name, {"op": "call", "context": context, "input": args})
        if result.get("error"):
            raise ValueError(str(result.get("content", "Edit tool failed")))
        content = result.get("content", result)
        if isinstance(content, str):
            try: return json.loads(content)
            except ValueError: return content
        return content

    async def open(self, row, descriptor=None):
        session = await self.client.call("sessions", {"op": "create", "name": "Scope edit", "cwd": self.client.project})
        self.context = {"session": session["id"], "run": "scope-edit-" + uuid.uuid4().hex, "cwd": self.client.project}
        disk = row.get("attributes", {}).get("scope.disk", {})
        path = disk.get("path") or (row["id"][5:] if row["id"].startswith("file:") else None)
        if descriptor:
            self.target = descriptor
            data = await self.tool(descriptor["tool"], descriptor["read"])
            if not isinstance(data, dict): raise ValueError("Edit provider did not return a document")
            self.original = data[descriptor.get("read_text_key", "text")]
            self.revision = data.get("revision")
        elif path:
            self.target = {"path": path}
            root = Path(self.client.project).resolve()
            target = (root / path).resolve()
            if not target.is_relative_to(root): raise ValueError("File is outside this project")
            def read():
                with target.open("rb") as source:
                    data = source.read(65537)
                if len(data) > 65536: raise ValueError("Inline edits support complete files up to 64 KiB")
                return data
            data = await asyncio.to_thread(read)
            self.original = data.decode("utf-8")
            self.revision = hashlib.sha256(data).hexdigest()
            await self.tool("read", {"path": path, "offset": 1, "limit": 1})
            check = await asyncio.to_thread(read)
            if data != check: raise ValueError("File changed while opening; open it again")
        else:
            self.target = {"request": row["id"]}
            self.original = json.dumps({key: row.get(key) for key in ("id", "name", "description", "attributes")}, ensure_ascii=False, indent=2)
        if not isinstance(self.original, str) or len(self.original.encode()) > 65536:
            raise ValueError("Inline edits support complete text documents up to 64 KiB")
        return self.original

    async def save(self, text):
        if len(text.encode()) > 65536: raise ValueError("Edited document exceeds 64 KiB")
        if text == self.original: return "Unchanged"
        if "request" in self.target:
            return {"request": "Apply this user-requested change through the owning tools. Refresh the entity first, inspect its current state and report the actual outcome. Entity: " + self.target["request"] + "\nOriginal preview:\n" + self.original + "\nUser-edited proposal:\n" + text}
        if "tool" in self.target:
            args = dict(self.target["write"])
            args[self.target.get("write_text_key", "body")] = text
            if self.revision: args["expected_revision"] = self.revision
            outcome = await self.tool(self.target["tool"], args)
        else:
            path = self.target["path"]
            outcome = await self.tool("write", {"path": path, "text": text})
            if isinstance(outcome, str) and "materialize to run it" in outcome:
                await self.tool("gitfs", {"op": "materialize", "paths": [path]})
        self.original = text
        return outcome
