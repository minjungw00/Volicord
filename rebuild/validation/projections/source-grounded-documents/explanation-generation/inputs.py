"""Disposable experiment preparation; no Product storage or generation semantics."""
from __future__ import annotations

import copy
import datetime as dt
import fcntl
import hashlib
import json
from pathlib import Path
import re


class InputError(ValueError):
    pass


def encoded(value):
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2,
                       allow_nan=False) + "\n").encode()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def binding(path):
    path = Path(path).resolve(strict=True)
    with path.open('rb') as stream:
        sha = hashlib.file_digest(stream, 'sha256').hexdigest()
    return {'path': str(path), 'sha256': sha, 'bytes': path.stat().st_size}


def require(condition, reason):
    if not condition:
        raise InputError(reason)


def check_binding(value):
    require(binding(value['path']) == value, 'changed bytes or stale preparation')


def append_index(root, state, paths):
    """One locked append of actual identities, never overwrite a previous state."""
    root = Path(root)
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    with (root / 'index.jsonl').open('a+b') as stream:
        fcntl.flock(stream, fcntl.LOCK_EX)
        stream.seek(0)
        rows = [json.loads(line) for line in stream]
        previous = None
        for number, row in enumerate(rows):
            claimed = row.pop('sha256')
            require(row['sequence'] == number and row['previous'] == previous
                    and digest(encoded(row)) == claimed, 'index chain changed')
            previous = claimed
        row = {'sequence': len(rows), 'previous': previous, 'state': state,
               'paths': [binding(p) for p in paths]}
        row['sha256'] = digest(encoded(row))
        stream.seek(0, 2)
        stream.write(json.dumps(row, sort_keys=True).encode() + b'\n')
        stream.flush()
        __import__('os').fsync(stream.fileno())
    return row


ROLES = {'source', 'task', 'user_response', 'agent_report', 'canonical_record',
         'command_observation', 'candidate_detail', 'current_preparation', 'instructions'}
REPRESENTATIONS = {'full_file', 'bounded_excerpt', 'verified_reconstruction', 'unavailable'}


def validate(spec):
    require(spec['format_version'] == 1, 'unsupported experiment input')
    scope = spec['scope']
    require(set(scope) == {'project', 'work', 'cutoff'}, 'exact scope required')
    cutoff = dt.datetime.fromisoformat(scope['cutoff'])
    require(cutoff.tzinfo is not None, 'cutoff must include timezone')
    require(all(spec['identities'].get(k) for k in
                ('product_candidate', 'explained_repository', 'experiment_producer')),
            'three independent identities required')
    for dependency in spec.get('contract', []):
        check_binding(dependency)
    adapter = spec['identities']['experiment_producer'].get('adapter')
    if adapter:
        check_binding(adapter)
    ids = set()
    for entry in spec['entries']:
        require(re.fullmatch(r'[a-z0-9-]+', entry['id']) and entry['id'] not in ids,
                'duplicate or unsafe input identity')
        ids.add(entry['id'])
        require(entry['project'] == scope['project'] and entry['work'] == scope['work'],
                'wrong Work/Project')
        require(entry['role'] in ROLES, 'editorial/evaluation input prohibited')
        require(entry['lane'] in {'product', 'archive_diagnostic'}, 'input lane required')
        require(entry['producer']['state'] in {'available', 'missing_capability'},
                'actual producer or missing capability required')
        require(bool(entry['producer']['name']), 'producer name required')
        require(entry['representation'] in REPRESENTATIONS, 'bytes representation required')
        chronology = entry['chronology']
        require(chronology['state'] in {'known', 'ambiguous'}, 'chronology state required')
        if chronology['state'] == 'known':
            time = dt.datetime.fromisoformat(chronology['observed_at'])
            require(time.tzinfo is not None and time <= cutoff, 'input after cutoff')
        else:
            require(bool(chronology['reason']), 'ambiguous chronology needs reason')
        require('before_state' in entry and 'attribution' in entry, 'file state/attribution required')
        if entry.get('origin_witness'):
            check_binding(entry['origin_witness'])
        if entry['representation'] == 'unavailable':
            require(entry['origin'] is None and entry['file_sha256'] is None
                    and bool(entry['missing']), 'unavailable bytes cannot be substituted')
            continue
        check_binding(entry['origin'])
        require(bool(entry['locator']), 'original record locator required')
        if entry['representation'] == 'bounded_excerpt':
            require(entry['file_sha256'] is None and bool(entry['extent']),
                    'excerpt is not a complete file')
        elif entry['role'] == 'source':
            require(entry['file_sha256'] == entry['origin']['sha256'], 'full-file hash mismatch')
        if entry['representation'] == 'verified_reconstruction':
            require(entry['proof']['expected_sha256'] == entry['origin']['sha256']
                    and entry['proof']['independent_witnesses'], 'unverified reconstruction')
            for witness in entry['proof']['independent_witnesses']:
                check_binding(witness)
    for witness in spec.get('archive_witnesses', []):
        check_binding(witness)
    for observation in spec['investigation_inventory']:
        require(observation['project'] == scope['project'] and observation['work'] == scope['work'],
                'foreign investigation')
    require(bool(spec['inventory_boundary']), 'complete inventory boundary required')


