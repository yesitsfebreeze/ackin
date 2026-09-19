"""Host requests with cancellation and bounded response sizes."""
import asyncio
import json
import os


class Client:
    def __init__(self, project, executable=None):
        self.project = project
        self.executable = executable or os.environ.get("CARTRIDGE_SCOPE_HOST", "cartridge")

    async def call(self, event, request, timeout=8):
        process = await asyncio.create_subprocess_exec(
            self.executable, "--dir", self.project, "call", event, json.dumps(request),
            cwd=self.project, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
            limit=4 * 1024 * 1024)
        async def bounded(stream, limit):
            result = bytearray()
            while chunk := await stream.read(65536):
                result.extend(chunk)
                if len(result) > limit:
                    raise ValueError("Host response exceeds the browser limit")
            return bytes(result)

        async def read():
            output, error = await asyncio.gather(bounded(process.stdout, 4 * 1024 * 1024), bounded(process.stderr, 8192))
            await process.wait()
            if process.returncode:
                raise ValueError(error.decode(errors="replace").strip() or "Host request failed")
            payload = json.loads(output)
            if not isinstance(payload, dict):
                raise ValueError("Expected a JSON object from the host")
            return payload
        try:
            return await asyncio.wait_for(read(), timeout)
        finally:
            if process.returncode is None:
                process.kill()
                await process.wait()

    async def asp(self, op, **kwargs):
        return await self.call("asp", dict(op=op, observe=False, **kwargs))
