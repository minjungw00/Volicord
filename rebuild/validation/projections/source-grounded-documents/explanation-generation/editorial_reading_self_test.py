"""Independent extractive editorial/offset/Source/feedback controls, no prose verdict."""
import copy
import json
import os
from pathlib import Path
import unittest

import inputs as i
import presentation_reading_self_test as presentation_controls
from editorial_reading import create, read, transform
from reader_presentation import paragraphs
from reading_input import verify_reading_input
from render_comparison import card, record_feedback, render
from render_readings import compare


class EditorialTests(unittest.TestCase):
    def setUp(self):
        control = presentation_controls.PresentationTests('runTest'); control.setUp()
        self.addCleanup(control.doCleanups)
        self.control, self.h = control, control.h
        self.response = control.fixture.response
        self.parts = ['이전 값은 1이다.\n\n', '새 값은 2이다.\n\n', '두 반환 문장을 비교한다.\n\n', '코드 확인 범위는 두 표현이다.']
        self.response['prose'] = ''.join(self.parts)
        self.response['gaps'] = ['Authored control: only the selected return expressions are examined.']
        start = 0; self.response['claims'] = []
        for n, text in enumerate(self.parts):
            self.response['claims'].append({'start':start, 'end':start+len(text.rstrip().encode()),
                'kind':'source_fact', 'selections':[0] if n==0 else [1] if n==1 else [0,1], 'authority':None})
            start += len(text.encode())
        self.response['primary_sites'][0]['claims'] = [0,1,2,3]
        control.fixture.sync()
        self.order = [0,2,1,3]
        self.reason = 'Authored sequence control: compare paired return statements before describing the new value.'

    def comparison(self, name='comparison'):
        root = self.h.root / name
        compare(self.h.attempt, root, editorial_order=self.order, editor='Codex current repository session', reason=self.reason)
        return root, root / 'editorial-reading/editorial.json', root / 'editorial/comparison.html'

    def test_extractively_reordered_claims_have_new_utf8_spans_and_original_sources(self):
        raw = self.h.response_path.read_bytes()
        root, path, display = self.comparison()
        revised, receipt = read(self.h.attempt, path)
        expected = self.parts[0]+self.parts[2]+self.parts[1]+self.parts[3]
        self.assertEqual(revised['prose'],expected)
        self.assertNotEqual(revised['prose'],self.response['prose'])
        self.control.assert_original(root/'presentation/comparison.html',self.response['prose'])
        self.control.assert_original(display,expected)
        self.assertEqual(revised['selections'],self.response['selections'])
        self.assertEqual(revised['primary_sites'],self.response['primary_sites'])
        self.assertEqual(revised['gaps'],self.response['gaps'])
        self.assertEqual(revised['claims'][1]['start'],len((self.parts[0]+self.parts[2]).encode()))
        self.assertEqual(revised['claims'][2]['start'],len(self.parts[0].encode()))
        for before, after in zip(self.response['claims'],revised['claims']):
            self.assertEqual(self.response['prose'].encode()[before['start']:before['end']],expected.encode()[after['start']:after['end']])
        _, witness = card(self.h.attempt,'Sample 01',presentation='reader',editorial=path)
        self.assertEqual(witness['outputs'][0]['derived_reading']['status'],'valid_binding')
        self.assertEqual(witness['presentation']['mode'],'manual_editorial')
        self.assertEqual(witness['outputs'][0]['original_prose_sha256'],i.digest(self.response['prose'].encode()))
        self.assertTrue(all('-reading-'+receipt['prose']['sha256'] in s['anchor'] for s in witness['outputs'][0]['selections']))
        feedback = record_feedback(display,'Agent structural comparison only.',root/'editorial-feedback.json',reviewer_kind='agent')
        value=json.loads(feedback.read_bytes()); self.assertEqual(value['H1'],'pending')
        self.assertEqual(value['original_outputs'],self.h.record['original_outputs'])
        self.assertEqual(value['editorial'],i.binding(path))
        self.assertEqual(self.h.response_path.read_bytes(),raw)

    def test_rehashed_unsupported_prose_stale_offsets_source_substitution_and_omission_rejected(self):
        root, path, display = self.comparison()
        receipt=json.loads(path.read_bytes()); response_path=Path(receipt['response']['path']); prose_path=Path(receipt['prose']['path'])
        original_receipt=path.read_bytes(); original_response=response_path.read_bytes(); original_prose=prose_path.read_bytes()
        for case in ['unsupported-claim','stale-offset','source-substitution','omitted-gap','changed-cutoff','changed-revision','model-editor','missing-paragraph']:
            value=copy.deepcopy(receipt); response=json.loads(original_response)
            if case=='unsupported-claim': response['prose']+=' All tests passed and the user accepted.'
            elif case=='stale-offset': response['claims']=copy.deepcopy(self.response['claims'])
            elif case=='source-substitution': response['claims'][1]['selections']=[0]
            elif case=='omitted-gap': response['gaps']=[]
            elif case=='changed-cutoff': value['scope']['cutoff']='2027-01-01T00:00:00Z'
            elif case=='changed-revision': value['revised_claims'][0]['authority']={'revision':999}
            elif case=='model-editor': value['editor']['authority']='autonomous model generation'
            elif case=='missing-paragraph': value['paragraph_order']=[0,2,3]
            response_path.write_bytes(i.encoded(response)); prose_path.write_bytes(response['prose'].encode())
            value['response']=i.binding(response_path); value['prose']=i.binding(prose_path); path.write_bytes(i.encoded(value))
            with self.subTest(case=case), self.assertRaises(ValueError): read(self.h.attempt,path)
            response_path.write_bytes(original_response); prose_path.write_bytes(original_prose); path.write_bytes(original_receipt)
        self.assertEqual(self.h.response_path.read_bytes(),i.encoded(self.response))
        # Rehashed display tampering and an older display both fail feedback.
        integrity_path=display.parent/'integrity.json'; integrity=json.loads(integrity_path.read_bytes()); saved=integrity_path.read_bytes()
        display.write_bytes(display.read_bytes().replace('두 반환'.encode(),'거짓 주장'.encode(),1))
        integrity['presentation']=i.binding(display); integrity_path.write_bytes(i.encoded(integrity))
        with self.assertRaisesRegex(ValueError,'displayed content changed'):
            record_feedback(display,None,root/'tampered-feedback.json')
        older=root/'presentation/comparison.html'; integrity=json.loads(saved); integrity['presentation']=i.binding(older)
        (older.parent/'integrity.json').write_bytes(i.encoded(integrity))
        with self.assertRaises(ValueError): record_feedback(older,None,root/'older-view-feedback.json')

    def test_invalid_transform_basis_and_cross_paragraph_claims_are_not_editorial_admission(self):
        root=self.h.root/'basis'; compare(self.h.attempt,root)
        display=root/'presentation/comparison.html'
        display.write_bytes(display.read_bytes()+b'changed display')
        with self.assertRaises(ValueError): create(self.h.attempt,self.order,'editor',display,self.reason,self.h.root/'rejected')
        for order in [[0,1,2,3],[0,2,2,3],[0,2,3],[1,2,0,3],[0,2,3,1]]:
            with self.assertRaises(ValueError): transform(self.response,order)
        response=copy.deepcopy(self.response); response['claims'][0]['end']=len((self.parts[0]+self.parts[1]).encode())
        with self.assertRaisesRegex(ValueError,'crosses paragraph'): transform(response,self.order)

    def test_editorial_view_through_actual_browser(self):
        _, path, display=self.comparison()
        revised,_=read(self.h.attempt,path)
        sample=self.control.sample(display,revised,'authored-editorial')
        sample['title']='이전 값은 1이다.'
        self.control.browser([sample])

    @unittest.skipUnless(os.environ.get('EXPLANATION_RETAINED_WORK_B'), 'explicit retained Work B required')
    def test_actual_work_b_editorial_keeps_all_original_facts_sites_limits_and_feedback(self):
        attempt=Path(os.environ['EXPLANATION_RETAINED_WORK_B']); original_attempt=attempt.read_bytes()
        record=json.loads(original_attempt)
        self.assertEqual(i.binding(attempt)['sha256'],'5984df1535f91c326b0f7f3285490ab60143e0907d806fff690b28f5eeae35e0')
        self.assertEqual(record['generation_output']['sha256'],'1d25357823a4407d5600e5414ca7d48ba327d388fa684b1f8429619f8e2f3522')
        raw=Path(record['generation_output']['path']).read_bytes(); original=json.loads(raw)
        parts=original['prose'].split('\n\n'); self.assertEqual(len(parts),8)
        self.assertTrue(parts[1].startswith('사용자는')); self.assertTrue(parts[2].startswith('구현은'))
        order=[0,2,3,1,4,5,6,7]
        root=Path(os.environ.get('EXPLANATION_EDITORIAL_OUTPUT',self.h.root))/'work-b'
        compare(attempt,root,editorial_order=order,editor='Codex current repository session',
            reason='The unchanged presentation places choice before mechanism. Read mechanism and action guidance together before choice; keep choice before its compact coverage details, then checks, limits and direction.')
        path=root/'editorial-reading/editorial.json'; display=root/'editorial/comparison.html'
        revised,receipt=read(attempt,path)
        self.assertEqual(revised['prose'],'\n\n'.join(parts[n] for n in order))
        self.assertEqual(revised['gaps'],original['gaps'])
        self.assertEqual(revised['primary_sites'],original['primary_sites'])
        self.assertEqual(revised['selections'],original['selections'])
        self.assertEqual(revised['prose'].encode().__len__(),4432)
        for old,new in zip(original['claims'],revised['claims']):
            self.assertEqual(original['prose'].encode()[old['start']:old['end']],revised['prose'].encode()[new['start']:new['end']])
            self.assertEqual(old['selections'],new['selections']);self.assertEqual(old['authority'],new['authority'])
        self.assertNotEqual(original['claims'][5]['start'],revised['claims'][5]['start'])
        self.control.assert_original(root/'presentation/comparison.html',original['prose'])
        self.control.assert_original(display,revised['prose'])
        feedback=record_feedback(display,None,root/'pending-editorial-feedback.json')
        self.assertEqual(json.loads(feedback.read_bytes())['H1'],'pending')
        spec=verify_reading_input(record['input']['path']);entries={e['id']:e for e in spec['entries']}
        codes=[]
        for index,start,end in [(51,1274,1284),(52,1274,1286),(53,1341,1378),(54,1378,1411),(55,1423,1440),(58,175,176),(59,175,176),(56,1470,1500),(57,1500,1523)]:
            selection=original['selections'][index];entry=entries[selection['id']]
            data=Path(entry['asset']['path']).read_bytes();i.check_binding(entry['asset'])
            self.assertEqual(data[:selection['start']].count(b'\n')+1,start);self.assertEqual(data[:selection['end']].count(b'\n')+1,end)
            selected=data[selection['start']:selection['end']];self.assertEqual(i.digest(selected),selection['sha256'])
            codes.append({'label':f"{entry['path']} · {selection['state']} · {start}:1–{end}:1",'text':selected.decode()})
        self.control.browser([{'name':'work-b-editorial','display':str(display),'prose':revised['prose'],
            'title':parts[0].split('. ')[0]+'.','purposes':[s['reason'] for s in original['primary_sites']],
            'codes':codes,'diffCount':2,'language':'ko'}])
        self.assertEqual(attempt.read_bytes(),original_attempt)
        self.assertEqual(Path(record['generation_output']['path']).read_bytes(),raw)


if __name__=='__main__':
    unittest.main()
