"""Actual Product returns/correction through the Naturalistic consumer.

Authoring envelopes/topology are synthetic fixture support. Rust supplies exact
MCP and CLI returns, a real correction receipt and a freshly exported Product
bundle. This proves shared-answer verification, not a measured campaign verdict.
"""
import copy
import json
from pathlib import Path
import sys

from answer_observations_self_test import AnswerTests
import harness as h
import machine_findings as machine
import answer_projection
import rehearsal_support
import explanation_evidence


def check_private_resume_projection(answer):
    """Actual Rust DTO, including field expansion locators, through review custody."""
    projected = answer_projection.project(answer, 'recall')
    answer_projection.validate(projected)
    assert projected['semantic_complete'] and not projected['consistency_errors'], projected
    evidence = answer['selected_work']['evidence']
    for name, field in [('latest_state', 'work_state'), ('verification', 'verification'),
            ('review', 'user_review'), ('acceptance', 'user_acceptance')]:
        state = evidence[name]
        assert isinstance(state, dict), (name, state)
        for mutation in ('field', 'revision', 'checkpoint', 'offset_type', 'omission'):
            value = copy.deepcopy(answer)
            changed = value['selected_work']['evidence'][name]
            locator = changed['detail_inspection']
            if mutation == 'field':
                locator['field'] = 'goal'
            elif mutation == 'revision':
                locator['revision'] += 1
            elif mutation == 'checkpoint':
                changed['checkpoint_id'] = locator['record_id'] = 'ab' * 16
            elif mutation == 'offset_type':
                locator['offset'] = '0'
            else:
                changed['detail_inspection'] = {'transport_omission': {
                    'reason': 'serialized_byte_budget', 'exact_json_bytes': len(json.dumps(locator).encode()),
                    'basis': 'inspect the complete field on the authoritative parent record'}}
            result = answer_projection.project(value, 'recall')
            answer_projection.validate(result)
            if mutation in {'offset_type', 'omission'}:
                assert not result['semantic_complete'], (name, mutation, result)
            else:
                assert result['consistency_errors'], (name, mutation, result)
            if mutation == 'omission':
                assert not result['consistency_errors'], result
    reports = evidence['verification']['verification']
    assert reports and reports[0]['outcome_recorded']
    for mutation in ('field', 'index'):
        value = copy.deepcopy(answer)
        report = value['selected_work']['evidence']['verification']['verification'][0]
        if mutation == 'field':
            report['detail_inspection']['field'] = 'next_step'
        else:
            report['verification_index'] += 1
        result = answer_projection.project(value, 'recall')
        assert 'resume verification field/index mismatch' in result['consistency_errors'], result


def fixture(config):
    f = AnswerTests(); f.setUp()
    replacements = {'01' * 16: config['project'], '08' * 16: config['work'],
        '03' * 16: config['source'], '09' * 16: config['checkpoint'],
        f.answer['next_step']: config['next_step']}
    def replace(value):
        text = json.dumps(value)
        for before, after in replacements.items():
            text = text.replace(before, after)
        return json.loads(text)
    f.events = replace(f.events)
    f.output = next(e['payload'] for e in f.events if e['payload'].get('type') == 'custom_tool_call_output'
        and 'recall-call' in e['payload'].get('call_id', ''))
    f.answer = copy.deepcopy(config['before'])
    reference = f.descriptor['evidence']['captures']['work']; path = f.root / reference['file']
    events = replace([json.loads(line) for line in path.read_text().splitlines()])
    # Explicit seed-authoring envelopes, independently backed by the actual
    # fixture's canonical identity/Source/Checkpoint, not claimed captured host calls.
    for event in events:
        payload = event['payload']
        if payload.get('type') == 'mcp_tool_call_end' and payload['invocation']['tool'] == 'checkpoint_record':
            payload['invocation']['arguments']['next_step'] = config['next_step']
    path.write_text(''.join(json.dumps(e) + '\n' for e in events)); reference['sha256'] = h.sha256(path)
    reference = f.descriptor['evidence']['canonical_bundle']; path = f.root / reference['file']
    path.write_bytes(Path(config['bundle']).read_bytes()); reference['sha256'] = h.sha256(path)
    return f


def correction(f, config, *, after=False):
    event = f.insert_correction(after=after)
    event['payload']['invocation']['arguments'] = copy.deepcopy(config['mutation_arguments'])
    event['payload']['result'] = {'Ok': copy.deepcopy(config['mutation_result'])}
    wrapper = next(e for e in f.events if e['payload'].get('call_id') == 'correction-wrapper')
    wrapper['payload']['input'] = 'const r=await tools.mcp__volicord__canonical_mutate(' + json.dumps(config['mutation_arguments']) + '); text(JSON.stringify(r));'
    return event


