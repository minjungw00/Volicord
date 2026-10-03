#!/usr/bin/env python3
"""Local Product-backed evidence pipeline; authored inputs never measured use.

Standalone execution is diagnostic. The gate runs this anew after ordered Final.
No host chat, provider, technical gate or operator approval is invoked here.
"""
from __future__ import annotations

import argparse
from contextlib import contextmanager
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import secrets
import selectors
import shutil
import signal
import subprocess
import sys
import time

sys.dont_write_bytecode = True

import campaign as c
import codex_events
import document_realization
import evidence_purpose as purpose
import explanation_evidence
import machine_findings
import qualification_policy
import qualitative_review
import resource_observer
import result_lineage
import rehearsal_support
import review_captures
import review_operations as review

from rehearsal_contract import CONTRACT, FIXTURE, identities, validate_result


def require(value, message):
    if not value:
        raise ValueError(message)


def rejected(operation):
    try:
        operation()
    except (ValueError, OSError, KeyError):
        return
    raise ValueError("negative control accepted")


@contextmanager
def changed(path, content=None):
    original, mode = path.read_bytes(), path.stat().st_mode & 0o777
    path.chmod(0o600)
    if content is None:
        path.unlink()
    else:
        path.write_bytes(content)
    try:
        yield
    finally:
        path.write_bytes(original)
        path.chmod(mode)


def controls(root, campaign_root, manifest, evaluation_path, review_root, qualified):
    checks = {}
    bundle_ref = manifest["journey_final_evidence"][0]["artifact_inventory"]["canonical_bundle"]
    bundle = c.harness.load_canonical_bundle(campaign_root / bundle_ref["file"])
    goals = [r for r in bundle.rows("context_items") if r["role"] == "goal"]
    require(len(goals) == 3 and len({r["id"] for r in goals}) == 3
        and len({r["statement"] for r in goals}) == 1, "duplicate titles lost distinct identities")
    checks["duplicate_titles"] = "passed"
    resource = manifest["naturalistic_memory_evidence"]
    rejected(lambda: resource_observer.validate(resource, "0" * 64))
    checks["memory_binary_mismatch"] = "passed"
    raw = campaign_root / "slots/journey-volicord-work-a/evidence/resume.rollout.jsonl"
    with changed(raw, raw.read_bytes() + b"\n"):
        rejected(lambda: c.load_evidence_set(campaign_root))
    checks["changed_artifact"] = "passed"
    preparation_path = explanation_evidence.preparations(campaign_root)[0]
    with changed(preparation_path):
        rejected(lambda: c.load_evidence_set(campaign_root))
    checks["missing_explanation_plan"] = "passed"
    response = c.read_json(preparation_path.parent / "response.json")
    response["plan_fingerprint"] = "sha256:" + "0" * 64
    rejected(lambda: explanation_evidence.validate_response(c.read_json(preparation_path)["plan"], response))
    checks["mismatched_explanation_response"] = "passed"
    # The actual observation engine consumes an independently substituted response.
    # Only a disposable diagnostic copy's raw/descriptor hash is updated.
    negative = root / "negative-answer"
    negative.mkdir()
    shutil.copytree(campaign_root / "tasks", negative / "tasks")
    shutil.copytree(campaign_root / "slots", negative / "slots")
    shutil.copytree(campaign_root / "journeys", negative / "journeys", ignore=shutil.ignore_patterns("repository", "runtime"))
    negative_raw = negative / raw.relative_to(campaign_root)
    events = [json.loads(line) for line in negative_raw.read_text().splitlines() if line]
    substituted = 0
    for event in events:
        payload = event["payload"]
        if payload.get("type") == "mcp_tool_call_end" and payload.get("invocation", {}).get("tool") == "recall":
            payload["result"]["Ok"]["structuredContent"]["selected_work"]["work_item_id"] = "0" * 32
            substituted += 1
    require(substituted == 1, "negative Recall was not uniquely selected")
    negative_raw.chmod(0o600)
    negative_raw.write_text("".join(json.dumps(e) + "\n" for e in events))
    descriptor = negative / "tasks/descriptors/journey-volicord-work-a.json"
    data = c.read_json(descriptor)
    data["evidence"]["captures"]["resume"]["sha256"] = c.harness.sha256(negative_raw)
    descriptor.chmod(0o600)
    c.write_json(descriptor, data)
    observed = c.evaluate_works(negative, manifest)[0]
    finding = next(f for f in observed["findings"] if f["check"] == "shared_answer_integrity")
    require(finding["status"] == "confirmed_violation" and finding["disposition"] == "hard_blocking",
        "contradictory shared answer evaded the maintained consumer")
    checks["contradictory_shared_answer"] = "passed"
    prep, _, _ = review.load_package(review_root)
    entry = next(e for e in prep["index"]["evidence"].values() if e["surface"] == "work_capture")
    projected = c.read_json(review_root / entry["path"])
    projected["records"] = []
    rejected(lambda: review_captures.validate(review.encoded(projected)))
    checks["review_projection_omission"] = "passed"
    require(qualified["qualitative_review"]["human_escalations"]
        and all(r["kind"] == "agent" for r in qualified["qualitative_review_runs"]), "fabricated human observations")
    checks["absent_human_observations"] = "passed"
    rejected(lambda: review.prepare(campaign_root, root / "forbidden-human", reviewer_kind="human"))
    rejected(lambda: qualification_policy.approval_value(qualified, b"", "support", "approve-phase-9"))
    checks["rehearsal_not_measured"] = "passed"
    c.load_evidence_set(campaign_root)
    return checks


