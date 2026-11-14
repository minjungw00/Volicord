"""Bound a retained collector to Git without requiring Git during copied reads.

Only commit/tree metadata and the already retained producer bytes are needed.
Historical Python is never executed, and unrelated file bodies are not copied.
"""
import base64
import binascii
import hashlib
from pathlib import Path
import re
import subprocess

import review_operations as operations


def require(condition, message):
    if not condition:
        raise ValueError('collector revision proof: ' + message)


def oid(kind, body):
    return hashlib.sha1(kind.encode() + b' ' + str(len(body)).encode() + b'\0' + body).hexdigest()


def tree_entries(body):
    entries, offset = {}, 0
    while offset < len(body):
        space = body.find(b' ', offset)
        end = body.find(b'\0', space + 1)
        require(space > offset and end > space + 1 and end + 21 <= len(body), 'malformed tree')
        mode, name = body[offset:space], body[space + 1:end]
        require(mode in {b'40000', b'100644', b'100755', b'120000', b'160000'}
            and b'/' not in name and name not in {b'.', b'..'} and name not in entries,
            'invalid or duplicate tree entry')
        entries[name] = (mode, body[end + 1:end + 21].hex())
        offset = end + 21
    return entries


def decode(value):
    require(isinstance(value, str) and len(value) <= operations.MAX_FILE_BYTES, 'invalid object encoding')
    try:
        result = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise ValueError('collector revision proof: invalid base64') from error
    require(base64.b64encode(result).decode() == value, 'noncanonical base64')
    return result


def check(value, proof, producer_bytes):
    require(isinstance(proof, dict) and set(proof) == {
        'kind', 'schema_version', 'collection_run_id', 'collector_revision', 'commit', 'trees'},
        'invalid envelope')
    require(proof['kind'] == 'dogfood_collector_revision_proof'
        and type(proof['schema_version']) is int and proof['schema_version'] == 1
        and proof['collection_run_id'] == value['run_id']
        and re.fullmatch('[0-9a-f]{64}', str(value['run_id'])) is not None
        and proof['collector_revision'] == value['collector_revision']
        and re.fullmatch('[0-9a-f]{40}', str(value['collector_revision'])) is not None,
        'foreign collection or revision')
    require(len(operations.encoded(proof)) <= operations.MAX_FILE_BYTES, 'proof exceeds metadata bound')
    commit = decode(proof['commit'])
    require(oid('commit', commit) == value['collector_revision'], 'commit hash mismatch')
    first = commit.split(b'\n', 1)[0]
    require(re.fullmatch(b'tree [0-9a-f]{40}', first) is not None, 'commit tree missing')
    root = first[5:].decode()
    require(isinstance(proof['trees'], dict) and proof['trees'], 'tree closure missing')
    trees = {}
    for identity, encoded in proof['trees'].items():
        require(isinstance(identity, str) and re.fullmatch('[0-9a-f]{40}', identity), 'invalid tree identity')
        body = decode(encoded)
        require(oid('tree', body) == identity, 'tree hash mismatch')
        trees[identity] = tree_entries(body)
    require(set(producer_bytes) == set(value['producers']), 'producer inventory mismatch')
    used = set()
    for name, expected in value['producers'].items():
        path = Path(name)
        require(isinstance(name, str) and name.startswith('rebuild/') and not path.is_absolute()
            and path.as_posix() == name and all(p not in {'', '.', '..'} for p in name.split('/')),
            'unsafe producer path')
        body = producer_bytes[name]
        require(isinstance(expected, dict) and set(expected) == {'bytes', 'sha256'}
            and type(expected['bytes']) is int and expected['bytes'] >= 0
            and isinstance(expected['sha256'], str)
            and re.fullmatch('[0-9a-f]{64}', expected['sha256']) is not None,
            'invalid producer binding')
        require(expected == {'bytes': len(body), 'sha256': operations.digest(body)}, 'producer bytes mismatch')
        current = root
        parts = name.split('/')
        for index, part in enumerate(parts):
            require(current in trees, 'required tree missing')
            used.add(current)
            entry = trees[current].get(part.encode())
            require(entry is not None, 'producer absent from Git tree')
            mode, identity = entry
            if index + 1 == len(parts):
                require(mode in {b'100644', b'100755'} and identity == oid('blob', body),
                    'producer blob or mode mismatch')
            else:
                require(mode == b'40000', 'producer ancestor is not a directory')
                current = identity
    require(used == set(trees), 'unrelated tree retained')
    return {'state': 'verified', 'collection_run_id': value['run_id'],
        'collector_revision': value['collector_revision'], 'producer_files': len(producer_bytes),
        'original_git_required': False}


def retained_bytes(root, value):
    result = {}
    for name in value['producers']:
        path = operations.safe_path(root, 'collection/producers/' + name)
        require(not path.is_symlink(), 'producer symlink')
        result[name] = operations.bounded_read(path)
    return result


def verify(root, value, proof):
    return check(value, proof, retained_bytes(root, value))


def create(root, value, repository):
    """Read local immutable Git objects once, then validate the exact witness."""
    def read(kind, identity, limit):
        require(re.fullmatch('[0-9a-f]{40}', identity) is not None, 'invalid requested object')
        process = subprocess.Popen(['git', 'cat-file', kind, identity], cwd=repository,
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        try:
            require(process.stdout is not None, 'Git object stream missing')
            body = process.stdout.read(limit + 1)
            require(len(body) <= limit, 'Git object exceeds remaining metadata bound')
            require(process.wait(timeout=30) == 0, 'Git object unavailable')
        finally:
            if process.poll() is None:
                process.kill()
            if process.stdout is not None:
                process.stdout.close()
            process.wait(timeout=30)
        require(oid(kind, body) == identity, 'Git object hash mismatch')
        return body
    proof = {'kind': 'dogfood_collector_revision_proof', 'schema_version': 1,
        'collection_run_id': value['run_id'], 'collector_revision': value['collector_revision'],
        'commit': '', 'trees': {}}
    def allowance():
        remaining = operations.MAX_FILE_BYTES - len(operations.encoded(proof))
        require(remaining >= 0, 'proof exceeds metadata bound')
        # Reserve actual JSON structure before reading and base64 expansion.
        return remaining // 4 * 3
    commit = read('commit', value['collector_revision'], allowance())
    proof['commit'] = base64.b64encode(commit).decode()
    first = commit.split(b'\n', 1)[0]
    require(re.fullmatch(b'tree [0-9a-f]{40}', first) is not None, 'commit tree missing')
    root_tree = first[5:].decode()
    trees = {}
    for name in value['producers']:
        current = root_tree
        parts = name.split('/')
        for index, part in enumerate(parts):
            if current not in trees:
                proof['trees'][current] = ''
                body = read('tree', current, allowance())
                proof['trees'][current] = base64.b64encode(body).decode()
                trees[current] = tree_entries(body)
            entry = trees[current].get(part.encode())
            require(entry is not None, 'producer absent from Git tree')
            if index + 1 < len(parts):
                require(entry[0] == b'40000', 'producer ancestor is not a directory')
                current = entry[1]
    verify(root, value, proof)
    return proof
