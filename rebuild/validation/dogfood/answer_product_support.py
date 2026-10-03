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
    for after, answer, expected in [(True, config['before'], 'confirmed_pass'),
            (False, config['after'], 'confirmed_pass'), (False, config['before'], 'confirmed_violation')]:
        f = fixture(config)
        try:
            correction(f, config, after=after)
            _, fact = f.evaluate(answer)
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
        assert fact['status'] == 'confirmed_pass', fact
        assert [o['goal_basis']['revision'] for o in fact['basis']['observations']] == [1, 2]
    finally:
        f.doCleanups()
    print('actual MCP/CLI Recall and correction receipt: pre/post revision pass, stale-current hard violation, later correction isolation passed; synthetic capture support only')


if __name__ == '__main__':
    run(json.loads(Path(sys.argv[1]).read_bytes()))