def copied_control(root):
    """Refresh all wrapper hashes; incompatible review semantics still fail."""
    target = root.parent / "negative-lineage"
    shutil.copytree(root, target)
    path = target / "qualification/qualification.json"
    value = c.read_json(path)
    q = value["qualitative_review"]
    cid = next(cid for cid in q["unresolved_criteria"] if "/documents/" in cid)
    q["unresolved_criteria"].remove(cid)
    q["resolved_criteria"].append(cid)
    q["resolved_criteria"].sort()
    value["run_id"] = machine_findings.digest({k: v for k, v in value.items() if k != "run_id"})
    qualification_policy.validate_result(value)
    path.chmod(0o600)
    c.write_json(path, value)
    index = c.read_json(target / "index.json")
    index["qualification"].update(run_id=value["run_id"], sha256=c.harness.sha256(path))
    index["lineage_id"] = machine_findings.digest({k: v for k, v in index.items() if k != "lineage_id"})
    receipt = c.read_json(target / "receipt.json")
    receipt.update(lineage_id=index["lineage_id"], index_sha256=review.digest(review.encoded(index)))
    receipt["artifacts"] = {name: result_lineage._binding((target / name).read_bytes()) for name in receipt["artifacts"]}
    for name, value in (("index.json", index), ("receipt.json", receipt)):
        (target / name).chmod(0o600)
        c.write_json(target / name, value)
    rejected(lambda: result_lineage.verify(target))
    return "passed"


def shared_finding(root, manifest, slot):
    observed = next(w for w in c.evaluate_works(root, manifest) if w['work_slot_id'] == slot)
    return next(f for f in observed['findings'] if f['check'] == 'shared_answer_integrity')


def temporal_controls(root, campaign_root, manifest, temporal):
    slot = json.loads(FIXTURE.read_bytes())['temporal_scenarios']['recall']['work_slot_id']
    positive = shared_finding(campaign_root, manifest, slot)
    observations = positive['basis']['observations']
    require(positive['status'] == 'confirmed_pass', 'actual temporal Recall did not pass: ' + str(positive))
    require([o['goal_basis']['revision'] for o in observations] == [1, 2, 2], 'pre/post/resume revision evidence missing')
    require(all(o['status'] == 'confirmed_pass' for o in observations), 'one Recall is not independently verified')
    sources = observations[0]['goal_basis']['sources']
    require(sources and all(o['goal_basis']['sources'] == sources for o in observations), 'Goal Sources changed')
    correction = [w for w in observations[1]['goal_basis']['witnesses'] if w.get('transition') == 'successful_correction']
    require(len(correction) == 1 and correction[0]['authorization_source_id'] not in sources,
        'successful independent authorization witness missing')
    temporal['recall'] = {'work_slot_id': slot, 'supporting_sources': sources,
        'authorization_source_id': correction[0]['authorization_source_id'],
        'observations': [{k: o[k] for k in ('transport', 'call_id', 'sequence', 'completion_sequence',
            'invocation_sequence', 'raw_capture_sha256', 'status')} | {
            'goal_revision': o['goal_basis']['revision'], 'basis_sha256': machine_findings.digest(o['goal_basis'])}
            for o in observations], 'correction': correction[0]}
    outcomes = temporal['outcomes']
    capture_ref = c.read_json(campaign_root / f'tasks/descriptors/{slot}.json')['evidence']['captures']['work']
    artifacts = [manifest['artifacts'][capture_ref['file']]]
    outcomes['temporal_recall_correction'] = control_evidence(root, 'temporal_recall_correction',
        'shared_answer_integrity', positive['status'], 'naturalistic_observation', positive['basis'], artifacts)
    # Both actual Product-generated returns are current at their own observation;
    # the later correction is excluded from the earlier response's witness set.
    require(not any(w.get('kind') == 'correct_context' for w in observations[0]['goal_basis']['witnesses']),
        'future correction was applied retroactively')
    raw = campaign_root / capture_ref['file']
    original_events = [json.loads(line) for line in raw.read_text().splitlines() if line]
    for control in ('post_correction_old_revision', 'future_correction_scope', 'missing_temporal_evidence', 'missing_temporal_evidence_with_violation'):
        negative = root / ('negative-' + control)
        negative.mkdir()
        for name in ('tasks', 'slots', 'journeys'):
            shutil.copytree(campaign_root / name, negative / name, ignore=shutil.ignore_patterns('repository', 'runtime'))
        events = copy.deepcopy(original_events)
        recalls = [e['payload'] for e in events if e['payload'].get('type') == 'mcp_tool_call_end'
            and e['payload'].get('invocation', {}).get('tool') == 'recall']
        require(len(recalls) == 2, 'control needs two actual captured Recalls')
        if control in {'post_correction_old_revision', 'future_correction_scope'}:
            answer = recalls[1 if control == 'post_correction_old_revision' else 0]['result']['Ok']['structuredContent']['selected_work']['answers']
            require(answer['explanation_state'] == 'current', 'old-revision control lacks a current claim')
            next(e for e in answer['provenance']['evidence'] if e['key'] == 'goal')['revision'] = 1 if control == 'post_correction_old_revision' else 2
        else:
            mutation = next(e['payload'] for e in events if e['payload'].get('type') == 'mcp_tool_call_end'
                and e['payload'].get('invocation', {}).get('tool') == 'canonical_mutate')
            del mutation['result']['Ok']['structuredContent']['revision']
            if control.endswith('with_violation'):
                recalls[1]['result']['Ok']['structuredContent']['next_step'] = 'Deliberately contradictory verifier input'
        changed_raw = negative / raw.relative_to(campaign_root)
        changed_raw.chmod(0o600)
        changed_raw.write_text(''.join(json.dumps(e) + '\n' for e in events))
        descriptor_path = negative / f'tasks/descriptors/{slot}.json'
        descriptor = c.read_json(descriptor_path)
        descriptor['evidence']['captures']['work']['sha256'] = c.harness.sha256(changed_raw)
        descriptor_path.chmod(0o600)
        c.write_json(descriptor_path, descriptor)
        finding = shared_finding(negative, manifest, slot)
        target = finding['basis']['observations'][0 if control == 'future_correction_scope' else 1]
        expected = 'indeterminate' if control == 'missing_temporal_evidence' else 'confirmed_violation'
        require(target['status'] == expected and finding['status'] == expected,
            'temporal negative control outcome differs: ' + str(finding))
        require(finding['basis']['observations'][1 if control == 'future_correction_scope' else 0]['status'] == 'confirmed_pass', 'scoped control damaged an unchanged observation')
        if control in {'post_correction_old_revision', 'future_correction_scope'}:
            require('generated goal revision basis' in target['errors'], 'old revision rejected for wrong reason')
        else:
            require(target['goal_basis']['revision'] is None and target['limits'], 'missing revision fabricated certainty')
        outcomes[control] = control_evidence(root, control, 'shared_answer_integrity', finding['status'],
            'naturalistic_observation', finding['basis'], [explanation_evidence.binding(changed_raw.read_bytes())])
    c.load_evidence_set(campaign_root)  # None of the invalid controls altered valid frozen evidence.


