#!/usr/bin/env python3
"""Cross-consumer original-byte controls; synthetic receipts, no user history."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from citation_originals import citation_bytes

HERE = Path(__file__).resolve().parent


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


class Originals(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.body = "검사\r\n1 passed\n".encode()
        self.observation = {
            "format_kind": "volicord_source_observation", "format_version": 4,
            "project_id": [2] * 16, "work_item_id": [3] * 16,
            "source_id": [1] * 16, "repository_source_id": [4] * 16,
            "source_revision": 1, "analysis_snapshot_id": "a" * 64,
            "repository_snapshot_id": "b" * 64,
            "detail": {"kind": "command", "stdout": {
                "body": self.body.decode(), "representation": "whole_stream"},
                "stderr": {"body": "separate stderr\n", "representation": "whole_stream"}},
        }
        fingerprint = digest(json.dumps(self.observation, ensure_ascii=False,
                                        sort_keys=True, separators=(",", ":")).encode())
        self.citation = dict(source_id="01" * 16, source_revision=1,
                             repository_source_id="04" * 16,
                             analysis_snapshot_id="a" * 64, repository_snapshot_id="b" * 64,
                             observation_fingerprint=fingerprint, locator="Focused assertion",
                             field="stdout", use_kind="observed_execution",
                             start_byte=0, end_byte=len(self.body), range_sha256=digest(self.body))
        self.raw = dict(observation=self.observation, observation_fingerprint=fingerprint,
                        citation_basis={k: self.citation[k] for k in (
                            "source_id", "source_revision", "analysis_snapshot_id", "repository_snapshot_id",
                            "repository_source_id", "observation_fingerprint")},
                        command_source={"command_label": "Focused assertion"})
        self.path = self.root / "original.json"
        self.path.write_text(json.dumps(self.raw, ensure_ascii=False))
        self.case = dict(project="02" * 16, work="03" * 16, repository=str(self.root),
                         source_observations={"01" * 16: str(self.path)})

    def node(self, case, citation):
        source = "const x=JSON.parse(require('fs').readFileSync(0));process.stdout.write(require(process.argv[1]).citationBytes(x.case,x.citation).toString('base64'));"
        return subprocess.run(["node", "-e", source, str(HERE / "citation_originals.cjs")],
                              input=json.dumps(dict(case=case, citation=citation)).encode(), capture_output=True)

    def both(self, case, citation, expected):
        self.assertEqual(citation_bytes(case, citation), expected)
        node = self.node(case, citation)
        self.assertEqual(node.returncode, 0, node.stderr.decode())
        import base64
        self.assertEqual(base64.b64decode(node.stdout), expected)

    def reject(self, case, citation):
        with self.assertRaises((AssertionError, KeyError, UnicodeError, OSError)):
            citation_bytes(case, citation)
        self.assertNotEqual(self.node(case, citation).returncode, 0)

    def test_whole_streams_partial_ranges_unicode_crlf_and_file_originals(self):
        self.both(self.case, self.citation, self.body)
        subset = dict(self.citation, start_byte=len("검사\r\n".encode()), range_sha256=digest(b"1 passed\n"))
        self.both(self.case, subset, b"1 passed\n")
        stderr = b"separate stderr\n"
        self.both(self.case, dict(self.citation, field="stderr", end_byte=len(stderr), range_sha256=digest(stderr)), stderr)
        (self.root / "file.py").write_bytes(self.body)
        self.both(self.case, dict(self.citation, field="body", use_kind="current_code", locator="file.py"), self.body)

    def test_foreign_identity_binding_field_and_ranges_cannot_substitute(self):
        variants = [
            (dict(self.case, project="ff" * 16), self.citation),
            (dict(self.case, work="ff" * 16), self.citation),
            (dict(self.case, work=None), self.citation),
            (self.case, dict(self.citation, source_id="ff" * 16)),
            (self.case, dict(self.citation, source_revision=2)),
            (self.case, dict(self.citation, analysis_snapshot_id="c" * 64)),
            (self.case, dict(self.citation, repository_snapshot_id="c" * 64)),
            (self.case, dict(self.citation, repository_source_id="ff" * 16)),
            (self.case, dict(self.citation, field="unknown")),
            (self.case, dict(self.citation, use_kind="current_code")),
            (self.case, dict(self.citation, locator="another invocation")),
            (self.case, dict(self.citation, end_byte=len(self.body) + 1)),
            (self.case, dict(self.citation, end_byte=1, range_sha256=digest(self.body[:1]))),
        ]
        for case, citation in variants:
            with self.subTest(case=case, citation=citation):
                self.reject(case, citation)

    def test_original_tampering_wrong_version_and_body_path_fallback_are_rejected(self):
        for mutate in (lambda raw: raw["observation"].update(format_version=3),
                       lambda raw: raw["observation"]["detail"]["stdout"].update(body="changed")):
            raw = copy.deepcopy(self.raw)
            mutate(raw)
            self.path.write_text(json.dumps(raw))
            self.reject(self.case, self.citation)
        self.path.write_text(json.dumps(self.raw))
        (self.root / "Focused assertion").write_bytes(self.body)
        self.reject(self.case, dict(self.citation, field="body"))
        self.reject(self.case, dict(self.citation, field="body", use_kind="current_code", locator="../escape"))


if __name__ == "__main__":
    unittest.main()
