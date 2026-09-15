"""Local V11 resource measurements; never retain RPC arguments, responses or source bodies."""
from __future__ import annotations

import json
import math
import os
from pathlib import Path
import threading
import time
import tempfile

METRICS = {
    "mcp_peak_rss_bytes", "max_snapshot_bytes", "v11_duration_ms", "max_mcp_call_ms",
    "analysis_storage_logical_bytes", "analysis_storage_physical_bytes",
    "post_warmup_analysis_growth_bytes", "analysis_storage_bytes_per_graph_item",
}


def qualify(observed: dict, limits: dict) -> dict:
    valid = set(limits) == METRICS and all(
        type(limits[key]) is int and limits[key] > 0
        and type(observed.get(key)) in (int, float)
        and math.isfinite(observed[key]) and observed[key] >= 0
        for key in METRICS
    )
    measured = valid and all(type(observed.get(key)) is int and observed[key] > 0
                            for key in ("mcp_sample_count", "mcp_call_count"))
    measured = measured and observed.get("sampling_error_count") == 0 and observed.get("max_snapshot_bytes", 0) > 0
    measured = measured and observed.get("analysis_snapshot_count", 0) > 0 and observed.get("analysis_graph_item_count", 0) > 0
    exceeded = sorted(key for key in METRICS if valid and observed[key] > limits[key])
    return {"status": "passed" if measured and not exceeded else "failed",
            "limits": limits, "observed": observed, "exceeded": exceeded,
            "measurement_complete": bool(measured)}


def maintained_limits():
    policy = json.loads(Path(__file__).with_name("performance-budgets.json").read_text())
    if policy.get("policy_version") != 1:
        raise ValueError("unsupported performance budget policy")
    return policy["limits"]


def accepted(report):
    return isinstance(report, dict) and report == qualify(report.get("observed", {}), maintained_limits()) and report["status"] == "passed"


