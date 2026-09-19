import asyncio
import importlib.util
import json
import time
import unittest
from pathlib import Path

from cartridge_scope.app import Scope
from cartridge_scope.detail import DetailTree, document
from cartridge_scope.model import Context
from cartridge_scope.tables import ItemTable, WaterfallTable, timeline


def context():
    model = Context()
    model.types = {"schemes": {"memo": {"owner": "memo"}, "task": {"owner": "prd"}},
                   "events": {"scope.detail.memo": {"owner": "memo"}}}
    model.merge({"nodes": [{"id": "memo:alpha", "name": "Alpha", "attributes": {"memo.scope": {"summary": "knowledge"}, "memo.body": "Nested meaning"}},
                            {"id": "task:root/plan", "name": "Plan", "attributes": {"prd.scope": {"summary": "specced · 2/8 checks"}}}],
                 "edges": [{"from": "memo:alpha", "to": "task:root/plan", "kind": "links"}]})
    model.activity({"generation": 1, "records": [{"seq": 1, "ts_ms": 1000, "entities": ["task:root/plan"]},
                                                  {"seq": 2, "ts_ms": 2000, "entities": ["memo:alpha"]}]})
    return model


class ModelTests(unittest.TestCase):
    def test_type_filter_and_search_include_custom_summary_and_body(self):
        model = context()
        self.assertEqual([row["id"] for row in model.items("type:task specced")], ["task:root/plan"])
        self.assertEqual([row["id"] for row in model.items("Nested meaning")], ["memo:alpha"])
        self.assertEqual(model.items("type:memo")[0]["summary"], "knowledge")

    def test_snapshot_replacement_and_restart_do_not_duplicate_usage(self):
        model = context()
        payload = {"generation": 1, "records": model.records}
        model.activity(payload)
        self.assertEqual(sum(row["uses"] for row in model.items()), 2)
        model.activity({"generation": 2, "records": []})
        self.assertFalse(model.observations(set(model.nodes)))

    def test_list_ranking_does_not_reorder_chronological_waterfall(self):
        model = context()
        model.scores = {"memo:alpha": 50}
        self.assertEqual(model.items()[0]["id"], "memo:alpha")
        self.assertEqual(model.observations(set(model.nodes))[0]["entities"], ["task:root/plan"])

    def test_only_owner_can_supply_detail_and_unknown_types_fall_back(self):
        model = context()
        self.assertEqual(model.detail_event("memo:alpha"), "scope.detail.memo")
        model.types["events"]["scope.detail.memo"]["owner"] = "other"
        self.assertIsNone(model.detail_event("memo:alpha"))
        self.assertIsNone(model.detail_event("unknown:item"))
        model.types["schemes"]["memo"]["owner"] = None
        self.assertEqual(model.items("type:memo")[0]["summary"], "")

    def test_timing_is_not_invented(self):
        self.assertEqual(timeline({}, 10000, 10000), "Time unavailable")
        self.assertEqual(timeline({"ts_ms": 1000}, 10000, 10000).count("●"), 1)
        self.assertNotIn("━", timeline({"ts_ms": 1000}, 10000, 10000))
        self.assertIn("━", timeline({"ts_ms": 1000, "end_ms": 5000}, 10000, 10000))

    def test_invalid_detail_document_is_rejected(self):
        for value in ({}, {"version": 1, "title": "x", "sections": ["bad"]}):
            with self.assertRaises(ValueError):
                document(value)


class BrowserTests(unittest.IsolatedAsyncioTestCase):
    async def test_shared_filter_three_panes_and_resize_preserve_selection(self):
        app = Scope(".", context=context(), live=False)
        async with app.run_test(size=(160, 40)) as pilot:
            await pilot.pause()
            self.assertEqual(len(app.query(".pane")), 3)
            app.choose("task:root/plan", "summary")
            before = (app.selected, app.field)
            await pilot.resize_terminal(80, 24)
            await pilot.pause()
            self.assertLessEqual(abs(app.query_one("#preview").size.height - app.query_one("#panes").size.height), 1)
            self.assertEqual((app.selected, app.field), before)
            self.assertLess(app.query_one("#detail-pane").region.y, app.query_one("#search").region.y)
            self.assertLess(app.query_one("#search").region.y, app.query_one("#list-pane").region.y)
            app.query_one("#search").value = "type:task"
            await pilot.pause()
            self.assertEqual([item["id"] for item in app.items], ["task:root/plan"])
            self.assertEqual(len(app.occurrences), 1)
            await pilot.resize_terminal(160, 40)
            await pilot.pause()
            self.assertFalse(app.query_one("#panes").has_class("stacked"))
            self.assertEqual(app.selected, "task:root/plan")

    async def test_live_reordering_keeps_cell_identity_and_table_order(self):
        app = Scope(".", context=context(), live=False)
        async with app.run_test(size=(150, 35)) as pilot:
            app.choose("task:root/plan", "summary")
            app.context.scores = {"memo:alpha": 100}
            app.refresh_projection()
            await pilot.pause()
            table = app.query_one("#items", ItemTable)
            self.assertEqual([row.key.value for row in table.ordered_rows], [item["id"] for item in app.items])
            self.assertEqual(app.selected, "task:root/plan")
            app.action_pause()
            app.context.activity({"generation": 1, "records": []})
            app.refresh_projection()
            self.assertEqual(len(app.occurrences), 2)
            app.action_pause()
            self.assertEqual(len(app.occurrences), 0)

    async def test_waterfall_selection_and_nested_details(self):
        app = Scope(".", context=context(), live=False)
        async with app.run_test(size=(150, 35)) as pilot:
            water = app.query_one("#waterfall", WaterfallTable)
            app.action_preview_mode()
            water.focus()
            water.move_cursor(row=1, animate=False)
            await pilot.pause()
            self.assertEqual(app.selected, "memo:alpha")
            self.assertEqual(app.observation["seq"], 2)
            tree = app.query_one("#detail", DetailTree)
            tree.focus()
            await pilot.press("down", "enter")
            self.assertTrue(tree.root.is_expanded)

    async def test_renderer_failure_keeps_generic_content_and_keyboard_alive(self):
        class Client:
            async def call(self, *args, **kwargs):
                raise ValueError("plugin failed")
        app = Scope(".", client=Client(), context=context(), live=False)
        async with app.run_test(size=(80, 24)) as pilot:
            app.choose("memo:alpha")
            tree = app.query_one("#detail", DetailTree)
            generic = tree.last_document[1]
            await app.load_detail("scope.detail.memo", app.context.nodes["memo:alpha"], {"width": 40, "height": 10}, app.detail_token, generic).wait()
            await pilot.pause()
            self.assertEqual(tree.last_document[1]["sections"][0]["label"], "Renderer unavailable")
            await pilot.press("ctrl+f")
            self.assertEqual(app.focused.id, "search")

    async def test_40_by_12_can_reach_every_pane(self):
        app = Scope(".", context=context(), live=False)
        async with app.run_test(size=(40, 12)) as pilot:
            for selector in ("#items", "#waterfall", "#detail"):
                if selector == "#waterfall": app.action_preview_mode()
                if selector == "#detail" and app.waterfall_preview: app.action_preview_mode()
                app.query_one(selector).focus()
                await pilot.pause()
                app.action_zoom_pane()
                await pilot.pause()
                self.assertGreater(app.focused.size.height, 0)
                app.action_zoom_pane()


if __name__ == "__main__":
    unittest.main()
