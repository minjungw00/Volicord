"""Local V11 resource measurements; never retain RPC arguments, responses or source bodies."""
from __future__ import annotations

import json
import math
import os
from pathlib import Path
import runpy
import threading
import time
import tempfile

ANALYSIS_FORMAT_KIND = "volicord.repository_analysis"
ANALYSIS_FORMAT_VERSION = 5

METRICS = {
    "mcp_peak_rss_bytes", "max_snapshot_bytes", "v11_duration_ms", "max_mcp_call_ms",
    "analysis_storage_logical_bytes", "analysis_storage_physical_bytes",
    "post_warmup_analysis_growth_bytes", "analysis_storage_bytes_per_graph_item",
}

RSS_BANDS_BYTES = (
    256 * 1024 * 1024,
    512 * 1024 * 1024,
    1024 * 1024 * 1024,
    2 * 1024 * 1024 * 1024,
    3 * 1024 * 1024 * 1024,
    4 * 1024 * 1024 * 1024,
)


def parse_linux_process_memory(status: str) -> dict[str, int]:
    """Return current and lifetime-high RSS without accepting partial evidence."""

    values: dict[str, int] = {}
    for line in status.splitlines():
        name, separator, remainder = line.partition(":")
        if not separator or name not in {"VmRSS", "VmHWM"}:
            continue
        fields = remainder.split()
        if len(fields) != 2 or fields[1] != "kB":
            raise ValueError(f"unsupported {name} format")
        value = int(fields[0]) * 1024
        if value <= 0:
            raise ValueError(f"invalid {name} value")
        if name in values:
            raise ValueError(f"duplicate {name} value")
        values[name] = value
    if set(values) != {"VmRSS", "VmHWM"}:
        raise ValueError("Linux process memory status is incomplete")
    if values["VmHWM"] < values["VmRSS"]:
        raise ValueError("Linux process high-water RSS is below current RSS")
    return {
        "current_rss_bytes": values["VmRSS"],
        "high_water_rss_bytes": values["VmHWM"],
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


def limits_for_workload(policy: dict, workload_identity: str) -> dict:
    if (not isinstance(policy, dict) or policy.get("policy_version") != 2
            or policy.get("workload_identity") != workload_identity):
        raise ValueError("unsupported performance budget policy or V11 workload")
    limits = policy.get("limits")
    if (not isinstance(limits, dict) or set(limits) != METRICS
            or any(type(value) is not int or value <= 0 for value in limits.values())):
        raise ValueError("invalid performance budget limits")
    return limits


def maintained_limits():
    directory = Path(__file__).parent
    policy = json.loads((directory / "performance-budgets.json").read_text())
    workload_identity = runpy.run_path(str(directory / "result_contract.py"))["WORKLOAD_IDENTITY"]
    return limits_for_workload(policy, workload_identity)


def accepted(report):
    return isinstance(report, dict) and report == qualify(report.get("observed", {}), maintained_limits()) and report["status"] == "passed"


class Collector:
    def __init__(self, *, status_reader=None, sampling_interval_seconds=0.05):
        self.enabled = False
        self.calls = []
        self.max_snapshot_bytes = 0
        self.analysis_storage_logical_bytes = 0
        self.analysis_storage_physical_bytes = 0
        self.analysis_snapshot_count = 0
        self.analysis_graph_item_count = 0
        self.analysis_storage_bytes_per_graph_item = 0
        self.project_warmup_bytes = {}
        self.post_warmup_analysis_growth_bytes = 0
        self.snapshot_errors = 0
        self.status_reader = status_reader or self._read_status
        self.sampling_interval_seconds = sampling_interval_seconds
        self.processes = []
        self._automatic_processes = {}
        self.first_band_crossings = []
        self._crossed_bands = set()

    @staticmethod
    def _read_status(pid):
        return Path(f"/proc/{pid}/status").read_text()

    def register_process(self, pid):
        process = {
            "pid": pid,
            "process_instance": f"mcp-{len(self.processes) + 1:03d}",
            "call_count": 0,
        }
        self.processes.append(process)
        return process

    @staticmethod
    def completed_graph_items(path, value):
        """Recognize current published manifests, without loading graph bodies.

        Production publishes referenced blobs before its atomic manifest. This
        check observes that publication boundary; product integrity validation
        continues to own blob decoding/content verification.
        """
        if not isinstance(value, dict):
            return None
        project = {"identity": path.parent.name}
        metadata = value.get("metadata")
        if (value.get("format_kind") != ANALYSIS_FORMAT_KIND
                or type(value.get("format_version")) is not int or value["format_version"] != ANALYSIS_FORMAT_VERSION
                or value.get("storage_format") != "volicord.normalized_analysis"
                or value.get("identity") != path.stem or value.get("project") != project
                or not isinstance(metadata, dict) or metadata.get("identity") != path.stem
                or metadata.get("project") != project
                or not isinstance(metadata.get("repository_snapshot"), str)
                or not metadata["repository_snapshot"]
                or not isinstance(metadata.get("capabilities"), list)):
            return None
        count_keys = ("inventory_entry_count", "entity_count", "relation_count")
        if any(type(value.get(key)) is not int or value[key] < 0 for key in
               (*count_keys, "logical_json_bytes", "scalar_count", "generated_at_unix_micros")):
            return None
        shapes = value.get("shape_blobs")
        if not isinstance(shapes, list) or not shapes or "values_base_blob" not in value:
            return None
        references = [(item, "shape") for item in shapes] + [(value.get("values_blob"), "values")]
        if value["values_base_blob"] is not None:
            references.append((value["values_base_blob"], "values"))
        for digest, extension in references:
            if (not isinstance(digest, str) or len(digest) != 64
                    or any(char not in "0123456789abcdef" for char in digest)):
                return None
            blob = path.parent / "blobs" / f"{digest}.{extension}"
            if blob.is_symlink() or not blob.is_file() or blob.stat().st_size == 0:
                return None
        return sum(value[key] for key in count_keys)

    def snapshots(self, env):
        if not self.enabled:
            return
        runtime = env.get("VOLICORD_RUNTIME_DIR")
        if runtime:
            try:
                analysis_root = Path(runtime) / "derived/analysis"
                project_totals = {}
                project_graph_items = {}
                valid_manifest_count = 0
                for path in analysis_root.glob("*/*.json"):
                    stat = path.stat()
                    self.max_snapshot_bytes = max(self.max_snapshot_bytes, stat.st_size)
                    try:
                        value = json.loads(path.read_text())
                    except (ValueError, UnicodeError):
                        # Controlled corruption/incomplete publication still costs
                        # storage, but cannot establish a completed warmup baseline.
                        continue
                    items = self.completed_graph_items(path, value)
                    if items is None:
                        continue
                    valid_manifest_count += 1
                    project = str(path.parent)
                    project_graph_items[project] = project_graph_items.get(project, 0) + items
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
                self.analysis_snapshot_count = max(self.analysis_snapshot_count, valid_manifest_count)
                self.analysis_graph_item_count = max(self.analysis_graph_item_count, sum(project_graph_items.values()))
                for project, total in project_totals.items():
                    if project in project_graph_items:
                        baseline = self.project_warmup_bytes.setdefault(project, total)
                        if project_graph_items[project] > 0:
                            self.analysis_storage_bytes_per_graph_item = max(
                                self.analysis_storage_bytes_per_graph_item,
                                total / project_graph_items[project])
                    else:
                        baseline = self.project_warmup_bytes.get(project)
                    if baseline is not None:
                        self.post_warmup_analysis_growth_bytes = max(
                            self.post_warmup_analysis_growth_bytes, max(0, total - baseline))
            except OSError:
                self.snapshot_errors += 1

    def measurement(self, process, operation, env):
        if isinstance(process, int):
            registered = self._automatic_processes.get(process)
            if registered is None:
                registered = self.register_process(process)
                self._automatic_processes[process] = registered
            process = registered
        process["call_count"] += 1
        return Measurement(
            self,
            process,
            process["call_count"],
            operation,
            env,
        )

    def record(self, call):
        self.calls.append(call)
        before = call.get("high_water_rss_before_bytes")
        after = call.get("high_water_rss_after_bytes")
        if before is None or after is None:
            return
        for band in RSS_BANDS_BYTES:
            if band not in self._crossed_bands and before < band <= after:
                self._crossed_bands.add(band)
                self.first_band_crossings.append({
                    "rss_band_bytes": band,
                    "process_instance": call["process_instance"],
                    "call_ordinal": call["call_ordinal"],
                    "operation": call["operation"],
                })

    def diagnostics(self):
        return {
            "schema_version": 1,
            "rss_bands_bytes": list(RSS_BANDS_BYTES),
            "processes": [
                {
                    "process_instance": process["process_instance"],
                    "call_count": process["call_count"],
                }
                for process in self.processes
            ],
            "first_band_crossings": self.first_band_crossings,
            "calls": self.calls,
        }

    def report(self, duration_ms):
        limits = maintained_limits()
        observed = {
            "mcp_peak_rss_bytes": max(
                (x["process_peak_rss_bytes"] or 0 for x in self.calls), default=0
            ),
            "max_snapshot_bytes": self.max_snapshot_bytes,
            "v11_duration_ms": duration_ms,
            "max_mcp_call_ms": max((x["duration_ms"] for x in self.calls), default=0),
            "analysis_storage_logical_bytes": self.analysis_storage_logical_bytes,
            "analysis_storage_physical_bytes": self.analysis_storage_physical_bytes,
            "post_warmup_analysis_growth_bytes": self.post_warmup_analysis_growth_bytes,
            "analysis_storage_bytes_per_graph_item": self.analysis_storage_bytes_per_graph_item,
            "analysis_snapshot_count": self.analysis_snapshot_count,
            "analysis_graph_item_count": self.analysis_graph_item_count,
            "mcp_total_call_ms": round(sum(x["duration_ms"] for x in self.calls), 3),
            "mcp_call_count": len(self.calls),
            "mcp_sample_count": sum(x["sample_count"] for x in self.calls),
            "sampling_error_count": self.snapshot_errors + sum(x["sampling_error_count"] for x in self.calls),
        }
        return qualify(observed, limits)


class Measurement:
    def __init__(self, collector, process, call_ordinal, operation, env):
        self.collector = collector
        self.process = process
        self.call_ordinal = call_ordinal
        self.operation = operation
        self.env = env
        self.peak = 0
        self.current_peak = 0
        self.samples = 0
        self.errors = []
        self.before = None
        self.after = None
        self.stop = threading.Event()

    def sample(self, phase):
        try:
            sample = parse_linux_process_memory(
                self.collector.status_reader(self.process["pid"])
            )
            self.peak = max(self.peak, sample["high_water_rss_bytes"])
            self.current_peak = max(
                self.current_peak, sample["current_rss_bytes"]
            )
            self.samples += 1
            return sample
        except (OSError, ValueError) as error:
            if len(self.errors) < 8:
                self.errors.append({
                    "phase": phase,
                    "kind": type(error).__name__,
                })
            return None

    def monitor(self):
        while not self.stop.wait(self.collector.sampling_interval_seconds):
            self.sample("during")

    def __enter__(self):
        self.started = time.monotonic_ns()
        if self.collector.enabled:
            self.before = self.sample("before")
            if self.collector.sampling_interval_seconds is not None:
                self.thread = threading.Thread(target=self.monitor, daemon=True)
                self.thread.start()
        return self

    def __exit__(self, *_):
        if self.collector.enabled:
            self.stop.set()
            if self.collector.sampling_interval_seconds is not None:
                self.thread.join()
            self.after = self.sample("after")
            self.collector.snapshots(self.env)
            before_hwm = (
                self.before["high_water_rss_bytes"] if self.before else None
            )
            after_hwm = self.after["high_water_rss_bytes"] if self.after else None
            self.collector.record({
                "process_instance": self.process["process_instance"],
                "call_ordinal": self.call_ordinal,
                "operation": self.operation,
                "duration_ms": round((time.monotonic_ns() - self.started) / 1_000_000, 3),
                "current_rss_before_bytes": (
                    self.before["current_rss_bytes"] if self.before else None
                ),
                "current_rss_after_bytes": (
                    self.after["current_rss_bytes"] if self.after else None
                ),
                "current_rss_peak_during_call_bytes": self.current_peak or None,
                "high_water_rss_before_bytes": before_hwm,
                "high_water_rss_after_bytes": after_hwm,
                "process_peak_rss_bytes": self.peak or None,
                "established_new_high_water": (
                    after_hwm > before_hwm
                    if before_hwm is not None and after_hwm is not None
                    else None
                ),
                "sample_count": self.samples,
                "sampling_error_count": len(self.errors),
                "measurement_errors": self.errors,
            })


def storage_self_check():
    # Bind this operational publication check to the maintained current format.
    model = Path(__file__).resolve().parents[3] / "crates/volicord-repository-intelligence/src/model.rs"
    source = model.read_text()
    assert f'pub const ANALYSIS_SNAPSHOT_KIND: &str = "{ANALYSIS_FORMAT_KIND}";' in source
    assert f"pub const ANALYSIS_SNAPSHOT_FORMAT_VERSION: u32 = {ANALYSIS_FORMAT_VERSION};" in source
    def manifest(project, name, items=10):
        blobs = project / "blobs"
        blobs.mkdir(parents=True, exist_ok=True)
        shape, values = "a" * 64, "b" * 64
        (blobs / f"{shape}.shape").write_bytes(b"s" * 100)
        (blobs / f"{values}.values").write_bytes(b"v" * 37)
        value = {
            "format_kind": ANALYSIS_FORMAT_KIND, "format_version": ANALYSIS_FORMAT_VERSION,
            "storage_format": "volicord.normalized_analysis", "identity": name,
            "project": {"identity": project.name}, "generated_at_unix_micros": 1,
            "logical_json_bytes": 100, "scalar_count": 10,
            "shape_blobs": [shape], "values_blob": values, "values_base_blob": None,
            "inventory_entry_count": items, "entity_count": 0, "relation_count": 0,
            "metadata": {"identity": name, "project": {"identity": project.name},
                         "repository_snapshot": "c" * 64, "capabilities": []},
        }
        (project / f"{name}.json").write_text(json.dumps(value))
        return value

    with tempfile.TemporaryDirectory() as temporary:
        runtime = Path(temporary)
        project = runtime / "derived/analysis/project"
        project.mkdir(parents=True)
        env = {"VOLICORD_RUNTIME_DIR": str(runtime)}
        storage = Collector()
        storage.enabled = True
        storage.snapshots(env)
        assert storage.project_warmup_bytes == {}
        (project / "partial.data").write_bytes(b"p" * 300)
        for partial in ("{", "null", "[]", "{}", '{"inventory_entry_count":1}'):
            (project / "incomplete.json").write_text(partial)
            storage.snapshots(env)
            assert storage.project_warmup_bytes == {} and storage.analysis_snapshot_count == 0
        (project / "incomplete.json").unlink()
        first = manifest(project, "first")
        for field, invalid in (("format_kind", "volicord.analysis_snapshot"),
                               ("inventory_entry_count", -1), ("entity_count", True),
                               ("relation_count", "3"), ("metadata", {}),
                               ("shape_blobs", []), ("values_blob", "../escape"),
                               ("values_base_blob", "d" * 64), ("format_version", 99)):
            (project / "first.json").write_text(json.dumps({**first, field: invalid}))
            storage.snapshots(env)
            assert storage.project_warmup_bytes == {} and storage.analysis_snapshot_count == 0
        (project / "first.json").write_text(json.dumps(first))
        blob = project / "blobs" / ("b" * 64 + ".values")
        blob.unlink()
        storage.snapshots(env)
        assert storage.project_warmup_bytes == {}
        blob.write_bytes(b"")
        storage.snapshots(env)
        assert storage.project_warmup_bytes == {}
        blob.write_bytes(b"v" * 37)
        storage.snapshots(env)
        baseline = sum(path.stat().st_size for path in project.rglob("*") if path.is_file())
        assert storage.project_warmup_bytes == {str(project): baseline}
        assert storage.post_warmup_analysis_growth_bytes == 0
        manifest(project, "second")
        (project / "more.values").write_bytes(b"v" * 37)
        storage.snapshots(env)
        assert storage.analysis_snapshot_count == 2 and storage.analysis_graph_item_count == 20
        assert storage.analysis_storage_logical_bytes > baseline
        assert storage.analysis_storage_physical_bytes >= storage.analysis_storage_logical_bytes
        assert storage.post_warmup_analysis_growth_bytes == storage.analysis_storage_logical_bytes - baseline
        (project / "corrupt.json").write_text("{ controlled corruption")
        storage.snapshots(env)
        assert storage.analysis_snapshot_count == 2 and storage.snapshot_errors == 0
        assert storage.project_warmup_bytes[str(project)] == baseline
        assert storage.post_warmup_analysis_growth_bytes == storage.analysis_storage_logical_bytes - baseline

    with tempfile.TemporaryDirectory() as temporary:
        runtime = Path(temporary)
        project = runtime / "derived/analysis/early"
        first = manifest(project, "first", items=1)
        (project / "data").write_bytes(b"x" * 5000)
        storage = Collector()
        storage.enabled = True
        env = {"VOLICORD_RUNTIME_DIR": str(runtime)}
        storage.snapshots(env)
        early_ratio = sum(path.stat().st_size for path in project.rglob("*") if path.is_file())
        assert storage.analysis_storage_bytes_per_graph_item == early_ratio > 2048
        # A later high-item observation and another large low-ratio Project must
        # not dilute the original sample or cross-pair project bytes/items.
        manifest(project, "second", items=10000)
        other = runtime / "derived/analysis/later"
        manifest(other, "large", items=100000)
        (other / "data").write_bytes(b"x" * 100000)
        storage.snapshots(env)
        with storage.measurement(os.getpid(), "storage_counterexample", env):
            pass
        report = storage.report(1)
        assert report["measurement_complete"] and report["status"] == "failed"
        assert report["observed"]["analysis_storage_bytes_per_graph_item"] == early_ratio
        assert "analysis_storage_bytes_per_graph_item" in report["exceeded"]
        diluted = storage.analysis_storage_logical_bytes / storage.analysis_graph_item_count
        assert diluted < 2048
        assert storage.project_warmup_bytes[str(other)] == sum(
            path.stat().st_size for path in other.rglob("*") if path.is_file())

    # No floor rounding at a maintained threshold, and prior large scale metric
    # values remain ordinary valid numbers in the existing report schema.
    observed = {key: 1 for key in METRICS}
    observed.update(mcp_sample_count=1, mcp_call_count=1, sampling_error_count=0,
                    analysis_snapshot_count=1, analysis_graph_item_count=92704,
                    analysis_storage_bytes_per_graph_item=2048.25)
    assert "analysis_storage_bytes_per_graph_item" in qualify(observed, maintained_limits())["exceeded"]
    observed["analysis_storage_bytes_per_graph_item"] = 2048
    assert qualify(observed, maintained_limits())["status"] == "passed"


def self_check():
    policy = json.loads(Path(__file__).with_name("performance-budgets.json").read_text())
    current_limits = maintained_limits()
    assert current_limits["v11_duration_ms"] == 20 * 60 * 1000
    assert current_limits["post_warmup_analysis_growth_bytes"] == 800 * 1024 * 1024
    for invalid in ({**policy, "workload_identity": "previous-v11-workload"},
                    {**policy, "policy_version": 1},
                    {**policy, "limits": {**current_limits, "v11_duration_ms": 0}}):
        try:
            limits_for_workload(invalid, policy["workload_identity"])
        except ValueError:
            pass
        else:
            raise AssertionError("invalid V11 performance policy accepted")

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

    storage_self_check()
    collector = Collector()
    collector.enabled = True
    with collector.measurement(os.getpid(), "self_check", {}):
        pass
    assert collector.calls[0]["process_peak_rss_bytes"] > 0
    assert collector.calls[0]["sample_count"] > 0
    assert collector.calls[0]["current_rss_before_bytes"] > 0
    assert collector.calls[0]["current_rss_after_bytes"] > 0
    assert collector.calls[0]["high_water_rss_before_bytes"] > 0
    assert collector.calls[0]["high_water_rss_after_bytes"] > 0
    assert collector.calls[0]["established_new_high_water"] in (True, False)
    assert collector.calls[0]["measurement_errors"] == []

    samples = iter([
        "VmRSS:\t262144 kB\nVmHWM:\t262144 kB\n",
        "VmRSS:\t786432 kB\nVmHWM:\t1048576 kB\n",
        "VmRSS:\t524288 kB\nVmHWM:\t1048576 kB\n",
        "VmRSS:\t393216 kB\nVmHWM:\t1048576 kB\n",
    ])
    attributed = Collector(
        status_reader=lambda _pid: next(samples),
        sampling_interval_seconds=None,
    )
    attributed.enabled = True
    process = attributed.register_process(4242)
    with attributed.measurement(process, "first_growth", {}):
        pass
    with attributed.measurement(process, "inherited_high_water", {}):
        pass
    first, inherited = attributed.calls
    assert first["process_instance"] == inherited["process_instance"] == "mcp-001"
    assert (first["call_ordinal"], inherited["call_ordinal"]) == (1, 2)
    assert first["current_rss_before_bytes"] == 256 * 1024 * 1024
    assert first["current_rss_after_bytes"] == 768 * 1024 * 1024
    assert first["established_new_high_water"] is True
    assert inherited["current_rss_after_bytes"] == 384 * 1024 * 1024
    assert inherited["established_new_high_water"] is False
    assert attributed.first_band_crossings == [
        {
            "rss_band_bytes": 512 * 1024 * 1024,
            "process_instance": "mcp-001",
            "call_ordinal": 1,
            "operation": "first_growth",
        },
        {
            "rss_band_bytes": 1024 * 1024 * 1024,
            "process_instance": "mcp-001",
            "call_ordinal": 1,
            "operation": "first_growth",
        },
    ]

    failures = iter([
        "VmRSS:\t1 kB\n",
        "VmRSS:\tinvalid kB\nVmHWM:\t2 kB\n",
    ])
    unavailable = Collector(
        status_reader=lambda _pid: next(failures),
        sampling_interval_seconds=None,
    )
    unavailable.enabled = True
    with unavailable.measurement(unavailable.register_process(4343), "broken", {}):
        pass
    broken = unavailable.calls[0]
    assert broken["current_rss_before_bytes"] is None
    assert broken["current_rss_after_bytes"] is None
    assert broken["established_new_high_water"] is None
    assert broken["sampling_error_count"] == 2
    assert [error["phase"] for error in broken["measurement_errors"]] == [
        "before", "after"
    ]


if __name__ == "__main__":
    self_check()
    print(json.dumps({"status": "passed", "checks": ["memory-attribution", "completed-snapshot-warmup", "same-sample-project-storage-ratio"]}))
