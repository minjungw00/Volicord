#!/usr/bin/env python3
"""Synthetic closed-context controls; actual browser proof is collected separately."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import viewer_observation as observer
import resource_observer


def manifest_fixture(candidate='a' * 40):
    return {'candidate_head': candidate, 'candidate_artifacts': {'volicord-viewer': {'sha256': 'a' * 64}},
        'journeys': {'journey-volicord': {'runtime_home': '/synthetic/runtime', 'project_id': 'a' * 32}},
        'work_evidence': [{'journey_id': 'journey-volicord', 'work_item_id': identity, 'project_id': 'a' * 32}
            for identity in ('1' * 32, '2' * 32)],
        'journey_final_evidence': [{'journey_id': 'journey-volicord',
            'artifact_inventory': {'canonical_bundle': {'sha256': '6' * 64}}}]}


def subject_fixture(manifest):
    return {'project_id': manifest['journeys']['journey-volicord']['project_id'],
        'work_ids': sorted(w['work_item_id'] for w in manifest['work_evidence']),
        'decision_ids': ['3' * 32], 'canonical_bundle_sha256': '6' * 64}


def display_fixture(manifest,locale):
    journey=manifest['journeys']['journey-volicord']
    return {'kind':'dogfood_viewer_display_capture','schema_version':1,'evidence_class':'browser_display_capture',
        'candidate_head':manifest['candidate_head'],'url':f'http://127.0.0.1:3219/?view=overview&locale={locale}&language={locale}',
        'context':{'kind':'volicord_viewer_observation_context','schema_version':1,'render_id':('1' if locale=='en' else '2')*32,
            'mode':'live','process':{'pid':123,'start_ticks':789,'boot_id':'12345678-1234-1234-1234-123456789abc',
                'executable_path':'/synthetic/volicord-viewer','executable_sha256':manifest['candidate_artifacts']['volicord-viewer']['sha256']},
            'runtime_binding':resource_observer.path_binding(Path(journey['runtime_home'])),'project_id':journey['project_id'],
            'locale':locale,'language':locale,'view':{'view':'overview'},'selected_work':None,'selected_decision':None,
            'canonical_read_fingerprint':'b'*64,'source_state_sha256':'c'*64,'source_count':2,'analysis':[],
            'explanations':[{'kind':'work','identity':'a'*32,'state':'current','plan_fingerprint':'sha256:'+'d'*64,
                'generated_at_unix_micros':123,'realization_sha256':'e'*64}]},
        'dom_sha256':'f'*64,'screenshot':{'path':'display.png','sha256':hashlib.sha256(b'synthetic screenshot fixture').hexdigest(),'bytes':28},
        'browser':{'version':'synthetic fixture','geometry':{'scrollX':0,'scrollY':0,'innerWidth':390,'innerHeight':900,'dpr':1},'zoom':None}}


def context_directory(parent,name,manifest,locale):
    path=parent/name;path.mkdir()
    value=display_fixture(manifest,locale)
    data=b'synthetic screenshot fixture';value['screenshot']['bytes']=len(data)
    (path/'display.png').write_bytes(data);(path/'display-context.json').write_text(json.dumps(value))
    return path


def prepared_context_directories(parent, name, manifest, *, campaign_root, decision=True):
    """Authored contexts for every asked block, never an actual browser capture."""
    paths = []
    subjects = observer.load_subjects(campaign_root, manifest)
    assert len(subjects['work_ids']) >= 2
    if decision: assert subjects['decision_ids']
    for locale in ("en", "ko"):
        for number, view in enumerate([{"view": "overview"},
                {"view": "work", "work": subjects['work_ids'][0]}, {"view": "work", "work": subjects['work_ids'][1]},
                {"view": "decisions", **({"decision": subjects['decision_ids'][0]} if decision else {})},
                {"view": "code"}, {"view": "tools", "tool": "status"}]):
            path = context_directory(parent, name + locale + str(number), manifest, locale)
            value = json.loads((path / "display-context.json").read_bytes())
            value["context"].update(view=view, selected_work=view.get("work"),
                selected_decision=view.get("decision"), render_id=f"{number + (1 if locale == 'en' else 10):032x}")
            value["url"] = "http://127.0.0.1:3219/?" + "&".join(f"{k}={v}" for k,v in
                {**view, "locale": locale, "language": locale}.items())
            (path / "display-context.json").write_text(json.dumps(value))
            paths.append(path)
    return paths


class ContextTests(unittest.TestCase):
    def test_same_campaign_subject_accepts_fresh_current_stale_and_changed_basis(self):
        for subject, identity in (("work", "1" * 32), ("decision", "3" * 32)):
            for n, state in enumerate(("current", "stale", "unavailable")):
                value = display_fixture(self.manifest, "en")
                view = {"view": "work" if subject == "work" else "decisions", subject: identity}
                value["context"].update(view=view, selected_work=identity if subject == "work" else None,
                    selected_decision=identity if subject == "decision" else None,
                    canonical_read_fingerprint=str(n + 4) * 64, source_state_sha256=str(n + 7) * 64)
                e = value["context"]["explanations"][0]
                e.update(kind=subject, identity=identity, state=state)
                if state != "current":
                    for key in ("plan_fingerprint", "generated_at_unix_micros", "realization_sha256"): e[key] = None
                value["url"] = "http://127.0.0.1:3219/?" + "&".join(f"{k}={v}" for k,v in
                    {**view, "locale": "en", "language": "en"}.items())
                observer.for_manifest(self.manifest, value, "en", self.subjects)

    def test_canonical_subject_inventory_excludes_unmeasured_work_decisions(self):
        from codex_events import CanonicalBundle
        project = "a" * 32
        bundle = CanonicalBundle("6" * 64, project, {
            "context_items": tuple({"id": work, "project_id": project, "role": "goal"} for work in ("1" * 32, "2" * 32)),
            "checkpoints": tuple({"work_item_id": work, "project_id": project} for work in ("1" * 32, "2" * 32)),
            "decisions": ({"id": "3" * 32, "project_id": project, "work_scope": "work_item", "work_item_id": "1" * 32},
                {"id": "4" * 32, "project_id": project, "work_scope": "project", "work_item_id": None},
                {"id": "d" * 32, "project_id": project, "work_scope": "work_item", "work_item_id": "b" * 32})})
        subjects = observer.subjects_from_bundle(project, ["1" * 32, "2" * 32], bundle)
        self.assertEqual(subjects["decision_ids"], ["3" * 32, "4" * 32])
        with self.assertRaisesRegex(ValueError, "canonical identity/history"):
            observer.subjects_from_bundle(project, ["1" * 32, "b" * 32], bundle)

    def test_campaign_foreign_well_formed_subjects_are_rejected(self):
        for view in ({"view": "work", "work": "a" * 32}, {"view": "work", "work": "b" * 32},
                {"view": "decisions", "decision": "d" * 32}):
            value = display_fixture(self.manifest, "en")
            value["context"].update(view=view, selected_work=view.get("work"), selected_decision=view.get("decision"))
            value["url"] = "http://127.0.0.1:3219/?" + "&".join(f"{k}={v}" for k,v in
                {**view, "locale": "en", "language": "en"}.items())
            with self.assertRaisesRegex(ValueError, "campaign subject"):
                observer.for_manifest(self.manifest, value, "en", self.subjects)

    def setUp(self):
        self.manifest = manifest_fixture()
        self.subjects = subject_fixture(self.manifest)
    def test_valid_states_and_independent_display_substitutions(self):
        value=display_fixture(self.manifest,'en')
        for state in ['current','stale','unavailable','unsupported','corrupt']:
            current=copy.deepcopy(value);e=current['context']['explanations'][0];e['state']=state
            if state!='current':
                for k in ['plan_fingerprint','generated_at_unix_micros','realization_sha256']: e[k]=None
            observer.for_manifest(self.manifest,current,'en',self.subjects)
        mutations=[lambda v:v.update(candidate_head='b'*40),
            lambda v:v['context']['process'].update(executable_sha256='b'*64),
            lambda v:v['context'].update(runtime_binding='b'*64),lambda v:v['context'].update(project_id='b'*32),
            lambda v:v['context'].update(locale='ko'),lambda v:v['context'].update(view={'view':'work','work':'b'*32}),
            lambda v:v['context']['explanations'][0].update(state='stale'),
            lambda v:v.update(evidence_class='http_markup_probe'),
            lambda v:v['context'].update(mode='snapshot'),lambda v:v['context']['process'].update(environment={'secret':'sentinel'})]
        for mutate in mutations:
            changed=copy.deepcopy(value);mutate(changed)
            with self.assertRaises(ValueError): observer.for_manifest(self.manifest,changed,'en',self.subjects)
    def test_both_locales_and_screenshot_bytes_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            p=Path(temporary);en=context_directory(p,'en',self.manifest,'en');ko=context_directory(p,'ko',self.manifest,'ko')
            with self.assertRaises(ValueError):observer.load_contexts([en],self.manifest,self.subjects)
            self.assertEqual(set(observer.load_contexts([en,ko],self.manifest,self.subjects)),{'en','ko'})
            with self.assertRaisesRegex(ValueError, 'cannot be reused'):observer.load_contexts([en,en,ko],self.manifest,self.subjects)
            (ko/'display.png').write_bytes(b'changed screen')
            with self.assertRaises(ValueError):observer.load_contexts([en,ko],self.manifest,self.subjects)

if __name__=='__main__':unittest.main()
