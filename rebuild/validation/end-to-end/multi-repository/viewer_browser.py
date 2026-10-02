#!/usr/bin/env python3
"""Bounded V11 browser support; no provider, installation, gate or human verdict."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.request

import harness

ROOT = harness.ROOT
HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures/viewer-reading"


class Blocked(RuntimeError):
    def __init__(self, status, detail):
        super().__init__(detail)
        self.status = status


def candidate():
    return subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()


def clean():
    return not subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT)


def snapshot_boundary(page):
    """Independent closed static surface check, also used by sensitivity self-tests."""
    from html.parser import HTMLParser

    class Surface(HTMLParser):
        def __init__(self):
            super().__init__()
            self.ids, self.links, self.failures = [], [], []
            self.in_style = False

        def css_references(self, value):
            for reference in re.findall(r"url\(([^)]+)\)", value, re.I):
                reference = reference.strip(" \"' ")
                if reference.startswith("#"):
                    self.links.append(reference[1:])
                else:
                    self.failures.append("snapshot_token_or_asset")
            if "@import" in value.lower():
                self.failures.append("snapshot_token_or_asset")

        def handle_data(self, data):
            if self.in_style:
                self.css_references(data)

        def handle_endtag(self, tag):
            if tag == "style":
                self.in_style = False

        def handle_starttag(self, tag, attrs):
            attrs = dict(attrs)
            if tag == "style":
                self.in_style = True
            if "id" in attrs:
                self.ids.append(attrs["id"])
            if tag in {"script", "form", "iframe", "object", "embed", "input", "button"}:
                self.failures.append("snapshot_active_content")
            for key, value in attrs.items():
                if value and key in {"style", "marker-end", "marker-start", "fill", "stroke", "filter", "clip-path", "mask"}:
                    self.css_references(value)
                if key.startswith("on") or key in {"src", "srcset", "action", "formaction"}:
                    self.failures.append("snapshot_active_attribute")
                if key in {"href", "xlink:href"}:
                    if not value or not value.startswith("#"):
                        self.failures.append("snapshot_live_link")
                    else:
                        self.links.append(value[1:])
            if tag == "meta" and attrs.get("http-equiv", "").lower() == "refresh":
                self.failures.append("snapshot_refresh")

    surface = Surface()
    surface.feed(page)
    if len(surface.ids) != len(set(surface.ids)):
        surface.failures.append("duplicate_fragment")
    if any(link not in surface.ids for link in surface.links):
        surface.failures.append("missing_fragment")
    if "request_authenticity" in page:
        surface.failures.append("snapshot_token_or_asset")
    return sorted(set(surface.failures))


def inspect_record(result):
    # Recorder already preserves full logs, exit/signal and group cleanup. No retry.
    if result["outcome"] != "succeeded":
        raise RuntimeError(f"{result['command']}: {result['outcome']}; see {result['stderr']}")
    return harness.decoded(result)


def validate_read_samples(samples, budgets, timing_enforced):
    """Independently require every named cold/warm sample and every stage/count."""
    expected = {(route, sample) for route in budgets["required_counts"] for sample in range(9)}
    seen = set()
    for row in samples:
        key = (row.get("workload"), row.get("sample"))
        if key not in expected or key in seen or type(key[1]) is not int:
            raise RuntimeError("Unexpected or duplicate cold/warm read sample")
        seen.add(key)
        route = key[0]
        for field, count in budgets["required_counts"][route].items():
            if type(row.get(field)) is not int or row[field] != count:
                raise RuntimeError(f"Read count invariant failed: {key}/{field}")
        for field in ["html_sha256", "canonical_basis_sha256"]:
            if not isinstance(row.get(field), str) or not re.fullmatch(r"[0-9a-f]{64}", row[field]):
                raise RuntimeError(f"Missing output/basis identity: {key}/{field}")
        if row.get("adapter") != "fresh-first-then-warm":
            raise RuntimeError("Read sample changed the adapter cold/warm definition")
        for stage, ceiling in budgets["ceilings_us"][route].items():
            value = row.get(stage + "_us")
            if type(value) is not int or value < 0:
                raise RuntimeError(f"Missing read stage measurement: {key}/{stage}")
            if timing_enforced and value > ceiling:
                raise RuntimeError(f"Read stage ceiling exceeded: {key}/{stage}")
    if seen != expected:
        raise RuntimeError("Missing cold/warm read samples")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--chromium", required=True, type=Path)
    parser.add_argument("--playwright-module", required=True, type=Path,
                        help="Existing playwright-core package directory; never installed by this check")
    parser.add_argument("--library-path", type=Path, help="Optional preinstalled Chromium shared libraries")
    parser.add_argument("--bin-dir", type=Path, help="Installed sibling executables from this candidate")
    parser.add_argument("--require-clean", action="store_true", help="Required for final candidate evidence")
    parser.add_argument("--enforce-read-budgets", action="store_true",
                        help="Use unchanged named debug workload timing ceilings on the documented hardware")
    args = parser.parse_args()
    output_parent = ROOT / "rebuild/.local/validation"
    output_parent.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="viewer-browser-", dir=output_parent))
    recorder = harness.Recorder(output)
    env = os.environ.copy()
    if args.library_path:
        env["LD_LIBRARY_PATH"] = str(args.library_path.resolve()) + (":" + env["LD_LIBRARY_PATH"] if env.get("LD_LIBRARY_PATH") else "")
    result = {"kind": "viewer_browser_support", "candidate_head": candidate(),
              "initial_clean": clean(), "status": "not_run", "human_qualification": "not_established",
              "gate": "separate_not_invoked", "external_transmission": "none", "credentials": "not_used", "transmission_authorization": "not_required_for_local_support",
              "environment": {"system": platform.system(), "release": platform.release(),
                              "machine": platform.machine(), "python": platform.python_version()},
              "checks": {}, "operations": [], "artifacts": str(output)}
    process = None
    started = time.monotonic()

    def run(label, argv, *, extra=None, timeout=300, cwd=ROOT):
        record = recorder.run(label, [str(a) for a in argv], {**env, **(extra or {})}, timeout=timeout, cwd=cwd)
        result["operations"].append(record)
        return inspect_record(record)

    try:
        if args.require_clean and not result["initial_clean"]:
            raise Blocked("candidate_blocked", "Final browser evidence requires a clean committed candidate")
        for name in ("node", "cargo", "python3"):
            if not shutil.which(name):
                raise Blocked("tooling_blocked", f"Missing {name}")
        if not args.chromium.is_file():
            raise Blocked("browser_unavailable", "Explicit Chromium executable is absent")
        package = args.playwright_module / "package.json"
        if not package.is_file():
            raise Blocked("driver_unavailable", "Explicit playwright-core package is absent")
        if not shutil.which("fc-list"):
            raise Blocked("font_prerequisite_blocked", "fontconfig fc-list is required to verify Korean glyph coverage")
        fonts = run("korean-fonts", ["fc-list", "--format", "%{file}\n", ":lang=ko"]).splitlines()
        if not fonts:
            raise Blocked("font_prerequisite_blocked", "No installed Korean font; configure fontconfig before the check instead of accepting missing-glyph screenshots")
        result["environment"]["korean_fonts"] = [{"path":p,"sha256":harness.sha256(Path(p))} for p in sorted(set(fonts))]
        if env.get("FONTCONFIG_FILE"):
            result["environment"]["fontconfig"] = {"path":env["FONTCONFIG_FILE"],"sha256":harness.sha256(Path(env["FONTCONFIG_FILE"]))}
        try:
            with socket.socket() as probe:
                probe.bind(("127.0.0.1", 0))
        except OSError as error:
            raise Blocked("loopback_blocked", str(error)) from error
        browser_probe = recorder.run("browser-version", [str(args.chromium), "--version"], env)
        result["operations"].append(browser_probe)
        if browser_probe["outcome"] != "succeeded":
            raise Blocked("browser_prerequisite_blocked", "Chromium version probe failed; inspect the full loader/launch log")
        run("driver-load", ["node", "-e", "const p=require(process.argv[1]);if(!p.chromium)process.exit(1)", str(args.playwright_module.resolve())])
        result["environment"].update({
            "browser_version": harness.decoded(browser_probe).strip(),
            "browser_sha256": harness.sha256(args.chromium),
            "driver_version": json.loads(package.read_text())["version"],
            "driver_package_sha256": harness.sha256(package),
            "node": run("node-version", ["node", "--version"]).strip(),
            "rustc": run("rustc-version", ["rustc", "--version"]).strip(),
            "logical_cpus": os.cpu_count(),
            "cpu": next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")), "unknown"),
        })
        if args.enforce_read_budgets:
            named = json.loads((HERE / "viewer-read-budgets.json").read_text())["environment"]
            observed = result["environment"]
            matches = (named["os"] == f'{observed["system"]} {observed["release"]} {observed["machine"]}'
                and named["cpu"].split(";")[0] == observed["cpu"] and observed["logical_cpus"] == 16
                and observed["rustc"].removeprefix("rustc ") == named["rustc"])
            if not matches:
                raise Blocked("budget_environment_blocked", "Timing enforcement requires the unchanged named hardware/tool/profile conditions; counts remain available without this option")
        result["inputs"] = {str(p.relative_to(ROOT)): harness.sha256(p) for p in [
            ROOT / "rebuild/Cargo.lock", FIXTURE / "scenario.json", FIXTURE / "expected.json",
            ROOT / "rebuild/crates/volicord-viewer/tests/browser_fixture.rs",
            ROOT / "rebuild/crates/volicord-viewer/tests/reading.rs",
            ROOT / "rebuild/crates/volicord-operations/tests/support/reading_fixture.rs",
            HERE / "viewer_browser.py", HERE / "viewer_browser_driver.cjs", FIXTURE / "answer-cases.json", HERE / "viewer-read-budgets.json",
            HERE / "harness.py",
        ]}
        result["fixture_tree_sha256"] = harness.tree_hash(FIXTURE)
        run("build-candidate", ["cargo", "build", "--manifest-path", "rebuild/Cargo.toml", "-p", "volicord-operations", "-p", "volicord-viewer"])
        binaries = args.bin_dir.resolve() if args.bin_dir else ROOT / "rebuild/target/debug"
        run("seed-runtime", ["cargo", "test", "--manifest-path", "rebuild/Cargo.toml", "-p", "volicord-viewer", "--test", "browser_fixture", "seed_browser_runtime", "--", "--exact", "--nocapture"], extra={"VOLICORD_VIEWER_FIXTURE_ROOT": str(output)})
        run("rebuild-after-fixture", ["cargo", "build", "--manifest-path", "rebuild/Cargo.toml", "-p", "volicord-operations", "-p", "volicord-viewer"])
        result["executables"] = {name: {"path": str(binaries / name), "sha256": harness.sha256(binaries / name)} for name in ("volicord", "volicord-viewer")}
        fixture = json.loads((output / "fixture.json").read_text())
        runtime = Path(fixture["runtime"])
        repository = Path(fixture["repository"])
        result["repository_fixture_tree_sha256"] = harness.tree_hash(repository)
        result["runtime_fixture_sha256"] = harness.sha256(output / "fixture.json")
        # Canonical semantic comparison uses deterministic public bundle export, not DB file bytes.
        before = output / "before.json"
        base = [binaries / "volicord", "--runtime", runtime, "--project", fixture["project"]]
        run("canonical-before", [*base, "context", "export", "--output", before], cwd=repository)
        absent_base = [binaries / "volicord", "--runtime", runtime, "--project", fixture["purpose_absent_project"]]
        run("canonical-absent-before", [*absent_base, "context", "export", "--output", output / "absent-before.json"], cwd=repository)
        snapshots = {}
        absent_snapshots = {}
        for locale in ("en", "ko"):
            destination = output / f"snapshot-{locale}.html"
            run("snapshot-" + locale, [*base, "--locale", locale, "viewer", "export", "--output", destination, "--language", locale], cwd=repository)
            failures = snapshot_boundary(destination.read_text())
            result["checks"]["snapshot-boundary-" + locale] = {"status": "failed" if failures else "passed", "failures": failures}
            if failures:
                raise RuntimeError(f"Snapshot boundary: {failures}")
            snapshots[locale] = str(destination)
            absent_destination = output / f"purpose-absent-{locale}.html"
            run("purpose-absent-" + locale, [binaries / "volicord", "--runtime", runtime, "--project", fixture["purpose_absent_project"], "--locale", locale, "viewer", "export", "--output", absent_destination, "--language", locale], cwd=repository)
            if snapshot_boundary(absent_destination.read_text()):
                raise RuntimeError("Purpose-absent snapshot boundary failed")
            absent_snapshots[locale] = str(absent_destination)
        prefix_snapshots=[]
        for prefix in fixture["prefixes"]:
            for locale in ("en","ko"):
                destination=output / f"prefix-{prefix['prefix']}-{locale}.html"
                run(f"prefix-{prefix['prefix']}-{locale}", [binaries / "volicord", "--runtime", prefix["runtime"], "--project", prefix["project"], "--locale", locale, "viewer", "export", "--output", destination, "--language", locale])
                if snapshot_boundary(destination.read_text()):
                    raise RuntimeError("Prefix snapshot boundary failed")
                prefix_snapshots.append({**prefix,"locale":locale,"snapshot":str(destination)})
        stdout = (output / "server.stdout.log").open("wb")
        stderr = (output / "server.stderr.log").open("wb")
        argv = [str(a) for a in [*base, "viewer", "open", "--view", "overview", "--bind", "127.0.0.1:0", "--language", "en"]]
        harness.write_json(output / "server-command.json", {"argv": argv, "cwd": str(repository)})
        try:
            process = subprocess.Popen(argv, cwd=repository, env=env, stdout=stdout, stderr=stderr, start_new_session=True)
        finally:
            stdout.close()
            stderr.close()
        deadline = time.monotonic() + 15
        url = None
        while time.monotonic() < deadline:
            lines = (output / "server.stderr.log").read_text().splitlines()
            url = next((line.split("Volicord local viewer: ", 1)[1] for line in lines if line.startswith("Volicord local viewer: http://127.0.0.1:")), None)
            if url:
                break
            if process.poll() is not None:
                raise RuntimeError("CLI/server exited during startup; inspect server.stderr.log")
            time.sleep(0.05)
        if not url:
            raise Blocked("listener_blocked", "No actual ephemeral server authority within 15 seconds")
        extension = output / "zoom-extension"
        extension.mkdir()
        harness.write_json(extension / "manifest.json", {"manifest_version": 3, "name": "Viewer supporting tab zoom", "version": "1.0", "permissions": ["tabs"], "background": {"service_worker": "worker.js"}})
        (extension / "worker.js").write_text("chrome.runtime.onInstalled.addListener(() => {});\n")
        config = {"output": str(output), "fixture": fixture, "scenario": json.loads((FIXTURE / "scenario.json").read_text()),
                  "expected": json.loads((FIXTURE / "expected.json").read_text()), "url": url,
                  "chromium": str(args.chromium.resolve()), "playwright": str(args.playwright_module.resolve()),
                  "extension": str(extension), "snapshots": snapshots, "absent_snapshots": absent_snapshots, "prefix_snapshots":prefix_snapshots}
        harness.write_json(output / "browser-config.json", config)
        # Isolate actual HTTP completion from automation input/frame observations.
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        completions = []
        for route, query in [("overview", "view=overview"), ("work", "view=work&work=" + fixture["goals"]["older"]), ("decision", "view=decisions&decision=" + fixture["decisions"]["explicit"]), ("code", "view=code&scope=repository")]:
            for sample in range(9):
                began = time.monotonic_ns()
                with opener.open(url + "?" + query, timeout=15) as response:
                    body = response.read()
                    if response.status != 200:
                        raise RuntimeError("HTTP completion sample failed")
                completions.append({"route":route, "sample":sample, "completion_us":(time.monotonic_ns()-began)//1000, "bytes":len(body), "cold_definition":"first route request in this server, then eight warm; no OS cache flush", "scope":"response completion including loopback transport, not browser input or paint"})
        harness.write_json(output / "http-completions.json", completions)
        result["checks"]["http-completion"] = {"status":"passed", "samples":len(completions)}
        browser = recorder.run("live-browser", ["node", str(HERE / "viewer_browser_driver.cjs"), str(output / "browser-config.json"), "live"], env, timeout=240)
        result["operations"].append(browser)
        if (output / "live-result.json").is_file():
            result["checks"]["live-browser"] = json.loads((output / "live-result.json").read_text())
        if result["checks"].get("live-browser", {}).get("status") == "browser_launch_blocked":
            raise Blocked("browser_launch_blocked", "Driver could not launch Chromium; inspect live-result and full browser streams")
        inspect_record(browser)
        restored_tree = harness.tree_hash(repository)
        result["checks"]["fixture-restoration"] = {"status":"passed" if restored_tree == result["repository_fixture_tree_sha256"] and Path(fixture["analysis_directory"]).is_dir() else "failed", "repository_tree_sha256":restored_tree}
        if result["checks"]["fixture-restoration"]["status"] != "passed":
            raise RuntimeError("Browser source/analysis controls did not restore the fixture")
        cleanup = harness.cleanup_process_group(process)
        harness.write_json(output / "server-result.json", {"exit_code":process.returncode, "cleanup":cleanup, "stop_cause":"intentional_shutdown_for_offline_check"})
        process = None
        if not cleanup["complete"]:
            raise RuntimeError("Server process group cleanup incomplete")
        run("canonical-after", [*base, "context", "export", "--output", output / "after.json"], cwd=repository)
        run("canonical-absent-after", [*absent_base, "context", "export", "--output", output / "absent-after.json"], cwd=repository)
        if before.read_bytes() != (output / "after.json").read_bytes():
            raise RuntimeError("Viewer paths changed canonical bundle")
        if (output / "absent-before.json").read_bytes() != (output / "absent-after.json").read_bytes():
            raise RuntimeError("Purpose-absent snapshot changed canonical bundle")
        result["checks"]["canonical-read-purity"] = {"status":"passed", "before_sha256":harness.sha256(before), "after_sha256":harness.sha256(output / "after.json"), "purpose_absent_before_sha256":harness.sha256(output / "absent-before.json"), "purpose_absent_after_sha256":harness.sha256(output / "absent-after.json")}
        # Runtime physically unavailable; no substitute HTTP server for exported snapshots.
        runtime.rename(runtime.with_name("runtime-unavailable"))
        offline = recorder.run("offline-browser", ["node", str(HERE / "viewer_browser_driver.cjs"), str(output / "browser-config.json"), "offline"], env, timeout=240)
        result["operations"].append(offline)
        if (output / "offline-result.json").exists():
            result["checks"]["offline-browser"] = json.loads((output / "offline-result.json").read_text())
        inspect_record(offline)
        # Cargo test may relink binaries; preserve the bytes actually exercised.
        evidence_bins = output / "tested-bin"
        evidence_bins.mkdir()
        for name, identity in result["executables"].items():
            preserved = evidence_bins / name
            shutil.copy2(identity["path"], preserved)
            identity["preserved_path"] = str(preserved)
            if harness.sha256(preserved) != identity["sha256"]:
                raise RuntimeError("Tested executable changed before preservation")
        budget_extra = {"VOLICORD_VIEWER_BUDGETS": "1"} if args.enforce_read_budgets else {}
        samples = run("read-profiles", ["cargo", "test", "--manifest-path", "rebuild/Cargo.toml", "-p", "volicord-viewer", "--test", "reading", "requested_sections_on_large_repository", "--", "--exact", "--nocapture"], extra=budget_extra)
        raw_samples = [json.loads(line.split("VIEWER_READ_SAMPLE ", 1)[1]) for line in samples.splitlines() if line.startswith("VIEWER_READ_SAMPLE ")]
        validate_read_samples(raw_samples, json.loads((HERE / "viewer-read-budgets.json").read_text()), args.enforce_read_budgets)
        harness.write_json(output / "read-samples.json", raw_samples)
        result["checks"]["read-cost"] = {"status":"passed", "samples":len(raw_samples), "timing_enforced":args.enforce_read_budgets, "budget_sha256":harness.sha256(HERE / "viewer-read-budgets.json"), "cold_definition":"adapter cold, not OS cache cold", "browser_timing":"separate diagnostic input/two-frame/navigation observations; no percentile claim"}
        result["status"] = "passed"
    except Blocked as error:
        result.update(status=error.status, detail=str(error))
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        result.update(status="failed", detail=str(error))
    finally:
        if process is not None:
            result["server_cleanup"] = harness.cleanup_process_group(process)
        result["final_candidate_head"] = candidate()
        result["final_clean"] = clean()
        if result["final_candidate_head"] != result["candidate_head"] or (args.require_clean and result["initial_clean"] and not result["final_clean"]):
            result.update(prior_status=result["status"], prior_detail=result.get("detail"), status="candidate_changed", detail="Candidate continuity failed")
        for name, identity in result.get("executables", {}).items():
            if harness.sha256(Path(identity.get("preserved_path", identity["path"]))) != identity["sha256"]:
                result.update(prior_status=result["status"], prior_detail=result.get("detail"), status="executable_changed", detail=name)
        result["screenshots"] = [{"path":p.name,"sha256":harness.sha256(p),"bytes":p.stat().st_size} for p in sorted(output.glob("*.png"))]
        result["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        summary={"candidate_head":result["candidate_head"],"status":result["status"],"human_qualification":"not_established",
                 "checks":{k:{"status":v.get("status"),"checks":[{"id":c["id"],"status":c["status"],**({"reason":c["reason"]} if "reason" in c else {})} for c in v.get("checks",[])]} for k,v in result["checks"].items()},
                 "executables":result.get("executables"),"environment":result["environment"]}
        harness.write_json(output / "browser-summary.json",summary)
        harness.write_json(output / "result.json", result)
        print(json.dumps({"status":result["status"], "candidate_head":result["candidate_head"], "result":str(output / "result.json"), "detail":result.get("detail")}, indent=2))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