def copied_temporal(copied, temporal):
    """Inspect the copied consumer's safe bodies/locators, with staging unavailable."""
    import review_explanations
    index = c.read_json(copied / 'index.json')
    review_root = copied / index['qualitative_reviews'][0]['root']
    preparation, _, _ = review.load_package(review_root)
    evidence = preparation['index']['evidence']
    lifecycle_bindings = {}
    for life in temporal['regeneration']['lifecycles']:
        entry = evidence['explanation-' + life['identity']]
        path = review_root / entry['path']
        value = review_explanations.validate(path.read_bytes())
        require(value['publication'] == {k: life[k] for k in ('publication_role', 'selected_identity')}
            and value['semantic_complete'] and value['readback_subject_locators']['after'], 'copied lifecycle lost classification/locators')
        require(all(value['private_artifacts'][k] == v for k, v in life['stages'].items()), 'copied historical/final bytes changed')
        goal = rehearsal_support.goal_evidence(value['stages']['plan']['value']['value']['plan'])
        require(goal['revision'] == life['goal_revision'] and goal['sources'] == life['goal_sources'], 'copied Goal revision/Source changed')
        lifecycle_bindings[life['identity']] = explanation_evidence.binding(path.read_bytes())
    matches = [v for v in evidence.values() if v['surface'] == 'work_capture'
        and temporal['recall']['work_slot_id'] in v['sample_ids']]
    require(len(matches) == 1, 'copied temporal capture missing')
    path = review_root / matches[0]['path']
    capture = review_captures.validate(path.read_bytes())
    mutation = next(r for r in capture['records'] if r.get('operation') == 'canonical_mutate')
    receipt = mutation['body']['value']['result']
    require(receipt['revision'] == 2 and receipt['user_response_source_id'] == temporal['recall']['authorization_source_id']
        and mutation['body']['value']['request']['expected_revision'] == 1
        and mutation['sequence'] == temporal['recall']['correction']['sequence']
        and mutation['completion_sequence'] == temporal['recall']['correction']['completion_sequence'], 'copied correction witness lost')
    require(any(p['value'].endswith('/body/value/result/revision') for p in matches[0]['locators']), 'copied revision locator missing')
    evaluation_path = copied / index['evaluation']['path']
    evaluation = c.read_json(evaluation_path)
    finding = next(f for w in evaluation['works'] if w['work_slot_id'] == temporal['recall']['work_slot_id']
        for f in w['findings'] if f['check'] == 'shared_answer_integrity')
    require([machine_findings.digest(o['goal_basis']) for o in finding['basis']['observations']]
        == [o['basis_sha256'] for o in temporal['recall']['observations']], 'copied invocation windows changed')
    return {'lifecycles': lifecycle_bindings, 'recall_capture': explanation_evidence.binding(path.read_bytes()),
        'evaluation': explanation_evidence.binding(evaluation_path.read_bytes())}


