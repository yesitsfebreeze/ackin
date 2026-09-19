import tempfile
import unittest
from pathlib import Path
from textual.widgets import Input, OptionList
from cartridge_scope.app import Scope
from cartridge_scope.leader import LeaderMenu, choices
from cartridge_scope.model import Context


class LeaderTests(unittest.IsolatedAsyncioTestCase):
    async def test_ff_is_discoverable_and_command_keys_do_not_enter_search(self):
        with tempfile.TemporaryDirectory() as directory:
            Path(directory,'example.txt').write_text('hello\n')
            app=Scope(directory,context=Context(),live=False)
            async with app.run_test(size=(80,24)) as pilot:
                entry=app.query_one('#search',Input)
                entry.value='memo jev'
                await pilot.press('ctrl+space')
                self.assertIsInstance(app.screen,LeaderMenu)
                self.assertIn(('f','Find …'),choices(''))
                await pilot.press('f')
                self.assertIn(('ff','Find files'),choices(app.screen.query_one(Input).value))
                self.assertEqual(entry.value,'memo jev')
                await pilot.press('f');await pilot.pause(.3)
                self.assertNotIsInstance(app.screen,LeaderMenu)
                self.assertEqual(entry.value,'Files ')
                self.assertEqual(app.items[0]['name'],'example.txt')
                self.assertEqual(app.focused.id,'search')

    async def test_cancel_backspace_and_name_search(self):
        app=Scope('.',context=Context(),live=False)
        async with app.run_test(size=(40,12)) as pilot:
            entry=app.query_one('#search',Input);entry.value='memo jev'
            await pilot.press('ctrl+space','f','backspace')
            self.assertEqual(app.screen.query_one(Input).value,'')
            await pilot.press('escape')
            self.assertEqual(entry.value,'memo jev')
            self.assertEqual(app.focused.id,'search')
            await pilot.press('ctrl+space')
            menu=app.screen
            menu.query_one(Input).value='waterfall'
            await pilot.pause()
            self.assertEqual(menu.query_one(OptionList).option_count,1)
            await pilot.press('enter');await pilot.pause()
            self.assertTrue(app.waterfall_preview)
            await pilot.press('ctrl+space','ctrl+space')
            self.assertNotIsInstance(app.screen,LeaderMenu)
            await pilot.press('ctrl+@')
            self.assertIsInstance(app.screen,LeaderMenu)
            await pilot.press('escape')

    async def test_agent_and_options_share_one_leader(self):
        app=Scope('.',context=Context(),live=False)
        async with app.run_test() as pilot:
            await pilot.press('ctrl+space','a','a')
            self.assertTrue(app.agent_mode)
            await pilot.press('ctrl+space','a','l')
            self.assertFalse(app.agent_mode)
            await pilot.press('ctrl+space','s','r')
            self.assertTrue(app.engine.options['regex'])
            await pilot.press('ctrl+space','s','r')
            self.assertFalse(app.engine.options['regex'])
