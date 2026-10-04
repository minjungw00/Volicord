"""Candidate route and bounded Runtime access proof. Run in the actual host tool shell before use.

Local invocation proves only that execution channel, never another host's shell.
No environment/argv inventory or credential access is performed.
"""
import argparse
import hashlib
import json
import re
from pathlib import Path
import shlex
import shutil
import subprocess
import tomllib
import tempfile


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def host_context(value):
    """Caller-observed conditions, never host authentication or automated trust."""
    fields = {'candidate_head', 'execution_channel', 'host_version', 'sandbox_mode',
        'sandbox_permissions', 'writable_roots', 'workspace_trust', 'hook_trust',
        'permission_basis'}
    if not isinstance(value, dict) or set(value) != fields:
        raise ValueError('readiness context requires exact host/sandbox/trust conditions')
    if (re.fullmatch(r'[0-9a-f]{40}', str(value['candidate_head'])) is None
        or value['execution_channel'] not in {'vscode_tool_shell', 'codex_cli_tool_shell', 'local_subprocess'}
        or value['sandbox_mode'] not in {'workspace-write', 'read-only'}
        or value['sandbox_permissions'] not in {'use_default', 'require_escalated'}
        or any(value[key] not in {'user_confirmed', 'unverified'} for key in ('workspace_trust', 'hook_trust'))
        or any(not isinstance(value[key], str) or not value[key].strip() for key in ('host_version', 'permission_basis'))
        or not isinstance(value['writable_roots'], list)
        or any(not isinstance(path, str) or not Path(path).is_absolute() for path in value['writable_roots'])):
        raise ValueError('invalid readiness host context')
    return {**value, 'writable_roots': sorted(set(str(Path(p).resolve()) for p in value['writable_roots']))}


def file_access_identity(path):
    stat = path.stat()
    return {'device': stat.st_dev, 'inode': stat.st_ino, 'mode': stat.st_mode,
        'uid': stat.st_uid, 'gid': stat.st_gid}


def retained_observation(output, execution):
    """Read the existing stdout/result pair; preserve numeric completion/integrity."""
    data = Path(output).read_bytes()
    result = json.loads(Path(execution).read_bytes())
    if (result.get('exit_code') != 0 or result.get('termination') is not None
        or result.get('stdout_sha256') != hashlib.sha256(data).hexdigest()):
        raise ValueError('readiness reuse requires successful hash-bound execution evidence')
    observation = json.loads(data)
    if (observation.get('status') != 'ready' or observation.get('exit_code') != 0
        or observation.get('probe') != 'scoped_status' or not observation.get('project_id')
        or re.fullmatch(r'[0-9a-f]{64}', str(observation.get('readback_sha256'))) is None
        or not isinstance(observation.get('scope'), dict)):
        raise ValueError('readiness observation is incomplete; inspect original host evidence')
    return observation


