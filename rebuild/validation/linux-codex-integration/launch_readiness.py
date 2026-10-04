"""Read-only candidate route proof. Run in the actual host tool shell before use.

Local invocation proves only that execution channel, never another host's shell.
No environment/argv inventory or credential access is performed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shlex
import shutil
import subprocess
import tomllib


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def probe(binary, runtime, repository, cli_hash, mcp_hash):
    binary, runtime, repository = map(lambda p: Path(p).resolve(), (binary, runtime, repository))
    manifest = json.loads((repository / '.codex/volicord-integration.json').read_bytes())
    mcp = binary.with_name('volicord-mcp').resolve()
    owned_config = tomllib.loads((repository / '.codex/config.toml').read_text())
    config = owned_config['mcp_servers']['volicord']
    hooks = owned_config['hooks']['SessionStart']
    command = [str(binary), '--runtime', str(runtime), '--repository', str(repository), 'codex', 'hook']
    hook_matches = any(shlex.split(h['command']) == command for group in hooks for h in group['hooks'])
    if (manifest['volicord'] != str(binary) or manifest['volicord_mcp'] != str(mcp)
        or manifest['runtime'] != str(runtime) or manifest['repository'] != str(repository)
        or not hook_matches or config.get('enabled') is not True or config.get('required') is not True
        or config['command'] != str(mcp) or config['env']['VOLICORD_RUNTIME_DIR'] != str(runtime)
        or digest(binary) != cli_hash or digest(mcp) != mcp_hash
        or binary.read_bytes()[:4] != b'\x7fELF' or mcp.read_bytes()[:4] != b'\x7fELF'):
        raise ValueError('candidate installation/Runtime/repository mismatch')
    argv = [str(binary), '--runtime', str(runtime), '--repository', str(repository), '--json', 'status']
    result = subprocess.run(argv, capture_output=True, timeout=30, check=False)
    if result.returncode != 0:
        raise ValueError('scoped status probe failed')
    status = json.loads(result.stdout)
    if not status.get('project_id'):
        raise ValueError('scoped probe has no bound Project')
    bare = shutil.which('volicord')
    if digest(binary) != cli_hash or digest(mcp) != mcp_hash:
        raise ValueError('candidate bytes changed during probe')
    return {'status': 'ready', 'invoked_executable': str(binary), 'resolved_executable': str(binary),
        'cli_sha256': cli_hash, 'mcp_sha256': mcp_hash,
        'runtime_binding': hashlib.sha256(str(runtime).encode()).hexdigest(),
        'repository_binding': hashlib.sha256(str(repository).encode()).hexdigest(),
        'project_id': status['project_id'], 'probe': 'scoped_status', 'exit_code': result.returncode,
        'readback_sha256': hashlib.sha256(result.stdout).hexdigest(),
        'bare_executable': str(Path(bare).resolve()) if bare else None,
        'bare_sha256': digest(bare) if bare else None,
        'bare_matches_candidate': bool(bare and Path(bare).resolve() == binary and digest(bare) == cli_hash)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('binary', 'runtime', 'repository', 'cli-sha256', 'mcp-sha256'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    print(json.dumps(probe(args.binary, args.runtime, args.repository, args.cli_sha256, args.mcp_sha256), sort_keys=True))


if __name__ == '__main__':
    main()
