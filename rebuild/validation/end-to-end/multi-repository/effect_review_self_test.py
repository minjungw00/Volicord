#!/usr/bin/env python3
"""Protocol controls. Real repository effect judgments require original evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import effect_review as review


class EffectReview(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.code = b'function receive() { finish(); }\nfunction finish() { reply(); }\n'
        self.file = self.root / 'source.js'
        self.file.write_bytes(self.code)
        self.binding = dict(source_id='source', source_revision=1, analysis_snapshot_id='analysis',
                            repository_snapshot_id='repository', repository_source_id='repository-source',
                            observation_fingerprint='observation')
        receipt = self.root / 'receipt.json'
        receipt.write_bytes(review.encoded(dict(citation_basis=self.binding, observation=dict(detail=dict(
            body=self.code.decode(), region=dict(start_byte=0), sha256=review.digest(self.code))))))
        source = dict(path=str(self.file), file_sha256=review.digest(self.code),
                      source_id='source', source_revision=1, start_byte=32, end_byte=len(self.code),
                      read_receipt=str(receipt))
        original = self.root / 'original.json'
        original.write_bytes(review.encoded(dict(classification='dependent', scope='exact original',
            reviewer='test author', sources={'callee': source}, expectations=[dict(id='completion',
                condition='receiver called', effect='reply callback invoked', non_effects='no queue operation',
                evidence_scope='static', required_ranges=['callee'])])))
        self.frozen = self.root / 'frozen'
        review.freeze(original, self.frozen)
        text = 'Completion calls reply. 종료.'
        self.publication = dict(explanation_state='current', provenance=dict(language='en', identity='publication'),
                                prose=[dict(text=text, citations=[dict(self.binding, start_byte=0,
                                    end_byte=len(self.code), claim_start_byte=0, claim_end_byte=len(text.encode()),
                                    range_sha256='sha256:' + review.digest(self.code),
                                    claim_sha256='sha256:' + review.digest(text.encode()))])])
        self.response = dict(reviewer='attributed reader', isolation='dependent', expectations=dict(completion=[dict(
            unit_index=0, start_byte=0, end_byte=22, quote='Completion calls reply', verdict='supported',
            reason='Inspected callee invokes callback.', evidence_scope='static')]))

    def prepare(self, name='attempt', publication=None):
        publication = publication or self.publication
        public_path = self.root / (name + '.json')
        public_path.write_bytes(review.encoded(publication))
        attempt = self.root / name
        review.prepare(self.frozen, public_path, attempt)
        return attempt

    def assess(self, attempt, response=None):
        response = copy.deepcopy(response or self.response)
        response['publication_sha256'] = review.digest((attempt / 'publication.json').read_bytes())
        path = self.root / (attempt.name + '-response.json')
        path.write_bytes(review.encoded(response))
        return review.assess(attempt, path)

    def test_supported_comparison_and_no_retry_overwrite(self):
        attempt = self.prepare()
        self.assertEqual(self.assess(attempt)['status'], 'passed')
        original = {p.name: p.read_bytes() for p in attempt.iterdir()}
        with self.assertRaises(FileExistsError):
            self.assess(attempt)
        self.assertEqual(original, {p.name: p.read_bytes() for p in attempt.iterdir()})
        with self.assertRaises(FileExistsError):
            review.prepare(self.frozen, self.root / 'attempt.json', attempt)

    def test_caller_only_cannot_support_callee_effect(self):
        publication = copy.deepcopy(self.publication)
        citation = publication['prose'][0]['citations'][0]
        citation['end_byte'] = 31
        citation['range_sha256'] = 'sha256:' + review.digest(self.code[:31])
        result = self.assess(self.prepare(publication=publication))
        self.assertIn('missing effect-producing range callee', str(result))

    def test_contradiction_and_absent_observation_are_not_passes(self):
        for verdict in ('contradicted', 'unestablished'):
            response = copy.deepcopy(self.response)
            response['expectations']['completion'][0]['verdict'] = verdict
            self.assertEqual(self.assess(self.prepare(verdict), response)['status'], 'failed')
        response = copy.deepcopy(self.response)
        response['expectations']['completion'][0]['evidence_scope'] = 'observed'
        self.assertIn('evidence scope inflated', str(self.assess(self.prepare('scope'), response)))

    def test_changed_source_and_inputs_require_new_assessment(self):
        attempt = self.prepare()
        self.file.write_bytes(self.code + b'// changed\n')
        self.assertIn('Source changed', str(self.assess(attempt)))
        self.file.write_bytes(self.code)
        second = self.prepare('second')
        (second / 'publication.json').write_bytes(b'{}')
        self.assertIn('assessment input changed', str(self.assess(second)))

    def test_exact_claim_identity_utf8_and_hashes(self):
        for name, mutation in [('quote', lambda v: v.update(quote='different claim')),
                               ('utf8', lambda v: v.update(end_byte=25))]:
            response = copy.deepcopy(self.response)
            mutation(response['expectations']['completion'][0])
            self.assertEqual(self.assess(self.prepare(name), response)['status'], 'failed')
        publication = copy.deepcopy(self.publication)
        publication['prose'][0]['citations'][0]['claim_sha256'] = 'sha256:forged'
        self.assertIn('citation code/claim bytes changed', str(self.assess(self.prepare('hash', publication))))

    def test_whole_file_work_receipts_need_no_region(self):
        original = json.loads((self.root / 'original.json').read_bytes())
        receipt = Path(original['sources']['callee']['read_receipt'])
        body = json.loads(receipt.read_bytes())
        del body['observation']['detail']['region']
        receipt.write_bytes(review.encoded(body))
        review.freeze(self.root / 'original.json', self.root / 'whole-file')
        self.assertEqual(review.verified_basis(self.root / 'whole-file')['sources']['callee']['start_byte'], 32)

    def test_missing_effect_and_missing_review_are_explicit_failures(self):
        response = copy.deepcopy(self.response)
        response['expectations']['completion'] = [dict(verdict='not_asserted', reason='Not explained.')]
        self.assertIn('required explanation absent', str(self.assess(self.prepare(), response)))
        response['expectations'] = {}
        self.assertIn('unreviewed', str(self.assess(self.prepare('missing'), response)))


if __name__ == '__main__':
    unittest.main()