def freeze(spec_path, destination, index_root):
    """Freeze already classified evidence. Never choose functions or author prose."""
    spec_path, destination = Path(spec_path), Path(destination)
    spec = json.loads(spec_path.read_bytes())
    validate(spec)
    destination.mkdir(parents=True, mode=0o700, exist_ok=False)
    (destination / 'assets').mkdir()
    frozen = copy.deepcopy(spec)
    frozen['preparation_source'] = binding(spec_path)
    for entry in frozen['entries']:
        if entry['origin'] is not None:
            data = Path(entry['origin']['path']).read_bytes()
            require(digest(data) == entry['origin']['sha256'], 'input changed during preparation')
            path = destination / 'assets' / entry['id']
            path.write_bytes(data)
            entry['asset'] = binding(path)
        else:
            entry['asset'] = None
    path = destination / 'manifest.json'
    path.write_bytes(encoded(frozen))
    append_index(index_root, {'input': 'frozen', 'approach': 'not_run', 'feedback': 'not_requested'}, [path])
    return path


def verify(path):
    spec = json.loads(Path(path).read_bytes())
    check_binding(spec['preparation_source'])
    validate(spec)
    for entry in spec['entries']:
        if entry['asset'] is not None:
            check_binding(entry['asset'])
    return spec


def generation_inventory(spec, lane):
    """No private origin paths, evaluation material or curated source-tour ordering."""
    require(lane in {'product', 'archive_diagnostic'}, 'unknown input lane')
    entries = [e for e in spec['entries'] if lane == 'archive_diagnostic' or e['lane'] == 'product']
    identities = copy.deepcopy(spec['identities'])
    # Generator sees content identities, not filesystem paths to private producers.
    identities['product_candidate'].pop('artifacts', None)
    identities['experiment_producer'] = {'head': identities['experiment_producer']['head']}
    boundary = copy.deepcopy(spec['inventory_boundary'])
    for omission in boundary.get('omissions', []):
        omission.pop('later_preparations', None)  # Private provenance, never supplied evidence.
    return {'scope': spec['scope'], 'identities': identities,
            'inventory_boundary': boundary,
            'investigation_inventory': spec['investigation_inventory'],
            'entries': [{k: e[k] for k in ('id', 'path', 'role', 'lane', 'producer',
                        'representation', 'extent', 'file_sha256', 'chronology',
                        'before_state', 'attribution', 'missing')} for e in entries]}


class Reader:
    """Opaque-ID byte reads. Source text is data, never parsed as tool instructions."""
    def __init__(self, spec, lane, *, max_reads, max_bytes):
        require(lane in {'product', 'archive_diagnostic'}, 'unknown input lane')
        require(type(max_reads) is int and max_reads > 0 and type(max_bytes) is int and max_bytes > 0,
                'numerical positive read budgets required')
        self.entries = {e['id']: e for e in spec['entries']
                        if lane == 'archive_diagnostic' or e['lane'] == 'product'}
        self.max_reads, self.max_bytes = max_reads, max_bytes
        self.reads, self.bytes, self.trace = 0, 0, []

    def read(self, identity, offset=0, limit=2048):
        require(identity in self.entries, 'read outside supplied inventory')
        require(type(offset) is int and offset >= 0 and type(limit) is int and limit > 0,
                'invalid read range')
        require(self.reads < self.max_reads and self.bytes + limit <= self.max_bytes,
                'read budget exhausted')
        entry = self.entries[identity]
        require(entry['asset'] is not None, 'bytes unavailable')
        check_binding(entry['asset'])
        with Path(entry['asset']['path']).open('rb') as stream:
            stream.seek(offset)
            data = stream.read(limit)
        self.reads += 1
        self.bytes += len(data)
        result = {'id': identity, 'offset': offset, 'bytes': len(data), 'sha256': digest(data),
                  'role': 'untrusted_evidence', 'representation': entry['representation'],
                  'complete_asset': offset == 0 and len(data) == entry['asset']['bytes']}
        self.trace.append(result)
        return result, data
