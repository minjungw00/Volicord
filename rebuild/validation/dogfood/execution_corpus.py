#!/usr/bin/env python3
"""Read-only execution normalization diagnostic, never collection or publication.

The output contains hashes and bounded locators only. Original campaign/candidate
identity is retained separately from the development evaluator that reads it.
"""
import argparse
from collections import Counter
import json
from pathlib import Path
import subprocess
import tempfile

import campaign
import codex_events as c
import explanation_evidence as explanations
import review_captures


def inspect(root, before):
    manifest = c.strict_json((root / 'campaign.json').read_text())
    paths = sorted((root / 'raw-rollouts').glob('*.jsonl'))
    if len(paths) != 8:
        raise ValueError('diagnostic requires exactly eight preserved measured rollouts')
    previous = {value['file']: value for value in c.strict_json(before.read_text())}
    captures = []
    for path in paths:
        raw = path.read_bytes()
        capture = c.parse_codex_capture(raw)
        baseline = previous.get(path.name)
        if not baseline or baseline['sha256'] != capture.source_sha256:
            raise ValueError('before/after raw bytes differ')
        slots = []
        for work in manifest['works'].values():
            for role, artifact in work['operator_task_artifacts'].items():
                task = (root / artifact['path']).read_bytes()
                if c.sha256_bytes(task) != artifact['sha256'] or len(task) != artifact['bytes']:
                    raise ValueError('frozen task bytes differ from original campaign')
                if capture.user_turns and campaign.harness.codex_user_turn_transport_identity_matches(
                        capture.user_turns[0].text, task.decode('utf-8')):
                    slots.append((work['repository_class'], work['work_label'], role))
        if len(slots) != 1:
            raise ValueError('measured task has no unique frozen campaign slot')
        slot = slots[0]
        wrappers = {value.sequence: value for value in capture.execution_wrappers}
        # Literal hints locate reviewed wrapper text, not inferred inner execution.
        # No command or source text is included in the diagnostic.
        key_locators = []
        for sequence, line in enumerate(raw.decode('utf-8').splitlines()):
            event = c.strict_json(line)
            payload = event.get('payload', {})
            source = payload.get('input', '')
            if sequence in wrappers:
                hints = [hint for hint in ('explain prepare', 'explain record') if hint in source]
                if hints:
                    key_locators.append({'sequence': sequence, 'call_id': wrappers[sequence].call_id,
                        'literal_hints_not_execution_assertions': hints, 'state': wrappers[sequence].state})
        commands = [{k: v for k, v in vars(command).items() if k not in {'parsed_command', 'output'}} | {
            'command_sha256': c.sha256_bytes(c.canonical_json(command.parsed_command)),
            'output_sha256': c.sha256_bytes(command.output.encode('utf-8')),
            'command_role': c.command_role(command.parsed_command)} for command in capture.commands]
        operations = [{k: v for k, v in value.items() if k != 'result'} | {
            'returned_payload_sha256': c.sha256_bytes(c.canonical_json(value['result']))}
            for value in explanations.measured_cli_operations(capture)]
        # Exercise the final collection-index consumer in a disposable empty
        # steward directory; this creates no evidence set or campaign publication.
        with tempfile.TemporaryDirectory(prefix='volicord-execution-index-') as temporary:
            from types import SimpleNamespace
            index = explanations.collection_index(Path(temporary), {
                slot: SimpleNamespace(capture=capture)})
            if c.strict_json(c.canonical_json(index).decode()) != index:
                raise ValueError('collection index is not JSON-stable')
        # Prepare the reviewer-safe projection only in memory, against exact raw
        # bytes. This is a diagnostic binding, never a published evidence-set hash.
        try:
            projection, _ = review_captures.project(raw, origin={'kind': 'evidence_set_member',
                'path': 'raw-rollouts/' + path.name, 'raw_bytes': len(raw), 'raw_sha256': capture.source_sha256},
                role=slot[2], session_id=capture.session_id, candidate_head=manifest['candidate_head'],
                evidence_set_sha256='0' * 64)
        except ValueError as error:
            if str(error) != 'unsupported unnormalized review user interaction':
                raise  # Do not hide a new execution/projection regression.
            review_coverage = {'state': 'unsupported', 'limit': 'unnormalized_user_interaction'}
        else:
            review = review_captures.validate(projection)
            review_coverage = review['execution_coverage']
            if review_coverage['unsupported_wrapper_count'] != sum(w.state == 'unsupported' for w in wrappers.values()):
                raise ValueError('review projection lost unsupported execution')
        captures.append({'file': path.name, 'sha256': capture.source_sha256,
            'before': baseline, 'coverage': capture.execution_evidence(), 'commands': commands,
            'measured_operation_observations': operations, 'key_wrapper_locators': key_locators,
            'session_slot_id': campaign.session_slot_id(*slot), 'role': slot[2],
            'review_execution_coverage': review_coverage,
            'successful_recall_count': len(capture.successful_calls('recall')),
            'repeated_recall_work_ids': campaign.observed_work_item_ids(capture, 'resume')
                if len(capture.successful_calls('recall')) > 1 else None})
    return {'kind': 'dogfood_execution_normalization_diagnostic',
        'product_candidate': manifest['candidate_head'],
        'evaluator_head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=campaign.ROOT).decode().strip(),
        'evaluator_worktree_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=campaign.ROOT)),
        'evaluator_source_sha256': {name: c.sha256_bytes(Path(__file__).with_name(name).read_bytes())
            for name in ('campaign.py', 'codex_events.py', 'harness.py', 'explanation_evidence.py',
                'review_captures.py', 'interaction_diagnostics.py', 'machine-policy.json')},
        'role': 'read_only_support_not_measured_session_or_publication', 'captures': captures,
        'before_command_count': sum(value['before']['commands'] for value in captures),
        'after_command_count': sum(len(value['commands']) for value in captures),
        'wrapper_states': dict(Counter(w['state'] for value in captures for w in value['coverage']['wrappers']))}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--campaign-root', type=Path, required=True)
    parser.add_argument('--before', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    local = (campaign.ROOT / 'rebuild/.local').resolve()
    if not output.is_relative_to(local) or output.exists():
        raise ValueError('diagnostic output must be new and reconstruction-local')
    result = inspect(args.campaign_root.resolve(), args.before.resolve())
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open('x', encoding='utf-8') as target:
        json.dump(result, target, ensure_ascii=False, indent=2)
        target.write('\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'captures'}, indent=2))


if __name__ == '__main__':
    main()
