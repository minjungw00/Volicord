#!/usr/bin/env python3
"""Finite authored regression execution for the mandatory local rehearsal."""
import json
import sys
import unittest

from rehearsal_contract import COLLECTION_CONTROLS, digest


def main():
    checks = []
    for identity, name in COLLECTION_CONTROLS.items():
        result = unittest.TextTestRunner(stream=sys.stderr, verbosity=2).run(
            unittest.defaultTestLoader.loadTestsFromName(name))
        checks.append({'id': identity, 'test_name': name, 'tests_run': result.testsRun,
            'status': 'passed' if result.wasSuccessful() and result.testsRun == 1 else 'failed',
            'errors': len(result.errors), 'failures': len(result.failures)})
    value = {'kind': 'dogfood_collection_support', 'purpose': 'self_authored_support', 'checks': checks}
    value['result_id'] = digest(value)
    print(json.dumps(value, indent=2, sort_keys=True))
    return int(any(c['status'] != 'passed' for c in checks))


if __name__ == '__main__':
    raise SystemExit(main())
