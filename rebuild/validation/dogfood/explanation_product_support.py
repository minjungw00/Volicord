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
        def complete(prepared, *, observed_source=None):
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
                response = {'format_kind': 'volicord_explanation', 'format_version': 10, 'diagrams': [],
                    'plan_fingerprint': plan['fingerprint'], 'language': p['language'],
                    'generator': {'host': 'structural-support', 'session': 'self-authored', 'agent': None, 'model': None},
                    'paragraphs': [{'question': q, 'text': '구조 검증 예제.' if p['language'] == 'ko' else 'Structural support example.',
                        'citations': [], 'evidence_keys': [work_keys[q] if p['subject']['kind'] == 'work' else q]} for q in sorted(
                            e.WORK_QUESTIONS if p['subject']['kind'] == 'work' else e.DECISION_QUESTIONS)]}
                if observed_source is not None and p['subject']['kind'] == 'work':
                    draft = {k: response[k] for k in ('plan_fingerprint', 'language', 'generator', 'diagrams')}
                    draft['paragraphs'] = [{k: v for k, v in unit.items() if k != 'citations'} | {'sources': []}
                                           for unit in response['paragraphs']]
                    text = ('포함된 TypeScript 파일을 현재 코드로 관측했습니다. 이 관측은 실행 결과를 증명하지 않습니다.'
                            if p['language'] == 'ko' else
                            'The included TypeScript file was observed as current code; this does not prove execution.')
                    selection = {'source_id': observed_source['source_id'], 'field': 'body',
                                 'use_kind': 'current_code', 'start_byte': 0,
                                 'end_byte': observed_source['bytes'], 'claim': None}
                    draft['paragraphs'][0:0] = [
                        {'question': 'heading', 'text': ('관측한 TypeScript 코드 읽기' if p['language'] == 'ko'
                            else 'Read observed TypeScript code'), 'evidence_keys': ['goal'], 'sources': [selection]},
                        {'question': 'synopsis', 'text': text, 'evidence_keys': ['goal'], 'sources': [selection]}]
                    draft['paragraphs'].append({'question': 'observation', 'text': text, 'evidence_keys': ['goal'],
                        'sources': [{'source_id': observed_source['source_id'], 'field': 'body',
                                     'use_kind': 'current_code', 'start_byte': 0,
                                     'end_byte': observed_source['bytes'], 'claim': None}]})
                    draft_path = root / ('cited-draft-' + p['language'] + '.json')
                    draft_path.write_bytes(c.json_bytes(draft))
                    composed, _ = e.invoke(binary, runtime, project, ['work', 'explain', 'compose',
                        '--work', work, '--input', str(draft_path)])
                    response = composed['realization']
                    assert response['citation_contexts'] and response['citation_sources']
                    assert len(response['paragraphs'][-1]['citations'][0]) == 9
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
            # Actual current cited CLI compose/record/read -> measured/steward
            # typed projection. No model, naturalistic claim or wider Source scope.
            analysis, _ = e.invoke(binary, runtime, project, ['analyze'])
            observed, _ = e.invoke(binary, runtime, project, ['work', 'source', 'observe',
                '--work', work, '--analysis', analysis['analysis_snapshot'],
                '--path', 'runtime/query_boundary.ts', '--retain-seconds', '3600'])
            source_read, _ = e.invoke(binary, runtime, project, ['work', 'source', 'read',
                '--work', work, '--source', observed['source_id']])
            original_file = (repository / 'runtime/query_boundary.ts').read_bytes()
            assert source_read['observation']['detail']['body'].encode() == original_file
            cited = prepare()
            complete(cited, observed_source={'source_id': observed['source_id'], 'bytes': len(original_file)})
            e.require_ready(root, campaign, mapped)
            for item in cited['explanations']:
                if item['subject']['kind'] == 'work':
                    directory = e.entry_path(root, item['identity'])
                    data, metadata = review.project(root, directory / 'preparation.json', evidence_set_sha256='b' * 64)
                    projected = review.validate(data)
                    assert metadata['semantic_complete'], {'before_state': projected['before_state'], 'stages': {name: {'state': body['state'], 'reason': body['reason'], 'bytes': body['source_body_bytes'], 'omissions': body.get('value', {}).get('omissions', [])[:6]} for name, body in projected['stages'].items()}}
                    actual = json.loads(e.bound(root, directory / 'response.json'))
                    assert projected['stages']['response']['value'] == actual
                    assert projected['stages']['record']['value']['semantic_complete']
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
