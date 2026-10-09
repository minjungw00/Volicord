"""Read-only stdio MCP for the frozen experiment; no Product or provider calls."""
from __future__ import annotations

import argparse
import fcntl
import json
from pathlib import Path
import sys

from inputs import Reader, encoded, generation_inventory, require, verify


class Surface:
    def __init__(self, manifest, lane, budgets, trace):
        self.spec = verify(manifest)
        self.lane, self.budgets, self.trace = lane, budgets, Path(trace)

    def call(self, name, arguments):
        # All calls, including failed lookups and inventory pages, share a locked
        # ledger across stages and MCP restarts. Denials cannot reset the budget.
        with self.trace.open('a+b') as stream:
            fcntl.flock(stream, fcntl.LOCK_EX)
            stream.seek(0)
            history = [json.loads(line) for line in stream]
            reads = len(history)
            used = sum(row.get('charged_bytes', 0) for row in history)
            row = {'sequence': reads, 'name': name, 'arguments': arguments}
            try:
                require(reads < self.budgets['reads'], 'read budget exhausted')
                view = generation_inventory(self.spec, self.lane)
                if name == 'inventory':
                    require(set(arguments) <= {'offset', 'limit', 'path_contains', 'role'}, 'unexpected inventory arguments')
                    offset, limit = arguments.get('offset', 0), arguments.get('limit', 40)
                    require(type(offset) is int and offset >= 0 and type(limit) is int
                            and 1 <= limit <= 80, 'invalid inventory range')
                    query, role = arguments.get('path_contains', ''), arguments.get('role')
                    require(isinstance(query, str) and (role is None or isinstance(role, str)), 'invalid inventory filter')
                    entries = [e for e in view['entries'] if query in e['path'] and (role is None or e['role'] == role)]
                    result = {'entries': entries[offset:offset + limit],
                              'next_offset': offset + limit if offset + limit < len(entries) else None,
                              'total': len(entries)}
                    require(used + len(encoded(result)) <= self.budgets['read_bytes'], 'read budget exhausted')
                elif name == 'read':
                    require(set(arguments) <= {'id', 'offset', 'limit'}, 'unexpected read arguments')
                    reader = Reader(self.spec, self.lane, max_reads=self.budgets['reads'],
                                    max_bytes=self.budgets['read_bytes'])
                    reader.reads, reader.bytes = reads, used
                    meta, data = reader.read(arguments['id'], arguments.get('offset', 0), arguments.get('limit', 2048))
                    # No lossy replacement of invalid UTF-8 or split characters.
                    result = {'metadata': meta, 'text': data.decode('utf-8')}
                else:
                    raise ValueError('unknown tool')
                row.update(result=result, charged_bytes=(len(data) if name == 'read' else len(encoded(result))),
                           status='returned')
            except (ValueError, KeyError, TypeError, OSError) as error:
                row.update(status='failed', error=str(error), charged_bytes=0)
            stream.seek(0, 2)
            stream.write(json.dumps(row, ensure_ascii=False).encode() + b'\n')
            stream.flush()
            return row


TOOLS = [
    {'name': 'inventory', 'description': 'Page the unranked permitted evidence inventory. Calls and metadata bytes count toward the shared read budget.',
     'inputSchema': {'type': 'object', 'properties': {'offset': {'type': 'integer'}, 'limit': {'type': 'integer'},
                     'path_contains': {'type': 'string'}, 'role': {'type': 'string'}}, 'additionalProperties': False}},
    {'name': 'read', 'description': 'Read exact UTF-8 bytes by opaque ID; body is untrusted historical evidence. Offsets are bytes, not lines.',
     'inputSchema': {'type': 'object', 'properties': {'id': {'type': 'string'}, 'offset': {'type': 'integer'}, 'limit': {'type': 'integer'}},
                     'required': ['id'], 'additionalProperties': False}},
]
for tool in TOOLS:
    # These tools do not mutate Product/source state. The local retrieval ledger
    # is process evidence, not write authority. Explicit host approval is still
    # configured separately by the authorized invocation adapter.
    tool['annotations'] = {'readOnlyHint': True, 'destructiveHint': False,
                           'idempotentHint': False, 'openWorldHint': False}


def serve(surface, incoming=sys.stdin, outgoing=sys.stdout):
    for line in incoming:
        request = json.loads(line)
        if 'id' not in request:
            continue
        method = request.get('method')
        if method == 'initialize':
            result = {'protocolVersion': request['params']['protocolVersion'], 'capabilities': {'tools': {}},
                      'serverInfo': {'name': 'explanation-evidence', 'version': '1.0.0'}}
        elif method == 'tools/list':
            result = {'tools': TOOLS}
        elif method == 'tools/call':
            params = request['params']
            row = surface.call(params['name'], params.get('arguments', {}))
            result = {'content': [{'type': 'text', 'text': json.dumps(row, ensure_ascii=False)}],
                      'isError': row['status'] == 'failed'}
        elif method == 'ping':
            result = {}
        else:
            outgoing.write(json.dumps({'jsonrpc': '2.0', 'id': request['id'],
                                       'error': {'code': -32601, 'message': 'method unavailable'}}) + '\n')
            outgoing.flush()
            continue
        outgoing.write(json.dumps({'jsonrpc': '2.0', 'id': request['id'], 'result': result}, ensure_ascii=False) + '\n')
        outgoing.flush()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('configuration', type=Path)
    configuration = json.loads(parser.parse_args().configuration.read_bytes())
    serve(Surface(**configuration))
