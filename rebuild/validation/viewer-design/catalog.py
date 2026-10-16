"""Read-only review catalog from frozen canonical export records and a document quote.

This is disposable validation support, not a Product importer or storage decoder.
Titles/summaries are explicitly authored review input, never extracted from prototype prose.
"""
import hashlib
import json
from pathlib import Path
import re


def verified(path, digest):
    raw = Path(path).read_bytes()
    if hashlib.sha256(raw).hexdigest() != digest.removeprefix('sha256:'):
        raise ValueError('catalog source identity changed')
    return raw


def string_list(raw):
    data = bytes.fromhex(raw)
    if len(data) < 8:
        raise ValueError('invalid historical string list')
    count = int.from_bytes(data[:8], 'big'); offset = 8; values = []
    if count > 1024:
        raise ValueError('historical list bound')
    for _ in range(count):
        if offset + 8 > len(data):
            raise ValueError('invalid historical list length')
        size = int.from_bytes(data[offset:offset+8], 'big'); offset += 8
        if offset + size > len(data):
            raise ValueError('invalid historical list body')
        values.append(data[offset:offset+size].decode()); offset += size
    if offset != len(data):
        raise ValueError('trailing historical list bytes')
    return values


def admit(path):
    spec = json.loads(Path(path).read_text())
    raw = verified(spec['bundle']['input'], spec['bundle']['sha256'])
    bundle = json.loads(raw)
    tables = {}
    for table in bundle['payload']['tables']:
        if table['name'] not in {'projects','context_items','checkpoints'}:
            continue
        tables[table['name']] = [{key: value.get('value') for key,value in zip(table['columns'],row,strict=True)} for row in table['rows']]
    purpose = spec['purpose']
    document = verified(purpose['input'], purpose['sha256']).decode()
    if not purpose['text'] or purpose['text'] not in document.split('\n\n'):
        raise ValueError('project goal must be an exact independent document paragraph')
    projects = [p for p in tables['projects'] if p['id'] == spec['project_id']]
    if len(projects) != 1:
        raise ValueError('wrong catalog Project')
    keys = set(); items = []
    for authored in spec['works']:
        key = authored['key']
        if not re.fullmatch('[a-z][a-z0-9-]{0,31}',key) or key in keys:
            raise ValueError('invalid or duplicate catalog key')
        keys.add(key)
        goals = [r for r in tables['context_items'] if r['id'] == authored['work'] and r['role'] == 'goal' and r['project_id'] == spec['project_id']]
        checkpoints = [r for r in tables['checkpoints'] if r['work_item_id'] == authored['work'] and r['project_id'] == spec['project_id']]
        if len(goals) != 1 or not checkpoints:
            raise ValueError('wrong Work or unavailable checkpoint')
        latest = max(checkpoints,key=lambda r:r['recorded_at'])
        if latest['id'] != authored['checkpoint']:
            raise ValueError('catalog checkpoint is not the latest retained record')
        if latest['work_state'] not in {'completed','in_progress','paused','open'}:
            raise ValueError('unsupported recorded Work state')
        if not authored['title'] or not authored['summary']:
            raise ValueError('explicit authored title and summary required')
        items.append(dict(authored=authored,goal=goals[0],checkpoint=latest,
                          limits=string_list(latest['known_limits']),non_goals=string_list(latest['non_goals'])))
    return dict(spec=spec,items=items,bundle_bytes=raw,purpose_bytes=document.encode())
