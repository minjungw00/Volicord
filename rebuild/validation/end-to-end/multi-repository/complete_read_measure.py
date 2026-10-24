#!/usr/bin/env python3
"""Compare complete MCP reads with independent recording inputs, without generation.

Private configuration supplies exact selectors, permitted Runtime copies, original
realizations/record receipts/public answers and their unchanged repository copies.
All attempts require new output directories. Cold means a new server process, not
flushed OS caches. This is focused technical evidence, not reader acceptance.
"""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import threading
import time


def write(path, value):
    with Path(path).open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)


def digest(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


class Server:
    def __init__(self, binary, case, output):
        startup = time.perf_counter()
        self.output = output
        self.stderr = (output / "server.stderr").open("xb")
        self.process = subprocess.Popen(
            [str(binary)], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=self.stderr, env={**os.environ, "VOLICORD_RUNTIME_DIR": case["runtime"]},
        )
        self.number = 0
        self.rows = []
        self.rss = 0
        self.stopped = threading.Event()
        self.sampler = threading.Thread(target=self.sample, daemon=True)
        self.sampler.start()
        request = dict(jsonrpc="2.0", id=0, method="initialize", params={"capabilities": {}})
        encoded = json.dumps(request).encode() + b"\n"
        self.process.stdin.write(encoded)
        self.process.stdin.flush()
        raw = self.process.stdout.readline()
        with (output / "initialize.request.json").open("xb") as stream:
            stream.write(encoded)
        with (output / "initialize.response.json").open("xb") as stream:
            stream.write(raw)
        assert "result" in json.loads(raw), raw
        self.process.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n')
        self.process.stdin.flush()
        self.startup_seconds = time.perf_counter() - startup
        self.startup_cpu_seconds = self.cpu()
        self.startup_transport_bytes = len(raw)

    def sample(self):
        while not self.stopped.wait(0.01):
            try:
                fields = Path(f"/proc/{self.process.pid}/status").read_text().splitlines()
                self.rss = max([self.rss] + [int(s.split()[1]) for s in fields if s.startswith("VmHWM:")])
            except (FileNotFoundError, ProcessLookupError):
                return

    def cpu(self):
        # comm may contain spaces; fields after its closing parenthesis start at 3.
        fields = Path(f"/proc/{self.process.pid}/stat").read_text().rsplit(")", 1)[1].split()
        return sum(int(fields[n]) for n in (11, 12)) / os.sysconf("SC_CLK_TCK")

    def call(self, name, args, phase):
        self.number += 1
        request = dict(jsonrpc="2.0", id=self.number, method="tools/call",
                       params=dict(name=name, arguments=args))
        encoded = json.dumps(request, ensure_ascii=False).encode() + b"\n"
        start = time.perf_counter()
        self.process.stdin.write(encoded)
        self.process.stdin.flush()
        raw = self.process.stdout.readline()
        elapsed = time.perf_counter() - start
        stem = self.output / f"call-{self.number:04}"
        with stem.with_suffix(".request.json").open("xb") as stream:
            stream.write(encoded)
        with stem.with_suffix(".response.json").open("xb") as stream:
            stream.write(raw)
        response = json.loads(raw)
        result = response["result"]
        assert "error" not in response and not result.get("isError"), response
        self.rows.append(dict(phase=phase, elapsed_seconds=elapsed,
                              request_bytes=len(encoded), transport_bytes=len(raw)))
        # Includes the JSON-RPC envelope and all escaped/structured representations.
        assert len(raw) <= 65536, len(raw)
        return result["structuredContent"]

    def close(self):
        self.process.stdin.close()
        code = self.process.wait(timeout=30)
        self.stopped.set()
        self.sampler.join()
        self.stderr.close()
        write(self.output / "server-exit.json", dict(exit_code=code, peak_rss_kib=self.rss))
        assert code == 0, code


def original(case):
    realization = json.loads(Path(case["realization"]).read_bytes())
    public = json.loads(Path(case["public"]).read_bytes())
    receipt = json.loads(Path(case["recording_receipt"]).read_bytes())
    retained = receipt["explanation"]
    assert retained["realization"] == realization
    for key in ("project_id", "subject", "question", "generated_at_unix_micros",
                "generator_identity_status", "semantic_approval_status"):
        assert public["provenance"][key] == retained[key]
    assert public["provenance"]["fingerprint"] == realization["plan_fingerprint"]
    assert public["provenance"]["generator"] == realization["generator"]
    assert public["provenance"]["language"] == realization["language"]
    used = {key for unit in realization["paragraphs"] for key in unit["evidence_keys"]}
    assert public["provenance"]["evidence"] == [e for e in retained["evidence"] if e["key"] in used]
    if case.get("publication_receipt"):
        original_read = json.loads(Path(case["publication_receipt"]).read_bytes())
        expected_provenance = dict(public["provenance"])
        if "citations" not in original_read["provenance"]:
            # Compact read receipts carry the original unit bindings separately.
            # The recording realization below remains the citation oracle.
            expected_provenance.pop("citations")
        assert original_read["provenance"] == expected_provenance
        case["expected_publication_id"] = original_read["publication_id"]
    # The original record response, rather than compact_publication, is the oracle.
    citations = [c for u in realization["paragraphs"] for c in u["citations"]]
    assert public["provenance"]["citations"] == citations
    windows = {}
    for i, unit in enumerate(realization["paragraphs"]):
        for j, citation in enumerate(unit["citations"]):
            c = citation
            assert digest(unit["text"].encode()[c["claim_start_byte"]:c["claim_end_byte"]]) == c["claim_sha256"]
            key = (c["source_id"], c["field"], c["start_byte"], c["end_byte"], c["use_kind"])
            if key not in windows:
                assert c["field"] == "body", "This corpus reader expects original file Sources"
                data = (Path(case["repository"]) / c["locator"]).read_bytes()[c["start_byte"]:c["end_byte"]]
                assert digest(data) == c["range_sha256"]
                windows[key] = (i, j, c, data)
    return realization, public, list(windows.values())


def journey(server, case, expected, output):
    output.mkdir()
    realization, public, windows = expected
    start = time.perf_counter()
    cpu = server.cpu()
    before = len(server.rows)
    args = dict(action="page", project_id=case["project"], subject_kind=case["subject_kind"],
                subject_id=case["subject_id"], language=case["language"], offset=0)
    chunks = []
    while True:
        check_start = time.time_ns() // 1000
        page = server.call("explanation_read", args, "publication")
        check_end = time.time_ns() // 1000
        chunk = page["chunk"].encode()
        assert len(chunk) <= 8192
        assert page["offset"] == sum(map(len, chunks))
        assert page["applicability"]["limitation"] is None, page["applicability"]
        if windows:
            assert check_start <= page["applicability"]["checked_at_unix_micros"] <= check_end
        chunks.append(chunk)
        if page["next_offset"] is None:
            assert page["state"] == "complete"
            break
        assert page["state"] == "partial"
        args.update(offset=page["next_offset"], expected_fingerprint=page["fingerprint"])
    reconstruction_start = time.perf_counter()
    data = b"".join(chunks)
    assert digest(data) == page["fingerprint"]
    doc = json.loads(data)
    if case.get("expected_publication_id"):
        assert doc["publication_id"] == case["expected_publication_id"]
    units = [dict(u, citations=[dict(doc["bindings"][c["binding"]], **{
        k: v for k, v in c.items() if k != "binding"
    }) for c in u["citations"]]) for u in doc["units"]]
    assert units == realization["paragraphs"]
    # The independently preserved recording is the semantic oracle, including
    # equal-worded endpoints, guarded groups and parallel message vector order.
    assert doc["diagrams"] == realization["diagrams"]
    expected_provenance = {k: v for k, v in public["provenance"].items() if k != "citations"}
    if "publication_id" not in expected_provenance:
        # Historical measurement oracle: pin the actual original public receipt,
        # never infer a managed identity from wording or the helper under test.
        assert case.get("expected_publication_id"), "An original public receipt must pin the publication"
        expected_provenance["publication_id"] = case["expected_publication_id"]
    assert doc["provenance"] == expected_provenance
    reconstruction = time.perf_counter() - reconstruction_start
    write(output / "expanded-publication.json", dict(doc, units=units))
    source_bytes = 0
    for i, j, citation, original_bytes in windows:
        args = dict(action="citation", project_id=case["project"], subject_kind=case["subject_kind"],
                    subject_id=case["subject_id"], language=case["language"], publication_id=doc["publication_id"],
                    realization_fingerprint=public["provenance"]["realization_fingerprint"],
                    unit_index=i, citation_index=j, offset=0)
        parts = []
        while True:
            selected = server.call("explanation_read", args, "evidence")
            chunk = selected["chunk"]
            assert chunk["citation"] == citation
            assert selected["claim"] == realization["paragraphs"][i]["text"].encode()[
                citation["claim_start_byte"]:citation["claim_end_byte"]].decode()
            part = chunk["text"].encode()
            assert len(part) <= 4096
            parts.append(part)
            if chunk["next_offset"] is None:
                break
            args["offset"] = chunk["next_offset"]
        assert b"".join(parts) == original_bytes
        source_bytes += len(original_bytes)
    rows = server.rows[before:]
    startup = dict(elapsed_seconds=server.startup_seconds, cpu_seconds=server.startup_cpu_seconds,
                   transport_bytes=server.startup_transport_bytes) if before == 0 else dict(
                       elapsed_seconds=0, cpu_seconds=0, transport_bytes=0)
    metrics = dict(elapsed_seconds=time.perf_counter() - start + startup["elapsed_seconds"],
                   cpu_seconds=server.cpu() - cpu + startup["cpu_seconds"], startup=startup,
                   peak_rss_kib=server.rss, content_bytes=len(data), evidence_bytes=source_bytes,
                   transport_bytes=sum(r["transport_bytes"] for r in rows), calls=len(rows),
                   reconstruction_seconds=reconstruction, phases={p: dict(
                       calls=sum(r["phase"] == p for r in rows),
                       elapsed_seconds=sum(r["elapsed_seconds"] for r in rows if r["phase"] == p),
                   ) for p in ("publication", "evidence")})
    write(output / "metrics.json", metrics)
    return metrics


def measure(binary, case, expected, output, mode, repetitions):
    output.mkdir()
    results = []
    if mode == "warm":
        server = Server(binary, case, output)
        try:
            journey(server, case, expected, output / "warmup")
            for n in range(repetitions):
                results.append(journey(server, case, expected, output / f"repeat-{n}"))
        finally:
            server.close()
    else:
        for n in range(repetitions):
            attempt = output / f"repeat-{n}"
            attempt.mkdir()
            readers = 2 if mode == "contention" else 1
            barrier = threading.Barrier(readers)
            def read(index):
                child = attempt / f"reader-{index}"
                child.mkdir()
                server = Server(binary, case, child)
                try:
                    barrier.wait(timeout=30)
                    return journey(server, case, expected, child / "journey")
                finally:
                    server.close()
            with ThreadPoolExecutor(max_workers=readers) as pool:
                results.extend(pool.map(read, range(readers)))
    write(output / "summary.json", dict(mode=mode, samples=len(results), results=results,
          statistics={key: dict(median=statistics.median(r[key] for r in results),
                                min=min(r[key] for r in results), max=max(r[key] for r in results))
                      for key in ("elapsed_seconds", "cpu_seconds", "peak_rss_kib", "transport_bytes", "calls")}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--mode", choices=("cold", "warm", "contention"), required=True)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--case")
    args = parser.parse_args()
    assert args.repetitions > 0
    args.output.mkdir()  # Refuse any existing attempt, including partial failures.
    write(args.output / "invocation.json", dict(binary=str(args.binary.resolve()),
          binary_sha256=digest(args.binary.read_bytes()), cases_sha256=digest(args.cases.read_bytes()),
          driver_sha256=digest(Path(__file__).read_bytes()),
          head=subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
          mode=args.mode, repetitions=args.repetitions,
          cold_definition="new process; OS caches uncontrolled", percentile_policy="no p95 at these small sample sizes"))
    cases = [case for case in json.loads(args.cases.read_bytes())
             if not args.case or args.case == case["label"]]
    assert cases, "No matching publication cases"
    assert len({case["label"] for case in cases}) == len(cases), "Duplicate case labels"
    for case in cases:
        assert case["label"] and all(c.isascii() and (c.isalnum() or c in "-_") for c in case["label"])
        write(args.output / (case["label"] + "-inputs.json"), {
            key: dict(path=case[key], sha256=digest(Path(case[key]).read_bytes()))
            for key in ("realization", "recording_receipt", "public", "publication_receipt") if case.get(key)
        })
        expected = original(case)
        measure(args.binary.resolve(), case, expected, args.output / case["label"], args.mode, args.repetitions)
        print(case["label"], args.mode, "passed", flush=True)


if __name__ == "__main__":
    main()
