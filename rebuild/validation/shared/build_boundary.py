#!/usr/bin/env python3
"""Gate-owned Cargo outputs and local, append-only build/execution receipts.

This is an execution boundary, not a qualification result or a cache cleaner.
"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import uuid

CONTEXT = "VOLICORD_BUILD_CONTEXT"
CONTEXT_HASH = "VOLICORD_BUILD_CONTEXT_SHA256"
ROOT = Path(__file__).resolve().parents[3]


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write(path, value):
    Path(path).write_text(json.dumps(value, sort_keys=True, indent=2) + '\n')


def identity(path):
    path = Path(path).resolve(strict=True)
    return {'path': str(path), 'sha256': digest(path)}


def tool(name):
    path = shutil.which(name)
    if path is None:
        raise ValueError(f'required build tool unavailable: {name}')
    result = subprocess.run([path, '-vV'], capture_output=True, text=True, check=True)
    # Rustup proxies share bytes; retain selected compiler/driver bytes as well.
    value = {**identity(path), 'version': result.stdout}
    if name == 'rustc':
        sysroot = Path(subprocess.run([path, '--print', 'sysroot'], capture_output=True,
                                     text=True, check=True).stdout.strip()).resolve()
        value['sysroot'] = str(sysroot)
        value['toolchain_binaries'] = {name: identity(sysroot / 'bin' / name)
            for name in ('rustc', 'clippy-driver') if (sysroot / 'bin' / name).is_file()}
    return value


def configurations(root, env):
    paths = set()
    for base in (root, *root.parents, root / 'rebuild'):
        paths.update((base / '.cargo/config', base / '.cargo/config.toml'))
        paths.update((base / 'rust-toolchain', base / 'rust-toolchain.toml'))
    cargo_home = Path(env.get('CARGO_HOME', str(Path.home() / '.cargo')))
    paths.update((cargo_home / 'config', cargo_home / 'config.toml'))
    return [identity(p) for p in sorted(paths) if p.exists()]


def inputs(root):
    """Hash actual tracked bytes, including skip-worktree/assume-unchanged files."""
    result = subprocess.run(['git', 'ls-files', '-s', '-z'], cwd=root, capture_output=True, check=True)
    files = {}
    for entry in result.stdout.split(b'\0'):
        if not entry:
            continue
        header, raw_path = entry.split(b'\t', 1)
        mode, expected, stage = header.decode().split()
        name = os.fsdecode(raw_path)
        if not (name.startswith('rebuild/') or name in ('Cargo.toml', 'Cargo.lock', '.gitignore') or name.startswith('.cargo/') or name.startswith('rust-toolchain')):
            continue
        path = root / name
        if stage != '0' or mode not in ('100644', '100755') or path.is_symlink():
            raise ValueError(f'unsupported build input: {name}')
        content = path.read_bytes()
        # Git's SHA-1 object format is part of the current repository identity.
        actual = hashlib.sha1(b'blob ' + str(len(content)).encode() + b'\0' + content).hexdigest()
        if actual != expected:
            raise ValueError(f'build input differs from index: {name}')
        files[name] = digest(path)
    # Ignored Rust/config/build-script inputs are not hidden qualification inputs.
    for path in (root / 'rebuild/crates').rglob('*'):
        if path.is_file() and path.relative_to(root).as_posix() not in files:
            raise ValueError(f'untracked or ignored crate input: {path.relative_to(root)}')
    return files


def inherited_environment(env):
    env = dict(env)
    if CONTEXT in env:
        value = read_context(env)
        # Nested orchestration fixtures get a new target, never the parent's cache.
        for key, expected in value['owned_environment'].items():
            if env.get(key) != expected:
                raise ValueError(f'changed gate build setting: {key}')
            env.pop(key, None)
        env.pop(CONTEXT, None)
        env.pop(CONTEXT_HASH, None)
    return env


def prepare(root, directory, environment):
    """Called before any maintained gate admission/support/build invocation."""
    root, directory = Path(root).resolve(), Path(directory).resolve()
    env = inherited_environment(environment)
    allowed_cargo = {'CARGO_HOME', 'CARGO_TERM_COLOR', 'CARGO_TERM_PROGRESS_WHEN', 'CARGO_TERM_PROGRESS_WIDTH'}
    rejected = sorted(k for k, v in env.items() if v and (
        (k.startswith('CARGO_') and k not in allowed_cargo)
        or k.startswith(('RUSTFLAGS', 'RUSTDOCFLAGS', 'RUSTC', 'SCCACHE_', 'CCACHE_'))
        or k in {'RUSTDOC', 'RUSTUP_TOOLCHAIN', 'LD_PRELOAD', 'LD_LIBRARY_PATH', 'CC', 'CXX', 'CFLAGS', 'CXXFLAGS', 'LDFLAGS', 'AR'}))
    if rejected:
        raise ValueError('qualification refuses inherited compiler/cache/output settings: ' + ', '.join(rejected))
    configs = configurations(root, env)
    # No ambient Cargo or toolchain configuration is currently part of this workspace.
    # Refuse rather than silently discard a diagnostic wrapper/flag/target override.
    if configs:
        raise ValueError('qualification refuses ambient Cargo/toolchain configuration: ' + ', '.join(p['path'] for p in configs))
    source = inputs(root)
    cargo, rustc = tool('cargo'), tool('rustc')
    host = next(line.removeprefix('host: ') for line in rustc['version'].splitlines() if line.startswith('host: '))
    target = directory / 'build/target'
    target.mkdir(parents=True, exist_ok=False)
    receipts = directory / 'build/receipts'
    receipts.mkdir()
    observer = root / 'rebuild/validation/shared/build_boundary.py'
    # Cargo's runner string is split on whitespace, not interpreted by a shell.
    if any(c.isspace() for c in str(observer)):
        raise ValueError('qualification build observer path must contain no whitespace')
    owned = {'CARGO_HOME': env.get('CARGO_HOME', str(Path.home() / '.cargo')),
             'RUSTUP_HOME': env.get('RUSTUP_HOME', str(Path.home() / '.rustup')),
             'CARGO_TARGET_DIR': str(target), 'RUSTC_WRAPPER': str(observer),
             'CARGO_TARGET_' + host.upper().replace('-', '_') + '_RUNNER': str(observer) + ' execute'}
    head = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=root, text=True, capture_output=True, check=True).stdout.strip()
    value = {'kind': 'qualification_build_inputs', 'attempt': directory.name, 'root': str(root),
             'candidate_head': head, 'source_sha256': source, 'configuration': configs,
             'tools': {'cargo': cargo, 'rustc': rustc}, 'owned_environment': owned,
             'compiler_environment': compiler_environment(env | owned),
             'receipts': str(receipts), 'initial_target_empty': not any(target.iterdir()),
             'cargo_home': env.get('CARGO_HOME', str(Path.home() / '.cargo')),
             'rustup_home': env.get('RUSTUP_HOME', str(Path.home() / '.rustup'))}
    path = directory / 'build/context.json'
    write(path, value)
    return env | owned | {CONTEXT: str(path), CONTEXT_HASH: digest(path)}


def read_context(env=None):
    env = os.environ if env is None else env
    path = Path(env[CONTEXT])
    if digest(path) != env.get(CONTEXT_HASH):
        raise ValueError('gate build context bytes changed')
    return json.loads(path.read_text())


def compiler_environment(env):
    return {k: v for k, v in env.items() if
            k.startswith(('CARGO_BUILD_', 'CARGO_TARGET_', 'CARGO_PROFILE_', 'CARGO_ENCODED_',
                          'RUSTC', 'RUSTFLAGS', 'RUSTDOC', 'SCCACHE_', 'CCACHE_'))
            or k in {'CARGO_HOME', 'CARGO_INCREMENTAL', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN',
                     'LD_PRELOAD', 'LD_LIBRARY_PATH', 'CC', 'CXX', 'CFLAGS', 'CXXFLAGS', 'LDFLAGS', 'AR'}}


def check(env=None):
    env = os.environ if env is None else env
    value = read_context(env)
    for key, expected in value['owned_environment'].items():
        if env.get(key) != expected:
            raise ValueError(f'changed gate build setting: {key}')
    if compiler_environment(env) != value['compiler_environment']:
        raise ValueError('gate compiler environment changed')
    root = Path(value['root'])
    if configurations(root, env) != value['configuration'] or inputs(root) != value['source_sha256']:
        raise ValueError('gate build source/configuration changed')
    for name, expected in value['tools'].items():
        if tool(name) != expected:
            raise ValueError(f'gate build tool changed: {name}')
    return value


def receipt(value, record):
    path = Path(value['receipts']) / (str(time.time_ns()) + '-' + uuid.uuid4().hex + '.json')
    pending = path.with_suffix('.pending')
    write(pending, {'attempt': value['attempt'], 'context_sha256': os.environ[CONTEXT_HASH], **record})
    pending.replace(path)


def observe_compiler(argv):
    value = read_context()
    # A compiler cache wrapper is not chained. Cargo supplies the selected compiler.
    compiler = identity(argv[0])
    observed = compiler_environment(os.environ)
    expected = dict(value['compiler_environment'])
    selected = value['tools']['rustc']
    if os.environ.get('RUSTUP_TOOLCHAIN') == Path(selected['sysroot']).name:
        expected['RUSTUP_TOOLCHAIN'] = os.environ['RUSTUP_TOOLCHAIN']
    driver = selected['toolchain_binaries'].get('clippy-driver')
    if driver and observed.get('RUSTC_WORKSPACE_WRAPPER') == driver['path']:
        if compiler != driver or len(argv) < 2 or identity(argv[1]) != selected['toolchain_binaries']['rustc']:
            raise ValueError('Clippy invocation differs from the recorded toolchain')
        expected['RUSTC_WORKSPACE_WRAPPER'] = driver['path']
    elif compiler != selected['toolchain_binaries']['rustc']:
        raise ValueError('compiler invocation differs from the recorded toolchain')
    # Cargo adds its toolchain loader search path and an empty encoded-flags
    # variable. Neither may hide nonempty caller-supplied compilation settings.
    normalize = lambda env: {k: v for k, v in env.items() if v and k != 'LD_LIBRARY_PATH'}
    if normalize(observed) != normalize(expected):
        changed = sorted(k for k in normalize(observed).keys() | normalize(expected).keys()
                         if normalize(observed).get(k) != normalize(expected).get(k))
        raise ValueError('compiler invocation changed the gate environment: ' + ', '.join(changed))
    root = Path(value['root'])
    if inputs(root) != value['source_sha256'] or configurations(root, os.environ) != value['configuration']:
        raise ValueError('compiler invocation changed gate source/configuration')
    registry = Path(value['cargo_home']).resolve() / 'registry/src'
    for arg in argv[1:]:
        if arg.endswith('.rs') and Path(arg).is_file():
            source = Path(arg).resolve()
            relative = source.relative_to(root).as_posix() if source.is_relative_to(root) else None
            if relative not in value['source_sha256'] and not source.is_relative_to(registry):
                raise ValueError('compiler source is outside the recorded candidate and registry')
    out = None
    if '--out-dir' in argv:
        out = Path(argv[argv.index('--out-dir') + 1]).resolve()
        target = Path(value['owned_environment']['CARGO_TARGET_DIR'])
        if not out.is_relative_to(target):
            raise ValueError('compiler output escaped gate target')
    # Preserve Cargo jobserver descriptors, just as a transparent exec wrapper does.
    code = subprocess.call(argv, close_fds=False)
    outputs = []
    if code == 0 and out is not None and '--crate-name' in argv:
        name = argv[argv.index('--crate-name') + 1]
        extra = next((arg.split('=', 1)[1] for arg in argv if arg.startswith('extra-filename=')), '')
        for path in out.glob('*' + name + extra + '*'):
            if path.is_file():
                outputs.append(identity(path))
    receipt(value, {'kind': 'compiler', 'argv': argv, 'cwd': str(Path.cwd()), 'compiler': compiler,
                    'compiler_environment': observed,
                    'exit_code': code, 'outputs': outputs,
                    'inputs': [identity(arg) for arg in argv[1:] if arg.endswith('.rs') and Path(arg).is_file()]})
    return code


def execute(argv):
    value = read_context()
    binary = identity(argv[0])
    matches = [p.name for p in Path(value['receipts']).glob('*.json')
               if any(output == binary for output in json.loads(p.read_text()).get('outputs', []))]
    if not matches:
        raise ValueError('executed binary has no matching gate compiler receipt')
    execution_id = uuid.uuid4().hex
    receipt(value, {'kind': 'execution_started', 'execution_id': execution_id,
                    'binary': binary, 'compiler_receipts': matches, 'argv': argv})
    # Preserve Cargo jobserver descriptors, just as a transparent exec wrapper does.
    code = subprocess.call(argv, close_fds=False)
    receipt(value, {'kind': 'execution_finished', 'execution_id': execution_id, 'binary': binary, 'exit_code': code})
    return code


def build_binaries(root, names, *, release=False):
    root = Path(root).resolve()
    local = root / 'rebuild/.local/builds'
    local.mkdir(parents=True, exist_ok=True)
    attempt = Path(tempfile.mkdtemp(prefix='cargo-', dir=local))
    before = check() if CONTEXT in os.environ else None
    command = ['cargo', 'build', '--manifest-path', str(root / 'rebuild/Cargo.toml'), '--message-format=json']
    if release:
        command.append('--release')
    for name in names:
        command.extend(['--bin', name])
    write(attempt / 'command.json', {'argv': command, 'cwd': str(root),
          'context': os.environ.get(CONTEXT), 'context_sha256': os.environ.get(CONTEXT_HASH),
          'source_sha256': {str(p.relative_to(root)): digest(p) for p in
              [*(root/'rebuild').glob('Cargo.*'), *(root/'rebuild/crates').rglob('*')] if p.is_file()},
          'compiler_environment_sha256': {k: hashlib.sha256(v.encode()).hexdigest() for k, v in os.environ.items()
              if k.startswith(('CARGO_', 'RUST', 'CC', 'CXX', 'LD_', 'AR'))},
          'configuration': configurations(root, os.environ), 'cargo': tool('cargo'), 'rustc': tool('rustc')})
    with (attempt / 'stdout.log').open('wb') as stdout, (attempt / 'stderr.log').open('wb') as stderr:
        result = subprocess.run(command, cwd=root, stdout=stdout, stderr=stderr, check=False)
    artifacts = {}
    # Preserve process completion even when artifact admission subsequently refuses.
    write(attempt / 'result.json', {'exit_code': result.returncode, 'selection': 'pending' if result.returncode == 0 else 'not_run', 'artifacts': artifacts})
    print(f'Cargo build receipt: {attempt}', file=sys.stderr)
    try:
        if result.returncode == 0:
            for line in (attempt / 'stdout.log').read_text().splitlines():
                item = json.loads(line)
                if item.get('reason') == 'compiler-artifact' and item.get('executable') and item['target']['name'] in names:
                    name = item['target']['name']
                    if name in artifacts:
                        raise ValueError(f'ambiguous executable: {name}')
                    artifacts[name] = identity(item['executable'])
            if set(artifacts) != set(names):
                raise ValueError('Cargo did not produce the requested executables')
            if before:
                check()
                target = Path(before['owned_environment']['CARGO_TARGET_DIR'])
                if any(not Path(item['path']).is_relative_to(target) for item in artifacts.values()):
                    raise ValueError('Cargo selected an executable outside the gate target')
                for item in artifacts.values():
                    # Selection must bind to compilation, not merely to a familiar path.
                    matches = [p.name for p in Path(before['receipts']).glob('*.json')
                               if any(output['sha256'] == item['sha256'] for output in json.loads(p.read_text()).get('outputs', []))]
                    if not matches:
                        raise ValueError('Cargo artifact lacks a gate compiler receipt')
                receipt(before, {'kind': 'cargo_artifact_selection', 'artifacts': artifacts, 'build_attempt': str(attempt)})
    except (OSError, ValueError, KeyError) as error:
        write(attempt / 'result.json', {'exit_code': result.returncode,
              'selection': 'refused', 'reason': str(error), 'artifacts': {}})
        raise
    write(attempt / 'result.json', {'exit_code': result.returncode,
          'selection': 'admitted' if result.returncode == 0 else 'not_run', 'artifacts': artifacts})
    if result.returncode:
        sys.stderr.write((attempt / 'stderr.log').read_text())
        raise ValueError(f'Cargo build failed: exit={result.returncode}')
    return artifacts


def install_binaries(prefix):
    artifacts = build_binaries(ROOT, ('volicord', 'volicord-viewer', 'volicord-mcp'), release=True)
    destination = Path(prefix) / 'bin'
    destination.mkdir(parents=True, exist_ok=True)
    for name, artifact in artifacts.items():
        if identity(artifact['path']) != artifact:
            raise ValueError('selected Cargo executable changed before installation')
        subprocess.run(['install', '-m', '755', artifact['path'], str(destination / name)], check=True)
        if digest(destination / name) != artifact['sha256']:
            raise ValueError('installed executable differs from Cargo artifact')
    if CONTEXT in os.environ:
        receipt(read_context(), {'kind': 'installed_executables', 'artifacts': artifacts,
                                'installed': {name: identity(destination/name) for name in artifacts}})


def main():
    try:
        if sys.argv[1] == 'install':
            install_binaries(sys.argv[2])
            return 0
        if sys.argv[1] == 'execute':
            return execute(sys.argv[2:])
        return observe_compiler(sys.argv[1:])
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        if CONTEXT in os.environ:
            try:
                receipt(read_context(), {'kind': 'observer_refused', 'argv': sys.argv[1:],
                                         'observer_exit_code': 2, 'reason': str(error)})
            except (OSError, ValueError, KeyError):
                pass
        print(f'build boundary refused: {error}', file=sys.stderr)
        return 2


if __name__ == '__main__':
    raise SystemExit(main())
