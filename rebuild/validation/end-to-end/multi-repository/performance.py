"""Local V11 resource measurements; never retain RPC arguments, responses or source bodies."""
from __future__ import annotations

import json
import math
import os
from pathlib import Path
import threading
import time

METRICS = {"mcp_peak_rss_bytes", "max_snapshot_bytes", "v11_duration_ms", "max_mcp_call_ms"}


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
        self.snapshot_errors = 0

    def snapshots(self, env):
        if not self.enabled:
            return
        runtime = env.get("VOLICORD_RUNTIME_DIR")
        if runtime:
            try:
                for path in (Path(runtime) / "derived/analysis").glob("*/*.json"):
                    self.max_snapshot_bytes = max(self.max_snapshot_bytes, path.stat().st_size)
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
    observed = {**{key: 100 for key in METRICS}, "mcp_sample_count": 1, "mcp_call_count": 1, "sampling_error_count": 0}
    assert qualify(observed, limits)["status"] == "passed"
    for key in METRICS:
        assert qualify({**observed, key: 101}, limits)["status"] == "failed"
        assert qualify({**observed, key: float("nan")}, limits)["status"] == "failed"
    for key, value in [("mcp_sample_count", 0), ("mcp_call_count", 0), ("sampling_error_count", 1), ("max_snapshot_bytes", 0)]:
        assert qualify({**observed, key: value}, limits)["status"] == "failed"
    collector = Collector()
    collector.enabled = True
    with collector.measurement(os.getpid(), "self_check", {}):
        pass
    assert collector.calls[0]["process_peak_rss_bytes"] > 0
    assert collector.calls[0]["sample_count"] > 0
    assert set(collector.calls[0]) == {"operation", "duration_ms", "process_peak_rss_bytes", "sample_count", "sampling_error_count"}
