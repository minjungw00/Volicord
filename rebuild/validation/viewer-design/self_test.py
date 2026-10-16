#!/usr/bin/env python3
"""Input rejection, content parity and package closure; independent authored controls."""
import argparse
import copy
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('design',HERE/'build.py');design=importlib.util.module_from_spec(spec);spec.loader.exec_module(design)


class ReviewTests(unittest.TestCase):
    def setUp(self):
        self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name)
        for name in ['review/identities.json','review-input-final.json','maintained-review/fixtures.json','browser-final/b-status.stdout','browser-final/click-status.stdout','browser-final/b-ko.html','browser-final/click-ko.html']:
            dest=self.root/name;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(ARGS.retained/name,dest)

    def change_readback(self,fn):
        p=self.root/'browser-final/b-status.stdout';d=json.loads(p.read_text());fn(d);p.write_text(json.dumps(d,ensure_ascii=False))
        p=self.root/'review/identities.json';d=json.loads(p.read_text());d['files']['browser-final/b-status.stdout']=design.sha((self.root/'browser-final/b-status.stdout').read_bytes());p.write_text(json.dumps(d))

    def test_positive_originals(self):
        _,_,works=design.admit(self.root);self.assertEqual(len(works),2)

    def test_wrong_work(self):
        self.change_readback(lambda d:d['work_history'][0].update(work_item_id='wrong-work'))
        with self.assertRaisesRegex(ValueError,'wrong Work'):design.admit(self.root)

    def test_unavailable_source(self):
        self.change_readback(lambda d:d['work_history'][0]['answers']['source_excerpts'].clear())
        with self.assertRaisesRegex(ValueError,'Source unavailable'):design.admit(self.root)

    def test_changed_range(self):
        self.change_readback(lambda d:d['work_history'][0]['answers']['source_excerpts'][0]['citation'].update(start_byte=0))
        with self.assertRaisesRegex(ValueError,'range mismatch'):design.admit(self.root)

    def test_wrong_claim_site(self):
        self.change_readback(lambda d:d['work_history'][0]['answers']['prose'][5]['citations'].clear())
        with self.assertRaisesRegex(ValueError,'claim-to-Source'):design.admit(self.root)

    def test_claim_hash(self):
        self.change_readback(lambda d:d['work_history'][0]['answers']['prose'][5]['citations'][0].update(claim_sha256='sha256:wrong'))
        with self.assertRaisesRegex(ValueError,'claim hash'):design.admit(self.root)

    def test_changed_fixture(self):
        p=self.root/'maintained-review/fixtures.json';d=json.loads(p.read_text());d['cases'][0]['expected_units'][0]['text']='a prototype-only answer';p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'identity mismatch'):design.admit(self.root)

    def test_no_inferred_before_state(self):
        self.change_readback(lambda d:d['work_history'][0]['answers']['source_excerpts'][0].update(comparison='changed'))
        with self.assertRaisesRegex(ValueError,'missing-before'):design.admit(self.root)

    def test_source_first_outcomes_reject_deleted_adverse_fact(self):
        _,_,works=design.admit(self.root)
        self.assertEqual(len(design.journeys.source_check(works)),7)
        # Rehashing a candidate cannot make a removed source fact true.
        works[0]['sources']['report']['text']='All checks passed.'
        with self.assertRaisesRegex(ValueError,'source-first outcome missing'):
            design.journeys.source_check(works)

    def test_missing_case_cannot_acquire_generated_success(self):
        if not ARGS.missing_case:self.skipTest('no independent missing case supplied')
        root=self.root/'missing';root.mkdir()
        for name in ['case.json','canonical.json','source.py','status.json','baseline.html']:
            shutil.copyfile(ARGS.missing_case/name,root/name)
        design.journeys.missing_admit(root)
        p=root/'status.json';d=design.load(p)
        d['work_history'][0]['answers']['explanation_state']='current'
        p.write_text(json.dumps(d));p=root/'case.json';d=design.load(p)
        d['files']['status.json']=design.sha((root/'status.json').read_bytes());p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'absent explanation'):
            design.journeys.missing_admit(root)

    def test_invalid_json(self):
        (self.root/'review-input-final.json').write_text('{ invalid')
        with self.assertRaises(ValueError):design.admit(self.root)

    def test_withheld_and_omitted(self):
        self.change_readback(lambda d:d['work_history'][0]['answers'].update(explanation_state='withheld',omitted_source_excerpt_count=1))
        with self.assertRaisesRegex(ValueError,'unavailable/incomplete'):design.admit(self.root)

    def test_hostile_and_missing_content(self):
        _,_,works=design.admit(self.root);w=copy.deepcopy(works[0]);attack='</script><img src=https://example.invalid onerror="window.__attack=1">'
        long=attack+(' 긴 설명 mixed-language λ漢字 아주긴경로_without_breaks' * 100)
        for u in w['authored']['units']:u['text']=long
        for s in w['sources'].values():s['text']=attack+'\n'
        w['authored']['goal']=long;w['selected']['attack']=long
        page=design.work_page(w,False)
        self.assertNotIn('<img',page);self.assertNotIn('<script',page);self.assertIn('&lt;img',page)
        if ARGS.stress:ARGS.stress.write_text(page)
        w['authored']['units']=[u for u in w['authored']['units'] if u['role'] not in {'heading','synopsis','comparison'}]
        missing=design.work_page(w,False,True)
        self.assertIn('<h1>기록된 작업</h1>',missing);self.assertIn('Source unavailable',missing);self.assertNotIn('class="comparisons"',missing)

    def catalog_path(self):
        if not ARGS.build:self.skipTest('no catalog package supplied')
        p=self.root/'catalog.json';shutil.copyfile(ARGS.build/'inputs/catalog.json',p);return p

    def test_catalog_canonical_binding(self):
        p=self.catalog_path();catalog=design.catalog_input.admit(p)
        self.assertEqual(len(catalog['items']),3)
        self.assertTrue(all(i['checkpoint']['work_state']=='completed' for i in catalog['items']))
        d=json.loads(p.read_text());d['works'][0]['checkpoint']=d['works'][1]['checkpoint'];p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'latest retained'):design.catalog_input.admit(p)

    def test_catalog_rejects_wrong_work_or_goal(self):
        p=self.catalog_path();d=json.loads(p.read_text());d['works'][0]['work']='wrong-work';p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'wrong Work'):design.catalog_input.admit(p)
        p=self.catalog_path();d=json.loads(p.read_text());d['purpose']['text']='a prototype-only purpose';p.write_text(json.dumps(d))
        with self.assertRaisesRegex(ValueError,'exact independent'):design.catalog_input.admit(p)

    def test_card_dates_do_not_invent_missing_history(self):
        catalog=design.catalog_input.admit(self.catalog_path())
        item=catalog['items'][0];item['goal']['revision']=2
        item['checkpoint']['work_state']='in_progress'
        page=design.catalog_rows(dict(items=[item]))
        self.assertNotIn('<time ',page)
        self.assertEqual(page.count('기록 없음'),2)
        self.assertNotIn('<h3><a',page)
        self.assertEqual(page.count('<a '),1)

    def test_highlighting_preserves_multiline_source(self):
        for locator,text in [('a.rs','/* first\nsecond */\nlet s = "<script>";\n'),('a.py','def read():\n    return "<img src=x>" # comment\n')]:
            rendered=design.syntax.lines(text,locator)
            self.assertEqual([''.join(design.Text(line).parts) for line in rendered],text.rstrip('\n').split('\n'))
            self.assertNotIn('<script>', ''.join(rendered));self.assertNotIn('<img ', ''.join(rendered))
            self.assertIn('syntax-keyword',''.join(rendered))

    def test_package_links_and_hashes(self):
        if not ARGS.build:self.skipTest('no package supplied')
        root=ARGS.build.resolve();manifest=design.load(root/'manifest.json')
        for name,d in manifest['files'].items():self.assertEqual(design.sha((root/name).read_bytes()),d['sha256'],name)
        class Links(design.HTMLParser):
            def handle_starttag(inner,tag,attrs):
                for key,value in attrs:
                    if key not in {'href','src'} or not value or value.startswith('#'):continue
                    self.assertNotIn('://',value,'remote/external link')
                    self.assertTrue((root/value.split('#')[0]).is_file(),value)
        for p in root.glob('*.html'):Links().feed(p.read_text())
        self.assertFalse(any(p.suffix=='.html' for p in (root/'inputs').rglob('*')), 'executable historical prototype must not be delivered')


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--retained',type=Path,required=True);p.add_argument('--build',type=Path);p.add_argument('--stress',type=Path);p.add_argument('--missing-case',type=Path);ARGS=p.parse_args()
    unittest.main(argv=['self_test.py'],verbosity=2)
