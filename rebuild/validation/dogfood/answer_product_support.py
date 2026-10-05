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
    event['payload']['invocation']['arguments'] = config['mutation_arguments']
    event['payload']['result'] = {'Ok': config['mutation_result']}
    wrapper = next(e for e in f.events if e['payload'].get('call_id') == 'correction-wrapper')
    wrapper['payload']['input'] = 'const r=await tools.mcp__volicord__canonical_mutate(' + json.dumps(config['mutation_arguments']) + '); text(JSON.stringify(r));'


def run(config):
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
    print('actual MCP/CLI Recall and correction receipt: pre/post revisions verified with bounded Goal uncertainty retained, stale-current hard violation, later correction isolation passed; synthetic capture support only')


if __name__ == '__main__':
    run(json.loads(Path(sys.argv[1]).read_bytes()))
