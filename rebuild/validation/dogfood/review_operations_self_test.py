#!/usr/bin/env python3
"""Reviewer isolation, bounded packaging and append-only review operations."""
import copy
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import cli_observations as cli_obs
import harness
import human_review
import qualitative_review as q
import review_operations as ops
import review_captures as captures
import codex_events
from viewer_observation_self_test import display_fixture, context_directory


def snapshot(root):
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()}


def collect_cli_fixture(campaign_root, output, *, emitted_private_paths=None,
                        raw_stream_identities=None):
    def cloner(source, destination, _revision):
        assert not source.resolve().is_relative_to(campaign_root.resolve())
        assert not destination.resolve().is_relative_to(campaign_root.resolve())
        destination.mkdir(parents=True)

    def revision(repository):
        kind = repository.parent.name
        return harness.git_head(c.ROOT) if kind == "volicord" else fixtures.REVISION

    def runner(binary, runtime, repository, argv, directory, order, criterion):
        logical = ["volicord", "--runtime", "<isolated-runtime>", *argv]
        stdout = f"observed {criterion or 'setup'}\n"
        stderr = "representative nonzero result\n" if criterion == "doctor_without_project_id" else ""
        if emitted_private_paths is not None:
            private_paths = [binary, runtime, repository, directory.parents[2],
                directory / "portable-context.json", campaign_root, output, c.ROOT]
            emitted_private_paths.extend(str(path.resolve(strict=False)) for path in private_paths)
            payload = "\n".join(str(path.resolve(strict=False)) for path in private_paths) + "\n"
            stdout += payload
            stderr += payload
        if raw_stream_identities is not None:
            for stream_name, text in (("stdout", stdout), ("stderr", stderr)):
                raw_stream_identities.append({"order": order, "criterion": criterion,
                    "stream": stream_name, "bytes": len(text.encode()),
                    "sha256": cli_obs.digest(text.encode())})
        return {"order": order, "criterion": criterion,
            "command_identity": cli_obs.digest(cli_obs.encoded(logical)), "argv": logical,
            "raw_argv_sha256": "1" * 64, "working_directory": "<observation-workspace>",
            "working_directory_sha256": "2" * 64, "started_at": "2026-09-14T00:00:00+00:00",
            "ended_at": "2026-09-14T00:00:01+00:00", "duration_ms": 1,
            "stdout": {"encoding": "utf-8", "text": stdout, "bytes": len(stdout.encode()),
                "sha256": cli_obs.digest(stdout.encode())},
            "stderr": {"encoding": "utf-8", "text": stderr, "bytes": len(stderr.encode()),
                "sha256": cli_obs.digest(stderr.encode())},
            "exit_code": 7 if criterion == "doctor_without_project_id" else 0, "termination": "exited"}

    return cli_obs.collect(campaign_root, output, cloner=cloner, runner=runner,
        revision_reader=revision, clean_reader=lambda _path: True, run_id="9" * 32)


def insufficient_draft(root):
    p, sha, _ = ops.load_package(root)
    value = q.template(p, sha)
    value["observation_scope"]["inspected_evidence"] = [s["sample_id"] + "-availability"
        for s in [*p["index"]["samples"], *p["index"]["journey_samples"], *p["index"]["cli_samples"]]]
    for spec, finding in zip(q.criterion_specs(p["index"], p["rubric"]), value["assessments"]):
        finding.update(assessment="insufficient_evidence", reasoning="Only the bounded evidence availability inventory was inspected.",
            inspected_evidence=([spec["sample_id"] + "-availability"] if spec["sample_id"] is not None else []),
            evidence=[],
            uncertainty="No substantive judgment has been established from actual observations.",
            counterevidence={"state": "not_observable", "reasoning": "Missing inspection limits both positive and contrary observations.", "evidence": []},
            human_answer_trace=([{"prompt": "What is your bounded judgment?",
                "answer": "Insufficient evidence; only the availability inventory was inspected."}]
                if p["reviewer"]["kind"] == "human" else None))
    (root / "draft.json").write_bytes(ops.encoded(value))
    return value


def assert_review_workflow(root, parent):
    """Also called by the maintained campaign self-test on its collected fixture."""
    before = snapshot(root)
    target = parent / "review-safe"
    result = ops.prepare(root, target, reviewer_kind="agent", session_id="review-session", run_id="a" * 32)
    assert result["state"] == "prepared"
    assert snapshot(root) == before
    p, _, package = ops.load_package(target)
    brief = ops.inspect_agent_criterion(target, 1)
    assert brief["semantic_judgment_suggested"] is False
    assert brief["criterion"]["criterion_id"] == q.criterion_specs(p["index"], p["rubric"])[0]["criterion_id"]
    assert brief["evidence"] and brief["mutation"] == "none"
    assert len(p["index"]["samples"]) == 5
    assert len(p["index"]["journey_samples"]) == 3
    cli_specs = [spec for spec in q.criterion_specs(p["index"], p["rubric"]) if spec["group"] == "cli"]
    assert len(cli_specs) == 21
    assert {spec["sample_id"] for spec in cli_specs} == set(c.CLASSES)
    assert not any(spec["sample_id"] in {sample["sample_id"] for sample in p["index"]["samples"]}
                   for spec in cli_specs)
    assert len([x for x in p["index"]["evidence"].values() if x["surface"] == "documents"]) == 24
    assert not any(name.startswith("private-rollouts/") for name in package["artifacts"])
    assert any(u["surface"] == "live_viewer_observation" for u in p["unavailable_surfaces"])
    obligations = p["completion_obligations"]
    assert obligations["semantic_judgment_automatic"] is False
    assert obligations["criterion_coverage"]["required_count"] == len(q.criterion_specs(p["index"], p["rubric"]))
    assert set(obligations["cli_class_coverage"]) == set(c.CLASSES)
    assert all(len(items) == 7 for items in obligations["cli_class_coverage"].values())
    assert obligations["human_only_criteria"]
    initial_preflight = ops.validate(target, target / "draft.json")
    assert initial_preflight["assessment_state"] == "not_reviewed"
    completion = initial_preflight["completion_preflight"]
    assert completion["semantic_correctness_assessed"] is False
    assert completion["reviewed_criterion_count"] == 0
    assert len(completion["missing_criterion_ids"]) == obligations["criterion_coverage"]["required_count"]
    assert set(completion["missing_cli_criterion_ids_by_class"]) == set(c.CLASSES)
    assert completion["missing_authority_criterion_ids"]
    assert completion["human_only_criterion_ids_requiring_human_review"] == obligations["human_only_criteria"]
    copy_target = parent / "review-safe-repeat"
    repeated = ops.prepare(root, copy_target, reviewer_kind="agent", session_id="review-session", run_id="a" * 32)
    assert repeated["package_id"] == result["package_id"]
    assert snapshot(copy_target) == snapshot(target)
    insufficient_draft(target)
    original = snapshot(target)
    with patch.object(c, "load_evidence_set", side_effect=AssertionError("preflight read campaign")), \
         patch.object(harness, "git_blob_bytes", side_effect=AssertionError("preflight read repository")), \
         patch.object(harness, "real_session_evidence", side_effect=AssertionError("semantic rerun")):
        preflight = ops.validate(target, target / "draft.json")
        assert preflight["assessment_state"] == "insufficient_evidence"
        assert preflight["mutation"] == "none"
        assert snapshot(target) == original
        recorded = ops.record(target, target / "draft.json")
    assert recorded["result"]["qualification_state"] == "not_run"
    assert recorded["result"]["phase_9_ready"] is False
    expected_escalations = sorted(spec["criterion_id"]
        for spec in q.criterion_specs(p["index"], p["rubric"])
        if spec["group"] in {"authority", "context_recovery", "campaign_interaction"})
    assert "campaign/campaign_interaction/interaction_coverage_adequacy" in expected_escalations
    for result in (preflight, recorded["result"]):
        assert result["completion_preflight"]["targeted_escalations"] \
            ["high_impact_insufficient_criterion_ids"] == expected_escalations
    assert (target / "recorded/review.json").read_bytes() == original["draft.json"]
    fixed = snapshot(target)
    try:
        ops.record(target, target / "draft.json")
    except ValueError:
        pass
    else:
        raise AssertionError("review run overwritten")
    assert snapshot(target) == fixed
    archive = parent / "review-safe.tar.gz"
    ops.package_review(target, archive)
    with tarfile.open(archive, "r:gz") as opened:
        names = opened.getnames()
        assert not any(name.startswith(("evaluator/", "operator/", "reviewer/", "private-rollouts/")) for name in names)
        body = b"".join(opened.extractfile(m).read() for m in opened.getmembers())
        assert b"BATCH-PRIVATE-STORE" not in body and b"BATCH-PRIVATE-DERIVED" not in body
        assert "recorded/review.json" in names and "recorded/receipt.json" in names
    assert snapshot(root) == before
    return target


