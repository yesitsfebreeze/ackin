import asyncio
from pathlib import Path
import tempfile
import unittest
from cartridge_scope.app import Scope
from cartridge_scope.filter_chain import parse, Stage, fuzzy, compatible, output_type
from cartridge_scope.search_engine import SearchEngine
from cartridge_scope.model import Context


class Client:
    async def asp(self, op, **kwargs):
        if op == 'search':
            return {'hits': [{'node': {'id': 'memo:jev', 'name': 'Jev decisions', 'attributes': {'memo.body': 'Find the deployment failure'}}, 'score': 9}], 'sources': []}
        return {'nodes': [{'id': kwargs.get('entity', 'memo:jev'), 'name': 'Jev decisions', 'attributes': {'memo.body': 'Find the deployment failure'}}]}


class FinderTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.root = Path(self.tmp.name)
        (self.root/'src').mkdir()
        (self.root/'src'/'application.py').write_text('TODO deploy\nkeep this\nTODO retry\n')
        (self.root/'src'/'other.py').write_text('TODO unrelated\n')
        (self.root/'space > name.txt').write_text('literal > text\n')
        (self.root/'.gitignore').write_text('ignored.txt\n')
        (self.root/'ignored.txt').write_text('secret TODO\n')
        self.model = Context(); self.model.types = {'schemes': {'memo': {'owner': 'memo'}}}
        self.engine = SearchEngine(str(self.root), Client(), self.model)

    def tearDown(self): self.tmp.cleanup()

    def test_typed_stages_and_quoted_separator(self):
        self.assertEqual(parse('Files "space > name" > Grep literal'), [Stage('Files', '"space > name"'), Stage('Grep', 'literal')])
        self.assertNotIn('ASP', compatible('file'))
        with self.assertRaises(ValueError): parse('Files > ASP jev')
        self.assertEqual(fuzzy([{'id':'x','name':'cartridge_scope/app.py'}], 'cscp')[0]['id'], 'x')

    async def test_disk_chain_scopes_grep_and_then_narrows_lines(self):
        result = await self.engine.evaluate('Files apppy > Grep TODO > Fuzzy retry')
        self.assertEqual([row["content"] for row in result], ["TODO retry"])
        result = await self.engine.evaluate('Files apppy > Grep TODO > Grep retry')
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]['content'], 'TODO retry')
        self.assertEqual(result[0]['attributes']['scope.disk']['line'], 3)
        preview = await self.engine.disk.preview(result[0])
        self.assertIn('keep this', preview['attributes']['scope.preview'])
        self.assertEqual(await self.engine.evaluate('Files absent > Grep TODO'), [])

    async def test_asp_and_disk_use_the_same_pipeline(self):
        result = await self.engine.evaluate('ASP memo jev > Grep deployment')
        self.assertEqual([row['id'] for row in result], ['memo:jev'])
        result = await self.engine.evaluate('All app')
        self.assertIn('file:src/application.py', [row['id'] for row in result])
        self.assertIn('memo:jev', [row['id'] for row in result])

    async def test_scope_and_regex_flags(self):
        rows = await self.engine.evaluate('Files py')
        chosen = [row for row in rows if row['name']=='src/other.py']
        result = await self.engine.evaluate('Files py > Grep TODO', {1: chosen})
        self.assertEqual([row['content'] for row in result], ['TODO unrelated'])
        self.engine.options['regex'] = True
        with self.assertRaises(ValueError): await self.engine.evaluate('Grep [')
        with self.assertRaises(ValueError): self.engine.disk.path('../outside')

    async def test_centered_layout_tab_completion_chain_and_back(self):
        app = Scope(str(self.root), client=Client(), context=self.model, live=False)
        async with app.run_test(size=(100, 35)) as pilot:
            entry=app.query_one('#search')
            self.assertLess(app.query_one('#preview').region.bottom, app.query_one('#panes').region.y)
            self.assertLess(abs(app.query_one('#preview').size.height-app.query_one('#panes').size.height), 2)
            entry.value='Fi'
            await pilot.pause(); await pilot.press('tab'); await pilot.pause(.2)
            self.assertEqual(entry.value, 'Files ')
            entry.value='Files apppy'
            await pilot.pause(.4)
            self.assertEqual(len(app.items), 1)
            await pilot.press('tab'); await pilot.pause()
            self.assertTrue(entry.value.endswith(' > '))
            entry.value += 'Grep TODO'
            await pilot.pause(.4)
            self.assertEqual(len(app.items),2, str(app.context.errors))
            await pilot.press('shift+tab'); await pilot.pause(.3)
            self.assertEqual(entry.value,'Files apppy')
            self.assertEqual(len(app.items),1)
            app.action_preview_mode(); await pilot.pause()
            self.assertTrue(app.query_one('#waterfall-pane').display)

    async def test_backend_cancellation_reaps_child(self):
        import os, sys
        marker=self.root/'child.pid'
        code="import os,time;open("+repr(str(marker))+",'w').write(str(os.getpid()));time.sleep(60)"
        task=asyncio.create_task(self.engine.disk.run([sys.executable,'-c',code]))
        for _ in range(100):
            if marker.exists():break
            await asyncio.sleep(.01)
        self.assertTrue(marker.exists())
        pid=int(marker.read_text());task.cancel()
        with self.assertRaises(asyncio.CancelledError):await task
        with self.assertRaises(ProcessLookupError):os.kill(pid,0)

    async def test_cache_keys_include_middle_input_identity(self):
        rows=[{'id':f'file:{name}','name':name,'type':'file','rank':0,'uses':0,'summary':''} for name in ('first','middle','last','changed')]
        first=await self.engine.evaluate('Files > Fuzzy mid',{1:rows[:3]})
        second=await self.engine.evaluate('Files > Fuzzy mid',{1:[rows[0],rows[3],rows[2]]})
        self.assertEqual([row['id'] for row in first],['file:middle'])
        self.assertEqual(second,[])

    async def test_project_ignores_apply_without_parent_or_user_rg_config(self):
        import os
        from unittest.mock import patch
        nested=self.root/'project';nested.mkdir()
        (self.root/'.gitignore').write_text('*\n')
        (nested/'.gitignore').write_text('ignored.txt\n')
        (nested/'visible.txt').write_text('needle\n')
        (nested/'ignored.txt').write_text('needle\n')
        config=self.root/'rg-config';config.write_text('--glob=!visible.txt\n')
        engine=SearchEngine(str(nested),Client(),self.model)
        with patch.dict(os.environ,{'RIPGREP_CONFIG_PATH':str(config)}):
            rows=await engine.evaluate('Files txt')
            self.assertEqual([r['name'] for r in rows],['visible.txt'])
            rows=await engine.evaluate('Grep needle')
            self.assertEqual([r['content'] for r in rows],['needle'])

    async def test_agent_chain_and_refresh_use_finder_backend(self):
        from unittest.mock import Mock
        app=Scope(str(self.root),client=Client(),context=self.model,live=False)
        async with app.run_test() as pilot:
            await app.apply_agent_result({'query':'Files apppy > Grep retry','entities':None})
            await pilot.pause(.4)
            self.assertEqual([r['content'] for r in app.items],['TODO retry'])
            app.live=True
            app.bootstrap=Mock();app.poll=Mock();app.search_remote=Mock()
            app.action_refresh()
            await pilot.pause(.4)
            app.search_remote.assert_not_called()
            self.assertEqual([r['content'] for r in app.items],['TODO retry'])
            app.live=False

    async def test_tab_waits_for_current_filter_before_capturing_scope(self):
        app=Scope(str(self.root),client=Client(),context=self.model,live=False)
        async with app.run_test() as pilot:
            entry=app.query_one('#search')
            entry.value='Files apppy'
            await pilot.pause(.3)
            self.assertEqual(len(app.items),1)
            entry.value='Files otherpy'
            app.finder_forward()
            await pilot.pause(.4)
            self.assertTrue(entry.value.endswith(' > '))
            self.assertEqual([r['name'] for r in app.finder_scopes[1]],['src/other.py'])