def probe(binary, runtime, repository, cli_hash, mcp_hash, *, context=None, previous=None, inspect=False):
    binary, runtime, repository = map(lambda p: Path(p).resolve(), (binary, runtime, repository))
    manifest = json.loads((repository / '.codex/volicord-integration.json').read_bytes())
    mcp = binary.with_name('volicord-mcp').resolve()
    owned_config = tomllib.loads((repository / '.codex/config.toml').read_text())
    config = owned_config['mcp_servers']['volicord']
    hooks = owned_config['hooks']['SessionStart']
    command = [str(binary), '--runtime', str(runtime), '--repository', str(repository), 'codex', 'hook']
    hook_matches = any(group.get('matcher') == '^(startup|resume|clear|compact)$' and h.get('type') == 'command' and shlex.split(h['command']) == command for group in hooks for h in group['hooks'])
    if (manifest.get('kind') != 'volicord_codex_repository_integration' or manifest.get('schema_version') != 1
        or manifest['volicord'] != str(binary) or manifest['volicord_mcp'] != str(mcp)
        or manifest['runtime'] != str(runtime) or manifest['repository'] != str(repository)
        or not hook_matches or config.get('enabled') is not True or config.get('required') is not True
        or config['command'] != str(mcp) or config['env']['VOLICORD_RUNTIME_DIR'] != str(runtime)
        or digest(binary) != cli_hash or digest(mcp) != mcp_hash
        or binary.read_bytes()[:4] != b'\x7fELF' or mcp.read_bytes()[:4] != b'\x7fELF'):
        raise ValueError('candidate installation/Runtime/repository mismatch')
    # Only relevant integration/permission settings enter the scope. Ordinary
    # task changes, campaign labels, PATH and unrelated config do not invalidate it.
    scope = {'route': {'binary': str(binary), 'mcp': str(mcp), 'runtime': str(runtime),
        'repository': str(repository), 'cli_sha256': cli_hash, 'mcp_sha256': mcp_hash},
        'integration': {'mcp': config, 'hook_command': command,
            'hooks': [group for group in hooks if any(shlex.split(h['command']) == command for h in group['hooks'])],
            'hooks_enabled': all(owned_config.get('features', {}).get(key, True) is not False for key in ('hooks', 'codex_hooks'))},
        'permissions': {key: owned_config.get(key) for key in
            ('sandbox_mode', 'sandbox_workspace_write', 'permissions', 'default_permissions', 'approval_policy', 'approvals_reviewer')},
        'host': host_context(context) if context is not None else None,
        'runtime_access': file_access_identity(runtime),
        'store_access': {p.name: file_access_identity(p) for p in sorted(runtime.glob('*.sqlite3'))},
        'executable_access': {'cli': file_access_identity(binary), 'mcp': file_access_identity(mcp)}}
    if scope['integration']['hooks_enabled'] is False:
        raise ValueError('repository lifecycle hooks are disabled')
    if inspect:
        return {'status': 'unverified', 'scope': scope,
            'review_config': str(repository / '.codex/config.toml'),
            'review_hook': shlex.join(command),
            'runtime_permission_config': '[sandbox_workspace_write]\nwritable_roots = ' + json.dumps([str(runtime)]) + '\n',
            'next_action': 'Review generated integration, satisfy only missing workspace/hook trust and exact Runtime write permission in the selected host, then run the scoped probe there.'}
    if context is not None and context['sandbox_permissions'] == 'use_default':
        roots = [repository, *map(Path, scope['host']['writable_roots'])]
        if context['sandbox_mode'] != 'workspace-write' or not any(runtime.is_relative_to(p) for p in roots):
            raise ValueError(f'Runtime write permission missing: {runtime}; allow only this Runtime in the selected host')
    reused = previous is not None and scope['host'] is not None and previous.get('scope') == scope
    if previous is not None and (previous.get('status') != 'ready' or previous.get('exit_code') != 0
        or previous.get('probe') != 'scoped_status' or not previous.get('project_id')
        or re.fullmatch(r'[0-9a-f]{64}', str(previous.get('readback_sha256'))) is None):
        raise ValueError('invalid retained readiness observation')
    if reused:
        status = {'project_id': previous['project_id']}
        readback_hash = previous['readback_sha256']
    else:
        # Exact bounded write, before Product opening. No repository/global writes,
        # canonical records, lifecycle operations or telemetry setup are performed.
        try:
            with tempfile.TemporaryFile(dir=runtime) as access:
                access.write(b'volicord readiness\n')
                access.flush()
        except OSError as error:
            raise ValueError(f'Runtime write permission missing: {runtime}; allow only this Runtime in the selected host') from error
        status, readback_hash = scoped_status(binary, runtime, repository)
    bare = shutil.which('volicord')
    if digest(binary) != cli_hash or digest(mcp) != mcp_hash:
        raise ValueError('candidate bytes changed during probe')
    unverified = ['actual_SessionStart_and_MCP_connection', 'current_health_after_observation']
    if scope['host'] is None or scope['host']['execution_channel'] == 'local_subprocess':
        unverified.append('other_host_tool_shell')
    unverified.extend(key for key in ('workspace_trust', 'hook_trust')
        if scope['host'] is None or scope['host'][key] == 'unverified')
    return {'status': 'ready', 'invoked_executable': str(binary), 'resolved_executable': str(binary),
        'cli_sha256': cli_hash, 'mcp_sha256': mcp_hash,
        'runtime_binding': hashlib.sha256(str(runtime).encode()).hexdigest(),
        'repository_binding': hashlib.sha256(str(repository).encode()).hexdigest(),
        'project_id': status['project_id'], 'probe': 'scoped_status', 'exit_code': 0,
        'readback_sha256': readback_hash, 'scope': scope,
        'execution': 'reused' if reused else 'checked',
        'checked': ['candidate_route', 'static_integration', 'scope_conditions'] + ([] if reused else ['Runtime_write', 'scoped_status']),
        'reused': ['Runtime_write', 'scoped_status'] if reused else [],
        'unverified': unverified, 'context_authority': 'caller_observed_not_host_authenticated',
        'bare_executable': str(Path(bare).resolve()) if bare else None,
        'bare_sha256': digest(bare) if bare else None,
        'bare_matches_candidate': bool(bare and Path(bare).resolve() == binary and digest(bare) == cli_hash)}


def scoped_status(binary, runtime, repository):
    argv = [str(binary), '--runtime', str(runtime), '--repository', str(repository), '--json', 'status']
    result = subprocess.run(argv, capture_output=True, timeout=30, check=False)
    if result.returncode != 0:
        raise ValueError('scoped status probe failed')
    status = json.loads(result.stdout)
    if not isinstance(status, dict) or re.fullmatch(r'[0-9a-f]{32}', str(status.get('project_id'))) is None:
        raise ValueError('scoped probe has no bound Project')
    return status, hashlib.sha256(result.stdout).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'runtime', 'repository', 'cli-sha256', 'mcp-sha256'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--inspect', action='store_true', help='Review actual integration and bounded permission guidance without Product execution')
    parser.add_argument('--context', type=Path, help='Current caller-observed host/sandbox/trust scope; no authentication claim')
    parser.add_argument('--reuse-output', type=Path, help='Prior scoped readiness stdout')
    parser.add_argument('--reuse-execution', type=Path, help='Existing numeric execution result bound to that stdout SHA-256')
    args = parser.parse_args()
    if bool(args.reuse_output) != bool(args.reuse_execution):
        parser.error('reuse requires both output and execution evidence')
    context = json.loads(args.context.read_bytes()) if args.context else None
    previous = retained_observation(args.reuse_output, args.reuse_execution) if args.reuse_output else None
    print(json.dumps(probe(args.binary, args.runtime, args.repository, args.cli_sha256, args.mcp_sha256,
        context=context, previous=previous, inspect=args.inspect), sort_keys=True))


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        print(json.dumps({'status': 'blocked', 'reason': str(error),
            'next_action': 'Inspect the selected route/configuration and allow only its exact Runtime; retry this scoped check in the intended host.'}, sort_keys=True))
        raise SystemExit(1)
