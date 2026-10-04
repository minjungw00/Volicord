"""Independent schema-3 tick/instance controls; authored verifier support only."""
import unittest
import tempfile
from pathlib import Path

import resource_observer as o


def observation(processes=1, ticks=7, candidate='a' * 64, purpose='dogfood_rehearsal'):
    """Current-contract primitive fixture, independent of observe/coverage_errors."""
    value = o.initial({'volicord-mcp': {'sha256': candidate}}, purpose=purpose)
    value.update(status='measured', interval_ns=50_000_000, duration_ns=ticks * 50_000_000,
        termination='duration_elapsed', observer_lifecycle='stopped')
    value['runtimes'] = [{'runtime_binding': 'b' * 64, 'initial_expectation': 'unknown'}]
    for index in range(processes):
        ident = format(index + 1, '032x')
        identity = dict(kind='volicord_mcp_lifecycle', schema_version=1, instance_id=ident,
            pid=index + 1, boot_id='00000000-0000-0000-0000-000000000000', start_ticks=index + 1,
            executable_sha256=candidate, executable_path='/support/volicord-mcp',
            runtime_binding='b' * 64, cwd_binding='c' * 64, host_session=ident,
            host_session_authority='server_generated_correlation_only', state='running',
            registration_duration_ns=1, lifetime_ns=None)
        value['instances'].append({'identity': identity, 'samples': [
            {'elapsed_ns': tick * 50_000_000 + index + 1, 'rss_bytes': 1024 * (tick + index + 1)}
            for tick in range(ticks)], 'lifecycle': 'running_at_detach', 'errors': [], 'binding_state': 'verified'})
    for tick in range(ticks):
        ids = [i['identity']['instance_id'] for i in value['instances']]
        value['ticks'].append({'elapsed_ns': tick * 50_000_000, 'errors': [], 'runtimes': [{
            'runtime_binding': 'b' * 64, 'expectation': 'unknown', 'authority': 'none',
            'registered': [{'instance_id': ident, 'state': 'running'} for ident in ids], 'sampled': ids}]})
    derived(value)
    return value


def derived(value):
    # Independent expected derivation: mutation controls do not call coverage_errors.
    numbers = [s['rss_bytes'] for i in value['instances'] for s in i['samples']]
    value['measurement'].update(sample_count=len(numbers), peak_rss_bytes=max(numbers) if numbers else None)
    return value


def omit(value, tick=3, instance=0):
    ident = value['instances'][instance]['identity']['instance_id']
    value['instances'][instance]['samples'].pop(tick)
    value['ticks'][tick]['runtimes'][0]['sampled'].remove(ident)
    return derived(value)


