"""Cancellable disk enumeration and content search; no shell interpolation."""
import asyncio
import json
import os
from pathlib import Path
import time
from .filter_chain import fuzzy, literal


class DiskSearch:
    def __init__(self, root):
        self.root = Path(root).resolve()
        self.paths = None
        self.scanned = 0
        self.partial = False
        self.file_rows = []
        self.directory_rows = []

    def path(self, relative):
        path = (self.root / relative).resolve()
        if not path.is_relative_to(self.root):
            raise ValueError("Path is outside this project")
        return path

    async def run(self, args, limit=16 * 1024 * 1024):
        process = await asyncio.create_subprocess_exec(*args, cwd=self.root, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
        async def read(stream, maximum):
            data = bytearray()
            while chunk := await stream.read(65536):
                data.extend(chunk)
                if len(data) > maximum:
                    raise ValueError("Disk search output limit reached; narrow the preceding filter")
            return bytes(data)
        try:
            stdout, stderr = await asyncio.wait_for(asyncio.gather(read(process.stdout, limit), read(process.stderr, 16384)), 15)
            await process.wait()
            if process.returncode not in (0, 1):
                raise ValueError(stderr.decode(errors="replace").strip() or "Disk search failed")
            return stdout
        finally:
            if process.returncode is None:
                process.kill(); await process.wait()

    def node(self, path, line=None, content=None, directory=False):
        relative = str(path)
        kind = "directory" if directory else "disk-line" if line is not None else "file"
        return {"id": f"{kind}:{relative}" + (f":{line}" if line is not None else ""),
                "name": relative + (f":{line}" if line is not None else ""), "type": kind,
                "summary": content.strip()[:240] if content else "Directory" if directory else "File",
                "rank": 0, "uses": 0, "content": content or "",
                "attributes": {"scope.disk": {"path": relative, "line": line}},
                "description": "Disk content match" if line is not None else "Project directory" if directory else "Project file"}

    async def files(self, query="", inputs=None, directories=False, case=False):
        if self.paths is None or time.monotonic() - self.scanned > 10:
            data = await self.run(["rg", "--no-config", "--no-ignore-parent", "--files", "--null", "--hidden", "--no-require-git", "--glob", "!.git", "--", "."])
            self.paths = sorted({os.fsdecode(path).removeprefix("./") for path in data.split(b"\0") if path})
            self.scanned = time.monotonic()
            self.file_rows = [self.node(path) for path in self.paths]
            parents = sorted({str(parent) for path in self.paths for parent in Path(path).parents if str(parent) != "."})
            self.directory_rows = [self.node(path, directory=True) for path in parents]
        paths = self.paths
        if inputs is not None:
            direct, roots = set(), []
            for row in inputs:
                path = row.get("attributes", {}).get("scope.disk", {}).get("path")
                if path is None and row["id"].startswith("file:"):
                    path = row["id"][5:]
                if path is not None:
                    if row.get("type") == "directory": roots.append(path.rstrip("/") + "/")
                    else: direct.add(path)
            paths = [p for p in paths if p in direct or any(p.startswith(root) for root in roots)]
        if directories:
            paths = sorted({str(parent) for path in paths for parent in Path(path).parents if str(parent) != "."})
        if inputs is None:
            rows = self.directory_rows if directories else self.file_rows
        else:
            if directories and roots:
                paths = [path for path in paths if any(path == root.rstrip("/") or path.startswith(root) for root in roots)]
            rows = [self.node(path, directory=directories) for path in paths]
        return await asyncio.to_thread(fuzzy, rows, query, case)

    async def grep(self, query, inputs=None, case=False, word=False, regex=False):
        if not query:
            return []
        paths = None
        if inputs is not None:
            paths = sorted({row.get("attributes", {}).get("scope.disk", {}).get("path") or row["id"][5:]
                            for row in inputs if "scope.disk" in row.get("attributes", {}) or row["id"].startswith("file:")})
            if not paths:
                return []
            for path in paths:
                self.path(path)
        args = ["rg", "--no-config", "--no-ignore-parent", "--json", "--hidden", "--no-require-git", "--glob", "!.git", "--max-columns", "10000", "-s" if case else "-i"]
        if not regex: args.append("-F")
        if word: args.append("-w")
        groups = [paths[i:i+128] for i in range(0, len(paths), 128)] if paths is not None else [["."]]
        result = []
        for group in groups:
            data = await self.run(args + ["-e", literal(query), "--"] + group)
            for line in data.splitlines():
                event = json.loads(line)
                if event.get("type") != "match": continue
                item = event["data"]
                path, content = item["path"].get("text"), item["lines"].get("text")
                if path is None or content is None: continue
                result.append(self.node(path.removeprefix("./"), item["line_number"], content.rstrip("\n")))
                if len(result) >= 10000:
                    self.partial = True
                    return result
        return result

    async def preview(self, row):
        disk = row.get("attributes", {}).get("scope.disk")
        if not disk or row.get("type") == "directory": return row
        path = self.path(disk["path"])
        def read():
            with path.open("rb") as source:
                return source.read(131072).decode(errors="replace")
        text = await asyncio.to_thread(read)
        return dict(row, attributes={**row.get("attributes", {}), "scope.preview": text, "scope.preview_limit": "First 128 KiB"})