def synthetic_rollout(user="Please inspect the integration boundary.", agent="I inspected the boundary and validation passed."):
    events = [
        {"type": "session_meta", "payload": {"id": "fixture-session", "session_id": "fixture-session",
            "cwd": "/synthetic/repository", "source": "vscode", "originator": "fixture",
            "cli_version": "fixture", "thread_source": "user"}},
        {"type": "event_msg", "payload": {"type": "task_started", "turn_id": "fixture-turn"}},
        {"type": "event_msg", "payload": {"type": "user_message", "message": user, "client_id": "fixture-client"}},
        {"type": "event_msg", "payload": {"type": "item_completed", "thread_id": "fixture-session",
            "turn_id": "fixture-turn", "item": {"type": "AgentMessage", "id": "fixture-agent",
            "phase": "final_answer", "content": [{"type": "Text", "text": agent}]}}},
        {"type": "response_item", "payload": {"type": "message", "role": "assistant",
            "id": "fixture-agent", "phase": "final_answer", "content": [{"type": "output_text", "text": agent}],
            "internal_chat_message_metadata_passthrough": {"turn_id": "fixture-turn"}}},
        {"type": "event_msg", "payload": {"type": "task_completed", "turn_id": "fixture-turn"}},
    ]
    return events


def rollout_bytes(events):
    return b"".join(ops.encoded(e).replace(b"\n", b"") + b"\n" for e in events)