class Collector:
    def __init__(self):
        self.enabled = False
        self.calls = []
        self.max_snapshot_bytes = 0
        self.analysis_storage_logical_bytes = 0
        self.analysis_storage_physical_bytes = 0
        self.analysis_snapshot_count = 0
        self.analysis_graph_item_count = 0
        self.project_warmup_bytes = {}
        self.post_warmup_analysis_growth_bytes = 0
        self.snapshot_errors = 0

    def snapshots(self, env):
        if not self.enabled:
            return
        runtime = env.get("VOLICORD_RUNTIME_DIR")
        if runtime:
            try:
                analysis_root = Path(runtime) / "derived/analysis"
                manifests = list(analysis_root.glob("*/*.json"))
                project_totals = {}
                graph_items = 0
                for path in manifests:
                    stat = path.stat()
                    self.max_snapshot_bytes = max(self.max_snapshot_bytes, stat.st_size)
                    value = json.loads(path.read_text())
                    graph_items += sum(value.get(key, 0) for key in (
                        "inventory_entry_count", "entity_count", "relation_count"))
                logical = physical = 0
                for project in analysis_root.glob("*"):
                    if not project.is_dir():
                        continue
                    project_bytes = 0
                    for path in project.rglob("*"):
                        if not path.is_file():
                            continue
                        stat = path.stat()
                        logical += stat.st_size
                        physical += getattr(stat, "st_blocks", 0) * 512
                        project_bytes += stat.st_size
                    project_totals[str(project)] = project_bytes
                self.analysis_storage_logical_bytes = max(self.analysis_storage_logical_bytes, logical)
                self.analysis_storage_physical_bytes = max(self.analysis_storage_physical_bytes, physical)
                self.analysis_snapshot_count = max(self.analysis_snapshot_count, len(manifests))
                self.analysis_graph_item_count = max(self.analysis_graph_item_count, graph_items)
                for project, total in project_totals.items():
                    baseline = self.project_warmup_bytes.setdefault(project, total)
                    self.post_warmup_analysis_growth_bytes = max(
                        self.post_warmup_analysis_growth_bytes, max(0, total - baseline))
            except OSError:
                self.snapshot_errors += 1

    def measurement(self, pid, operation, env):
        return Measurement(self, pid, operation, env)

    def report(self, duration_ms):
        limits = maintained_limits()
        observed = {
            "mcp_peak_rss_bytes": max((x["process_peak_rss_bytes"] for x in self.calls), default=0),
            "max_snapshot_bytes": self.max_snapshot_bytes,
            "v11_duration_ms": duration_ms,
            "max_mcp_call_ms": max((x["duration_ms"] for x in self.calls), default=0),
            "analysis_storage_logical_bytes": self.analysis_storage_logical_bytes,
            "analysis_storage_physical_bytes": self.analysis_storage_physical_bytes,
            "post_warmup_analysis_growth_bytes": self.post_warmup_analysis_growth_bytes,
            "analysis_storage_bytes_per_graph_item": (
                self.analysis_storage_logical_bytes // max(1, self.analysis_graph_item_count)),
            "analysis_snapshot_count": self.analysis_snapshot_count,
            "analysis_graph_item_count": self.analysis_graph_item_count,
            "mcp_total_call_ms": round(sum(x["duration_ms"] for x in self.calls), 3),
            "mcp_call_count": len(self.calls),
            "mcp_sample_count": sum(x["sample_count"] for x in self.calls),
            "sampling_error_count": self.snapshot_errors + sum(x["sampling_error_count"] for x in self.calls),
        }
        return qualify(observed, limits)


class Measurement:
    def __init__(self, collector, pid, operation, env):
        self.collector, self.pid, self.operation, self.env = collector, pid, operation, env
        self.peak, self.samples, self.errors = 0, 0, 0
        self.stop = threading.Event()

    def sample(self):
        try:
            status = Path(f"/proc/{self.pid}/status").read_text()
            values = [int(line.split()[1]) * 1024 for line in status.splitlines() if line.startswith("VmHWM:")]
            if values:
                self.peak = max(self.peak, values[0])
                self.samples += 1
            else:
                self.errors += 1
        except FileNotFoundError:
            pass  # A process that exited is handled by its RPC/exit result.
        except (OSError, ValueError):
            self.errors += 1

    def monitor(self):
        while not self.stop.wait(0.05):
            self.sample()

    def __enter__(self):
        self.started = time.monotonic_ns()
        if self.collector.enabled:
            self.sample()
            self.thread = threading.Thread(target=self.monitor, daemon=True)
            self.thread.start()
        return self

    def __exit__(self, *_):
        if self.collector.enabled:
            self.stop.set()
            self.thread.join()
            self.sample()
            self.collector.snapshots(self.env)
            self.collector.calls.append({"operation": self.operation,
                "duration_ms": round((time.monotonic_ns() - self.started) / 1_000_000, 3),
                "process_peak_rss_bytes": self.peak, "sample_count": self.samples,
                "sampling_error_count": self.errors})


def self_check():
    limits = {key: 100 for key in METRICS}
    observed = {**{key: 100 for key in METRICS}, "mcp_sample_count": 1, "mcp_call_count": 1,
                "sampling_error_count": 0, "analysis_snapshot_count": 1,
                "analysis_graph_item_count": 1}
    assert qualify(observed, limits)["status"] == "passed"
    for key in METRICS:
        assert qualify({**observed, key: 101}, limits)["status"] == "failed"
        assert qualify({**observed, key: float("nan")}, limits)["status"] == "failed"
    for key, value in [("mcp_sample_count", 0), ("mcp_call_count", 0), ("sampling_error_count", 1), ("max_snapshot_bytes", 0), ("analysis_snapshot_count", 0), ("analysis_graph_item_count", 0)]:
        assert qualify({**observed, key: value}, limits)["status"] == "failed"
    stable = {**observed, "post_warmup_analysis_growth_bytes": 0}
    assert qualify(stable, limits)["status"] == "passed"
    assert qualify({**stable, "analysis_storage_logical_bytes": 101}, limits)["status"] == "failed"
    assert qualify({**stable, "analysis_storage_bytes_per_graph_item": 101}, limits)["status"] == "failed"

    with tempfile.TemporaryDirectory() as temporary:
        runtime = Path(temporary)
        project = runtime / "derived/analysis/project"
        blobs = project / "blobs"
        blobs.mkdir(parents=True)
        first = {"inventory_entry_count": 2, "entity_count": 3, "relation_count": 5}
        (project / "first.json").write_text(json.dumps(first))
        (blobs / "first.shape").write_bytes(b"s" * 100)
        storage = Collector()
        storage.enabled = True
        storage.snapshots({"VOLICORD_RUNTIME_DIR": str(runtime)})
        baseline = storage.analysis_storage_logical_bytes
        (project / "second.json").write_text(json.dumps(first))
        (blobs / "second.values").write_bytes(b"v" * 37)
        storage.snapshots({"VOLICORD_RUNTIME_DIR": str(runtime)})
        assert storage.analysis_snapshot_count == 2
        assert storage.analysis_graph_item_count == 20
        assert storage.analysis_storage_logical_bytes > baseline
        assert storage.analysis_storage_physical_bytes >= storage.analysis_storage_logical_bytes
        assert storage.post_warmup_analysis_growth_bytes == storage.analysis_storage_logical_bytes - baseline
    collector = Collector()
    collector.enabled = True
    with collector.measurement(os.getpid(), "self_check", {}):
        pass
    assert collector.calls[0]["process_peak_rss_bytes"] > 0
    assert collector.calls[0]["sample_count"] > 0
    assert set(collector.calls[0]) == {"operation", "duration_ms", "process_peak_rss_bytes", "sample_count", "sampling_error_count"}
