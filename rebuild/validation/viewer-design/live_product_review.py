#!/usr/bin/env python3
"""Open actual localhost Product readers; static packages remain separate evidence.

Private configuration uses the existing viewer/cases document-reader fields.
No observations, authorship, publication or provider operations are performed.
"""
import argparse
import hashlib
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import signal
import subprocess
import time
from urllib.parse import quote, urlencode
from urllib.request import urlopen


class Figures(HTMLParser):
    def __init__(self):
        super().__init__()
        self.targets = {}

    def handle_starttag(self, tag, attributes):
        attrs = dict(attributes)
        if tag == 'figure' and 'admitted-diagram' in attrs.get('class', '').split():
            self.targets[attrs['data-semantic-fingerprint']] = attrs['id']


def write(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)


def run(config, output, primary, check_only=False):
    cases = config['cases']
    keys = [case['key'] for case in cases]
    if not keys or len(keys) != len(set(keys)) or primary not in keys:
        raise ValueError('select unique actual cases and an existing primary case')
    viewer = Path(config['viewer']).resolve(strict=True)
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    listeners = {}
    streams = []
    stopping = False

    def stop(_signal, _frame):
        nonlocal stopping
        stopping = True

    for name in (signal.SIGINT, signal.SIGTERM):
        signal.signal(name, stop)
    try:
        entries = []
        for case in cases:
            if stopping:
                return
            scope = (str(Path(case['runtime']).resolve(strict=True)), case['project'])
            bind = case.get('bind', '127.0.0.1:0')
            if not isinstance(bind, str) or not re.fullmatch(r'127\.0\.0\.1:\d+', bind) or int(bind.rsplit(':', 1)[1]) > 65535:
                raise ValueError('review bind must be a loopback address and valid port')
            if scope in listeners and listeners[scope]['bind'] != bind:
                raise ValueError('cases sharing a listener must use the same bind')
            if scope not in listeners:
                index = len(listeners)
                log = output / f'listener-{index}.stderr'
                stream = log.open('xb')
                streams.append(stream)
                child = subprocess.Popen(
                    [str(viewer), '--runtime', scope[0], '--project', scope[1],
                     '--bind', bind],
                    stdout=subprocess.DEVNULL, stderr=stream)
                listeners[scope] = {'child': child, 'base': None, 'bind': bind}
                deadline = time.monotonic() + 15
                while time.monotonic() < deadline:
                    match = re.search(r'http://127\.0\.0\.1:\d+/', log.read_text())
                    if match:
                        listeners[scope]['base'] = match[0]
                        break
                    if child.poll() is not None or stopping:
                        raise RuntimeError('Product listener stopped before readiness')
                    time.sleep(.05)
                if listeners[scope]['base'] is None:
                    raise RuntimeError('Product listener readiness timed out')
            query = {'project': case['project'], 'view': 'work' if case.get('work') else 'repository',
                     'language': case['language'], 'locale': case.get('locale', case['language'])}
            if case.get('work'):
                query['work'] = case['work']
            address = listeners[scope]['base'] + '?' + urlencode(query)
            with urlopen(address, timeout=15) as response:
                if response.status != 200:
                    raise RuntimeError('Product document unavailable')
                document = response.read().decode('utf-8')
            if '<script' not in document:
                raise ValueError('primary review must be the live Product reader')
            parsed = Figures()
            parsed.feed(document)
            figures = []
            if case.get('public'):
                for diagram in json.loads(Path(case['public']).read_text())['diagrams']:
                    target = parsed.targets.get(diagram['semantic_fingerprint'])
                    # Input fixtures never restore labels/links absent from the current reader.
                    if target:
                        figures.append({'kind': diagram['meaning']['kind'],
                                        'url': address + '#' + quote(target, safe='-_:'),
                                        'id': diagram['id']})
            entries.append({'key': case['key'], 'url': address, 'mode': 'live',
                            'project': case['project'], 'work': case.get('work'),
                            'language': case['language'], 'locale': query['locale'],
                            'figures': figures})
        if stopping:
            return
        session = {'kind': 'live_product_review_session', 'format_version': 1,
                   'candidate': config.get('candidate'),
                   'viewer_sha256': hashlib.sha256(viewer.read_bytes()).hexdigest(),
                   'primary': next(entry for entry in entries if entry['key'] == primary),
                   'entries': entries,
                   'listeners': [{'pid': item['child'].pid, 'base': item['base']}
                                 for item in listeners.values()],
                   'scope': 'task-owned localhost readers; native exports and synthetic images are supplementary'}
        write(output / 'session.json', session)
        print('Primary live Product reader: ' + session['primary']['url'], flush=True)
        for entry in entries:
            print(entry['key'] + ': ' + entry['url'], flush=True)
        if not check_only:
            while not stopping:
                if any(item['child'].poll() is not None for item in listeners.values()):
                    raise RuntimeError('a Product listener exited; restart this review session')
                time.sleep(.25)
    finally:
        exits = []
        for item in listeners.values():
            child = item['child']
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            exits.append({'pid': child.pid, 'exit_code': child.returncode})
        for stream in streams:
            stream.close()
        write(output / 'termination.json', {'kind': 'live_product_review_termination',
                                            'format_version': 1, 'listeners': exits})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--primary', required=True)
    parser.add_argument('--check', action='store_true', help='verify startup/reads and stop all listeners')
    args = parser.parse_args()
    run(json.loads(args.config.read_text()), args.output.absolute(), args.primary, args.check)


if __name__ == '__main__':
    main()