class ProjectionTests(unittest.TestCase):
    def project(self, events=None, *, role="start", data=None, origin=None):
        data = data if data is not None else rollout_bytes(events or synthetic_rollout())
        origin = origin or {"kind": "evidence_set_member", "path": "slots/fixture/evidence/" + role + ".rollout.jsonl",
            "raw_bytes": len(data), "raw_sha256": ops.digest(data)}
        return captures.project(data, origin=origin, role=role, session_id="fixture-session",
            candidate_head="a" * 40, evidence_set_sha256="b" * 64)

    def test_complete_work_resume_distinct_identity_and_reproducible_projection(self):
        for role in ("start", "resume"):
            with self.subTest(role=role):
                data, metadata = self.project(role=role)
                value = captures.validate(data)
                self.assertEqual(value["role"], role)
                self.assertTrue(metadata["semantic_complete"])
                self.assertEqual(metadata["semantic_omission_count"], 0)
                self.assertEqual(value["origin"]["raw_bytes"], len(rollout_bytes(synthetic_rollout())))
                self.assertEqual(value["origin"]["raw_sha256"], ops.digest(rollout_bytes(synthetic_rollout())))
                self.assertNotEqual(metadata["review_sha256"], value["origin"]["raw_sha256"])
                self.assertEqual(metadata["review_bytes"], len(data))
                self.assertEqual(self.project(role=role), (data, metadata))
                texts = [r["body"]["value"] for r in value["records"] if r["semantic_role"] in captures.SEMANTIC_ROLES]
                self.assertIn("Please inspect the integration boundary.", texts)
                self.assertIn({"text": "I inspected the boundary and validation passed.", "questions": None}, texts)
                agent = next(r for r in value["records"] if r["semantic_role"] == "agent_message")
                self.assertEqual(agent["source_sequences"], [3, 4])

    def test_current_agent_text_transport_rejects_unsupported_content(self):
        for content in ([{"type": "Image", "text": "untrusted content"}],
                        [{"type": "Text", "text": {"unexpected": "body"}}]):
            with self.subTest(content=content):
                events = synthetic_rollout()
                events[3]["payload"]["item"]["content"] = content
                with self.assertRaisesRegex(ValueError, "unsupported review agent message content"):
                    self.project(events)

    def test_daemon_recovery_context_requires_current_host_binding(self):
        body = ('<codex_internal_context source="daemon_recovery">\n'
                'Synthetic host recovery instructions.\n</codex_internal_context>')
        event = {"type": "response_item", "payload": {"type": "message", "role": "user",
            "content": [{"type": "input_text", "text": body}],
            "internal_chat_message_metadata_passthrough": {"turn_id": "fixture-turn",
                "content_item_kinds": ["daemon_recovery.internal_context"]}}}
        events = synthetic_rollout()
        events.insert(3, event)
        projected, metadata = self.project(events)
        self.assertTrue(metadata["semantic_complete"])
        self.assertNotIn(body.encode(), projected)
        self.assertEqual(json.loads(projected)["excluded_records"][1],
                         {"sequence": 3, "reason": "non_semantic_by_design"})
        for key, value in (("turn_id", "unknown-turn"), ("content_item_kinds", ["user"])):
            invalid = copy.deepcopy(events)
            invalid[3]["payload"]["internal_chat_message_metadata_passthrough"][key] = value
            with self.assertRaisesRegex(ValueError, "unnormalized review user interaction"):
                self.project(invalid)
        invalid = copy.deepcopy(events)
        invalid[3]["payload"]["content"][0]["text"] = "Actual unbound user response."
        with self.assertRaisesRegex(ValueError, "unnormalized review user interaction"):
            self.project(invalid)

    def test_nonsemantic_payloads_are_excluded_without_literal_allowlisting(self):
        # Synthetic values are deliberately varied; no real campaign literal is stored.
        for spelling in ("fixture-output-value-381957", "different-fixture-value-964201"):
            events = synthetic_rollout()
            secret = "access_token=" + spelling
            for envelope, payload in (
                ("response_item", {"type": "function_call_output", "call_id": "generic-tool", "output": secret}),
                ("response_item", {"type": "message", "role": "developer", "content": [{"type": "input_text", "text": secret}]}),
                ("response_item", {"type": "reasoning", "encrypted_content": secret}),
                ("event_msg", {"type": "item_completed", "item": {"type": "CommandExecution", "stdout": secret}}),
            ):
                events.insert(-1, {"type": envelope, "payload": payload})
            data, metadata = self.project(events)
            self.assertNotIn(spelling.encode(), data)
            self.assertTrue(metadata["semantic_complete"])
            self.assertGreater(metadata["non_semantic_omission_count"], 0)
            ops.require_review_artifact_safe(data)

    def test_sensitive_user_and_agent_bodies_are_omitted_with_safe_provenance(self):
        for role in ("user", "agent"):
            secret = "access_token=synthetic-sensitive-value-529163"
            events = synthetic_rollout(**{role: secret})
            data, metadata = self.project(events)
            value = captures.validate(data)
            self.assertNotIn(b"synthetic-sensitive-value-529163", data)
            self.assertFalse(metadata["semantic_complete"])
            self.assertEqual(metadata["semantic_omission_count"], 1)
            omitted = next(r for r in value["records"] if r.get("body", {}).get("state") == "omitted")
            self.assertEqual(omitted["body"]["reason"], "sensitive_payload")
            self.assertIsNone(omitted["body"]["value"])
            self.assertGreater(omitted["body"]["source_body_bytes"], 0)
            self.assertEqual(len(omitted["body"]["source_body_sha256"]), 64)
            self.assertIn("turn_id", omitted)
            self.assertIn("sequence", omitted)

    def test_question_operation_interruption_and_resume_coordinates_are_preserved(self):
        events = synthetic_rollout()
        events.insert(3, {"type": "response_item", "payload": {"type": "function_call",
            "name": "request_user_input_async", "call_id": "fixture-question",
            "arguments": json.dumps({"questions": [{"title": "Which behavior should apply?", "options": ["Preserve", "Change"]}]}),
            "internal_chat_message_metadata_passthrough": {"turn_id": "fixture-turn"}}})
        events.insert(4, {"type": "response_item", "payload": {"type": "function_call_output",
            "call_id": "fixture-question", "output": json.dumps({"accepted": True}),
            "internal_chat_message_metadata_passthrough": {"turn_id": "fixture-turn"}}})
        events[-1]["payload"]["type"] = "turn_aborted"
        events.extend([
            {"type": "event_msg", "payload": {"type": "task_started", "turn_id": "resumed-turn"}},
            {"type": "event_msg", "payload": {"type": "user_message", "client_id": "response-client", "message": "Preserve the behavior."}},
            {"type": "event_msg", "payload": {"type": "context_compacted"}},
            {"type": "event_msg", "payload": {"type": "task_completed", "turn_id": "resumed-turn"}},
        ])
        data, metadata = self.project(events, role="resume")
        self.assertTrue(metadata["semantic_complete"])
        records = json.loads(data)["records"]
        question = next(r for r in records if r["semantic_role"] == "question_request")
        self.assertEqual(question["body"]["value"], [{"title": "Which behavior should apply?", "options": ["Preserve", "Change"]}])
        self.assertEqual(question["call_id"], "fixture-question")
        self.assertEqual(question["source_sequences"], [3, 4])
        self.assertTrue(any(r["semantic_role"] == "turn_terminal" and r["state"] == "interrupted" for r in records))
        self.assertTrue(any(r["semantic_role"] == "context_compacted" for r in records))
        self.assertTrue(any(r["semantic_role"] == "user_turn" and r["turn_id"] == "resumed-turn" for r in records))
        path = Path(__file__).with_name("fixtures") / "current-codex-mcp-completion.jsonl"
        raw = path.read_bytes()
        capture = codex_events.parse_codex_capture(raw)
        projected, _ = captures.project(raw, origin={"kind": "evidence_set_member", "path": "fixture.jsonl",
            "raw_bytes": len(raw), "raw_sha256": ops.digest(raw)}, role="start", session_id=capture.session_id,
            candidate_head="a" * 40, evidence_set_sha256="b" * 64)
        operations = [r for r in json.loads(projected)["records"] if r["semantic_role"] == "volicord_operation"]
        self.assertEqual(len(operations), len(capture.tool_calls))
        for record, call in zip(operations, capture.tool_calls):
            self.assertEqual((record["sequence"], record["completion_sequence"], record["turn_id"], record["call_id"], record["operation"], record["outcome"]),
                (call.sequence, call.completion_sequence, call.turn_id, call.call_id, call.operation, call.outcome))

    def test_normalized_execution_facts_exclude_command_and_output_bodies(self):
        path = Path(__file__).with_name("fixtures") / "current-codex-execution-evidence.jsonl"
        raw = path.read_bytes()
        capture = codex_events.parse_codex_capture(raw)
        projected, metadata = captures.project(raw, origin={"kind": "evidence_set_member", "path": "execution-fixture.jsonl",
            "raw_bytes": len(raw), "raw_sha256": ops.digest(raw)}, role="start", session_id=capture.session_id,
            candidate_head="a" * 40, evidence_set_sha256="b" * 64)
        facts = [r for r in json.loads(projected)["records"] if r["semantic_role"] == "execution_fact"]
        self.assertTrue(facts)
        self.assertEqual(len(facts), len(capture.commands))
        self.assertTrue(metadata["semantic_complete"])
        for fact, command in zip(facts, capture.commands):
            self.assertEqual(fact["command_role"], codex_events.command_role(command.parsed_command))
            self.assertEqual(fact["exit_code"], command.exit_code)
            self.assertEqual(fact["normalized_command_sha256"], ops.digest(ops.encoded(command.parsed_command)))
            self.assertEqual(fact["output_retention"], "non_semantic_by_design")
            self.assertNotIn("output", fact)
            self.assertNotIn("parsed_command", fact)

    def test_maintained_capture_owners_schema_and_help_agree(self):
        import assertions
        assertions.check_review_capture_contract()

    def test_size_omission_is_explicit_and_semantically_incomplete(self):
        events = synthetic_rollout(agent="safe " * (captures.MAX_BODY_BYTES // 5))
        data, metadata = self.project(events)
        self.assertFalse(metadata["semantic_complete"])
        self.assertIn(b'"body_limit"', data)

    def test_invalid_raw_capture_and_hash_mismatch_fail_closed(self):
        for data in (b"not json", b'{}\n', b'{"type":"session_meta","payload":{}}\n',
                     b'{"type":"session_meta","type":"other","payload":{}}\n'):
            with self.subTest(data=data):
                with self.assertRaises(ValueError):
                    self.project(data=data)
        origin = {"kind": "evidence_set_member", "path": "fixture.jsonl", "raw_bytes": 1, "raw_sha256": "0" * 64}
        with self.assertRaisesRegex(ValueError, "source binding"):
            self.project(origin=origin)
        events = synthetic_rollout()
        events[4]["payload"]["content"][0]["text"] = "Conflicting duplicate message."
        with self.assertRaisesRegex(ValueError, "transports conflict"):
            self.project(events)
        events[2]["payload"].pop("client_id")
        with self.assertRaisesRegex(ValueError, "user interaction"):
            self.project(events)

    def test_retained_sensitive_payload_and_projection_inconsistency_are_rejected(self):
        data, _ = self.project()
        value = json.loads(data)
        agent = next(r for r in value["records"] if r["semantic_role"] == "agent_message")
        agent["body"]["value"]["text"] = "access_token=synthetic-injected-value-718354"
        with self.assertRaisesRegex(ValueError, "sensitive payload"):
            captures.validate(ops.encoded(value))
        value = json.loads(data)
        value["semantic_omission_count"] = 2
        with self.assertRaisesRegex(ValueError, "inconsistency"):
            captures.validate(ops.encoded(value))
        value = json.loads(data)
        value["records"][0]["sequence"] = 10000
        with self.assertRaisesRegex(ValueError, "coordinates"):
            captures.validate(ops.encoded(value))


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.parent = Path(cls.temp.name)
        binary = cls.parent / "bin/volicord"
        fixtures.write_fake_binary(binary)
        with patch.object(harness, "git_clean", return_value=True):
            cls.root, raw, bundles = fixtures.prepared_batch(cls.parent, "campaign", binary)
            import hashlib
            import explanation_evidence as explanations
            from explanation_evidence_self_test import publish_fixture
            mapped = c.map_batch_rollouts(cls.root, raw)
            candidate = c.load_campaign(cls.root)
            work_id = hashlib.sha256(b'work:volicord:A').hexdigest()[:32]
            project = hashlib.sha256(b'project:volicord').hexdigest()[:32]
            publish_fixture(cls.root, mapped=mapped, candidate=candidate, subject_id=work_id, project_id=project)
            publish_fixture(cls.root, mapped=mapped, candidate=candidate, subject_id=work_id, project_id=project,
                identity='aa' * 16, revision=2)
            canonical = harness.load_canonical_bundle(bundles[c.work_key('volicord', 'B')])
            decision_id = canonical.rows('decisions')[0]['id']
            publish_fixture(cls.root, kind='decision', language='ko', mapped=mapped, candidate=candidate,
                project_id=project, subject_id=decision_id, identity='cc' * 16, before_state='stale')
            def plan_read(binary, runtime, project, args):
                is_read = args[0] in {'status', 'decisions'}
                subject, language = (None, args[2]) if is_read else (args[4], args[6])
                plans = [json.loads(explanations.bound(cls.root, path)) for path in explanations.preparations(cls.root)]
                final = explanations.publication_relations(cls.root)
                selected = next(p for p in plans if (subject is None or p['subject']['identity'] == subject)
                    and p['language'] == language and final[p['identity']]['publication_role'] == 'final')
                if is_read:
                    return json.loads(explanations.bound(cls.root, explanations.entry_path(cls.root, selected['identity']) / 'after.json')), {}
                return {'operation': 'explanation_prepare', 'plan': selected['plan']}, {}
            with patch.object(explanations, 'invoke', side_effect=plan_read):
                c.collect_batch(cls.root, raw, exporter=fixtures.batch_exporter(bundles),
                    documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
            cls.evaluation_result = c.evaluate_campaign(cls.root)
        cls.evaluation = cls.root / cls.evaluation_result["evaluation"]

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def target(self):
        return self.parent / self._testMethodName

    def test_collected_lifecycle_and_returned_meaning_reach_review_with_exact_locators(self):
        import review_explanations
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind='agent', session_id='meaning-reviewer', include_raw=True)
        preparation, _, _ = ops.load_package(target)
        lifecycle_entries = [v for v in preparation['index']['evidence'].values() if v['surface'] == 'explanation_lifecycle']
        self.assertEqual(len(lifecycle_entries), 3)
        roles = []
        for entry in lifecycle_entries:
            value = review_explanations.validate((target / entry['path']).read_bytes())
            review_explanations.verify_manifest(value, c.load_evidence_set(self.root))
            roles.append(value['publication']['publication_role'])
            self.assertEqual(entry['origin']['publication_role'], value['publication']['publication_role'])
            self.assertTrue(entry['projection']['semantic_complete'])
            self.assertTrue(any(p['value'].endswith('/paragraphs/0/text') for p in entry['locators']))
            self.assertEqual(value['context']['phase'], 'post_session_steward')
        self.assertEqual(sorted(roles), ['final', 'final', 'historical'])
        work = preparation['index']['evidence'][c.work_key('volicord', 'A') + '-resume']
        captured = captures.validate((target / work['path']).read_bytes())
        actual = next(r for r in captured['records'] if r.get('operation') == 'recall')
        fact = actual['body']['value']['returned_meaning']['value']['selected_work']['answers']['facts'][0]
        self.assertIn('Recorded next action quotation', fact['text'])
        self.assertTrue(any(p['value'].endswith('/recorded_action/recorded_text') for p in work['locators']))

    def test_package_rejects_fresh_hashes_with_corrupted_returned_meaning(self):
        from review_meaning_self_test import rehash_package
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind='agent', session_id='corruption-reviewer', include_raw=True)
        preparation, _, _ = ops.load_package(target)
        identity = c.work_key('volicord', 'A') + '-resume'
        value = json.loads((target / preparation['index']['evidence'][identity]['path']).read_bytes())
        recall = next(v for v in value['records'] if v.get('operation') == 'recall')
        recall['body']['value']['returned_meaning']['value']['next_step'] = 'Corrupted returned continuation'
        recall['body'] = captures.body_projection(recall['body']['value'])
        rehash_package(target, identity, ops.encoded(value))
        with self.assertRaisesRegex(ValueError, 'omissions/consistency changed'):
            ops.load_package(target)

    def test_round_trip_isolated_deterministic_and_append_only(self):
        assert_review_workflow(self.root, self.parent)

    def test_campaign_coverage_and_intents_are_visible_without_semantic_answers(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="agent", session_id="coverage-reviewer", include_raw=True)
        p, _, _ = ops.load_package(target)
        specs = q.criterion_specs(p["index"], p["rubric"])
        number = next(n for n, s in enumerate(specs, 1) if s["name"] == "interaction_coverage_adequacy")
        inspection = ops.inspect_agent_criterion(target, number)
        self.assertEqual(inspection["criterion"]["sample_id"], None)
        self.assertFalse(inspection["semantic_judgment_suggested"])
        self.assertEqual(sum(e["surface"] == "task_selection" for e in inspection["evidence"]), 5)
        self.assertEqual(sum(e["surface"] in {"work_capture", "resume_capture"} for e in inspection["evidence"]), 8)
        self.assertTrue(any(e["surface"] == "interaction_diagnostics" for e in inspection["evidence"]))
        learning = next(n for n, s in enumerate(specs, 1) if s.get("workload_intent") == "learning_collaborative")
        self.assertIn("explicitly requests", ops.inspect_agent_criterion(target, learning)["workload_prompt"])

    def test_secret_like_excluded_tool_output_does_not_block_package_preparation(self):
        binary = self.parent / "projection-fixture-bin/volicord"
        fixtures.write_fake_binary(binary)
        with patch.object(harness, "git_clean", return_value=True):
            root, raw, bundles = fixtures.prepared_batch(self.parent, "projection-fixture-campaign", binary)
            secret = "synthetic-third-party-value-637195"
            events = codex_events.capture_events(raw[0].read_bytes())
            events.insert(-1, {"type": "response_item", "payload": {"type": "function_call_output",
                "call_id": "unrelated-tool", "output": "access_token=" + secret}})
            # Publish the synthetic immutable campaign once, after fixture construction.
            raw[0].write_bytes(rollout_bytes(events))
            c.collect_batch(root, raw, exporter=fixtures.batch_exporter(bundles),
                documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
        before = snapshot(root)
        ops.prepare(root, self.target(), reviewer_kind="agent", session_id="projection-fixture-reviewer", include_raw=True)
        preparation, _, _ = ops.load_package(self.target())
        for data in snapshot(self.target()).values():
            self.assertNotIn(secret.encode(), data)
            ops.require_review_artifact_safe(data)
        self.assertEqual(snapshot(root), before)
        self.assertTrue(all(e["projection"]["semantic_complete"] for e in preparation["index"]["evidence"].values()
            if e["surface"] in captures.CAPTURE_SURFACES))
        archive = self.target().with_suffix(".tar.gz")
        ops.package_review(self.target(), archive)
        with tarfile.open(archive, "r:gz") as opened:
            for member in opened.getmembers():
                self.assertNotIn(secret.encode(), opened.extractfile(member).read())

    def test_review_selection_rejects_raw_hash_mismatch_before_projection(self):
        manifest = copy.deepcopy(c.load_evidence_set(self.root))
        session = manifest["work_evidence"][0]["sessions"]["start"]
        manifest["artifacts"][session["relative_evidence_path"]]["sha256"] = "0" * 64
        with patch.object(captures, "project", side_effect=AssertionError("projected before binding verification")):
            with self.assertRaisesRegex(ValueError, "source hash mismatch"):
                ops.select_evidence(self.root, manifest, None, include_raw=True)

    def test_review_reads_recorded_machine_policy_without_current_qualification_claim(self):
        from evaluation_runs import load
        value = c.read_json(self.evaluation)
        value["policy"]["sha256"] = "0" * 64
        value["run_id"] = ops.machine.digest({k: v for k, v in value.items() if k != "run_id"})
        run_root = self.parent / (self._testMethodName + "-machine")
        ops.publish_directory(run_root, {"evaluation.json": ops.encoded(value),
            "receipt.json": ops.encoded({"kind": "dogfood_evaluation_receipt", "run_id": value["run_id"],
                "evaluation_sha256": ops.digest(ops.encoded(value))})})
        with self.assertRaisesRegex(ValueError, "policy"):
            load(run_root / "evaluation.json")
        self.assertEqual(load(run_root / "evaluation.json", for_review=True), value)
        ops.prepare(self.root, self.target(), reviewer_kind="agent", session_id="recorded-policy-reviewer",
            evaluation_path=run_root / "evaluation.json", include_raw=True)
        preparation, _, _ = ops.load_package(self.target())
        self.assertEqual(preparation["binding"]["machine_evaluation"]["recorded_policy"], value["policy"])
        self.assertEqual(preparation["binding"]["machine_evaluation"]["policy_verification"],
            "recorded_identity_not_current_equivalence")
        changed = copy.deepcopy(value)
        changed["works"][0]["findings"][0]["status"] = "fabricated"
        with self.assertRaises(ValueError):
            ops.machine.validate_run(changed, require_current_policy=False)
        changed = copy.deepcopy(value)
        changed["run_id"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "identity"):
            ops.machine.validate_run(changed, require_current_policy=False)
        receipt = run_root / "receipt.json"
        receipt.chmod(0o600)
        receipt.write_bytes(ops.encoded({"kind": "dogfood_evaluation_receipt", "run_id": value["run_id"],
            "evaluation_sha256": "0" * 64}))
        with self.assertRaisesRegex(ValueError, "publication"):
            load(run_root / "evaluation.json", for_review=True)

    def test_raw_opt_in_and_machine_binding(self):
        target = self.target()
        before = snapshot(self.root)
        ops.prepare(self.root, target, reviewer_kind="agent", session_id="reviewer", evaluation_path=self.evaluation, include_raw=True)
        p, _, package = ops.load_package(target)
        self.assertEqual(p["binding"]["machine_evaluation"]["run_id"], self.evaluation_result["run_id"])
        self.assertFalse(any(n.startswith("private-rollouts/") for n in package["artifacts"]))
        self.assertEqual(sum(e["surface"] in captures.CAPTURE_SURFACES for e in p["index"]["evidence"].values()), 8)
        evaluation = c.read_json(self.evaluation)
        self.assertEqual(len(p["index"]["machine_findings"]),
            sum(len(item["findings"]) for item in [*evaluation["works"], *evaluation["journeys"]]))
        self.assertEqual(snapshot(self.root), before)
        for entry in p["index"]["evidence"].values():
            if entry["surface"] == "work_capture":
                projected = (target / entry["path"]).read_bytes()
                raw = (self.root / entry["origin"]["path"]).read_bytes()
                self.assertNotEqual(projected, raw)
                self.assertEqual(entry["origin"]["raw_sha256"], ops.digest(raw))
                self.assertEqual(entry["projection"]["review_sha256"], ops.digest(projected))
                self.assertTrue(entry["projection"]["semantic_complete"])

    def test_candidate_bound_cli_observations_are_isolated_hash_checked_and_reviewer_safe(self):
        observation_root = self.parent / (self._testMethodName + "-observations")
        before = snapshot(self.root)
        result = collect_cli_fixture(self.root, observation_root)
        self.assertEqual(result["criterion_count"], 21)
        value, data, _receipt = cli_obs.load(self.root, observation_root)
        self.assertEqual([v["repository_class"] for v in value["repository_observations"]], list(c.CLASSES))
        self.assertTrue(all(len(v["invocations"]) == 8 for v in value["repository_observations"]))
        self.assertEqual(value["repository_observations"][0]["invocations"][-1]["exit_code"], 7)
        self.assertNotIn(str(self.root).encode(), data)
        self.assertEqual(snapshot(self.root), before)

        review_root = self.target()
        ops.prepare(self.root, review_root, reviewer_kind="agent", session_id="cli-reviewer",
            cli_observation_root=observation_root)
        preparation, _, _ = ops.load_package(review_root)
        cli_evidence = [entry for entry in preparation["index"]["evidence"].values()
                        if entry["surface"] == "cli_observation"]
        self.assertEqual(len(cli_evidence), 3)
        self.assertEqual({entry["repository_class"] for entry in cli_evidence}, set(c.CLASSES))
        self.assertEqual({entry["sample_id"] for entry in cli_evidence}, set(c.CLASSES))
        foreign_id = "small-python-cli-observation"
        foreign = preparation["index"]["evidence"][foreign_id]
        with self.assertRaisesRegex(ValueError, "another sample"):
            spec = {"criterion_id": "volicord/cli/discover_with_cli_help",
                "sample_id": "volicord", "group": "cli", "name": "discover_with_cli_help", "locale": None}
            ops.review.validate_references(
                [{"evidence_id": foreign_id, "locator": foreign["locators"][0],
                  "criterion_id": spec["criterion_id"], "relevance": "Foreign CLI observation control."}],
                preparation["index"], set(preparation["index"]["evidence"]), spec)

        original = (observation_root / "observations.json").read_bytes()
        changed = json.loads(original)
        changed["candidate_head"] = "0" * 40
        (observation_root / "observations.json").chmod(0o600)
        (observation_root / "observations.json").write_bytes(ops.encoded(changed))
        with self.assertRaisesRegex(ValueError, "candidate/evidence"):
            cli_obs.load(self.root, observation_root)
        (observation_root / "observations.json").write_bytes(original + b"\n")
        with self.assertRaisesRegex(ValueError, "receipt"):
            cli_obs.load(self.root, observation_root)

    def test_cli_stdout_and_stderr_private_paths_do_not_enter_review_evidence(self):
        observation_root = self.parent / (self._testMethodName + "-observations")
        emitted_private_paths = []
        raw_stream_identities = []
        collect_cli_fixture(self.root, observation_root,
            emitted_private_paths=emitted_private_paths,
            raw_stream_identities=raw_stream_identities)
        observation_bytes = (observation_root / "observations.json").read_bytes()
        self.assertTrue(emitted_private_paths)
        for private_path in set(emitted_private_paths):
            self.assertNotIn(private_path.encode(), observation_bytes)
        value, _data, _receipt = cli_obs.load(self.root, observation_root)
        retained_identities = {
            (process["order"], process["criterion"], stream_name,
             process[stream_name]["raw_bytes"], process[stream_name]["raw_sha256"])
            for observation in value["repository_observations"]
            for process in observation["invocations"]
            for stream_name in ("stdout", "stderr")
        }
        self.assertEqual(retained_identities, {
            (item["order"], item["criterion"], item["stream"], item["bytes"], item["sha256"])
            for item in raw_stream_identities
        })
        self.assertTrue(all(process[stream_name]["projection"]["changed"]
            for observation in value["repository_observations"]
            for process in observation["invocations"]
            for stream_name in ("stdout", "stderr")))
        self.assertEqual(value["repository_observations"][0]["invocations"][-1]["exit_code"], 7)

        review_root = self.target()
        ops.prepare(self.root, review_root, reviewer_kind="agent", session_id="path-safe-reviewer",
            cli_observation_root=observation_root)
        review_bytes = b"".join(
            path.read_bytes() for path in review_root.rglob("*") if path.is_file()
        )
        for private_path in set(emitted_private_paths):
            self.assertNotIn(private_path.encode(), review_bytes)
        self.assertTrue(all(item["sha256"].encode() in review_bytes
            for item in raw_stream_identities))

    def test_cli_observation_revision_and_process_integrity_fail_closed(self):
        observation_root = self.parent / (self._testMethodName + "-observations")
        collect_cli_fixture(self.root, observation_root)
        original = json.loads((observation_root / "observations.json").read_bytes())
        for label, edit, message in [
            ("revision", lambda v: v["repository_observations"][1].update(repository_revision="0" * 40), "revision"),
            ("class", lambda v: v["repository_observations"][1].update(repository_class="volicord"), "classes"),
            ("process", lambda v: v["repository_observations"][0]["invocations"][1].update(exit_code=None), "exit or termination"),
            ("raw-stream", lambda v: v["repository_observations"][0]["invocations"][1]["stdout"].update(raw_sha256="0" * 64), "projection"),
            ("projection", lambda v: v["repository_observations"][0]["invocations"][1]["stdout"]["projection"].update(changed=True), "projection"),
        ]:
            with self.subTest(label=label):
                changed = copy.deepcopy(original)
                edit(changed)
                campaign = c.load_campaign(self.root)
                bound = campaign["candidate_artifacts"]["volicord"]
                with self.assertRaisesRegex(ValueError, message):
                    cli_obs.validate_value(changed, candidate_head=original["candidate_head"],
                        evidence_sha256=original["evidence_set_sha256"],
                        revisions={item["repository_class"]: item["repository_revision"]
                                   for item in original["repository_observations"]},
                        candidate_executable={"name": "volicord", "sha256": bound["sha256"],
                            "path_sha256": cli_obs.path_fingerprint(Path(bound["path"]))})

    def test_cli_observation_rejects_same_path_candidate_replacement(self):
        campaign = c.load_campaign(self.root)
        binary = Path(campaign["candidate_binary"])
        original = binary.read_bytes()
        binary.write_bytes(original + b"\n# same-path CLI replacement\n")
        binary.chmod(0o755)
        try:
            with self.assertRaisesRegex(ValueError, "candidate executable content mismatch"):
                collect_cli_fixture(self.root, self.target())
            self.assertFalse(self.target().exists())
        finally:
            binary.write_bytes(original)
            binary.chmod(0o755)

    def test_review_privacy_distinguishes_terminology_from_sensitive_payloads(self):
        benign = [
            b"the capsule does not retain auth.json content",
            b"private prompt bodies are excluded",
            b"documentation discusses an api_key field and an api-key option",
            b"source and tests name access_token, refresh_token, and credential_content",
            b"security guidance refers to a Bearer token concept",
            ops.encoded({"api_key": "<redacted>", "access_token": None,
                         "refresh_token": "not retained", "credential_content": False,
                         "private_prompt": "excluded"}),
        ]
        for number, data in enumerate(benign):
            with self.subTest(kind="benign", number=number):
                ops.require_review_artifact_safe(data)

        sensitive = [
            b"Authorization: Bearer retained-review-token-1234567890",
            ops.encoded({"api_key": "sk-retained-review-key-1234567890"}),
            ops.encoded({"api-key": "retained-api-key-value-1234567890"}),
            ops.encoded({"access_token": "retained-access-token-1234567890"}),
            ops.encoded({"refresh_token": "retained-refresh-token-1234567890"}),
            ops.encoded({"credential_content": "retained-credential-payload-1234567890"}),
            ops.encoded({"private_prompt": "PROMPT_SENTINEL_MUST_NOT_SURVIVE_7f91"}),
            ops.encoded({"file": "auth.json", "content": {"id_token": "retained-id-token-1234567890"}}),
            ops.encoded({"message": 'captured auth.json: {"access_token": "retained-nested-token-1234567890"}'}),
            b"api_key=retained-unquoted-key-1234567890",
            b"-----BEGIN PRIVATE KEY-----\nretained-key-material",
        ]
        for number, data in enumerate(sensitive):
            with self.subTest(kind="sensitive", number=number):
                with self.assertRaisesRegex(ValueError, "sensitive payload"):
                    ops.require_review_artifact_safe(data)

    def test_human_observations_use_review_payload_privacy(self):
        evidence_hash = ops.digest((self.root / "evidence-set.json").read_bytes())
        observation = {
            "kind": "dogfood_human_observations",
            "schema_version": 4,
            "candidate_head": c.load_evidence_set(self.root)["candidate_head"],
            "evidence_set_sha256": evidence_hash,
            "observer": q.reviewer("human", "b" * 32),
            "observations": [
                {"sample_id": "journey-volicord", "surface": "live_viewer_observation", "locale": "en",
                 "contexts": [display_fixture(c.load_evidence_set(self.root), "en")], "personally_observed": True,
                 "control": {"action": "direct", "reference_locale": None},
                 "response": {"observation": "The view states that auth.json content is not retained.",
                    "limits": "Private prompt bodies were excluded from inspection."}},
                {"sample_id": "journey-volicord", "surface": "live_viewer_observation", "locale": "ko",
                 "contexts": [display_fixture(c.load_evidence_set(self.root), "ko")], "personally_observed": True,
                 "control": {"action": "direct", "reference_locale": None},
                 "response": {"observation": "Bearer token terminology is visible as security guidance.",
                    "limits": "The api_key field name is documentation, not a retained value."}},
            ],
        }
        source = self.parent / (self._testMethodName + "-benign.json")
        source.write_bytes(ops.encoded(observation))
        result = ops.prepare(self.root, self.target(), reviewer_kind="human", human_observations=source)
        self.assertEqual(result["state"], "prepared")

        observation["observations"][0]["response"]["observation"] = (
            "Authorization: Bearer retained-human-observation-token-1234567890")
        sensitive = self.parent / (self._testMethodName + "-sensitive.json")
        sensitive.write_bytes(ops.encoded(observation))
        rejected = self.parent / (self._testMethodName + "-rejected")
        with self.assertRaisesRegex(ValueError, "human observations contain sensitive payload"):
            ops.prepare(self.root, rejected, reviewer_kind="human", human_observations=sensitive)
        self.assertFalse(rejected.exists())

    def test_conversational_human_observations_bind_candidate_and_receipt(self):
        observation_root = self.parent / (self._testMethodName + "-observations")
        manifest = c.load_evidence_set(self.root)
        context_paths = [context_directory(self.parent,self._testMethodName+locale,manifest,locale) for locale in ["en","ko"]]
        answers = iter([
            "1",
            "OBSERVATION:\nKeyboard focus, narrow layout, input response and resulting paint were personally inspected in the English Viewer.\n\nA second paragraph remains one answer.\nLIMITS:\nScreen reader output and other pages were not inspected.",
            "1", "SAME AS ENGLISH",
        ])
        result = human_review.capture_viewer_observations(
            self.root, observation_root, input_fn=answers.__next__, output_fn=lambda _text: None,
            run_id="c" * 32, context_paths=context_paths)
        self.assertEqual(result["state"], "captured")
        self.assertTrue((observation_root / "observations.json").is_file())
        self.assertTrue((observation_root / "receipt.json").is_file())
        prepared = self.target()
        ops.prepare(self.root, prepared, reviewer_kind="human",
            human_observations=observation_root)
        preparation, _, _ = ops.load_package(prepared)
        live = [entry for entry in preparation["index"]["evidence"].values()
                if entry["surface"] == "live_viewer_observation"]
        self.assertEqual({entry["locale"] for entry in live}, {"en", "ko"})
        self.assertFalse(any(entry["surface"] == "long_lived_project_observation"
            for entry in preparation["index"]["evidence"].values()))
        self.assertEqual(preparation["binding"]["candidate_head"], result["candidate_head"])
        captured = json.loads((observation_root / "observations.json").read_bytes())
        self.assertEqual(captured["observations"][1]["control"],
            {"action": "same_as_locale", "reference_locale": "en"})

    def test_human_display_context_rejects_missing_locale_personal_denial_and_foreign_candidate(self):
        manifest = c.load_evidence_set(self.root)
        contexts = [context_directory(self.parent, self._testMethodName + locale, manifest, locale)
            for locale in ("en", "ko")]
        target = self.parent / (self._testMethodName + "-observations")
        with self.assertRaisesRegex(ValueError, "both locales"):
            human_review.capture_viewer_observations(self.root, target, context_paths=contexts[:1],
                input_fn=lambda: self.fail("missing context must fail before conversation"), output_fn=lambda _: None)
        with self.assertRaisesRegex(ValueError, "direct human observation is unavailable"):
            human_review.capture_viewer_observations(self.root, target, context_paths=contexts,
                input_fn=iter(["2"]).__next__, output_fn=lambda _: None)
        self.assertFalse(target.exists())
        value = json.loads((contexts[1] / "display-context.json").read_bytes())
        value["candidate_head"] = "0" * 40
        (contexts[1] / "display-context.json").write_text(json.dumps(value))
        with self.assertRaisesRegex(ValueError, "capture binding"):
            human_review.capture_viewer_observations(self.root, target, context_paths=contexts,
                input_fn=lambda: self.fail("foreign candidate must fail before conversation"), output_fn=lambda _: None)
        self.assertFalse(target.exists())

    def test_conversational_human_judgment_generates_reviewable_draft(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human", include_raw=True)
        answers = iter([
            "The question was necessary for the material user-owned outcome shown in the work capture.",
            "1",
            "1",
            "START",
            "The work capture is the direct interaction evidence for question necessity.",
            "No uncertainty remains within the inspected interaction.",
            "2",
            "No contrary interaction was found in the inspected capture.",
        ])
        result = human_review.converse_one(
            target, input_fn=answers.__next__, output_fn=lambda _text: None)
        self.assertEqual(result["state"], "draft_updated")
        self.assertEqual(result["assessment"], "satisfied")
        validated = ops.validate(target, target / "draft.json")
        self.assertEqual(validated["counts"]["satisfied"], 1)
        value = json.loads((target / "draft.json").read_bytes())
        finding = value["assessments"][result["criterion_number"] - 1]
        self.assertTrue(finding["human_answer_trace"])
        self.assertEqual(finding["criterion_id"], result["criterion_id"])
        self.assertEqual(result["candidate_head"], value["binding"]["candidate_head"])

    def test_human_controls_preserve_partial_reference_and_insufficient_semantics(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human", include_raw=True)
        direct = iter([
            "The inspected interaction supports the first criterion.", "1", "1", "START",
            "The work capture is relevant to the first criterion.",
            "No further uncertainty in this bounded observation.", "2",
            "No contrary evidence was found in the inspected capture.",
        ])
        human_review.converse_one(target, input_fn=direct.__next__, output_fn=lambda _text: None)
        referenced = human_review.converse_one(target,
            input_fn=iter([
                "SAME AS PREVIOUS",
                "The same observation shows that ownership was violated for this distinct criterion.",
                "2",
                "The reused work capture is relevant to the separate ownership criterion.",
                "No uncertainty remains for this bounded ownership judgment.",
                "2",
                "No contrary ownership evidence was found in the reused observation.",
            ]).__next__, output_fn=lambda _text: None)
        self.assertEqual(referenced["control"], "same_as_prior")
        value = json.loads((target / "draft.json").read_bytes())
        reference_id = referenced["reference_criterion_id"]
        finding = value["assessments"][referenced["criterion_number"] - 1]
        prior = next(item for item in value["assessments"] if item["criterion_id"] == reference_id)
        self.assertEqual(prior["assessment"], "satisfied")
        self.assertEqual(finding["assessment"], "violated")
        self.assertNotEqual(finding["reasoning"], prior["reasoning"])
        self.assertEqual(finding["inspected_evidence"], prior["inspected_evidence"])
        control = value["human_controls"][finding["criterion_id"]]
        self.assertEqual(control["reference_criterion_id"], reference_id)
        self.assertEqual(control["reuse_scope"], "observation_evidence_context")
        cloned = copy.deepcopy(value)
        cloned_finding = cloned["assessments"][referenced["criterion_number"] - 1]
        cloned_finding["assessment"] = prior["assessment"]
        cloned_finding["reasoning"] = prior["reasoning"]
        cloned_path = self.parent / (self._testMethodName + "-cloned-verdict.json")
        cloned_path.write_bytes(ops.encoded(cloned))
        with self.assertRaisesRegex(ValueError, "separately authored"):
            ops.validate(target, cloned_path)

        covered = human_review.converse_one(target,
            input_fn=iter([
                "ALREADY COVERED",
                "The reused observation separately establishes source grounding for this criterion.",
                "1",
                "The reused work capture directly grounds the separate source criterion.",
                "No uncertainty remains for this bounded grounding judgment.",
                "2",
                "No contrary grounding evidence was found in the reused observation.",
            ]).__next__, output_fn=lambda _text: None)
        self.assertEqual(covered["control"], "already_covered")
        value = json.loads((target / "draft.json").read_bytes())
        covered_finding = value["assessments"][covered["criterion_number"] - 1]
        self.assertEqual(covered_finding["assessment"], "satisfied")
        self.assertEqual(value["human_controls"][covered_finding["criterion_id"]]["reuse_scope"],
            "observation_evidence_context")

        skipped = human_review.converse_one(target,
            input_fn=iter(["SKIP"]).__next__, output_fn=lambda _text: None)
        self.assertEqual(skipped["assessment"], "not_reviewed")
        insufficient = human_review.converse_one(target,
            input_fn=iter(["CANNOT ASSESS", "I inspected the listed inventory, but the required live observation is missing."]).__next__,
            output_fn=lambda _text: None)
        self.assertEqual(insufficient["assessment"], "insufficient_evidence")
        value = json.loads((target / "draft.json").read_bytes())
        gap = value["assessments"][insufficient["criterion_number"] - 1]
        self.assertEqual(gap["evidence"], [])
        self.assertEqual(gap["inspected_evidence"], [])
        self.assertEqual(value["human_controls"][gap["criterion_id"]]["action"], "cannot_assess")
        self.assertEqual(ops.validate(target, target / "draft.json")["counts"]["not_reviewed"],
            len(value["assessments"]) - 4)

    def test_same_as_english_requires_the_identical_criterion_and_rebinds_locale_evidence(self):
        observation_root = self.parent / (self._testMethodName + "-observations")
        manifest = c.load_evidence_set(self.root)
        contexts = [context_directory(self.parent, self._testMethodName + "-" + locale, manifest, locale)
            for locale in ("en", "ko")]
        observation_answers = iter([
            "1", "OBSERVATION:\nEnglish keyboard use was directly observed.\nLIMITS:\nOnly the bounded journey was inspected.",
            "1", "SAME AS ENGLISH",
        ])
        human_review.capture_viewer_observations(self.root, observation_root, context_paths=contexts,
            input_fn=observation_answers.__next__, output_fn=lambda _text: None,
            run_id="d" * 32)
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human", human_observations=observation_root)
        preparation, _, _ = ops.load_package(target)
        specs = q.criterion_specs(preparation["index"], preparation["rubric"])
        english_position = next(index for index, spec in enumerate(specs)
            if spec["group"] == "live_viewer" and spec["locale"] == "en"
            and spec["name"] == "keyboard_reachability")
        english_spec = specs[english_position]
        eligible = human_review._eligible_evidence(preparation, english_spec)
        evidence_number = next(index for index, (_identity, entry) in enumerate(eligible, 1)
            if entry["surface"] == "live_viewer_observation" and entry["locale"] == "en")
        human_review.converse_one(target, criterion_number=english_position + 1,
            input_fn=iter([
                "Keyboard reachability was satisfied in the direct English observation.",
                "1", str(evidence_number), "START",
                "The direct English live observation is relevant to keyboard reachability.",
                "No uncertainty remains within the observed path.", "2",
                "No contrary keyboard observation was found.",
            ]).__next__, output_fn=lambda _text: None)
        incompatible_position = next(index for index, spec in enumerate(specs)
            if spec["group"] == "live_viewer" and spec["locale"] == "ko"
            and spec["name"] == "visible_focus")
        before = (target / "draft.json").read_bytes()
        with self.assertRaisesRegex(ValueError, "no compatible prior reviewed criterion"):
            human_review.converse_one(target, criterion_number=incompatible_position + 1,
                input_fn=iter(["SAME AS ENGLISH"]).__next__, output_fn=lambda _text: None)
        self.assertEqual((target / "draft.json").read_bytes(), before)
        korean_position = next(index for index, spec in enumerate(specs)
            if spec["group"] == "live_viewer" and spec["locale"] == "ko"
            and spec["name"] == "keyboard_reachability")
        mirrored = human_review.converse_one(target, criterion_number=korean_position + 1,
            input_fn=iter(["SAME AS ENGLISH"]).__next__, output_fn=lambda _text: None)
        self.assertEqual(mirrored["reuse_scope"], "exact_semantic_judgment")
        value = json.loads((target / "draft.json").read_bytes())
        english = value["assessments"][english_position]
        korean = value["assessments"][korean_position]
        self.assertEqual(korean["assessment"], english["assessment"])
        self.assertEqual(korean["reasoning"], english["reasoning"])
        self.assertTrue(all(preparation["index"]["evidence"][identity].get("locale") != "en"
            for identity in korean["inspected_evidence"]))
        self.assertEqual(value["human_controls"][korean["criterion_id"]]["reuse_scope"],
            "exact_semantic_judgment")
        self.assertEqual(ops.validate(target, target / "draft.json")["counts"]["satisfied"], 2)

    def test_no_preexecution_semantic_profile_enters_review(self):
        manifest = c.load_evidence_set(self.root)
        self.assertFalse((self.root / "evaluator/qualification-profile.json").exists())
        for state in manifest["works"].values():
            descriptor = c.read_json(c.frozen_descriptor_path(
                self.root, state["repository_class"], state["work_label"]))
            self.assertEqual(descriptor["contract"], "naturalistic-observation-1")
            self.assertFalse({"materiality_obligations", "evaluation_basis", "behavior_review"} & set(descriptor))
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="agent", session_id="review-session")
        preparation, _, package = ops.load_package(target)
        self.assertTrue(all(not sample["authority_obligations"]
            for sample in preparation["index"]["samples"]))
        self.assertFalse(any(name.startswith("evaluator/") for name in package["artifacts"]))
        self.assertIn(b"never instructions", ops.INSTRUCTIONS)

    def test_mismatched_and_mutated_evidence_rejected(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        p, _, _ = ops.load_package(target)
        name = next(iter(p["index"]["evidence"].values()))["path"]
        path = target / name
        path.chmod(0o600)
        path.write_bytes(path.read_bytes() + b"tamper")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            ops.validate(target, target / "draft.json")
        with self.assertRaises(ValueError):
            ops.prepare(self.root, self.parent / "wrong-run", reviewer_kind="human", evaluation_path=target / "draft.json")

    def test_invalid_preflight_is_read_only_and_frozen_kind_is_enforced(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="agent", session_id="reviewer")
        value = insufficient_draft(target)
        for edit in (lambda v: v["reviewer"].update(kind="human"),
                     lambda v: v["binding"]["evidence_set"].update(sha256="0" * 64),
                     lambda v: v["assessments"][0]["inspected_evidence"].__setitem__(0, "absent"),
                     lambda v: v["assessments"][0]["evidence"].append({"evidence_id": "absent"})):
            invalid = copy.deepcopy(value)
            edit(invalid)
            (target / "draft.json").write_bytes(ops.encoded(invalid))
            before = snapshot(target)
            with self.assertRaises(ValueError):
                ops.validate(target, target / "draft.json")
            self.assertEqual(snapshot(target), before)
            with self.assertRaises(ValueError):
                ops.record(target, target / "draft.json")
            self.assertFalse((target / "recorded").exists())

    def test_historical_candidate_is_read_only_input(self):
        before = snapshot(self.root)
        with patch.object(harness, "git_head", return_value="0" * 40), patch.object(harness, "git_clean", return_value=False):
            result = ops.prepare(self.root, self.target(), reviewer_kind="human")
        self.assertEqual(result["state"], "prepared")
        self.assertEqual(snapshot(self.root), before)

    def test_preparation_failure_and_collision_leave_no_partial_run(self):
        target = self.target()
        with patch.object(ops.os, "rename", side_effect=OSError("injected publication failure")):
            with self.assertRaises(OSError):
                ops.prepare(self.root, target, reviewer_kind="human")
        self.assertFalse(target.exists())
        self.assertFalse(target.with_name(target.name + ".publication-lock").exists())
        ops.prepare(self.root, target, reviewer_kind="human")
        before = snapshot(target)
        with self.assertRaises(ValueError):
            ops.prepare(self.root, target, reviewer_kind="agent", session_id="reviewer")
        self.assertEqual(snapshot(target), before)

    def test_empty_review_and_inventory_bound_drafts_cannot_be_recorded(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        with self.assertRaisesRegex(ValueError, "empty draft"):
            ops.record(target, target / "draft.json")
        with self.assertRaisesRegex(ValueError, "mutable draft"):
            ops.validate(target, target / "preparation.json")

    def test_symlink_and_bounds_fail_closed(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        p, _, _ = ops.load_package(target)
        name = next(iter(p["index"]["evidence"].values()))["path"]
        path = target / name
        outside = self.parent / "outside.txt"
        outside.write_bytes(path.read_bytes())
        path.unlink()
        path.symlink_to(outside)
        with self.assertRaisesRegex(ValueError, "escaped|symlink"):
            ops.validate(target, target / "draft.json")
        with patch.object(ops, "MAX_FILES", 1):
            with self.assertRaisesRegex(ValueError, "bounds"):
                ops.prepare(self.root, self.parent / "over-bound", reviewer_kind="human")

    def test_recording_failure_and_tampering_are_not_success(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        insufficient_draft(target)
        before = snapshot(target)
        with patch.object(ops.os, "rename", side_effect=OSError("injected recording failure")):
            with self.assertRaises(OSError):
                ops.record(target, target / "draft.json")
        self.assertEqual(snapshot(target), before)
        ops.record(target, target / "draft.json")
        path = target / "recorded/review.json"
        path.chmod(0o600)
        path.write_bytes(path.read_bytes() + b"\n")
        with self.assertRaisesRegex(ValueError, "recorded review hash"):
            ops.package_review(target, self.parent / "corrupt-review.tar.gz")

    def test_matching_evidence_set_is_required_for_machine_run(self):
        invalid = c.read_json(self.evaluation)
        invalid["evidence_set"]["sha256"] = "0" * 64
        invalid["run_id"] = ops.machine.digest({k: v for k, v in invalid.items() if k != "run_id"})
        data = ops.encoded(invalid)
        from evaluation_runs import publish
        target = self.parent / "mismatched-machine-run"
        evaluation = publish(target, invalid)
        with self.assertRaisesRegex(ValueError, "machine run evidence-set/candidate mismatch"):
            ops.prepare(self.root, self.target(), reviewer_kind="human", evaluation_path=evaluation)

    def test_hard_and_indeterminate_machine_works_are_reviewable(self):
        invalid = c.read_json(self.evaluation)
        observation = invalid["works"][0]["observation"]
        observation["checks"]["canonical_bundle_and_provenance"] = "failed"
        observation["checks"]["appropriate_inquiry_outcome"] = "partial"
        invalid["works"][0]["findings"] = ops.machine.from_observation(observation)
        invalid["finding_state"] = ops.machine.evaluation_state(
            [f for item in [*invalid["works"], *invalid["journeys"]] for f in item["findings"]])
        invalid["run_id"] = ops.machine.digest({k: v for k, v in invalid.items() if k != "run_id"})
        ops.machine.validate_run(invalid)
        files, index, _ = ops.select_evidence(self.root, c.load_evidence_set(self.root), invalid, include_raw=False)
        self.assertEqual(len(index["samples"]), 5)
        self.assertTrue(any(v["finding"]["disposition"] == "hard_blocking" for v in index["machine_findings"].values()))
        self.assertTrue(any(v["finding"]["disposition"] == "qualitative_review_required" for v in index["machine_findings"].values()))
        self.assertIn("evidence/machine-findings.json", files)

    def test_cli_preflight_and_record_share_validation(self):
        target = self.target()
        ops.prepare(self.root, target, reviewer_kind="human")
        insufficient_draft(target)
        for operation in ("validate-qualitative-review", "record-qualitative-review"):
            result = subprocess.run(["python3", "-B", str(Path(c.__file__)), operation,
                "--review-root", str(target), "--draft", str(target / "draft.json")], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertNotIn('"phase_9_ready": true', result.stdout)


def run_workflow_tests():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.TestSuite([unittest.defaultTestLoader.loadTestsFromTestCase(cls)
        for cls in (ProjectionTests, WorkflowTests)]))
    if not result.wasSuccessful():
        raise AssertionError("qualitative review operations self-test failed")


if __name__ == "__main__":
    unittest.main()
