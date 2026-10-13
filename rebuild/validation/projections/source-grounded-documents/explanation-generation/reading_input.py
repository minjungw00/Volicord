"""Read immutable input with independently hash-checked historical producer bytes."""
import copy
import json
from pathlib import Path
import re
import subprocess

from inputs import binding, check_binding, digest, require, validate


def verify_reading_input(path):
    spec = json.loads(Path(path).read_bytes())
    check_binding(spec['preparation_source'])
    validated = copy.deepcopy(spec)
    producer = spec['identities']['experiment_producer']
    adapter = producer.get('adapter')
    if adapter:
        try:
            check_binding(adapter)
        except (ValueError, OSError):
            # Historical provenance is not a requirement that today's producer
            # still have those bytes. No content or Source dependency is waived.
            root = Path(__file__).resolve().parents[5]
            expected = Path(__file__).with_name('archive.py').resolve()
            require(Path(adapter['path']).resolve() == expected, 'unsupported historical producer path')
            head = producer['head']
            require(re.fullmatch(r'[0-9a-f]{40}', head), 'historical producer revision invalid')
            relative = expected.relative_to(root).as_posix()
            result = subprocess.run(['git', 'show', head + ':' + relative], cwd=root,
                                    capture_output=True, timeout=10)
            require(result.returncode == 0, 'historical producer bytes unavailable')
            require(len(result.stdout) == adapter['bytes'] and digest(result.stdout) == adapter['sha256'],
                    'historical producer hash differs')
            validated['identities']['experiment_producer']['adapter'] = None
    validate(validated)
    for entry in spec['entries']:
        if entry['asset'] is not None:
            check_binding(entry['asset'])
    return spec
