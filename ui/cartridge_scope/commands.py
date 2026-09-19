"""The centered input coordinates filtering and an independent conversation."""
import asyncio
import json
import re
from rich.text import Text
from textual import on, work
from textual.widgets import Input, RichLog, Static
from .conversation import scope_result


class Commands:
    def set_mode(self, agent):
        if self.editing:
            self.close_editor()
        self.agent_mode = agent
        self.query_one("#detail-pane").display = not agent and not self.waterfall_preview
        self.query_one("#waterfall-pane").display = not agent and self.waterfall_preview
        self.query_one("#conversation").display = agent
        entry = self.query_one("#search", Input)
        with entry.prevent(Input.Changed):
            entry.value = self.agent_draft if agent else self.filter_query
        entry.placeholder = "AGENT · Ask about this context · /list to return · /cancel to stop" if agent else "FILTER · memo jev · /agent to talk · Ctrl-Space switches"
        entry.border_title = "Agent" if agent else "Filter"
        entry.focus()

    def action_mode(self):
        self.set_mode(not self.agent_mode)

    @on(Input.Submitted, "#search")
    def submit_command(self, event):
        value = event.value.strip()
        if not value:
            if not self.agent_mode:
                self.query_one("#items").focus()
            return
        command, _, tail = value.partition(" ")
        if command in ("/agent", "/list", "/ask", "/cancel", "/allow", "/deny", "/help"):
            self.agent_draft = ""
        if command == "/agent":
            self.set_mode(True)
            if tail:
                self.ask_agent(tail)
        elif command == "/list":
            self.set_mode(False)
            if tail:
                self.query_one("#search", Input).value = tail
        elif command == "/ask":
            self.set_mode(True)
            self.ask_agent(tail or self.filter_query)
        elif command in ("/cancel", "/allow", "/deny"):
            self.control_agent(command[1:])
        elif command == "/help":
            self.set_mode(True)
            self.chat("Scope", "Type memo jev to filter. /agent or Ctrl-Space opens conversation; Enter sends. /list returns to results. /ask asks the agent to interpret your search. /cancel stops a turn. /allow and /deny answer a displayed tool approval.")
        elif command == "/find":
            self.set_mode(False)
            self.query_one("#search", Input).value = tail or "All "
        elif value.startswith("/"):
            self.query_one("#status", Static).update("Unknown command. Use /help, /agent, /list or /ask.")
        elif self.agent_mode:
            self.ask_agent(value)
        else:
            self.query_one("#items").focus()

    def chat(self, speaker, text):
        self.query_one("#conversation", RichLog).write(Text(speaker, style="bold"))
        self.query_one("#conversation", RichLog).write(Text(text))
        self.query_one("#conversation", RichLog).write("")

    @work(group="agent")
    async def ask_agent(self, request):
        if not request or self.agent.busy:
            self.chat("Scope", "Agent is working. Use /cancel to stop it." if self.agent.busy else "Enter a question.")
            return
        revision = self.search_revision
        self.chat("You", request)
        self.agent_draft = ""
        if self.agent_mode:
            self.query_one("#search", Input).value = ""
        try:
            await self.agent.start(request, self.filter_query, self.items)
            self.chat("Scope", "Agent connected. You can return to /list while it works.")
            last_state = None
            while self.agent.busy:
                status, messages = await self.agent.poll()
                for message in messages:
                    visible = re.sub(r"```scope\s*\n.*?```", "", message, flags=re.DOTALL).strip()
                    if visible:
                        self.chat("Agent", visible)
                state = (status.get("phase"), status.get("step"), status.get("pending"))
                if state != last_state:
                    if self.agent.pending:
                        self.chat("Approval needed", json.dumps(self.agent.pending, ensure_ascii=False) + "\n/allow or /deny")
                    else:
                        self.chat("Scope", status.get("error") or " · ".join(str(value) for value in state[:2] if value))
                    last_state = state
                if self.agent.busy:
                    await asyncio.sleep(.75)
            if status.get("phase") == "completed" and self.agent.replies:
                result = scope_result(self.agent.replies[-1])
                if result is not None:
                    if revision != self.search_revision:
                        self.chat("Scope", "Your filter changed while the agent was working; its result has not replaced your list.")
                    else:
                        await self.apply_agent_result(result)
        except (ValueError, OSError, asyncio.TimeoutError, KeyError) as error:
            self.chat("Agent unavailable", str(error) or "Host request timed out. /list returns to your results.")
            if self.agent.busy:
                self.chat("Scope", "The host turn may still be running. Use /cancel to stop it.")

    async def apply_agent_result(self, result):
        revision = self.search_revision
        query, identities = result["query"], result["entities"]
        if identities is not None:
            unknown = [key for key in identities if key not in self.context.nodes]
            semaphore = asyncio.Semaphore(4)
            async def hydrate(key):
                async with semaphore:
                    try:
                        self.context.merge(await self.client.asp("expand", entity=key, depth=1, limit=128))
                    except (ValueError, OSError, asyncio.TimeoutError):
                        pass
            await asyncio.gather(*(hydrate(key) for key in unknown[:32]))
            missing = len(set(identities) - self.context.nodes.keys())
            if missing:
                self.chat("Scope", f"{missing} agent results are unavailable in ASP and are not displayed.")
        if revision != self.search_revision:
            self.chat("Scope", "Your newer filter was preserved while agent results loaded.")
            return
        self.filter_query = query
        self.finder_rows = None
        self.result_ids = set(identities) if identities is not None else None
        self.search_revision += 1
        if not self.agent_mode:
            entry = self.query_one("#search", Input)
            with entry.prevent(Input.Changed):
                entry.value = query
        self.finder_scopes.clear()
        self.finder_prefixes.clear()
        self.finder_marks.clear()
        if identities is None:
            if self.finder_active(query):
                self.run_finder(query, self.search_revision)
            elif self.live:
                self.search_remote(query, self.search_revision)
        self.refresh_projection()
        self.chat("Scope", f"List updated: {query or 'all context'} · {len(self.items)} loaded matches. /list shows them; editing the filter clears the agent selection.")

    @work(group="agent-control")
    async def control_agent(self, command):
        try:
            if command == "cancel":
                await self.agent.cancel()
            else:
                await self.agent.answer(command)
            self.chat("Scope", "Cancellation requested." if command == "cancel" else "Approval answered.")
        except (ValueError, OSError, asyncio.TimeoutError) as error:
            self.chat("Scope", str(error))

    async def action_quit(self):
        if self.agent.busy:
            try:
                await asyncio.wait_for(self.agent.cancel(), 2)
            except (ValueError, OSError, asyncio.TimeoutError):
                pass
        self.exit()
