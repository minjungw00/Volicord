"""Authored old-candidate support; never measured user evidence."""
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import collection_runs as runs
import harness
import machine_findings as machine
import review_operations


class CollectionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.binary = self.parent / 'candidate/bin/volicord'
        fixtures.write_fake_binary(self.binary)
        old = '12' * 20
        for target, name, value in ((harness, 'git_clean', lambda _: True),
                (c, 'revision_is_bound', fixtures.fixture_revision_is_bound),
                (c, 'committed_delta_paths', fixtures.fixture_committed_delta_paths),
                (c.repository_state, 'observe', fixtures.observe_fixture_repository),
                (c.repository_state, 'commit_history', fixtures.fixture_commit_history)):
            replacement = patch.object(target, name, value)
            replacement.start()
            self.addCleanup(replacement.stop)
        # Authored target history is a fixture, not a checkout of the real campaign.
        with patch.object(harness, 'git_head', return_value=old):
            self.source, self.raw, self.bundles = fixtures.prepared_batch(self.parent, 'source', self.binary)
        for journey in c.load_campaign(self.source)['journeys'].values():
            runtime = Path(journey['runtime_home'])
            (runtime / 'canonical.sqlite3').write_bytes(b'Authored canonical seam, not a Product database')
        self.rejected = self.source / 'steward/rejected.json'
        c.write_json(self.rejected, {'kind': 'dogfood_integrity_rejection', 'collection_state': 'rejected',
            'qualification_state': 'not_run', 'candidate_head': old})
        self.before = runs.tree_identity(self.source)
        self.hash = harness.sha256(self.source / 'campaign.json')
        self.output = self.parent / 'publication'
        self.expected_collector = harness.git_head(c.ROOT)
        self.expected_producers = {name: runs.binding(path) for name, path in runs.producer_paths().items()}

        def authored_revision(value):
            # No fake Git blob history is asserted. This seam independently binds
            # the authored fixture to exact currently running source hashes.
            self.assertEqual(value['collector_revision'], self.expected_collector)
            self.assertEqual(value['producers'], self.expected_producers)
        replacement = patch.object(runs, 'verify_collector_revision', authored_revision)
        replacement.start()
        self.addCleanup(replacement.stop)

    def process(self, **changes):
        observed = []
        exporter = fixtures.batch_exporter(self.bundles)

        def export(binary, runtime, repository, destination):
            self.assertEqual(binary, self.binary)
            self.assertTrue(runtime.is_relative_to(self.output.parent))
            self.assertFalse(runtime.is_relative_to(self.source))
            self.assertEqual((runtime / 'canonical.sqlite3').read_bytes(), b'Authored canonical seam, not a Product database')
            observed.append((binary, runtime))
            exporter(binary, runtime, repository, destination)
        options = {'source_sha256': self.hash, 'rejected': [self.rejected],
            'exporter': export, 'documenter': fixtures.documenter, 'snapshotter': fixtures.snapshotter}
        options.update(changes)
        result = runs.reprocess(self.source, self.raw, self.output, **options)
        self.assertEqual(runs.tree_identity(self.source), self.before)
        return result, observed

    def test_old_candidate_new_collector_immutable_publication_and_copy(self):
        result, observed = self.process()
        self.assertEqual(result['collection_state'], 'collected')
        self.assertEqual(len(observed), 3)
        self.assertEqual(c.load_campaign(self.source)['collection_state'], 'pending')
        manifest = c.load_evidence_set(self.output)
        run = runs.verify(self.output, manifest)
        self.assertEqual(run['candidate_head'], '12' * 20)
        self.assertEqual(run['collector_revision'], self.expected_collector)
        self.assertNotEqual(run['candidate_head'], run['collector_revision'])
        self.assertEqual((self.output / 'collection/inputs/rejected/0').read_bytes(), self.rejected.read_bytes())
        first = c.evaluate_campaign(self.output, self.parent / 'evaluation-1')
        second = c.evaluate_campaign(self.output, self.parent / 'evaluation-2')
        self.assertNotEqual(first['run_id'], second['run_id'])
        copied = self.parent / 'copied'
        shutil.copytree(self.output, copied)
        self.source.rename(self.parent / 'source-unavailable')
        self.output.rename(self.parent / 'publication-unavailable')
        self.assertEqual(runs.verify_publication(copied)['state'], 'verified')

    def test_wrong_source_hash_fails_before_product_reads(self):
        with self.assertRaisesRegex(c.CampaignError, 'source campaign hash'):
            self.process(source_sha256='00' * 32)
        self.assertFalse(self.output.exists())

    def test_wrong_candidate_binary_retains_normalization_stops_dependents(self):
        self.binary.write_bytes(b'changed executable')
        result, observed = self.process()
        self.assertEqual(observed, [])
        self.assertEqual(result['collection_state'], 'incomplete')
        self.assertIn('content mismatch', result['blocker']['detail'])
        self.assertEqual(c.read_json(self.output / 'collection/normalization.json')['candidate_head'], '12' * 20)
        for stage in result['dependent_stages'].values():
            self.assertEqual(stage['status'], 'not_run')
            self.assertEqual(stage['invocation_count'], 0)
            self.assertEqual(stage['prerequisite']['reason'], result['blocker'])
        with self.assertRaises(c.CampaignError) as error:
            c.evaluate_campaign(self.output)
        self.assertEqual(error.exception.diagnostic['status'], 'not_run')
        self.assertEqual(runs.verify_publication(self.output)['state'], 'verified')
        publication = self.output / 'collection/publication.json'
        publication.chmod(0o600)
        altered = c.read_json(publication)
        altered['dependent_stages']['evaluation'].update(status='executed', invocation_count=1)
        altered['publication_id'] = machine.digest({k: v for k, v in altered.items() if k != 'publication_id'})
        c.write_json(publication, altered)
        with self.assertRaisesRegex(c.CampaignError, 'without evidence-set prerequisite'):
            runs.verify_publication(self.output)

    def test_source_mutation_during_product_read_rejects_publication(self):
        exporter = fixtures.batch_exporter(self.bundles)

        def mutate(*args):
            self.rejected.write_bytes(b'changed historical diagnostic')
            exporter(*args)
        with self.assertRaisesRegex(c.CampaignError, 'source or collector changed'):
            runs.reprocess(self.source, self.raw, self.output, source_sha256=self.hash,
                rejected=[self.rejected], exporter=mutate, documenter=fixtures.documenter,
                snapshotter=fixtures.snapshotter)
        self.assertFalse(self.output.exists())

    def reject_run_mutation(self, key, changed):
        self.process()
        manifest = c.load_evidence_set(self.output)
        original = c.read_json(self.output / 'collection/run.json')
        (self.output / 'collection/run.json').chmod(0o600)
        altered = copy.deepcopy(original)
        altered[key] = changed
        altered['run_id'] = machine.digest({k: v for k, v in altered.items() if k != 'run_id'})
        c.write_json(self.output / 'collection/run.json', altered)
        manifest['collection_run'].update(run_id=altered['run_id'], sha256=harness.sha256(self.output / 'collection/run.json'))
        with self.assertRaises((ValueError, AssertionError)):
            runs.verify(self.output, manifest)

    def test_candidate_rebinding_rejected(self):
        self.reject_run_mutation('candidate_head', '34' * 20)

    def test_collector_tampering_rejected(self):
        self.reject_run_mutation('collector_revision', '56' * 20)

    def test_historical_rejection_omission_rejected(self):
        # Even retained unindexed source diagnostics require explicit references.
        self.reject_run_mutation('rejected_attempts', ['rejected/0'])

    def test_raw_mutation_rejected(self):
        self.process()
        manifest = c.load_evidence_set(self.output)
        raw = next((self.output / 'collection/inputs/raw').glob('*.jsonl'))
        raw.chmod(0o600)
        raw.write_bytes(raw.read_bytes() + b'\n')
        with self.assertRaisesRegex(c.CampaignError, 'retained input/producer changed'):
            runs.verify(self.output, manifest)


if __name__ == '__main__':
    unittest.main()
