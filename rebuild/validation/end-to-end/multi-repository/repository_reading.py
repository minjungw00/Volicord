#!/usr/bin/env python3
"""Focused public-entry-point contract control. Authored prose, no model or human verdict."""
import argparse
import hashlib
from html.parser import HTMLParser
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time
import urllib.request


class Fragments(HTMLParser):
    def __init__(self):
        super().__init__(); self.ids = []; self.targets = []; self.code = []; self.in_code = False
    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if tag == 'code': self.in_code = True
        if 'id' in values: self.ids.append(values['id'])
        if values.get('href', '').startswith('#'): self.targets.append(values['href'][1:])
    def handle_endtag(self, tag):
        if tag == 'code': self.in_code = False
    def handle_data(self, data):
        if self.in_code: self.code.append(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args(); binaries = args.bin_dir.resolve(); output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    operations = []; viewer = None
    def run(label, argv, *, payload=None, env=None, accepted=0):
        result = subprocess.run([str(v) for v in argv], input=payload, capture_output=True, env=env, timeout=120)
        (output / (label + '.stdout')).write_bytes(result.stdout); (output / (label + '.stderr')).write_bytes(result.stderr)
        operations.append({'label': label, 'argv': [str(v) for v in argv], 'exit': result.returncode,
                           'stdout_sha256': hashlib.sha256(result.stdout).hexdigest(), 'stderr_sha256': hashlib.sha256(result.stderr).hexdigest()})
        assert result.returncode == accepted, (label, result.returncode, result.stderr.decode())
        return result.stdout
    try:
        with tempfile.TemporaryDirectory(prefix='volicord-repository-reader-') as temporary:
            temporary = Path(temporary); root = temporary / 'repo'; root.mkdir(); runtime = temporary / 'runtime'
            source_text = "def greeting(name):\n    return 'Hello ' + name\n"
            (root / 'main.py').write_text(source_text); (root / 'old').mkdir(); (root / 'old/legacy.py').write_text('excluded = True\n')
            cli = [binaries / 'volicord', '--runtime', runtime, '--repository', root, '--json']
            def call(label, *argv, **options):
                return json.loads(run(label, cli + list(argv), **options))
            project = call('init', 'init', 'Authored repository control')['project_id']
            analysis = call('analyze', 'analyze', '--exclude', 'old')['analysis_snapshot']
            before = call('before', 'status')
            assert before['selected_work'] is None and before['project_purpose'] == []
            assert before['repository_explanation']['explanation_state'] == 'missing'
            prepared = call('prepare', 'repository', 'explain', 'prepare', '--analysis', analysis)['plan']
            assert prepared['subject'] == {'kind': 'repository', 'identity': analysis}
            receipt = call('observe', 'repository', 'source', 'observe', '--analysis', analysis, '--path', 'main.py', '--retain-seconds', '3600')['observation']
            source = receipt['source_id']
            read = call('source', 'repository', 'source', 'read', '--analysis', analysis, '--source', source)
            assert read['observation']['detail']['body'] == source_text
            assert 'work_item_id' not in read['observation']
            # This authored support text checks presentation/retention, not A3 prose adequacy.
            text = 'Authored control: greeting constructs a salutation from its input name.'
            paragraphs = [{'question': role, 'text': wording, 'evidence_keys': ['repository_basis'],
                           'sources': [{'source_id': source, 'start_byte': 0, 'end_byte': len(source_text.encode()), 'claim': None}]}
                          for role, wording in [('heading', 'Authored repository reading'), ('synopsis', text), ('observation', text),
                                                ('flow_step', 'Receive the supplied name as the greeting argument.'),
                                                ('flow_step', 'Return the salutation joined to that name.'),
                                                ('reading_point', 'Read greeting to inspect how the name enters the return value.')]]
            paragraphs.append({'question': 'limits', 'text': 'Authored control only; no execution, semantic review or user approval is established.', 'evidence_keys': ['repository_basis'], 'sources': []})
            draft = {'plan_fingerprint': prepared['fingerprint'], 'language': 'en', 'generator': {'host': 'authored-control', 'session': 'public-entrypoint', 'agent': None, 'model': None}, 'paragraphs': paragraphs}
            draft_path = temporary / 'draft.json'; draft_path.write_text(json.dumps(draft))
            response = call('compose', 'repository', 'explain', 'compose', '--analysis', analysis, '--input', draft_path)['realization']
            response_path = temporary / 'response.json'; response_path.write_text(json.dumps(response))
            call('record', 'repository', 'explain', 'record', '--analysis', analysis, '--input', response_path)
            answers = call('read', 'repository', 'explain', 'read')['answers']
            assert answers['explanation_state'] == 'current' and answers['prose'][1]['text'] == text
            assert [unit['text'] for unit in answers['prose']] == [unit['text'] for unit in response['paragraphs']]
            steps = [unit for unit in answers['prose'] if unit['question'] == 'FlowStep']
            assert len(steps) == 2 and all(unit['citations'] for unit in steps)
            facts = {a['question']: a['text'] for a in answers['facts']}
            assert 'Cited Source area: . (1 files)' in facts['RepositoryScope']
            assert 'Excluded boundaries: 1' in facts['RepositoryCoverage']
            assert 'Complete semantic understanding is not established' in facts['RepositoryCoverage']
            assert answers['source_excerpts'][0]['text'] == source_text and answers['source_excerpts'][0]['work_revision'] is None
            before_read = call('privacy-before', 'privacy', 'status')
            # Actual stdio MCP consumer, no new producer authority.
            messages = [dict(jsonrpc='2.0', id=1, method='initialize', params={'capabilities': {}}),
                        dict(jsonrpc='2.0', id=2, method='tools/call', params={'name': 'repository_understanding', 'arguments': {'project_id': project, 'requested_language': 'en','reading_focus':'repository_explanation'}})]
            env = os.environ.copy(); env['VOLICORD_RUNTIME_DIR'] = str(runtime)
            raw = run('mcp', [binaries / 'volicord-mcp'], payload=('\n'.join(json.dumps(m) for m in messages)+'\n').encode(), env=env)
            result = [json.loads(line) for line in raw.splitlines() if json.loads(line).get('id') == 2][0]['result']
            assert not result.get('isError'), result
            assert result['structuredContent']['repository_explanation']['provenance'] == answers['provenance']
            assert result['structuredContent']['repository_explanation']['facts'] == answers['facts']
            assert result['structuredContent']['repository_explanation']['prose'] == answers['prose']
            # Real loopback HTTP, complete process cleanup even on assertion failure.
            stderr = (output / 'viewer.stderr').open('wb'); stdout = (output / 'viewer.stdout').open('wb')
            viewer = subprocess.Popen([str(binaries / 'volicord-viewer'), '--runtime', str(runtime), '--project', project, '--bind', '127.0.0.1:0'], stdout=stdout, stderr=stderr)
            deadline = time.monotonic() + 15; url = None
            while time.monotonic() < deadline:
                assert viewer.poll() is None, 'Viewer exited before startup'
                match = re.search(r'http://127\.0\.0\.1:\d+/', (output / 'viewer.stderr').read_text())
                if match: url = match.group(); break
                time.sleep(.05)
            assert url, 'Viewer startup timeout'
            for name, query in [('overview', '?view=overview'), ('repository', '?view=repository'), ('work', '?view=work')]:
                page = urllib.request.urlopen(url + query, timeout=15).read().decode()
                (output / (name + '.html')).write_text(page)
                if name == 'work':
                    assert 'No Work has been recorded' in page
                    continue
                if name == 'overview':
                    # Overview links to the repository destination; it does not
                    # repeat a second repository-summary card (Reader D-28).
                    assert text not in page
                    assert 'view=repository' in page
                    continue
                assert 'Authored repository reading' in page
                if name == 'repository':
                    assert text in page
                    assert '<ol class="article-flow">' in page
                    assert page.index(steps[0]['text']) < page.index(steps[1]['text'])
                if name != 'overview':
                    parsed = Fragments(); parsed.feed(page)
                    assert source_text in ''.join(parsed.code)
                if name == 'repository':
                    assert 'data-question="RepositoryScope"' in page
                    assert 'class="document-nav"' in page
                assert 'repository-explanation' in page or 'view=repository' in page
            snapshot = output / 'offline.html'
            call('snapshot', 'viewer', 'export', '--output', snapshot)
            html = snapshot.read_text(); assert text in html and '<script' not in html and '<form' not in html
            assert '<ol class="article-flow">' in html
            assert html.index(steps[0]['text']) < html.index(steps[1]['text'])
            fragments = Fragments(); fragments.feed(html)
            assert len(fragments.ids) == len(set(fragments.ids)), 'duplicate offline fragments'
            assert not (set(fragments.targets) - set(fragments.ids)), 'broken offline source navigation'
            assert call('privacy-after', 'privacy', 'status') == before_read, 'reading mutated retention'
            after = call('after', 'status')
            assert after['project_purpose'] == [] and after['selected_work'] is None
            assert after['repository_explanation']['provenance'] == answers['provenance']
            run('human-cli', cli[:-1] + ['repository', 'explain', 'read'])
            # Rejection preserves the exact previous realization.
            response['paragraphs'][2]['citations'][0]['claim_sha256'] = 'sha256:' + '0'*64
            response_path.write_text(json.dumps(response))
            run('bad-record', cli + ['repository', 'explain', 'record', '--analysis', analysis, '--input', response_path], accepted=1)
            assert call('read-after-bad', 'repository', 'explain', 'read')['answers']['provenance'] == answers['provenance']
            call('delete-source', 'repository', 'source', 'delete', '--analysis', analysis, '--source', source)
            denied = call('read-deleted-source', 'repository', 'explain', 'read')['answers']
            assert denied['explanation_state'] != 'current' and denied['provenance'] is None
            (output / 'result.json').write_text(json.dumps({'status': 'passed', 'kind': 'authored_public_contract_control', 'human_acceptance': 'not_observed', 'actual_host_generation': 'not_supplied_by_this_control', 'operations': operations}, indent=2))
    finally:
        if viewer is not None:
            viewer.terminate()
            try: viewer.wait(timeout=10)
            except subprocess.TimeoutExpired: viewer.kill(); viewer.wait(timeout=10)
            stdout.close(); stderr.close()
        (output / 'operations.json').write_text(json.dumps(operations, indent=2))
    print(f'Public repository CLI/MCP/live/offline contract control passed: {output}')


if __name__ == '__main__':
    main()
