"""One discoverable command tree for Scope; ordinary typing remains search."""
import inspect
from rich.text import Text
from textual import on
from textual.binding import Binding
from textual.containers import Container
from textual.screen import ModalScreen
from textual.widgets import Input, OptionList, Static
from textual.widgets.option_list import Option
from .filter_chain import fuzzy_score

GROUPS = {"f": "Find", "a": "Agent", "v": "View", "e": "Edit", "s": "Search options", "t": "Timeline"}
COMMANDS = {
    "ff": ("Find files", "filter", "Files "),
    "fd": ("Find directories", "filter", "Dirs "),
    "fa": ("Find ASP entities", "filter", "ASP "),
    "fu": ("Find across ASP and disk", "filter", "All "),
    "fg": ("Find text with grep", "filter", "Grep "),
    "fm": ("Find memos", "filter", "memo "),
    "fs": ("Focus current search", "search", None),
    "aa": ("Open agent conversation", "agent_mode", True),
    "al": ("Return to list", "agent_mode", False),
    "as": ("Ask agent about current search", "ask", None),
    "ac": ("Cancel agent turn", "agent_control", "cancel"),
    "ay": ("Allow pending agent action", "agent_control", "allow"),
    "an": ("Deny pending agent action", "agent_control", "deny"),
    "vw": ("Switch detail / waterfall", "preview_mode", None),
    "vu": ("Toggle used / all items", "used", None),
    "vz": ("Enlarge focused pane", "zoom_pane", None),
    "vp": ("Pause / resume updates", "pause", None),
    "vr": ("Refresh current results", "refresh", None),
    "vb": ("Back to previous item", "back", None),
    "ee": ("Edit current item", "edit", None),
    "es": ("Save current edit", "save", None),
    "ex": ("Open external editor", "external_editor", None),
    "ec": ("Close editor and keep draft", "close_edit", None),
    "sc": ("Toggle case-sensitive search", "option", "case"),
    "sw": ("Toggle whole-word search", "option", "word"),
    "sr": ("Toggle regular expressions", "option", "regex"),
    "sm": ("Mark / unmark current result", "mark", None),
    "tl": ("Move timeline earlier", "pan", -1),
    "tr": ("Move timeline later", "pan", 1),
    "ti": ("Zoom timeline in", "scale", .5),
    "to": ("Zoom timeline out", "scale", 2),
    "tf": ("Follow latest activity", "follow", None),
    "q": ("Quit Scope", "quit", None),
}


def choices(query):
    query = query.strip().casefold()
    if not query:
        return [(key, label + " …") for key, label in GROUPS.items()] + [("q", "Quit Scope")]
    prefix = [(key, command[0]) for key, command in COMMANDS.items() if key.startswith(query)]
    if prefix:
        return prefix
    ranked = [(score, key, command[0]) for key, command in COMMANDS.items()
              if (score := fuzzy_score(query, command[0])) is not None]
    return [(key, label) for _, key, label in sorted(ranked, reverse=True)]


class LeaderMenu(ModalScreen):
    BINDINGS = [Binding("escape", "dismiss", "Cancel", priority=True),
                Binding("ctrl+space", "dismiss", "Cancel", priority=True),
                Binding("down", "next", "Next", priority=True),
                Binding("up", "previous", "Previous", priority=True)]
    DEFAULT_CSS = """
    LeaderMenu { align: center middle; background: ansi_default; }
    #leader-panel { width: 64; max-width: 100%; height: 18; max-height: 100%; border: round ansi_yellow; background: ansi_default; }
    #leader-title, #leader-hint { height: auto; padding: 0 1; color: ansi_yellow; }
    #leader-options { height: 1fr; min-height: 1; max-height: 12; border: none; background: ansi_default; color: ansi_default; }
    #leader-options > .option-list--option-highlighted { background: ansi_default; color: ansi_default; text-style: reverse; }
    #leader-options > .option-list--option-hover-highlighted, #leader-options > .option-list--option-hover { background: ansi_default; color: ansi_yellow; }
    """

    def compose(self):
        with Container(id="leader-panel"):
            yield Static("COMMANDS · type keys or search by name", id="leader-title")
            yield Input(placeholder="ff · find files", id="leader-input")
            yield OptionList(id="leader-options")
            yield Static("Esc cancels · Backspace goes back · Enter runs selection", id="leader-hint")

    def on_mount(self):
        self.update_choices("")
        self.query_one(Input).focus()

    def update_choices(self, query):
        options = self.query_one(OptionList)
        options.clear_options()
        options.add_options([Option(Text(f"{key:<4} {label}"), id=key) for key, label in choices(query)])
        options.highlighted = 0 if options.option_count else None
        self.query_one("#leader-title", Static).update("COMMANDS" + (" · " + query if query else " · choose a group"))
        self.query_one("#leader-hint", Static).update("Esc cancels · Backspace goes back · Enter runs selection" if options.option_count else "No matching commands · Backspace to correct · Esc cancels")

    @on(Input.Changed, "#leader-input")
    def query_changed(self, event):
        query = event.value.strip().casefold()
        if query in COMMANDS:
            self.dismiss(query)
        else:
            self.update_choices(query)

    def activate(self, key):
        if key in COMMANDS:
            self.dismiss(key)
        else:
            entry = self.query_one(Input)
            entry.value = key
            entry.cursor_position = len(key)
            entry.focus()

    @on(Input.Submitted, "#leader-input")
    def run_selected(self):
        options = self.query_one(OptionList)
        if options.highlighted is not None:
            self.activate(options.get_option_at_index(options.highlighted).id)

    @on(OptionList.OptionSelected)
    def option_selected(self, event):
        self.activate(event.option.id)

    def action_next(self):
        self.query_one(OptionList).action_cursor_down()

    def action_previous(self):
        self.query_one(OptionList).action_cursor_up()

    def action_dismiss(self):
        self.dismiss(None)


class Leader:
    def action_leader(self):
        if isinstance(self.screen, LeaderMenu):
            self.screen.dismiss(None)
        else:
            self.push_screen(LeaderMenu(), self.run_leader_command)

    async def run_leader_command(self, key):
        if key is None:
            return
        _, action, arg = COMMANDS[key]
        if action == "filter":
            self.set_mode(False)
            self.query_one("#search", Input).value = arg
        elif action == "agent_mode":
            self.set_mode(arg)
        elif action == "ask":
            self.set_mode(True)
            self.ask_agent(self.filter_query)
        elif action == "agent_control":
            self.control_agent(arg)
        elif action == "option":
            self.engine.options[arg] = not self.engine.options[arg]
            self.search_revision += 1
            self.run_finder(self.filter_query, self.search_revision)
        elif action == "mark":
            self.finder_mark()
        elif action == "close_edit":
            if self.editing:
                self.close_editor()
        elif action == "quit":
            await self.action_quit()
        else:
            callback = getattr(self, "action_" + action)
            result = callback() if arg is None else callback(arg)
            if inspect.isawaitable(result):
                await result
