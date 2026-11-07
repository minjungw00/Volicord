"""Independent original-byte oracles for files and recorded command observations.

Private validation inputs only. An original receipt supplies expected bytes, never
current permission; the actual public consumer must still admit every read.
"""
import hashlib
import json
from pathlib import Path


def identity(value):
    assert isinstance(value, list) and len(value) == 16, "original identity shape"
    assert all(type(n) is int and 0 <= n <= 255 for n in value), "original identity bytes"
    return bytes(value).hex()


def citation_bytes(case, citation):
    field = citation["field"]
    if field == "body":
        assert citation["use_kind"] in ("current_code", "before_code"), "file evidence role"
        root = Path(case["repository"]).resolve(strict=True)
        path = (root / citation["locator"]).resolve(strict=True)
        assert path.is_relative_to(root), "original file escapes repository"
        body = path.read_bytes()
    else:
        assert field in ("stdout", "stderr"), "unsupported original field"
        assert citation["use_kind"] == "observed_execution", "command evidence role"
        raw = json.loads(Path(case["source_observations"][citation["source_id"]]).read_bytes())
        observation, binding = raw["observation"], raw["citation_basis"]
        assert observation["format_kind"] == "volicord_source_observation"
        assert observation["format_version"] == 4, "current original observation format"
        assert identity(observation["project_id"]) == case["project"], "original Project"
        assert case.get("work") and identity(observation["work_item_id"]) == case["work"], "original Work"
        assert identity(observation["source_id"]) == citation["source_id"], "original Source"
        assert identity(observation["repository_source_id"]) == citation["repository_source_id"], "original repository Source"
        for key in ("source_id", "source_revision", "analysis_snapshot_id", "repository_snapshot_id",
                    "repository_source_id", "observation_fingerprint"):
            assert binding[key] == citation[key], f"original binding {key}"
        assert observation["source_revision"] == citation["source_revision"]
        assert observation["analysis_snapshot_id"] == citation["analysis_snapshot_id"]
        assert observation["repository_snapshot_id"] == citation["repository_snapshot_id"]
        encoded = json.dumps(observation, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
        fingerprint = "sha256:" + hashlib.sha256(encoded).hexdigest()
        assert fingerprint == raw["observation_fingerprint"] == citation["observation_fingerprint"], "original observation fingerprint"
        assert observation["detail"]["kind"] == "command", "original command kind"
        assert raw["command_source"]["command_label"] == citation["locator"], "original command label"
        stream = observation["detail"][field]
        assert stream["representation"] in ("whole_stream", "bounded_excerpt")
        body = stream["body"].encode("utf-8")
    start, end = citation["start_byte"], citation["end_byte"]
    assert type(start) is int and type(end) is int and 0 <= start < end <= len(body), "original byte range"
    selected = body[start:end]
    selected.decode("utf-8")
    assert "sha256:" + hashlib.sha256(selected).hexdigest() == citation["range_sha256"], "original range hash"
    return selected