class Processes:
    """Private complete streams with bounded portable hash/exit observations."""
    def __init__(self, root, process_timeout=300):
        self.root, self.records = root, []
        self.process_timeout = process_timeout
        root.mkdir(mode=0o700)

    def run(self, argv, *, cwd=c.ROOT, input=None):
        label = f"process-{len(self.records):04d}"
        started = time.monotonic_ns()
        child = subprocess.Popen([str(a) for a in argv], cwd=cwd,
            stdin=subprocess.PIPE if input else subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
        try:
            stdout, stderr = child.communicate(input, timeout=self.process_timeout)
        except BaseException:
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            stdout, stderr = child.communicate(timeout=5)
            self.save(label, stdout, stderr, child.returncode, started)
            raise
        self.save(label, stdout, stderr, child.returncode, started)
        require(child.returncode == 0, "Product/support process failed; inspect retained streams")
        return stdout

    def save(self, label, stdout, stderr, code, started):
        for name, data in (("stdout", stdout), ("stderr", stderr)):
            (self.root / (label + "." + name)).write_bytes(data)
        self.records.append({"identity": label, "exit_code": code,
            "termination": "exited" if code is not None and code >= 0 else "signal",
            "duration_ns": time.monotonic_ns() - started,
            "stdout": explanation_evidence.binding(stdout), "stderr": explanation_evidence.binding(stderr)})
        c.write_json(self.root / "manifest.json", self.records)


class Product:
    """Bounded stdio client with actual responses; no synthetic Product facts."""
    active = set()

    def __init__(self, binary, runtime, repository, logs):
        self.logs, self.counter, self.buffer, self.stdout = logs, 0, b"", b""
        self.started = time.monotonic_ns()
        self.label = "mcp-" + secrets.token_hex(8)
        self.stderr = (logs.root / (self.label + ".stderr-private")).open("w+b")
        self.process = subprocess.Popen([str(binary)], cwd=repository,
            env=os.environ | {"VOLICORD_RUNTIME_DIR": str(runtime)},
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr)
        self.active.add(self)
        self.rpc("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "self-authored-evidence-rehearsal", "version": "1"}})

    def rpc(self, method, params):
        self.counter += 1
        request = {"jsonrpc": "2.0", "id": self.counter, "method": method, "params": params}
        self.process.stdin.write((json.dumps(request) + "\n").encode())
        self.process.stdin.flush()
        deadline = time.monotonic() + 60
        with selectors.DefaultSelector() as selector:
            selector.register(self.process.stdout, selectors.EVENT_READ)
            while b"\n" not in self.buffer:
                require(time.monotonic() < deadline, "MCP response timed out")
                if selector.select(max(0, deadline - time.monotonic())):
                    data = os.read(self.process.stdout.fileno(), 65536)
                    require(data, "MCP ended without a complete response")
                    self.stdout += data
                    self.buffer += data
                    require(len(self.buffer) <= 32 << 20, "MCP response exceeds bound")
        line, self.buffer = self.buffer.split(b"\n", 1)
        response = json.loads(line)
        require(response.get("id") == self.counter and "error" not in response, "MCP correlation/error")
        return response["result"]

    def tool(self, name, arguments):
        response = self.rpc("tools/call", {"name": name, "arguments": arguments})
        require(response.get("isError") is False, "Product tool rejected support input")
        return response["structuredContent"]

    def close(self):
        try:
            self.process.stdin.close()
            self.process.wait(timeout=5)
        except (OSError, subprocess.TimeoutExpired):
            self.process.kill()
            self.process.wait(timeout=5)
        self.stdout += self.process.stdout.read()
        self.process.stdout.close()
        self.stderr.seek(0)
        stderr = self.stderr.read()
        self.stderr.close()
        self.logs.save(self.label, self.stdout, stderr, self.process.returncode, self.started)
        self.active.discard(self)
        require(self.process.returncode == 0, "MCP did not tear down normally")


def authored_capture(path, repository, revision, session, task, activation, operations):
    """Codex-shaped test transport, explicitly not a Codex runtime observation."""
    turn = "support-turn-" + session
    events = [{"type": "session_meta", "payload": {"id": session, "session_id": session,
        "cwd": str(repository), "git": {"commit_hash": revision},
        "source": "self_authored_test_support", "originator": "evidence_rehearsal_script",
        "cli_version": "support-transport-1", "thread_source": "user",
        "source_metadata": {"evidence_purpose": purpose.REHEARSAL,
            "authorship": "self_authored_support_not_host_observed"}}},
        {"type": "response_item", "payload": {"type": "message", "role": "developer",
            "content": [{"type": "input_text", "text": activation}]}},
        {"type": "event_msg", "payload": {"type": "task_started", "turn_id": turn}},
        {"type": "event_msg", "payload": {"type": "user_message", "message": task,
            "client_id": "support-user-" + session}}]
    for number, (name, arguments, result, duration, started, ended) in enumerate(operations):
        call = session + "-" + str(number)
        events.extend([
            {"timestamp": started, "type": "response_item", "payload": {"type": "custom_tool_call",
                "name": "exec", "status": "completed", "call_id": call + '-wrapper',
                "input": 'const r=await tools.mcp__volicord__' + name + '(' + json.dumps(arguments) + '); text(JSON.stringify(r));',
                "internal_chat_message_metadata_passthrough": {"turn_id": turn}}},
            {"timestamp": ended, "type": "event_msg", "payload": {"type": "mcp_tool_call_end", "turn_id": turn,
                "call_id": call, "invocation": {"server": "volicord", "tool": name, "arguments": arguments},
                "result": {"Ok": {"isError": False, "structuredContent": result}},
                "duration": {"secs": duration // 1_000_000_000, "nanos": duration % 1_000_000_000}}}])
    events.append({"type": "event_msg", "payload": {"type": "task_complete", "turn_id": turn}})
    # Wall observations come from real request/completion boundaries. The transport
    # is authored support; its chronology and Product results are retained separately.
    for event in events[:4]:
        event['timestamp'] = operations[0][4]
    events[-1]['timestamp'] = explanation_evidence.timestamp()
    path.write_text("".join(json.dumps(e) + "\n" for e in events))
    capture = codex_events.load_codex_capture(path)
    require(capture.repository_scoped_activation_observed and capture.tool_calls,
        "actual hook/response support transport was not normalized")
    require(all(type(capture.observed_metadata['mcp_invocations'].get(call.call_id)) is int
        and capture.observed_metadata['mcp_invocations'][call.call_id] < call.completion_sequence
        for call in capture.tool_calls), 'authored transport omitted supported request coordinates')


def repositories(root, candidate, logs):
    sources = root / "sources"
    sources.mkdir()
    result = [{"class": "volicord", "path": str(c.ROOT), "origin": c.harness.command_output(c.ROOT,
        "git", "remote", "get-url", "origin"), "revision": candidate}]
    fixture = json.loads(FIXTURE.read_bytes())
    for kind in ("small-python", "polyglot-medium"):
        source = sources / kind
        source.mkdir()
        for name, text in fixture["repositories"][kind].items():
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
        if kind == "polyglot-medium":
            for number in range(100):
                (source / "python" / f"component_{number}.py").write_text(
                    f'"""Self-authored bounded support component {number}."""\nVALUE = {number}\n')
        origin = f"https://example.invalid/self-authored-support/{kind}.git"
        for arguments in (("init", "-q"), ("config", "user.email", "support@example.invalid"),
            ("config", "user.name", "Evidence rehearsal support"), ("remote", "add", "origin", origin),
            ("add", "."), ("commit", "-q", "-m", "Self-authored pipeline support")):
            logs.run(["git", *arguments], cwd=source)
        result.append({"class": kind, "path": str(source), "origin": origin,
            "revision": c.harness.git_head(source), "license_file": "LICENSE", "license_spdx": "MIT"})
    path = root / "repositories.json"
    c.write_json(path, {"repositories": result, "document_language": "ko", "viewer_locale": "en"})
    return path


def tasks(root):
    fixture = json.loads(FIXTURE.read_bytes())
    task_dir = root / "task-inputs"
    task_dir.mkdir()
    result = copy.deepcopy(fixture["tasks"])
    for slot, task in result.items():
        for role in ("start", "resume"):
            if role in task:
                path = task_dir / (slot + "-" + role + ".txt")
                path.write_text(task[role])
                task[role] = str(path)
    path = root / "tasks.json"
    c.write_json(path, {"tasks": result})
    return path


def run_sessions(root, campaign_root, binary, logs):
    prepared = c.load_campaign(campaign_root)
    paths, resource = [], None
    for kind, label, role in c.harness.current_session_slots():
        state = prepared["works"][c.work_key(kind, label)]
        repository, runtime = Path(state["repository_path"]), Path(state["runtime_home"])
        session = "authored-support-" + secrets.token_hex(12)
        task = (campaign_root / state["operator_task_artifacts"][role]["path"]).read_text()
        hook = logs.run([binary, "--runtime", runtime, "--repository", repository, "codex", "hook"],
            cwd=repository, input=json.dumps({"hook_event_name": "SessionStart", "session_id": session,
                "cwd": str(repository), "source": "startup" if role == "start" else "resume",
                "model": "self-authored-test-support", "permission_mode": "default",
                "transcript_path": None}).encode())
        activation = json.loads(hook)["hookSpecificOutput"]["additionalContext"]
        client = Product(binary.with_name("volicord-mcp"), runtime, repository, logs)
        operations = []
        def call(name, arguments):
            wall_started = explanation_evidence.timestamp()
            started = time.monotonic_ns()
            value = client.tool(name, arguments)
            operations.append((name, arguments, value, time.monotonic_ns() - started,
                wall_started, explanation_evidence.timestamp()))
            return value
        try:
            if kind == "volicord" and label == "A" and role == "start":
                resource = resource_observer.observe(prepared["candidate_artifacts"], [runtime],
                    duration_seconds=1, interval_ms=250, purpose=purpose.REHEARSAL)
                require(resource["status"] == "measured" and resource["measurement"]["sample_count"] > 0,
                    "actual candidate process observation missing")
                c.write_json(root / "actual-resource.json", resource)
            if role == "start" and label == "A":
                resolved = call("project_resolve", {"repository": str(repository)})
                require(resolved["status"] == "not_found", "support Project unexpectedly preexists")
                project = call("project_initialize", {"display_name": "Authored support " + kind,
                    "repository": str(repository)})["project_id"]
            else:
                project = call("project_resolve", {"repository": str(repository)})["project_id"]
            if role == "start":
                goal = call("context_record", {"project_id": project, "user_turn": task,
                    "role": "goal", "work_transition": "start_new",
                    "statement": "Inspect the shared evidence support scenario."})
                learning = json.loads(FIXTURE.read_bytes())["tasks"][c.work_key(kind, label)]["learning_collaboration_statement"]
                learning_context = call("context_record", {"project_id": project, "role": "learning",
                    "user_turn": task, "statement": learning}) if learning else None
                baseline = call("repository_analyze", {"project_id": project,
                    "excluded_paths": ["crates", "tests", "docs", "xtask"] if kind == "volicord" else []})
                rehearsal_support.record_support_checkpoint(call, project, goal, baseline, repository,
                    label, learning, learning_context)
            scenario = json.loads(FIXTURE.read_bytes())['temporal_scenarios']['recall']
            if c.work_key(kind, label) == scenario['work_slot_id'] and role == 'start':
                work = goal['context_item_id']
                before_plan, before_response, before_receipt = rehearsal_support.product_explanation(
                    root, binary, runtime, project, work, logs, 'recall-before')
                before = call('recall', {'project_id': project, 'requested_language': 'en'})
                explanation_evidence.validate_lifecycle({'plan': before_plan, 'project_id': project,
                    'subject': {'kind': 'work', 'identity': work}, 'language': 'en'},
                    before_response, before_receipt, before)
                mutation = rehearsal_support.correction(call, project, work, 1, scenario)
                after_plan, after_response, after_receipt = rehearsal_support.product_explanation(
                    root, binary, runtime, project, work, logs, 'recall-after')
                after = call('recall', {'project_id': project, 'requested_language': 'en'})
                rehearsal_support.check_sources(before_plan, after_plan, mutation)
                explanation_evidence.validate_lifecycle({'plan': after_plan, 'project_id': project,
                    'subject': {'kind': 'work', 'identity': work}, 'language': 'en'},
                    after_response, after_receipt, after)
            else:
                call("recall", {"project_id": project, "requested_language": "en"})
        finally:
            client.close()
        path = root / (c.session_slot_id(kind, label, role) + ".jsonl")
        authored_capture(path, repository, state["repository_revision"], session, task, activation, operations)
        paths.append(path)
    c.record_resources(campaign_root, root / "actual-resource.json")
    return paths


def complete_explanations(root, campaign_root, prepared):
    for item in prepared["explanations"]:
        preparation = c.read_json(campaign_root / item["preparation"])
        plan = preparation["plan"]
        response = rehearsal_support.realization(plan)
        input_path = root / (item["identity"] + ".json")
        c.write_json(input_path, response)
        wrong = copy.deepcopy(response)
        wrong["plan_fingerprint"] = "sha256:" + "0" * 64
        rejected(lambda: explanation_evidence.validate_response(plan, wrong))
        explanation_evidence.record(campaign_root, item["identity"], input_path)


def rejected_for(operation, message):
    """Keep the actual rejection privately and assert its intended cause."""
    try:
        operation()
    except ValueError as error:
        require(message in str(error), 'control rejected for an unintended reason: ' + str(error))
        return {'reason_sha256': hashlib.sha256(str(error).encode()).hexdigest()}
    raise ValueError('negative temporal control accepted')


def control_evidence(root, name, check, status, consumer, basis, artifacts):
    finding = machine_findings.finding(check, status, basis)
    path = root / (name + '-finding.json')
    c.write_json(path, finding)
    return {'consumer': consumer, 'check': check, 'status': finding['status'],
        'disposition': finding['disposition'], 'basis_sha256': machine_findings.digest(basis),
        'finding': explanation_evidence.binding(path.read_bytes()), 'artifacts': artifacts,
        'observation_statuses': [o['status'] for o in basis.get('observations', [])],
        'goal_revisions': [o['goal_basis']['revision'] for o in basis.get('observations', [])],
        'error_classes': sorted({e for o in basis.get('observations', []) for e in o['errors']}),
        'rejection_sha256': basis.get('reason_sha256') or (machine_findings.digest(basis['rejections']) if 'rejections' in basis else None)}


def explanation_clone(campaign_root, target):
    # Disposable control inputs retain the authentic campaign bindings. Runtime
    # reads still reach the real Product, but all evidence mutations stay here.
    shutil.copytree(campaign_root, target, ignore=shutil.ignore_patterns('repository', 'runtime'))
    # A Campaign itself cannot be relocated. These are disposable artifact
    # snapshots for the source-independent verifier, not new Campaign roots.


def regenerate(root, campaign_root, paths, prepared, binary, logs):
    scenario = json.loads(FIXTURE.read_bytes())['temporal_scenarios']['regeneration']
    campaign = c.load_campaign(campaign_root)
    state = campaign['works'][scenario['work_slot_id']]
    journey = campaign['journeys'][state['journey_id']]
    selected = [item for item in prepared['explanations']
        if c.read_json(campaign_root / item['preparation'])['journey_id'] == state['journey_id']]
    require(len(selected) == 2 and {i['language'] for i in selected} == {'en', 'ko'}, 'regeneration subject/locale boundary changed')
    work = selected[0]['subject']['identity']
    project = c.read_json(campaign_root / selected[0]['preparation'])['project_id']
    mapped = c.map_batch_rollouts(campaign_root, paths)
    explanation_evidence.require_ready(campaign_root, campaign, mapped)
    originals = {p: p.read_bytes() for item in selected
        for p in explanation_evidence.entry_path(campaign_root, item['identity']).rglob('*') if p.is_file()}
    client = Product(binary.with_name('volicord-mcp'), Path(journey['runtime_home']), Path(journey['repository_path']), logs)
    try:
        mutation = rehearsal_support.correction(client.tool, project, work, 1, scenario)
    finally:
        client.close()
    c.write_json(root / 'regeneration-correction.json', {'scenario': scenario, 'project_id': project,
        'work_item_id': work, 'receipt': mutation, 'process_identity': client.label})
    outcomes = {}
    negative = root / 'negative-current-plan'
    explanation_clone(campaign_root, negative)
    rejection = rejected_for(lambda: explanation_evidence.require_ready(negative, campaign, mapped), 'basis changed')
    outcomes['mismatched_final_current_plan'] = control_evidence(root, 'mismatched_final_current_plan',
        'realization_binding', 'confirmed_violation', 'explanation_readiness', rejection,
        [explanation_evidence.binding((negative / selected[0]['preparation']).read_bytes())])
    new = explanation_evidence.prepare(campaign_root, paths, languages=['en', 'ko'], work_ids=[work])
    # Snapshot an actual, still incomplete Product preparation. The main pipeline
    # completes that same obligation next; the control retains its earlier state.
    negative = root / 'negative-pending'
    explanation_clone(campaign_root, negative)
    rejection = rejected_for(lambda: explanation_evidence.require_ready(negative, campaign, mapped), 'missing host response')
    outcomes['incomplete_attempt_fallback'] = control_evidence(root, 'incomplete_attempt_fallback',
        'realization_binding', 'confirmed_violation', 'explanation_readiness', rejection,
        [explanation_evidence.binding((negative / new['explanations'][0]['preparation']).read_bytes())])
    complete_explanations(root, campaign_root, new)
    relations = explanation_evidence.require_ready(campaign_root, campaign, mapped)
    require(all(p.read_bytes() == body for p, body in originals.items()), 'historical observation bytes were rewritten')
    lifecycles = []
    for item in selected + new['explanations']:
        path = campaign_root / item['preparation']
        preparation, receipt = explanation_evidence.verify(campaign_root, path)
        goal = rehearsal_support.goal_evidence(preparation['plan'])
        prior = next(i for i in selected if i['language'] == item['language'])
        if item in new['explanations']:
            rehearsal_support.check_sources(c.read_json(campaign_root / prior['preparation'])['plan'], preparation['plan'], mutation)
        lifecycles.append({'identity': item['identity'], **relations[item['identity']],
            'language': item['language'], 'goal_revision': goal['revision'], 'goal_sources': goal['sources'],
            'before_state': preparation['before_observation']['state'], 'after_state': receipt['after_state'],
            'plan_fingerprint': preparation['plan']['fingerprint'],
            'stages': {name: explanation_evidence.binding((path.parent / (name + '.json')).read_bytes())
                for name in ('attempt', 'preparation', 'response', 'record', 'after', 'receipt')}})
    outcomes['historical_explanation_regeneration'] = control_evidence(root, 'historical_explanation_regeneration',
        'realization_binding', 'confirmed_pass', 'explanation_readiness', {'lifecycles': lifecycles},
        [v['stages']['receipt'] for v in lifecycles])
    historical = campaign_root / selected[0]['preparation']
    negative = root / 'negative-historical'
    explanation_clone(campaign_root, negative)
    damaged = negative / historical.relative_to(campaign_root).parent / 'response.json'
    damaged.chmod(0o600)
    damaged.write_bytes(damaged.read_bytes() + b'\n')
    rejection = rejected_for(lambda: explanation_evidence.publication_relations(negative), 'inventory')
    outcomes['tampered_historical_explanation'] = control_evidence(root, 'tampered_historical_explanation',
        'campaign_inventory', 'confirmed_violation', 'explanation_readiness', rejection,
        [explanation_evidence.binding(damaged.read_bytes())])
    preparation = c.read_json(historical)
    response = c.read_json(historical.parent / 'response.json')
    failures = []
    for boundary in ('language', 'subject'):
        wrong = copy.deepcopy(preparation)
        wrong[boundary] = 'ko' if boundary == 'language' and wrong['language'] == 'en' else ('en' if boundary == 'language' else {'kind': 'work', 'identity': '0' * 32})
        failures.append(rejected_for(lambda: explanation_evidence.validate_lifecycle(wrong, response,
            c.read_json(historical.parent / 'record.json'), c.read_json(historical.parent / 'after.json')), 'subject/language'))
    outcomes['explanation_scope_boundary'] = control_evidence(root, 'explanation_scope_boundary',
        'realization_binding', 'confirmed_violation', 'explanation_lifecycle', {'rejections': failures},
        [explanation_evidence.binding(historical.read_bytes())])
    return {'regeneration': {'work_slot_id': scenario['work_slot_id'], 'project_id': project, 'work_item_id': work,
        'authorization_source_id': mutation['user_response_source_id'], 'correction_process': client.label,
        'correction_receipt': explanation_evidence.binding(c.json_bytes(mutation)), 'lifecycles': lifecycles},
        'outcomes': outcomes}


def realizations(root, campaign_root, paths, binary, logs):
    prepared = explanation_evidence.prepare(campaign_root, paths, languages=['en', 'ko'])
    complete_explanations(root, campaign_root, prepared)
    temporal = regenerate(root, campaign_root, paths, prepared, binary, logs)
    # Final regeneration precedes this existing immutable document/collection freeze.
    document_realization.prepare(campaign_root, paths)
    for item in c.read_json(campaign_root / "realizer/index.json")["documents"]:
        draft_path = campaign_root / item["draft"]
        draft = c.read_json(draft_path)
        plan = c.read_json(campaign_root / item["preparation"])["plan"]
        draft["all_generated_prose_realized"] = True
        draft["realization"]["title"] = "직접 작성한 검사 문서"
        for field in ("host", "session"):
            draft["provenance"][field] = {"state": "self_reported", "value": "self_authored_test_support"}
        for section, source in zip(draft["realization"]["sections"], plan["sections"]):
            section["title"] = "구조 검사 입력"
            for claim, original in zip(section["claims"], source["claims"]):
                claim["text"] = "직접 작성한 검사 입력: " + " ".join(original["protected_terms"])
        c.write_json(draft_path, draft)
        document_realization.record(campaign_root, item["realization_id"], draft_path)
    return temporal


def pipeline(root, candidate, binary, logs):
    campaign_root = root / "campaign"
    c.prepare_campaign(campaign_root, "evidence-rehearsal-" + secrets.token_hex(8), candidate,
        repositories(root, candidate, logs), tasks(root), candidate_binary=binary, purpose=purpose.REHEARSAL)
    c.activate_all(campaign_root)
    paths = run_sessions(root, campaign_root, binary, logs)
    temporal = realizations(root, campaign_root, paths, binary, logs)
    c.collect_batch(campaign_root, paths)
    manifest = c.load_evidence_set(campaign_root)
    evaluation = c.evaluate_campaign(campaign_root, root / "evaluation")
    evaluation_path = Path(evaluation["evaluation"])
    cli_root = root / "cli-observations"
    c.cli_observations.collect(campaign_root, cli_root)
    review_root = root / "review"
    review.prepare(campaign_root, review_root, reviewer_kind="agent", session_id="authored-support-review",
        evaluation_path=evaluation_path, include_raw=True, cli_observation_root=cli_root)
    draft_path = review_root / "draft.json"
    draft = c.read_json(draft_path)
    draft["observation_scope"]["limits"] = ["Self-authored support payload; no model quality or human judgment."]
    coverage = next(a for a in draft["assessments"]
        if a["criterion_id"] == qualification_policy.COVERAGE_CRITERION)
    coverage.update(assessment="insufficient_evidence",
        reasoning="Authored support construction cannot establish ordinary-user interaction coverage.",
        uncertainty="No measured Naturalistic interaction or independent qualitative judgment is supplied.",
        counterevidence={"state": "not_observable", "reasoning": "Support construction establishes neither positive nor contrary human experience.", "evidence": []})
    c.write_json(draft_path, draft)
    review.record(review_root, draft_path)
    qualified = qualification_policy.qualify(campaign_root, evaluation_path, root / "qualification",
        candidate=candidate, review_roots=[review_root])
    require(qualified["technical_gate"] == {"state": "not_provided"}, "inner archive dependency cycle")
    expected = json.loads(FIXTURE.read_bytes())["expected_inner"]
    require(qualified["replacement_qualification"] == expected["replacement_qualification"]
        and qualified["qualitative_review"]["unresolved_criteria"]
        and qualified["qualitative_review"]["human_escalations"]
        and not qualified["replacement_pass_candidate"] and not qualified["phase_9_ready"],
        "missing observations were promoted")
    rejected(lambda: purpose.require_measured(qualified))
    checked = controls(root, campaign_root, manifest, evaluation_path, review_root, qualified)
    temporal_controls(root, campaign_root, manifest, temporal)
    result_lineage.publish(campaign_root, evaluation_path, [review_root],
        root / "qualification/qualification.json", root / "lineage")
    copied = root / "copied-lineage"
    shutil.copytree(root / "lineage", copied)
    # Original staging is genuinely unavailable to the independent copied verifier.
    for name in ("campaign", "evaluation", "review", "qualification", "lineage", "sources", "task-inputs"):
        (root / name).rename(root / (name + "-unavailable"))
    verification = result_lineage.verify(copied)
    require(verification["state"] == "verified" and not verification["external_staging_paths_used"],
        "copied verification consulted original inputs")
    checked["copied_semantic_rehash"] = copied_control(copied)
    temporal['copied'] = copied_temporal(copied, temporal)
    checked.update(dict.fromkeys(temporal['outcomes'], 'passed'))
    require(set(checked) == set(json.loads(FIXTURE.read_bytes())["controls"]), "rehearsal control coverage changed")
    return {"evidence_set_sha256": c.harness.sha256(root / "campaign-unavailable/evidence-set.json"),
        "executables": {name: item["sha256"] for name, item in manifest["candidate_artifacts"].items()},
        "evaluation_run_id": evaluation["run_id"], "qualification_run_id": qualified["run_id"],
        "expected_inner_verdict": qualified["replacement_qualification"],
        "technical_evidence": "not_provided", "human_observations": "not_provided",
        "unresolved_criteria_count": len(qualified["qualitative_review"]["unresolved_criteria"]),
        "hard_findings": qualified["machine_summary"]["hard_findings"],
        "copied_lineage_id": verification["lineage_id"], "copied_verification": "verified",
        "resource_sample_count": manifest["naturalistic_memory_evidence"]["measurement"]["sample_count"],
        "topology": qualified["campaign_topology"], "measured_evidence_eligible": False, "controls": checked,
        "temporal_evidence": temporal}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--candidate-head", required=True)
    parser.add_argument("--final-artifact", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--bin-dir", type=Path)
    args = parser.parse_args()
    def interrupted(signum, frame):
        raise InterruptedError("rehearsal interrupted")
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(1800)
    output = args.output.resolve()
    c.require_current_candidate(args.candidate_head)
    output.mkdir(parents=True, mode=0o700, exist_ok=False)
    logs = Processes(output / "processes")
    result = {"kind": "dogfood_evidence_rehearsal", "candidate_head": args.candidate_head,
        "evidence_purpose": purpose.REHEARSAL, **identities(), "status": "not_run",
        "external_transmission": "none", "operator_approval": "not_provided", "gate_binding": None}
    try:
        if args.final_artifact:
            require(args.bin_dir is None, "gate rehearsal installs its own candidate binaries")
            spec = importlib.util.spec_from_file_location("rehearsal_final_binding",
                c.ROOT / "rebuild/validation/end-to-end/multi-repository/final_evidence.py")
            owner = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(owner)
            binding = owner.read_gate_final(c.ROOT, args.candidate_head, args.final_artifact)
            result["gate_binding"] = {"gate_invocation": binding["gate_invocation"],
                "final_sha256": binding["final_sha256"],
                "binding_sha256": os.environ[owner.BINDING_HASH_ENV]}
            require(output.parent == Path(binding["gate_directory"]), "rehearsal output escapes its gate")
        if args.bin_dir:
            binary = args.bin_dir.resolve() / "volicord"
        else:
            logs.run([c.ROOT / "rebuild/install.sh", "--prefix", output / "install",
                "--runtime-dir", output / "install-runtime"])
            binary = output / "install/bin/volicord"
        result["executables"] = {name: c.harness.sha256(binary.with_name(name)) for name in c.CANDIDATE_ARTIFACTS}
        result["pipeline"] = pipeline(output, args.candidate_head, binary, logs)
        c.require_current_candidate(args.candidate_head)
        require(result["executables"] == {name: c.harness.sha256(binary.with_name(name)) for name in c.CANDIDATE_ARTIFACTS},
            "candidate executable drift")
        result["status"] = "passed"
    except BaseException as error:
        result["status"], result["failure_kind"] = "failed", type(error).__name__
        (output / "failure.txt").write_text(str(error) + "\n")
    finally:
        teardown = "completed"
        for client in list(Product.active):
            try:
                client.close()
            except Exception:
                teardown = "failed"
                result["status"] = "failed"
                result["failure_kind"] = "ProcessTeardownError"
        signal.alarm(0)
        # Canonical/analysis/source copies stay private; no processes survive into V11.
        result["processes"] = logs.records
        result["teardown"] = teardown
        result["result_id"] = machine_findings.digest(result)
        if result["status"] == "passed":
            try:
                validate_result(result, args.candidate_head)
            except ValueError as error:
                result["status"], result["failure_kind"] = "failed", "ResultContractError"
                (output / "failure.txt").write_text(str(error) + "\n")
                result["result_id"] = machine_findings.digest({k: v for k, v in result.items() if k != "result_id"})
        c.write_json(output / "result.json", result)
        print(json.dumps({"status": result["status"], "result": str(output / "result.json")}))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
