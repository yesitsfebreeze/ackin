import asyncio
import json
import unittest
from cartridge_scope.app import Scope
from cartridge_scope.model import Context, terms
from cartridge_scope.conversation import AgentSession, scope_result
from cartridge_scope.detail import DetailTree
from cartridge_scope.tables import ItemTable
from test_scope import context


class SearchTests(unittest.TestCase):
    def test_natural_type_and_content_queries(self):
        for query in ('memo jev', 'memos from jev', 'show me memos containing jev', 'memo, jev'):
            self.assertEqual(terms(query, ['memo']), ({'memo'}, 'jev'))
        self.assertEqual(terms('unknown words', ['memo']), (set(), 'unknown words'))

    def test_cached_search_invalidates_on_body_score_and_usage_changes(self):
        model = context()
        self.assertEqual(model.items('memo jev'), [])
        model.merge({'nodes': [{'id': 'memo:alpha', 'attributes': {'memo.body': 'Jev is the decision service'}}]})
        self.assertEqual(model.items('memo jev')[0]['id'], 'memo:alpha')
        first = model.items('memo jev')
        model.merge({'nodes': [model.nodes['memo:alpha']]})
        self.assertIs(model.items('memo jev'), first)
        model.scores['memo:alpha'] = 42
        self.assertGreater(model.items('memo jev')[0]['rank'], 42)
        model.activity({'generation': 2, 'records': []})
        self.assertEqual(model.items('memo jev')[0]['uses'], 0)

    def test_agent_results_are_data_not_commands(self):
        self.assertEqual(scope_result('Answer\n```scope\n{"query":"memo jev","entities":[]}\n```')['entities'], [])
        self.assertIsNone(scope_result('Just a reply'))
        with self.assertRaises(ValueError):
            scope_result('```scope\n{"entities":["shell command"]}\n```')


class NavigationTests(unittest.IsolatedAsyncioTestCase):
    async def test_10000_items_have_at_most_300_rows_across_boundaries(self):
        model = Context()
        model.merge({'nodes': [{'id': f'memo:{i:05}', 'name': 'Duplicate display name'} for i in range(10000)]})
        app = Scope('.', context=model, live=False)
        async with app.run_test(size=(160, 35)) as pilot:
            table = app.query_one('#items', ItemTable)
            for index in (0, 99, 100, 199, 200, 299, 300, 5000, 9999, 100, 0):
                app.move_item(index)
                await pilot.pause()
                self.assertLessEqual(len(table.rows), 300)
                self.assertEqual(app.selected, f'memo:{index:05}')
                self.assertEqual(table.ordered_rows[table.cursor_row].key.value, app.selected)
            table.focus()
            await pilot.press('pagedown')
            self.assertEqual(app.selected, 'memo:00100')
            await pilot.press('pageup')
            self.assertEqual(app.selected, 'memo:00000')

    async def test_bottom_input_modes_and_agent_explicit_selection(self):
        app = Scope('.', context=context(), live=False)
        async with app.run_test(size=(100, 35)) as pilot:
            entry = app.query_one('#search')
            self.assertLess(entry.region.y, app.query_one('#panes').region.y)
            self.assertGreater(entry.region.y, app.query_one('#preview').region.y)
            entry.value = 'memo Nested'
            await pilot.pause()
            self.assertEqual([row['id'] for row in app.items], ['memo:alpha'])
            app.set_mode(True)
            entry.value = 'Explain this'
            await pilot.pause()
            self.assertEqual(app.filter_query, 'memo Nested')
            app.set_mode(False)
            await pilot.pause()
            self.assertEqual(entry.value, 'memo Nested')
            await app.apply_agent_result({'query': 'the best plan', 'entities': ['task:root/plan']})
            self.assertEqual([row['id'] for row in app.items], ['task:root/plan'])
            entry.value = 'memo'
            await pilot.pause()
            self.assertIsNone(app.result_ids)
            self.assertEqual([row['id'] for row in app.items], ['memo:alpha'])

    async def test_foreground_only_styles_and_long_details(self):
        app = Scope('.', context=context(), live=False)
        async with app.run_test(size=(160, 35)) as pilot:
            table = app.query_one('#items', ItemTable)
            table.focus()
            await pilot.pause()
            for component in ('datatable--header', 'datatable--fixed', 'datatable--hover'):
                self.assertEqual(table.get_component_rich_style(component).bgcolor.name, 'default')
            self.assertTrue(table.get_component_rich_style('datatable--cursor').reverse)
            tree = app.query_one('#detail', DetailTree)
            body = 'A long explanation ' * 40 + '\nSecond paragraph.'
            tree.show_document({'title': 'Memo', 'sections': [{'label': 'Body', 'value': body}]}, 'memo:alpha')
            node = tree.root.children[0]
            tree.populate(node)
            self.assertGreater(len(node.children), 2)
            self.assertIn('Second paragraph.', ''.join(str(child.label) for child in node.children))
            tree.focus()
            await pilot.pause()
            self.assertFalse(table.get_component_rich_style('datatable--cursor').reverse)
            self.assertTrue(tree.get_component_rich_style('tree--cursor').reverse)

    async def test_actual_agent_protocol_preserves_session_and_explicit_approval(self):
        class Client:
            project = '/project'
            calls = []
            async def call(self, event, request):
                self.calls.append((event, request))
                op = request['op']
                if event == 'sessions' and op == 'create': return {'id': 'real-session'}
                if event == 'agent' and op == 'start': return {'session': 'real-session', 'run': 'run:1'}
                if event == 'agent' and op == 'status': return {'seq': 3, 'phase': 'completed', 'pending': None}
                if event == 'sessions' and op == 'get': return {'transcript': 12}
                if event == 'buffers': return {'text': json.dumps({'run': 'run:1', 'kind': 'message', 'message': {'role': 'assistant', 'content': 'Found it.'}})}
                return {}
        client = Client()
        agent = AgentSession(client)
        await agent.start('Find Jev', '', [])
        state, messages = await agent.poll()
        self.assertEqual(messages, ['Found it.'])
        self.assertFalse(agent.busy)
        self.assertFalse(any(request['op'] == 'answer' for _, request in client.calls))
        agent.pending = {'call': 'tool:1', 'input': {'query': 'jev'}}
        await agent.answer('deny')
        self.assertEqual(client.calls[-1][1]['decision'], 'deny')
        await agent.start('Tell me more', '', [])
        self.assertEqual(sum(request['op'] == 'create' for _, request in client.calls), 1)
