"""Read-only stdio MCP for the frozen experiment; no Product or provider calls."""
from __future__ import annotations

import argparse
import fcntl
import json
from pathlib import Path
import sys
import time

from inputs import Reader, encoded, generation_inventory, require, verify


class Surface:
    def __init__(self, manifest, lane, budgets, trace, protocol_trace=None, *, stage=None,
                 read_deadline_monotonic=None, execution_deadline_monotonic=None):
        self.spec = verify(manifest)
        Reader(self.spec, lane, max_reads=budgets['reads'], max_bytes=budgets['read_bytes'])
        self.lane, self.budgets, self.trace = lane, budgets, Path(trace)
        self.protocol_trace = Path(protocol_trace) if protocol_trace else None
        require((read_deadline_monotonic is None and execution_deadline_monotonic is None) or
                (type(read_deadline_monotonic) in {int, float} and type(execution_deadline_monotonic) in {int, float}
                 and read_deadline_monotonic <= execution_deadline_monotonic), 'invalid evidence time allocation')
        self.stage, self.read_deadline = stage, read_deadline_monotonic
        self.execution_deadline = execution_deadline_monotonic

    def call(self, name, arguments, request_id=None):
        # All calls, including failed lookups and inventory pages, share a locked
        # ledger across stages and MCP restarts. Denials cannot reset the budget.
        with self.trace.open('a+b') as stream:
            fcntl.flock(stream, fcntl.LOCK_EX)
            stream.seek(0)
            history = [json.loads(line) for line in stream]
            reads = len(history)
            used = sum(row.get('charged_bytes', 0) for row in history)
            row = {'sequence': reads, 'name': name, 'arguments': arguments}
            started = time.monotonic()
            if request_id is not None:
                row['mcp_request_id'] = request_id
            try:
                require(isinstance(arguments, dict), 'tool arguments must be an object')
                require(reads < self.budgets['reads'], 'read budget exhausted')
                require(self.read_deadline is None or started < self.read_deadline, 'read time budget exhausted')
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
                           status='returned', outcome='source_returned' if name == 'read' and data else
                           'empty_read' if name == 'read' else 'inventory_returned')
            except (ValueError, KeyError, TypeError, OSError) as error:
                message = str(error)
                outcome = ('source_unavailable' if message == 'bytes unavailable' or isinstance(error, OSError)
                           else 'evidence_changed' if message == 'changed bytes or stale preparation'
                           else 'budget_exhausted' if message in {'read budget exhausted', 'read time budget exhausted'}
                           else 'invalid_utf8' if isinstance(error, UnicodeError) else 'policy_denied')
                row.update(status='failed', outcome=outcome, error=message, charged_bytes=0)
            if self.read_deadline is not None:
                ended = time.monotonic()
                row['execution'] = {'stage': self.stage, 'started_monotonic': started, 'ended_monotonic': ended,
                    'reads_remaining': max(0, self.budgets['reads'] - reads - 1),
                    'bytes_remaining': max(0, self.budgets['read_bytes'] - used - row['charged_bytes']),
                    'evidence_seconds_remaining': max(0, self.read_deadline - ended),
                    'total_seconds_remaining': max(0, self.execution_deadline - ended)}
            stream.seek(0, 2)
            # Escaping also preserves malformed surrogate arguments as evidence
            # without letting UTF-8 serialization kill the server after denial.
            stream.write(json.dumps(row, ensure_ascii=True).encode() + b'\n')
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
        request, response = None, None
        try:
            request = json.loads(line)
            require(isinstance(request, dict), 'request must be an object')
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
                row = surface.call(params['name'], params.get('arguments', {}), request['id'])
                result = {'content': [{'type': 'text', 'text': json.dumps(row, ensure_ascii=True)}],
                          'isError': row['status'] == 'failed'}
            elif method == 'ping':
                result = {}
            else:
                response = {'jsonrpc': '2.0', 'id': request['id'],
                            'error': {'code': -32601, 'message': 'method unavailable'}}
            if response is None:
                response = {'jsonrpc': '2.0', 'id': request['id'], 'result': result}
        except (ValueError, KeyError, TypeError) as error:
            response = {'jsonrpc': '2.0', 'id': request.get('id') if isinstance(request, dict) else None,
                        'error': {'code': -32700 if isinstance(error, json.JSONDecodeError) else -32602,
                                  'message': 'malformed request'}}
        finally:
            if surface.protocol_trace is not None:
                with surface.protocol_trace.open('ab') as trace:
                    fcntl.flock(trace, fcntl.LOCK_EX)
                    trace.write(json.dumps({'request': request, 'response': response}, ensure_ascii=True).encode() + b'\n')
            if response is not None:
                outgoing.write(json.dumps(response, ensure_ascii=True) + '\n')
                outgoing.flush()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('configuration', type=Path)
    configuration = json.loads(parser.parse_args().configuration.read_bytes())
    serve(Surface(**configuration))
