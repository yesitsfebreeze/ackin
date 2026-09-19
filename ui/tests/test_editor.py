import json
import unittest
import tempfile
from pathlib import Path
from cartridge_scope.edit_session import EditSession
from cartridge_scope.app import Scope
from cartridge_scope.model import Context


class Tools:
    project = '/project'
    def __init__(self): self.calls=[]; self.text='old\n'; self.revision='original'; self.observed=None
    async def call(self,event,args,**kwargs):
        self.calls.append((event,args))
        if event=='sessions': return {'id':'registered-editor-session'}
        if event=='fs.context':
            return {'status':'available' if args.get('expected_revision',self.revision)==self.revision else 'changed','text':self.text,'revision':self.revision}
        if event=='tool.read': self.observed=self.revision;return {'error':False,'content':'1 old'}
        if event=='tool.write':
            if self.observed!=self.revision: return {'error':True,'content':'file changed since it was read'}
            self.text=args['input']['text'];return {'error':False,'content':'wrote file'}
        if event=='tool.memo':
            if args['input']['op']=='read': return {'error':False,'content':json.dumps({'text':'---\nkind: note\n---\nOld', 'revision':'memo-revision'})}
            return {'error':False,'content':'saved'}
        raise ValueError(event)


class EditorTests(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory()
        Tools.project=self.tmp.name
        Path(self.tmp.name,'a.txt').write_text('old\n')
    def tearDown(self): self.tmp.cleanup()

    async def test_file_save_uses_registered_context_read_guard_and_write_event(self):
        client=Tools();edit=EditSession(client)
        self.assertEqual(await edit.open({'id':'file:a.txt'}),'old\n')
        await edit.save('new\n')
        self.assertEqual(client.text,'new\n')
        calls=[(event,args) for event,args in client.calls if event.startswith('tool.')]
        self.assertEqual([event for event,_ in calls],['tool.read','tool.write'])
        self.assertEqual(calls[-1][1]['context']['session'],'registered-editor-session')
        self.assertNotEqual(calls[0][1]['context']['call'],calls[-1][1]['context']['call'])

    async def test_conflict_does_not_overwrite_changed_file(self):
        client=Tools();edit=EditSession(client);await edit.open({'id':'file:a.txt'})
        client.revision='someone-else';client.text='theirs\n'
        with self.assertRaisesRegex(ValueError,'changed'):await edit.save('mine\n')
        self.assertEqual(client.text,'theirs\n')

    async def test_plugin_write_carries_revision_and_original_document(self):
        client=Tools();edit=EditSession(client)
        descriptor={'tool':'memo','read':{'op':'read','path':'note/a.md'},'write':{'op':'write','path':'note/a.md'}}
        text=await edit.open({'id':'memo:note/a.md'},descriptor)
        await edit.save(text.replace('Old','New'))
        args=client.calls[-1][1]['input']
        self.assertEqual(args['expected_revision'],'memo-revision')
        self.assertIn('kind: note',args['body'])

    async def test_unknown_type_edits_become_explicit_owner_change_request(self):
        edit=EditSession(Tools());text=await edit.open({'id':'plan:p','name':'Old plan','attributes':{}})
        response=await edit.save(text.replace('Old plan','New plan'))
        self.assertIn('New plan',response['request']);self.assertIn('Old plan',response['request'])

    async def test_editor_is_above_center_and_save_returns_to_results(self):
        client=Tools();model=Context();model.merge({'nodes':[{'id':'file:a.txt','name':'a.txt'}]})
        app=Scope(client.project,client=client,context=model,live=False)
        async with app.run_test(size=(90,32)) as pilot:
            await pilot.press('ctrl+e');await pilot.pause(.2)
            self.assertTrue(app.editing)
            editor=app.query_one('#editor');self.assertLess(editor.region.y,app.query_one('#search').region.y)
            editor.load_text('new\n')
            await pilot.press('ctrl+s');await pilot.pause(.2)
            self.assertFalse(app.editing);self.assertEqual(client.text,'new\n')