class CoverageTests(unittest.TestCase):
    def test_single_and_concurrent_complete(self):
        for count in (1, 2):
            value = observation(count)
            self.assertEqual(o.validate(value)['status'], 'measured')

    def test_missing_single_or_concurrent_sample_even_with_recomputed_derivations(self):
        for count in (1, 2):
            value = omit(observation(count))
            with self.subTest(processes=count), self.assertRaisesRegex(o.ObservationError, 'running instance lacks tick sample'):
                o.validate(value)
            # A global or lifetime error cannot stand in for this tick/instance failure.
            value['ticks'][3]['errors'] = ['inaccessible']
            value['instances'][0]['errors'] = ['inaccessible']
            value['status'] = 'partial'
            value['measurement']['measurement_errors'] = ['inaccessible']
            with self.assertRaisesRegex(o.ObservationError, 'running instance lacks tick sample'):
                o.validate(value)

    def test_explicit_scoped_sampling_failure_is_partial(self):
        value = omit(observation(2))
        value['ticks'][3]['runtimes'][0]['registered'][0]['state'] = 'inaccessible'
        value['instances'][0]['errors'] = ['inaccessible']
        value['measurement']['measurement_errors'] = ['inaccessible']
        value['status'] = 'partial'
        self.assertEqual(o.validate(value)['status'], 'partial')
        value['status'] = 'measured'
        with self.assertRaises(o.ObservationError): o.validate(value)

    def test_active_empty_waiting_and_sequential_future_homes(self):
        value = observation()
        empty = {'runtime_binding': 'd' * 64, 'initial_expectation': 'waiting'}
        value['runtimes'].append(empty)
        for tick in value['ticks']:
            tick['runtimes'].append({'runtime_binding': 'd' * 64, 'expectation': 'waiting',
                'authority': 'operator', 'registered': [], 'sampled': []})
        self.assertEqual(o.validate(value)['status'], 'measured')
        value['ticks'][2]['runtimes'][1]['expectation'] = 'active'
        value['status'] = 'partial'; value['measurement']['measurement_errors'] = ['missing_registration']
        self.assertEqual(o.validate(value)['status'], 'partial')
        waiting = observation(0)
        waiting['runtimes'][0]['initial_expectation'] = 'waiting'
        for tick in waiting['ticks']:
            tick['runtimes'][0].update(expectation='waiting', authority='operator')
        waiting['status'] = 'not_observed'
        self.assertEqual(o.validate(waiting)['measurement']['measurement_errors'], [])

    def test_confirmed_stop_does_not_cover_an_earlier_missing_running_sample(self):
        value = observation()
        instance = value['instances'][0]
        instance['identity'].update(state='stopped', lifetime_ns=1)
        instance['lifecycle'] = 'stopped'
        instance['samples'] = instance['samples'][:4]
        for tick in value['ticks'][4:]:
            tick['runtimes'][0]['sampled'] = []
            tick['runtimes'][0]['registered'][0]['state'] = 'stopped'
        derived(value)
        self.assertEqual(o.validate(value)['status'], 'measured')
        omit(value, tick=2)
        with self.assertRaisesRegex(o.ObservationError, 'running instance lacks tick sample'): o.validate(value)

    def test_gone_and_interrupted_preserve_useful_samples_as_partial(self):
        value = observation()
        instance = value['instances'][0]
        instance['samples'] = instance['samples'][:4]
        instance['lifecycle'] = 'gone'; instance['errors'] = ['process_gone']
        for tick in value['ticks'][4:]:
            tick['runtimes'][0]['sampled'] = []
            tick['runtimes'][0]['registered'][0]['state'] = 'gone'
        derived(value); value['status'] = 'partial'
        value['measurement']['measurement_errors'] = ['process_gone']
        self.assertEqual(o.validate(value)['status'], 'partial')
        value['instances'][0]['lifecycle'] = 'stopped'
        with self.assertRaises(o.ObservationError): o.validate(value)
        value = observation(); value['termination'] = 'interrupted'; value['status'] = 'partial'
        value['measurement']['measurement_errors'] = ['observer_interrupted']
        self.assertEqual(o.validate(value)['measurement']['sample_count'], 7)

    def test_wrong_instance_runtime_tick_and_candidate_cannot_supply_sample(self):
        for mutation in ('instance', 'runtime', 'tick', 'candidate', 'registration', 'reference_only', 'array_only'):
            value = observation(2)
            if mutation == 'instance': value['ticks'][3]['runtimes'][0]['sampled'][0] = 'f' * 32
            elif mutation == 'runtime': value['instances'][0]['identity']['runtime_binding'] = 'f' * 64
            elif mutation == 'tick': value['instances'][0]['samples'][3]['elapsed_ns'] = value['ticks'][4]['elapsed_ns']
            elif mutation == 'candidate': value['instances'][0]['identity']['executable_sha256'] = 'f' * 64
            elif mutation == 'registration': value['instances'][0]['identity']['instance_id'] = 'f' * 32
            elif mutation == 'reference_only': value['ticks'][3]['runtimes'][0]['sampled'].pop(0)
            elif mutation == 'array_only': value['instances'][0]['samples'].pop(3); derived(value)
            with self.subTest(mutation=mutation), self.assertRaises(o.ObservationError): o.validate(value)


class ConsumerTests(unittest.TestCase):
    def test_campaign_loader_rejects_recomputed_partial_instance_sampling(self):
        import campaign as c
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            resource = observation(2, purpose='naturalistic')
            value = {'kind': 'phase8_dogfood_campaign', 'schema_version': 9,
                'campaign_root': str(root), 'evidence_purpose': 'naturalistic',
                'candidate_artifacts': {'volicord-mcp': {'sha256': 'a' * 64}},
                'naturalistic_memory_evidence': resource}
            value['live_evidence_obligations'] = c.live_evidence_obligations(value['candidate_artifacts'], resource)
            c.write_json(root / 'campaign.json', value)
            c.load_campaign(root)
            omit(resource)
            c.write_json(root / 'campaign.json', value)
            with self.assertRaisesRegex(c.CampaignError, 'invalid naturalistic MCP resource evidence'):
                c.load_campaign(root)



if __name__ == '__main__': unittest.main()
