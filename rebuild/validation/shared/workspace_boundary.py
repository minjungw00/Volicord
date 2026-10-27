#!/usr/bin/env python3
"""Inspect nested Cargo ownership in an exclusive evidence attempt directory.

Replacement entry point for the auxiliary workspace-boundary diagnostic. It
never reuses a hard-coded historical evidence directory, including on failure.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]


def write(path, data):
    with path.open('xb') as stream:
        stream.write(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--context', type=Path)
    parser.add_argument('--cargo', default='cargo')
    args = parser.parse_args()
    output = args.output.absolute()
    # The maintained measurement drivers use this same exclusive-directory
    # primitive. Refuse before spawning Cargo or opening any evidence file.
    output.mkdir(parents=True, exist_ok=False)
    result = {'status': 'failed', 'exit_code': None, 'wrapper_exit_code': 1}
    try:
        command = [args.cargo, 'metadata', '--manifest-path', 'rebuild/Cargo.toml',
                   '--no-deps', '--format-version', '1']
        process = subprocess.run(command, cwd=ROOT, capture_output=True)
        result.update(command=command, exit_code=process.returncode)
        write(output / 'metadata.stdout', process.stdout)
        write(output / 'metadata.stderr', process.stderr)
        if process.returncode:
            raise ValueError(f'Cargo metadata failed: exit {process.returncode}')
        metadata = json.loads(process.stdout)
        rebuild = ROOT / 'rebuild'
        names = {p['name'] for p in metadata['packages']}
        for package in metadata['packages']:
            if not Path(package['manifest_path']).resolve().is_relative_to(rebuild):
                raise ValueError('package outside reconstruction workspace')
            for dependency in package['dependencies']:
                if dependency.get('path') and not Path(dependency['path']).resolve().is_relative_to(rebuild):
                    raise ValueError('dependency outside reconstruction workspace')
                if dependency['name'].startswith('volicord-') and dependency['name'] not in names:
                    raise ValueError('legacy Volicord dependency')
        result.update(packages=len(names), all_under_rebuild=True,
                      no_legacy_dependencies=True)
        if args.context:
            context = args.context.resolve(strict=True)
            relative = context.relative_to(ROOT)
            ignored = subprocess.run(['git', 'check-ignore', '--quiet', os.fspath(relative)], cwd=ROOT)
            tracked = subprocess.run(['git', 'ls-files', '--error-unmatch', '--', os.fspath(relative)],
                                     cwd=ROOT, capture_output=True)
            if ignored.returncode != 0 or tracked.returncode == 0:
                raise ValueError('context must remain ignored and untracked')
            data = context.read_bytes()
            result['context'] = {'path': str(relative), 'bytes': len(data),
                                 'sha256': hashlib.sha256(data).hexdigest()}
        result.update(status='passed', wrapper_exit_code=0)
    except Exception as error:
        result['error'] = str(error)
    write(output / 'boundary-result.json', (json.dumps(result, indent=2) + '\n').encode())
    print(json.dumps(result))
    return result['wrapper_exit_code']


if __name__ == '__main__':
    raise SystemExit(main())
