"""Disposable real-Product temporal lifecycle support, launched by Rust fixture.

Only campaign admission and synthetic raw mapping use maintained seams. Product
export/prepare/record/correct/readback and executable hash guards run unchanged.
Authored payloads prove structural linkage, never active-host prose quality.
"""
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
from unittest.mock import patch

import campaign as c
import explanation_evidence as e
import review_explanations as review


def run(config):
    binary, runtime, repository = (Path(config[k]) for k in ('binary', 'runtime', 'repository'))
    project, work, decision = (config[k] for k in ('project', 'work', 'decision'))
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        raw = root / 'synthetic-capture.jsonl'; raw.write_text('Labeled support capture seam; no measured host activity.\n')
        capture = SimpleNamespace(source_sha256=c.harness.sha256(raw), session_id='support-only',
            successful_calls=lambda operation: [SimpleNamespace(operation='project_resolve',
                arguments={'repository': str(repository)}, result={'project_id': project})]
                if operation == 'project_resolve' else [], commands=[], execution_wrappers=(),
            transport_issues=lambda *operations: [])
        mapped = {('volicord', 'A', 'start'): SimpleNamespace(source=raw, capture=capture)}
        artifact = {'path': str(binary), 'sha256': c.harness.sha256(binary)}
        campaign = {'evidence_purpose': 'dogfood_rehearsal', 'candidate_head': config['candidate_head'],
            'candidate_binary': str(binary), 'candidate_artifacts': {name: artifact for name in c.CANDIDATE_ARTIFACTS},
            'document_language': 'en', 'journeys': {'journey-volicord': {'repository_class': 'volicord',
                'runtime_home': str(runtime), 'repository_path': str(repository)}}}
        c.write_json(c.inventory_path(root), c.load_inventory(root))
        def prepare():
            return e.prepare(root, [raw], languages=['en', 'ko'], work_ids=[work], decision_ids=[decision])
        def complete(prepared):
            for item in prepared['explanations']:
                p = json.loads(e.bound(root, e.entry_path(root, item['identity']) / 'preparation.json'))
                plan = p['plan']
                offered = {v['key'] for v in plan['evidence']}
                # Cite the actual question role; uncited preparation remains retained.
                # Repeating the entire history in every paragraph can exceed the
                # response budget without testing any additional lifecycle property.
                work_keys = {'purpose': 'goal', 'reported_change': 'result' if 'result' in offered else 'goal',
                    'expected_effect': 'result' if 'result' in offered else 'goal',
                    'verification': 'verification' if 'verification' in offered else 'goal', 'next_step': 'next_step'}
                response = {'format_kind': 'volicord_explanation', 'format_version': 1,
                    'plan_fingerprint': plan['fingerprint'], 'language': p['language'],
                    'generator': {'host': 'structural-support', 'session': 'self-authored', 'agent': None, 'model': None},
                    'paragraphs': [{'question': q, 'text': '구조 검증 예제.' if p['language'] == 'ko' else 'Structural support example.',
                        'evidence_keys': [work_keys[q] if p['subject']['kind'] == 'work' else q]} for q in sorted(
                            e.WORK_QUESTIONS if p['subject']['kind'] == 'work' else e.DECISION_QUESTIONS)]}
                response_path = root / 'host-response.json'; response_path.write_bytes(c.json_bytes(response))
                e.record(root, p['identity'], response_path)
        with patch.object(c, 'load_campaign_for_mutation', return_value=campaign), \
                patch.object(c, 'verify_frozen_campaign'), patch.object(c, 'map_batch_rollouts', return_value=mapped):
            first = prepare(); complete(first)
            e.require_ready(root, campaign, mapped)
            originals = {p: p.read_bytes() for p in (root / 'explanations').rglob('*') if p.is_file()}
            # Actual user authorization Source is independent of the Goal's supporting Sources.
            authorization, _ = e.invoke(binary, runtime, project,
                ['advanced', 'records', 'source', '--host', 'support', '--session', 'correction', '--text', 'Correct punctuation only.'])
            exported = root / 'basis.json'; c.default_export(binary, runtime, repository, exported)
            basis = c.harness.load_canonical_bundle(exported)
            goal = basis.one('context_items', project_id=project, id=work)
            rationale = basis.one('decisions', project_id=project, id=decision)['user_rationale']
            for kind, identity, text in [('context', work, goal['statement']), ('decision', decision, rationale)]:
                corrected, _ = e.invoke(binary, runtime, project, ['advanced', 'records', 'correct-' + kind, identity, '--revision', '1',
                    '--source', authorization['identity'], '--text', text + '.'])
                assert corrected['revision'] == 2, corrected
            c.default_export(binary, runtime, repository, exported)
            after = c.harness.load_canonical_bundle(exported)
            associations = lambda bundle: [r for r in bundle.rows('context_item_sources') if r['context_item_id'] == work]
            assert associations(basis) == associations(after)
            assert authorization['identity'] not in [r['source_id'] for r in associations(after)]
            try:
                e.require_ready(root, campaign, mapped)
            except c.CampaignError as error:
                assert 'basis changed' in str(error), error
            else:
                raise AssertionError('stale selected observation accepted')
            second = prepare(); complete(second)
            e.require_ready(root, campaign, mapped)
            assert all(p.read_bytes() == data for p, data in originals.items())
            index = e.collection_index(root, mapped)
            review.validate_publication_index(index)
            assert len(index['steward_lifecycles']) == 8
            assert sum(i['publication_role'] == 'final' for i in index['steward_lifecycles']) == 4
            for item in index['steward_lifecycles']:
                data, _ = review.project(root, root / item['preparation'], evidence_set_sha256='b' * 64)
                review.validate(data)  # No original Runtime required by copied validation.
            # Same current plan with another realization cannot masquerade as
            # the collector's selected final receipt/readback.
            selected = next(i for i in second['explanations'] if i['subject']['kind'] == 'work' and i['language'] == 'en')
            directory = e.entry_path(root, selected['identity'])
            payload = json.loads(e.bound(root, directory / 'response.json'))
            payload['paragraphs'][0]['text'] = 'Another independently authored structural payload.'
            changed = root / 'changed-response.json'; changed.write_bytes(c.json_bytes(payload))
            e.invoke(binary, runtime, project, ['work', 'explain', 'record', '--work', work,
                '--language', 'en', '--input', str(changed)])
            try:
                e.require_ready(root, campaign, mapped)
            except c.CampaignError as error:
                assert 'readback' in str(error), error
            else:
                raise AssertionError('same-plan foreign realization accepted as selected final evidence')
            repeat = prepare(); complete(repeat)  # No basis change, still causal final selection.
            e.require_ready(root, campaign, mapped)
            pending = prepare()
            try:
                e.require_ready(root, campaign, mapped)
            except c.CampaignError as error:
                assert 'missing host response' in str(error), error
            else:
                raise AssertionError('later incomplete obligation fell back to older success')
            complete(pending)
            e.require_ready(root, campaign, mapped)
            (root / 'realization-bindings.json').write_text('{}')
            try:
                prepare()
            except c.CampaignError as error:
                assert 'before document realization' in str(error), error
            else:
                raise AssertionError('document freeze reopened')
        print('actual Product Work/Decision en/ko: corrected history, repeated basis, pending obligation, immutable bytes, copied review and freeze passed')


if __name__ == '__main__':
    run(json.loads(sys.argv[1]))
