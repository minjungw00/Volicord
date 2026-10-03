"""Candidate-bound Linux MCP observer. Never launches, signals or traces the MCP.

Only candidate-owned Runtime lifecycle entries select PIDs. Process start identity
and /proc executable bytes are verified before RSS; process names are not used.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import time

SCHEMA = 2
MAX_INSTANCES = 1024
MAX_TICKS = 20000
MAX_SAMPLES = 20000
STATUSES = {'not_observed', 'measured', 'partial', 'failed', 'environment_blocked', 'unsupported'}
ERRORS = {'missing_registration', 'invalid_registration', 'inaccessible', 'process_gone',
    'pid_reused', 'executable_mismatch', 'registration_changed', 'observer_failure', 'gap',
    'instance_bound', 'artifact_changed', 'sample_bound', 'unsupported_proc', 'unsampled_instance'}
PRIVACY = {name: False for name in ('rpc_arguments_retained', 'source_bodies_retained',
    'provider_responses_retained', 'credentials_retained', 'conversation_content_retained',
    'process_environment_retained')}
REG_KEYS = {'kind', 'schema_version', 'instance_id', 'pid', 'boot_id', 'start_ticks',
    'executable_sha256', 'executable_path', 'runtime_binding', 'cwd_binding', 'host_session',
    'host_session_authority', 'state', 'registration_duration_ns', 'lifetime_ns'}
HEX = re.compile(r'[0-9a-f]{64}')
ID = re.compile(r'[0-9a-f]{32}')


class ObservationError(ValueError):
    pass


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(65536), b''):
            h.update(chunk)
    return h.hexdigest()


def path_binding(path):
    return hashlib.sha256(os.fsencode(path.resolve())).hexdigest()


def initial(artifacts, *, purpose="naturalistic"):
    import evidence_purpose
    evidence_purpose.validate(purpose)
    return {'kind': 'dogfood_naturalistic_mcp_memory_evidence', 'schema_version': SCHEMA,
        'evidence_purpose': purpose, 'status': 'not_observed', 'candidate_mcp_sha256': artifacts.get('volicord-mcp', {}).get('sha256'),
        'process_ownership': ('codex_host_external_to_campaign_harness' if purpose == 'naturalistic'
            else 'test_support_owned_candidate_process'),
        'configured_launch': 'direct_candidate_local_volicord_mcp_executable',
        'observer_lifecycle': 'not_started', 'measurement': {'scope': 'registered_candidate_mcp_instances',
            'peak_rss_bytes': None, 'sample_count': 0, 'mechanism': 'linux_proc_status_vmrss_kib',
            'measurement_errors': [], 'peak_meaning': 'observed_sample_peak_not_absolute_maximum'},
        'instances': [], 'ticks': [], 'interval_ns': None, 'duration_ns': 0,
        'termination': 'not_started', 'observer_cpu_ns': 0, 'observer_peak_rss_bytes': None,
        'attribution': 'no_operation_or_codex_session_memory_attribution_claimed',
        'privacy': dict(PRIVACY), 'technical_gate_rss_evidence': 'retained_separately_not_relabelled_naturalistic'}


def validate_registration(value):
    if (not isinstance(value, dict) or set(value) != REG_KEYS
        or value['kind'] != 'volicord_mcp_lifecycle' or value['schema_version'] != 1
        or not isinstance(value['instance_id'], str) or not ID.fullmatch(value['instance_id'])
        or type(value['pid']) is not int or value['pid'] < 1
        or type(value['start_ticks']) is not int or value['start_ticks'] < 1
        or not isinstance(value['boot_id'], str) or not re.fullmatch(r'[0-9a-f-]{36}', value['boot_id'])
        or not isinstance(value['executable_path'], str) or not Path(value['executable_path']).is_absolute()
        or any(not isinstance(value[k], str) or not HEX.fullmatch(value[k])
            for k in ('executable_sha256','runtime_binding','cwd_binding'))
        or not isinstance(value['host_session'], str) or not ID.fullmatch(value['host_session'])
        or value['host_session_authority'] != 'server_generated_correlation_only'
        or value['state'] not in {'running', 'stopped'}
        or type(value['registration_duration_ns']) is not int or value['registration_duration_ns'] < 0
        or (value['lifetime_ns'] is not None and (type(value['lifetime_ns']) is not int or value['lifetime_ns'] < 0))
        or (value['state'] == 'stopped') != (value['lifetime_ns'] is not None)):
        raise ObservationError('invalid_registration')
    return value


def registration(path, runtime, executable):
    if path.is_symlink() or path.stat().st_size > 4096:
        raise ObservationError('invalid_registration')
    value = validate_registration(json.loads(path.read_bytes()))
    if (path.name != value['instance_id'] + '.json' or value['executable_path'] != str(executable.resolve())
        or value['runtime_binding'] != path_binding(runtime)):
        raise ObservationError('invalid_registration')
    return value


def start_identity(pid):
    stat = Path(f'/proc/{pid}/stat').read_text()
    fields = stat.rsplit(') ', 1)[1].split()
    return int(fields[19]), fields[0]


def rss(pid):
    for line in Path(f'/proc/{pid}/status').read_text().splitlines():
        if line.startswith('VmRSS:'):
            fields = line.split()
            if len(fields) == 3 and fields[2] == 'kB':
                return int(fields[1]) * 1024
    raise ObservationError('process_gone')


def sample(reg, expected_hash, cache):
    """Read start before/after RSS; executable replacement invalidates cached hash."""
    if reg['executable_sha256'] != expected_hash:
        raise ObservationError('executable_mismatch')
    identity, state = start_identity(reg['pid'])
    if identity != reg['start_ticks'] or reg['boot_id'] != Path('/proc/sys/kernel/random/boot_id').read_text().strip():
        raise ObservationError('pid_reused')
    if state == 'Z':
        raise ObservationError('process_gone')
    executable = Path(f"/proc/{reg['pid']}/exe")
    st = executable.stat()
    signature = (st.st_dev, st.st_ino, st.st_size, st.st_mtime_ns, st.st_ctime_ns)
    if os.readlink(executable) != reg['executable_path']:
        raise ObservationError('executable_mismatch')
    if cache.get(reg['instance_id']) != signature:
        if digest(executable) != expected_hash:
            raise ObservationError('executable_mismatch')
        cache[reg['instance_id']] = signature
    value = rss(reg['pid'])
    after, state = start_identity(reg['pid'])
    post = executable.stat()
    if after != identity or state == 'Z' or (post.st_dev,post.st_ino,post.st_size,post.st_mtime_ns,post.st_ctime_ns) != signature:
        raise ObservationError('pid_reused')
    return value


def validate(value, expected_hash=None):
    template = initial({'volicord-mcp': {'sha256': value.get('candidate_mcp_sha256')}}, purpose=value.get('evidence_purpose'))
    if (set(value) != set(template) or value['kind'] != template['kind'] or value['schema_version'] != SCHEMA
        or value['status'] not in STATUSES or not HEX.fullmatch(value.get('candidate_mcp_sha256') or '')
        or (expected_hash is not None and value['candidate_mcp_sha256'] != expected_hash)
        or value['privacy'] != PRIVACY or any(value[k] != template[k] for k in
            ('process_ownership', 'configured_launch', 'attribution', 'technical_gate_rss_evidence'))
        or set(value['measurement']) != set(template['measurement'])
        or any(value['measurement'][k] != template['measurement'][k] for k in ('scope', 'mechanism', 'peak_meaning'))
        or not isinstance(value['ticks'], list) or len(value['ticks']) > MAX_TICKS
        or not isinstance(value['instances'], list) or len(value['instances']) > MAX_INSTANCES
        or (value['interval_ns'] is not None and (type(value['interval_ns']) is not int or not 50_000_000 <= value['interval_ns'] <= 60_000_000_000))
        or (value['observer_peak_rss_bytes'] is not None and (type(value['observer_peak_rss_bytes']) is not int or value['observer_peak_rss_bytes'] < 0))
        or type(value['duration_ns']) is not int or value['duration_ns'] < 0
        or type(value['observer_cpu_ns']) is not int or value['observer_cpu_ns'] < 0
        or value['termination'] not in {'not_started','duration_elapsed','stop_requested','interrupted','failed'}
        or value['observer_lifecycle'] not in {'not_started','stopped','failed'}):
        raise ObservationError('invalid resource evidence')
    numbers, instance_ids, errors = [], set(), set()
    for instance in value['instances']:
        if (set(instance) != {'identity','samples','lifecycle','errors','binding_state'}
            or set(instance['identity']) != REG_KEYS or not ID.fullmatch(instance['identity']['instance_id'])
            or instance['identity']['instance_id'] in instance_ids
            or instance['identity']['executable_sha256'] != value['candidate_mcp_sha256']
            or instance['binding_state'] not in {'verified','unverified'}
            or instance['lifecycle'] not in {'running_at_detach','stopped','gone','identity_rejected'}
            or not isinstance(instance['errors'], list) or not set(instance['errors']) <= ERRORS):
            raise ObservationError('invalid process evidence')
        validate_registration(instance['identity'])
        instance_ids.add(instance['identity']['instance_id'])
        last = -1
        for record in instance['samples']:
            if (set(record) != {'elapsed_ns','rss_bytes'} or type(record['elapsed_ns']) is not int
                or not last < record['elapsed_ns'] <= value['duration_ns']
                or type(record['rss_bytes']) is not int or record['rss_bytes'] < 0):
                raise ObservationError('invalid resource sample')
            last = record['elapsed_ns']; numbers.append(record['rss_bytes'])
        if bool(instance['samples']) != (instance['binding_state'] == 'verified'):
            raise ObservationError('sample lacks process binding')
        errors.update(instance['errors'])
    last = -1
    for tick in value['ticks']:
        if (set(tick) != {'elapsed_ns', 'errors'} or type(tick['elapsed_ns']) is not int
            or not last < tick['elapsed_ns'] <= value['duration_ns'] or not isinstance(tick['errors'], list)
            or not set(tick['errors']) <= ERRORS):
            raise ObservationError('invalid tick or telemetry payload')
        last = tick['elapsed_ns']; errors.update(tick['errors'])
    m = value['measurement']
    if (len(numbers) > MAX_SAMPLES or type(m['sample_count']) is not int or m['sample_count'] != len(numbers) or m['peak_rss_bytes'] != (max(numbers) if numbers else None)
        or m['measurement_errors'] != sorted(errors)
        or (value['status'] == 'measured' and (not numbers or errors or value['termination'] == 'failed'))
        or (value['status'] == 'not_observed' and numbers)
        or (value['status'] == 'failed' and value['observer_lifecycle'] != 'failed')):
        raise ObservationError('resource completeness or peak disagreement')
    return value


def observe(artifacts, runtimes, *, duration_seconds=60, interval_ms=250, stop=None, proc_sample=sample, purpose="naturalistic"):
    if not 50 <= interval_ms <= 60000 or not 0 <= duration_seconds <= 86400:
        raise ObservationError('invalid observation bounds')
    result = initial(artifacts, purpose=purpose)
    result['interval_ns'] = interval_ms * 1_000_000
    start, cpu = time.monotonic_ns(), time.process_time_ns()
    instances, cache, rejected = {}, {}, set()
    total_samples = 0
    previous, terminate = None, 'duration_elapsed'
    environment_status = None
    deadline = start + int(duration_seconds * 1_000_000_000)
    executable = Path(artifacts['volicord-mcp']['path'])
    try:
        if not Path('/proc/self/status').is_file():
            environment_status = 'unsupported'
            raise ObservationError('unsupported_proc')
        if digest(executable) != result['candidate_mcp_sha256']:
            raise ObservationError('artifact_changed')
        while time.monotonic_ns() < deadline:
            elapsed = time.monotonic_ns() - start
            errors, found = set(), False
            if previous is not None and elapsed - previous > result['interval_ns'] * 2:
                errors.add('gap')
            previous = elapsed
            if len(result['ticks']) >= MAX_TICKS:
                raise ObservationError('sample_bound')
            for runtime in runtimes:
                directory = runtime / 'observations/mcp'
                if directory.is_symlink() or (runtime / 'observations').is_symlink():
                    errors.add('invalid_registration'); continue
                paths = sorted(directory.glob('*.json'))[:MAX_INSTANCES + 1]
                if not paths: errors.add('missing_registration')
                if len(paths) > MAX_INSTANCES:
                    raise ObservationError('instance_bound')
                for path in paths:
                    found = True
                    try:
                        reg = registration(path, runtime, executable)
                        if reg['executable_sha256'] != result['candidate_mcp_sha256']:
                            raise ObservationError('executable_mismatch')
                        ident = reg['instance_id']
                        if ident in rejected:
                            continue
                        if ident not in instances:
                            if len(instances) >= MAX_INSTANCES:
                                raise ObservationError('instance_bound')
                            instances[ident] = {'identity':reg, 'samples':[], 'lifecycle':'running_at_detach',
                                'errors':[], 'binding_state':'unverified'}
                        item = instances[ident]
                        immutable = REG_KEYS - {'state','lifetime_ns'}
                        if any(item['identity'][k] != reg[k] for k in immutable):
                            item['errors'] = sorted(set(item['errors']) | {'registration_changed'})
                            item['lifecycle'] = 'identity_rejected'; rejected.add(ident); continue
                        if reg['state'] == 'stopped':
                            item['lifecycle'] = 'stopped'
                            item['identity'] = reg
                            continue
                        try:
                            measured = proc_sample(reg, result['candidate_mcp_sha256'], cache)
                            if total_samples >= MAX_SAMPLES:
                                raise ObservationError('sample_bound')
                            total_samples += 1
                            item['samples'].append({'elapsed_ns':time.monotonic_ns() - start,'rss_bytes':measured})
                            item['binding_state'] = 'verified'
                        except (OSError, ObservationError) as error:
                            if str(error) == 'sample_bound': raise
                            code = ('process_gone' if isinstance(error, FileNotFoundError) else
                                'inaccessible' if isinstance(error, PermissionError) else
                                str(error) if isinstance(error, ObservationError) and str(error) in ERRORS else 'inaccessible')
                            item['errors'] = sorted(set(item['errors']) | {code})
                            if code in {'pid_reused','executable_mismatch'}:
                                item['lifecycle'] = 'identity_rejected'; rejected.add(ident)
                            elif code == 'process_gone': item['lifecycle'] = 'gone'
                    except (OSError, ValueError, TypeError, KeyError) as error:
                        if isinstance(error, ObservationError) and str(error) in {'sample_bound','instance_bound'}: raise
                        errors.add(str(error) if isinstance(error, ObservationError) and str(error) in ERRORS else 'invalid_registration')
            if not found: errors.add('missing_registration')
            result['ticks'].append({'elapsed_ns':elapsed, 'errors':sorted(errors)})
            if stop is not None and stop():
                terminate = 'stop_requested'; break
            time.sleep(min(interval_ms / 1000, max(0, (deadline - time.monotonic_ns()) / 1e9)))
        if digest(executable) != result['candidate_mcp_sha256']:
            raise ObservationError('artifact_changed')
    except KeyboardInterrupt:
        terminate = 'interrupted'
    except Exception as error:
        terminate = 'failed'
        code = str(error) if isinstance(error, ObservationError) and str(error) in ERRORS else 'observer_failure'
        if len(result['ticks']) >= MAX_TICKS:
            result['ticks'][-1]['errors'] = sorted(set(result['ticks'][-1]['errors']) | {code})
        else:
            result['ticks'].append({'elapsed_ns':time.monotonic_ns() - start, 'errors':[code]})
    result.update(instances=list(instances.values()), duration_ns=time.monotonic_ns() - start,
        observer_cpu_ns=time.process_time_ns() - cpu, termination=terminate,
        observer_lifecycle='failed' if terminate == 'failed' else 'stopped')
    result['observer_peak_rss_bytes'] = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * 1024
    for item in result['instances']:
        if not item['samples']:
            item['errors'] = sorted(set(item['errors']) | {'unsampled_instance'})
    samples = [s['rss_bytes'] for i in result['instances'] for s in i['samples']]
    errors = sorted({e for t in result['ticks'] for e in t['errors']} | {e for i in result['instances'] for e in i['errors']})
    result['measurement'].update(sample_count=len(samples), peak_rss_bytes=max(samples) if samples else None,
        measurement_errors=errors)
    result['status'] = environment_status or ('failed' if terminate == 'failed' else 'partial' if samples and errors else
        'measured' if samples else 'environment_blocked' if 'inaccessible' in errors else 'not_observed')
    validate(result)
    return result


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('command', choices=['start','attach','stop'])
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--campaign-root', type=Path)
    p.add_argument('--binary', type=Path, help='Standalone candidate attachment; no campaign qualification')
    p.add_argument('--runtime', type=Path, action='append', default=[])
    p.add_argument('--duration-seconds', type=float, default=60)
    p.add_argument('--interval-ms', type=int, default=250)
    args = p.parse_args()
    if args.command == 'stop':
        control = args.output / 'control.json'
        if not control.is_file() or (args.output / 'resource.json').exists():
            raise ObservationError('no running observer at this output')
        (args.output / 'stop.request').touch(exist_ok=False)
        return 0
    import campaign
    root = args.campaign_root.resolve() if args.campaign_root else None
    if root is not None:
        value = campaign.load_campaign_for_mutation(root)
        campaign.verify_candidate_artifacts(value)
        artifacts = value['candidate_artifacts']
        runtimes = [Path(j['runtime_home']) for j in value['journeys'].values()]
        candidate_head = value['candidate_head']
    else:
        if args.binary is None or not args.runtime:
            raise ObservationError('campaign root or explicit binary and runtime required')
        artifacts = {'volicord-mcp': {'path':str(args.binary.resolve()), 'sha256':digest(args.binary)}}
        runtimes = [r.resolve() for r in args.runtime]
        candidate_head = None
    args.output.mkdir(mode=0o700, parents=True, exist_ok=False)
    campaign.write_json(args.output / 'control.json', {'kind':'mcp_resource_observer_control',
        'candidate_head':candidate_head, 'candidate_mcp_sha256':artifacts['volicord-mcp']['sha256']})
    result = observe(artifacts, runtimes,
        duration_seconds=args.duration_seconds, interval_ms=args.interval_ms,
        stop=lambda: (args.output / 'stop.request').exists())
    campaign.write_json(args.output / 'resource.json', result)
    print(json.dumps({'status':result['status'], 'samples':result['measurement']['sample_count'],
        'termination':result['termination']}))
    return 1 if result['status'] in {'failed','unsupported','environment_blocked'} else 0


if __name__ == '__main__':
    raise SystemExit(main())
