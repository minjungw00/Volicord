"""Experimental in-memory citation handles; no Source API or historical rewriting."""
import copy
import json
from pathlib import Path

from approaches import events, retrieval_audit
from grounding import validate_output
from inputs import binding, check_binding, digest, encoded, require
from reading_input import verify_reading_input


class VerifiedReferences:
    """The caller chooses Source and range. Only byte metadata is derived here.

    Handles are local to this observation and instance, are not persisted, and
    attest neither relevance, entailment, user authority nor semantic truth.
    """
    def __init__(self, attempt):
        self._attempt = binding(attempt)
        self._issued = {}
        self._basis()

    def _basis(self):
        check_binding(self._attempt)
        record = json.loads(Path(self._attempt['path']).read_bytes())
        check_binding(record['input'])
        spec = verify_reading_input(record['input']['path'])
        require(record['scope'] == spec['scope'], 'reference observation scope changed')
        check_binding(record['retrievals'])
        rows, invalid = events(record['retrievals']['path'])
        require(not invalid, 'malformed reference ledger')
        calls = []
        for call in record['calls']:
            if call['kind'] != 'model_call':
                continue
            check_binding(call['process']['stdout'])
            public, invalid = events(call['process']['stdout']['path'])
            require(not invalid, 'malformed reference Host observations')
            calls.extend(e['item'] for e in public if e.get('type') == 'item.completed'
                         and e.get('item', {}).get('type') == 'mcp_tool_call')
        audit = retrieval_audit(spec, record['lane'], calls, rows, budgets=record.get('budgets'))
        require(not audit['issues'], 'unverified reference reads: ' + '; '.join(audit['issues']))
        verified = {o['sequence'] for o in audit['outcomes'] if o['outcome'] == 'source_read_verified'}
        return record, spec, [r for r in rows if r['sequence'] in verified]

    def issue(self, identity, start, end):
        """Issue a handle for the caller's exact selected UTF-8 byte range."""
        from grounding import source_state
        record, spec, rows = self._basis()
        entries = {e['id']: e for e in spec['entries']
                   if record['lane'] == 'archive_diagnostic' or e['lane'] == 'product'}
        require(isinstance(identity, str) and identity in entries, 'reference outside permitted inventory')
        entry = entries[identity]
        require(entry['asset'] is not None, 'unavailable source')
        require(type(start) is int and type(end) is int and 0 <= start < end <= entry['asset']['bytes'],
                'invalid byte span')
        selected = Path(entry['asset']['path']).read_bytes()[start:end]
        selection = {'id': identity, 'start': start, 'end': end, 'sha256': digest(selected),
                     'state': source_state(entry)}
        self._validate(selection, spec, record['lane'], rows)
        handle = 'reference-' + digest(encoded({'observation': self._attempt, 'input': record['input'],
                                               'lane': record['lane'], 'selection': selection}))
        self._issued[handle] = copy.deepcopy(selection)
        return handle

    @staticmethod
    def _validate(selection, spec, lane, rows):
        validation = validate_output({'prose': 'Citation metadata only.', 'selections': [selection], 'gaps': []},
                                     spec, lane, rows)
        result = validation['selections'][0]
        require(result['reference_status'] == 'valid_reference', '; '.join(result['issues']))

    def resolve(self, handle):
        require(isinstance(handle, str) and handle in self._issued, 'unknown or foreign reference handle')
        record, spec, rows = self._basis()
        selection = self._issued[handle]
        self._validate(selection, spec, record['lane'], rows)
        return copy.deepcopy(selection)

    def sidecar(self, prose, handles, gaps, *, claims=None, primary_sites=None):
        """New experimental response only; caller retains prose and claim choices."""
        require(isinstance(handles, list), 'ordered reference handles required')
        record, spec, rows = self._basis()
        response = {'prose': prose, 'selections': [self.resolve(h) for h in handles], 'gaps': copy.deepcopy(gaps)}
        if claims is not None or primary_sites is not None:
            response.update(claims=copy.deepcopy(claims), primary_sites=copy.deepcopy(primary_sites))
        validation = validate_output(response, spec, record['lane'], rows, contract=record.get('output_contract'))
        require(validation.get('reading', {}).get('status', 'valid_binding') == 'valid_binding',
                'reference handles cannot supply unsupported claim authority')
        return response, validation