def run(config):
    original_config = copy.deepcopy(config)
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'end-to-end/multi-repository'))
    import restart_recall
    for key in ('before', 'after', 'before_cli'):
        check_private_resume_projection(config[key])
        answer = config[key]
        for kind, selected in [('work', answer['selected_work'])] + [
                ('decision', item) for item in answer['decisions']]:
            observed = explanation_evidence.measured_subject_reading(selected, kind)
            reference = selected['reading_references']['article'] if kind == 'work' else selected['reading_reference']
            assert observed['explanation_state'] == reference['explanation_state'], observed
            assert observed['reading_body'] == 'deferred' and observed['returned_answer_sha256'] is None
            assert observed['returned_reference_sha256'] == explanation_evidence.binding(
                explanation_evidence.api().json_bytes(reference))['sha256']
    for key in ('before', 'after'):
        calls = []
        def call(name, arguments):
            calls.append(name)
            assert arguments == {'project_id': config['project'], 'requested_language': 'en'}
            return config[key] if name == 'recall' else config[key + '_full']
        compact, full = rehearsal_support.session_readback(call, config['project'], config['work'])
        assert calls == ['recall', 'repository_understanding']
        assert compact is config[key] and full is config[key + '_full']
        wrong = copy.deepcopy(config[key + '_full'])
        wrong['selected_work']['work_item_id'] = 'ab' * 16
        try:
            rehearsal_support.session_readback(lambda name, args: config[key] if name == 'recall' else wrong,
                config['project'], config['work'])
        except ValueError as error:
            assert 'session resume/full reader mismatch' in str(error)
        else:
            raise AssertionError('foreign full reader accepted as deferred Resume content')
    for key in ('before', 'after'):
        errors = restart_recall.resume_reader_errors(
            {'project_id': config['project'], 'goal_id': config['work']}, config[key], config[key + '_full'])
        assert not errors, (key, errors)
    def retained_goal_status(answer):
        # Product bounds the global Goal section separately from selected Work.
        # The support oracle uses the actual DTO's availability, not observer
        # output. A valid but unretained Goal basis is explicit uncertainty.
        expected_omission = {'identity': config['work'], 'kind': 'context_goal',
            'reason': 'bound', 'expandable_basis': 'expand context_goal by identity'}
        goals = answer.get('goal_basis', [])
        if (not any(g.get('identity') == config['work'] for g in goals if isinstance(g, dict))
                and expected_omission in answer.get('omissions', [])):
            selected = answer.get('selected_work') or {}
            evidence = selected.get('evidence') or {}
            reading = evidence.get('goal') or {}
            basis = reading.get('basis') or {}
            if not {'record_kind', 'identity', 'field', 'revision', 'source_ids'} <= set(basis):
                return 'indeterminate'
        return 'confirmed_pass'

    def bounded_goal_counterfactual(answer):
        # Authored availability mutation of real DTOs, not another Product read.
        value = copy.deepcopy(answer)
        value['goal_basis'] = [g for g in value['goal_basis'] if g.get('identity') != config['work']]
        report = {'identity': config['work'], 'kind': 'context_goal',
            'reason': 'bound', 'expandable_basis': 'expand context_goal by identity'}
        if report not in value.setdefault('omissions', []):
            value['omissions'].append(report)
        original_goal = value['selected_work'].get('evidence', {}).get('goal')
        value['selected_work']['evidence'] = {'goal': {'transport_omission': {
            'reason': 'serialized_byte_budget', 'exact_json_bytes': len(json.dumps(original_goal).encode()),
            'basis': 'inspect the complete field on the authoritative parent record'}}}
        assert retained_goal_status(value) == 'indeterminate'
        return value

    for after, answer, expected in [(True, config['before'], 'confirmed_pass'),
            (False, config['after'], 'confirmed_pass'), (False, config['before'], 'confirmed_violation'),
            (True, bounded_goal_counterfactual(config['before']), 'indeterminate'),
            (False, bounded_goal_counterfactual(config['after']), 'indeterminate')]:
        f = fixture(config)
        try:
            correction(f, config, after=after)
            _, fact = f.evaluate(answer)
            if expected != 'confirmed_violation':
                expected = retained_goal_status(answer)
                observation = fact['basis']['observations'][0]
                assert observation['goal_basis']['revision'] == (1 if after else 2), observation
                if expected == 'indeterminate':
                    assert observation['errors'] == [], observation
                    assert 'selected Goal basis unavailable after exact Goal-list bound omission' in observation['limits'], observation
            assert fact['status'] == expected, fact
            assert h.load_codex_capture(f.path).calls('canonical_mutate')[0].outcome == 'succeeded'
            if expected == 'confirmed_violation':
                assert machine.disposition('shared_answer_integrity', fact['status']) == 'hard_blocking'
        finally:
            f.doCleanups()
    # Exercise the actual rehearsal's temporal controls over actual Rust DTOs.
    # Independent canonical Goal revision cannot be inferred from article metadata.
    for key, revision in [('before', 2), ('after', 1)]:
        value = copy.deepcopy(config[key])
        rehearsal_support.change_returned_goal_revision(value, revision)
        f = fixture(config)
        try:
            correction(f, config, after=key == 'before')
            _, fact = f.evaluate(value)
            assert fact['status'] == 'confirmed_violation', (key, fact)
            rehearsal_support.require_goal_revision_rejection(fact['basis']['observations'][0])
        finally:
            f.doCleanups()
    for visible_violation in (False, True):
        f = fixture(config)
        try:
            event = correction(f, config)
            del event['payload']['result']['Ok']['structuredContent']['revision']
            value = copy.deepcopy(config['after'])
            if visible_violation:
                value['next_step'] = 'Deliberately contradictory verifier input'
            _, fact = f.evaluate(value)
            observed = fact['basis']['observations'][0]
            assert fact['status'] == ('confirmed_violation' if visible_violation else 'indeterminate'), fact
            assert observed['goal_basis']['revision'] is None and observed['limits'], observed
        finally:
            f.doCleanups()
    for field, bad in [('identity', 'ab' * 16), ('record_kind', 'decision'),
            ('field', 'user_rationale'), ('source_ids', ['ab' * 16]), ('revision', True)]:
        value = copy.deepcopy(config['after'])
        value['selected_work']['evidence']['goal']['basis'][field] = bad
        f = fixture(config)
        try:
            correction(f, config)
            _, fact = f.evaluate(value)
            assert fact['status'] == 'confirmed_violation', (field, fact)
        finally:
            f.doCleanups()
    for mode in ('whole_revision', 'source_suffix', 'source_suffix_conflict'):
        value = copy.deepcopy(config['after'])
        basis = value['selected_work']['evidence']['goal']['basis']
        if mode == 'whole_revision':
            basis['revision'] = {'transport_omission': {'reason': 'serialized_byte_budget',
                'exact_json_bytes': len(json.dumps(basis['revision']).encode()),
                'basis': 'inspect the complete field on the authoritative parent record'}}
        else:
            sources = basis['source_ids']
            basis['source_ids'] = [sources[0] if mode == 'source_suffix' else 'ab' * 16,
                {'transport_omission': {'reason': 'serialized_byte_budget',
                    'omitted_count': len(sources), 'first_omitted_identity': None,
                    'basis': 'same parent identity, field and stable input order; inspect the authoritative record'}}]
            # Keep an independently valid prefix/count for the positive omission.
            if mode == 'source_suffix':
                basis['source_ids'] = [dict(basis['source_ids'][-1])]
        f = fixture(config)
        try:
            correction(f, config)
            _, fact = f.evaluate(value)
            assert fact['status'] == ('confirmed_violation' if mode.endswith('conflict') else 'indeterminate'), (mode, fact)
        finally:
            f.doCleanups()
    f = fixture(config)
    try:
        correction(f, config)
        _, fact = f.evaluate(config['after'], cli=config['before_cli'], cli_before=True)
        expected = ('indeterminate' if 'indeterminate' in (
            retained_goal_status(config['before_cli']), retained_goal_status(config['after'])) else 'confirmed_pass')
        assert fact['status'] == expected, fact
        assert [o['goal_basis']['revision'] for o in fact['basis']['observations']] == [1, 2]
    finally:
        f.doCleanups()
    # Mutate actual Product DTOs, retaining real identity/Source/Checkpoint and
    # correction witnesses. Deferred reader references cannot excuse a visible
    # foreign selector, a revived unavailable publication or a wrong action.
    for case in ('project', 'work', 'language', 'role', 'unavailable', 'action', 'envelope', 'omitted_reference'):
        f = fixture(config)
        try:
            correction(f, config)
            value = copy.deepcopy(config['after'])
            selected = value['selected_work']
            reference = selected['reading_references']['article']
            if case in {'project', 'work', 'language'}:
                key = {'project': 'project_id', 'work': 'subject_id', 'language': 'language'}[case]
                reference['inspect']['arguments'][key] = 'foreign'
            elif case == 'role':
                reference['role'] = 'user_judgment'
            elif case == 'unavailable':
                reference['explanation_state'] = 'deleted'
            elif case == 'action':
                selected['next_action']['recorded_action']['checkpoint_id'] = 'ab' * 16
            elif case == 'envelope':
                del value['reading_profile']
            else:
                selected['reading_references']['article'] = {'transport_omission': {
                    'reason': 'serialized_byte_budget', 'exact_json_bytes': len(json.dumps(reference).encode()),
                    'basis': 'inspect the complete field on the authoritative parent record'}}
            _, fact = f.evaluate(value)
            expected = 'indeterminate' if case == 'omitted_reference' else 'confirmed_violation'
            assert fact['status'] == expected, (case, fact)
            if expected == 'confirmed_violation':
                assert machine.disposition('shared_answer_integrity', fact['status']) == 'hard_blocking'
        finally:
            f.doCleanups()
    assert config == original_config, 'counterfactual support changed an original Product DTO'
    print('actual MCP/CLI Recall and correction receipt: pre/post revisions verified with bounded Goal uncertainty retained, stale-current hard violation, later correction isolation passed; synthetic capture support only')


if __name__ == '__main__':
    run(json.loads(Path(sys.argv[1]).read_bytes()))
