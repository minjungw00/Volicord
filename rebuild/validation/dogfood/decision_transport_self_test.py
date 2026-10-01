"""Current-host response provenance keeps raw identities and a narrow comparison."""
from copy import deepcopy
from dataclasses import replace
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

import harness as h
import interaction_diagnostics as diagnostics


class DecisionTransportTests(unittest.TestCase):
    def test_directional_terminal_transport_only(self):
        caller = "Choose stable_order; keep errors explicit."
        for raw in (caller, caller + "\r\n", caller + " \t\r\n", caller.replace("_", "\\_") + " \n"):
            comparison = h.compare_current_host_response_transport(caller, raw)
            self.assertTrue(comparison["equivalent"])
            self.assertEqual(comparison["raw_host_text_sha256"], hashlib.sha256(raw.encode()).hexdigest())
            self.assertEqual(comparison["caller_text_sha256"], hashlib.sha256(caller.encode()).hexdigest())
        for raw in (caller.lower(), caller.replace("explicit", "implicit"), caller.replace(";", ","),
            caller.replace("keep errors", "keep  errors"), " " + caller, caller + "\u00a0", caller + "\nAlso delete errors"):
            self.assertFalse(h.compare_current_host_response_transport(caller, raw)["equivalent"])
        self.assertFalse(h.compare_current_host_response_transport(caller + " ", caller)["equivalent"])
        self.assertFalse(h.compare_current_host_response_transport(r"Choose stable\_order", "Choose stable_order")["equivalent"])
        self.assertFalse(h.compare_frozen_task_transport(caller, caller + " ").equivalent)

    def fixture(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        descriptor = h.real_session_fixture("volicord", 1, "0" * 40, root)
        capture = h.load_codex_capture(root / descriptor["evidence"]["captures"]["work"]["file"])
        bundle = h.load_canonical_bundle(root / descriptor["evidence"]["canonical_bundle"]["file"])
        return capture, bundle

    def async_response_fixture(self, *, structured=False, mutate=None):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        descriptor = h.real_session_fixture("volicord", 1, "0" * 40, root)
        path = root / descriptor["evidence"]["captures"]["work"]["file"]
        events = [json.loads(line) for line in path.read_text().splitlines()]
        task = next(e["payload"]["turn_id"] for e in events
                    if e.get("payload", {}).get("type") == "task_started")
        joined = []
        for event in events:
            payload = event.get("payload", {})
            if payload.get("type") == "task_complete" and payload.get("turn_id") == task:
                continue
            if payload.get("type") == "task_started" and payload.get("turn_id") != task:
                continue
            if "turn_id" in payload:
                payload["turn_id"] = task
            joined.append(event)
        if structured:
            response = next(e for e in joined if e.get("payload", {}).get("type") == "user_message"
                and e["payload"]["message"] != descriptor["work_user_task"])
            payload = response["payload"]
            answer = payload["message"]
            question = "Which report style should be used?"
            metadata = {"turn_id": task}
            request = {"type": "response_item", "payload": {"type": "function_call",
                "name": "request_user_input_async", "call_id": "async-request",
                "arguments": json.dumps({"questions": [{"title": question}]}),
                "internal_chat_message_metadata_passthrough": metadata}}
            completion = {"type": "response_item", "payload": {"type": "function_call_output",
                "call_id": "async-request", "output": '{"accepted":true}',
                "internal_chat_message_metadata_passthrough": metadata}}
            index = joined.index(response)
            joined[index:index] = [request, completion]
            response["payload"] = {"type": "item_completed",
                "thread_id": events[0]["payload"]["id"], "turn_id": task,
                "item": {"type": "UserMessage", "id": "async-user-item",
                    "client_id": payload["client_id"], "content": [{"type": "text", "text": self.envelope([
                        {"answer": answer, "question": question,
                         "questionItemId": json.dumps(["request_user_input_async", "async-request", 0])}])}]}}
        if mutate:
            mutate(joined)
        path.write_text("".join(json.dumps(e) + "\n" for e in joined))
        return h.load_codex_capture(path), h.load_canonical_bundle(
            root / descriptor["evidence"]["canonical_bundle"]["file"])

    @staticmethod
    def envelope(items):
        return '<send_user_message_question_reply>' + json.dumps(items) + '</send_user_message_question_reply>'

    def assert_response(self, capture, bundle, expected):
        facts = h.decision_facts(capture, bundle)
        self.assertEqual(facts[0], expected)
        summary = diagnostics.work_summary({"workload_intent": "decision_rich"}, capture, None, bundle)
        self.assertEqual(summary["counts"]["current_host_response_count"], int(expected))
        return facts

    def test_structured_reply_shared_integrity_and_diagnostics(self):
        capture, bundle = self.async_response_fixture(structured=True)
        facts = self.assert_response(capture, bundle, True)
        transport = facts[-1][facts[1]]["current_host_response_transport"]
        self.assertEqual(transport["response_kind"], "async_question_reply")
        self.assertTrue(transport["transport_equivalence_used"])
        self.assertFalse(transport["answer_transport_equivalence_used"])
        self.assertEqual(transport["async_request_call_id"], "async-request")
        self.assertEqual(transport["async_question_index"], 0)
        self.assertEqual(transport["raw_host_text_sha256"], hashlib.sha256(capture.user_turns[-1].text.encode()).hexdigest())
        self.assertNotEqual(transport["raw_host_text_sha256"], transport["answer_text_sha256"])

    def test_multiple_items_use_request_identity_not_array_position(self):
        def change(events):
            request = next(e["payload"] for e in events if e.get("payload", {}).get("name") == "request_user_input_async")
            request["arguments"] = json.dumps({"questions": [{"title": "An unrelated question?"},
                {"title": "Which report style should be used?"}]})
            reply = next(e["payload"]["item"] for e in events if e.get("payload", {}).get("item", {}).get("type") == "UserMessage")
            text = reply["content"][0]["text"]
            items = json.loads(text.split('>', 1)[1].rsplit('<', 1)[0])
            items[0]["questionItemId"] = json.dumps(["request_user_input_async", "async-request", 1])
            items.append({"questionItemId": json.dumps(["request_user_input_async", "async-request", 0]),
                "question": "An unrelated question?", "answer": "Another answer"})
            reply["content"][0]["text"] = self.envelope(items)
        capture, bundle = self.async_response_fixture(structured=True, mutate=change)
        facts = self.assert_response(capture, bundle, True)
        self.assertEqual(facts[-1][facts[1]]["current_host_response_transport"]["async_question_index"], 1)

    def test_structured_identity_text_and_transport_fail_closed(self):
        capture, bundle = self.async_response_fixture(structured=True)
        turn = capture.user_turns[-1]
        items = json.loads(turn.text.split('>', 1)[1].rsplit('<', 1)[0])
        invalid = []
        for field, value in (("questionItemId", '["request_user_input_async","wrong-request",0]'),
            ("questionItemId", '["request_user_input_async","async-request",1]'),
            ("questionItemId", '["request_user_input_async","async-request",true]'),
            ("question", "Wrong question?"), ("answer", "A different answer"),
            ("answer", ""), ("question", ""), ("questionItemId", "")):
            invalid.append(self.envelope([{**items[0], field: value}]))
        for field in items[0]:
            invalid.append(self.envelope([{k: v for k, v in items[0].items() if k != field}]))
        invalid += [self.envelope(items * 2), self.envelope([]), self.envelope(items[0]),
            '<send_user_message_question_reply>{bad JSON}</send_user_message_question_reply>',
            turn.text.removesuffix('</send_user_message_question_reply>'),
            turn.text.replace('send_user_message_question_reply', 'unknown_wrapper'),
            turn.text + ' trailing content', 'prefix ' + turn.text,
            '<unknown>' + items[0]["answer"] + '</unknown>',
            turn.text.replace('"answer":', '"answer":"duplicate", "answer":')]
        for text in invalid:
            with self.subTest(text=text):
                changed = replace(capture, user_turns=(*capture.user_turns[:-1], replace(turn, text=text)))
                self.assert_response(changed, bundle, False)

    def test_structured_order_scope_latest_and_request_ambiguity(self):
        capture, bundle = self.async_response_fixture(structured=True)
        turn = capture.user_turns[-1]
        decision = capture.successful_calls("decision_record")[0]
        request = capture.async_question_requests[0]
        for changed in (
            replace(capture, async_question_requests=()),
            replace(capture, async_question_requests=(request, request)),
            replace(capture, async_question_requests=(replace(request, session_id="other-session"),)),
            replace(capture, async_question_requests=(replace(request, turn_id="other-task"),)),
            replace(capture, async_question_requests=(replace(request, sequence=turn.sequence + 1),)),
            replace(capture, async_question_requests=(replace(request, completion_sequence=turn.sequence + 1),)),
            replace(capture, user_turns=(*capture.user_turns[:-1], replace(turn, sequence=decision.sequence + 1))),
            replace(capture, user_turns=(*capture.user_turns[:-1], replace(turn, turn_id="other-task"))),
            replace(capture, user_turns=(*capture.user_turns, replace(turn, user_turn_id="duplicate-client"))),
            replace(capture, user_turns=(*capture.user_turns, replace(turn, sequence=decision.sequence - 1,
                user_turn_id="newer-client", text="A new answer replaces the old reply"))),
        ):
            with self.subTest(changed=changed.async_question_requests):
                self.assert_response(changed, bundle, False)

    def test_structured_reply_still_requires_canonical_provenance(self):
        capture, bundle = self.async_response_fixture(structured=True)
        for table in ("sources", "question_response_sources", "decisions",
            "question_decision_history_witnesses", "question_revisions"):
            with self.subTest(missing=table):
                tables = deepcopy(bundle.tables)
                tables[table] = ()
                self.assert_response(capture, replace(bundle, tables=tables), False)
        for table, field, value in (
            ("sources", "actor_kind", "agent"), ("sources", "detail_one", "another-host"),
            ("sources", "locator", "another answer"), ("sources", "project_id", "another-project"),
            ("question_response_sources", "question_revision", 99),
            ("decisions", "question_id", "another-question"), ("decisions", "user_authority", "agent"),
            ("question_decision_history_witnesses", "root_decision_id", "another-decision"),
            ("question_decision_history_witnesses", "response_source_id", "another-source"),
            ("question_decision_history_witnesses", "response_authority", "agent")):
            with self.subTest(table=table, field=field):
                tables = deepcopy(bundle.tables)
                for row in tables[table]:
                    row[field] = value
                self.assert_response(capture, replace(bundle, tables=tables), False)

    def test_multi_item_same_answer_is_ambiguous(self):
        capture, bundle = self.async_response_fixture(structured=True)
        turn = capture.user_turns[-1]
        items = json.loads(turn.text.split('>', 1)[1].rsplit('<', 1)[0])
        request = capture.async_question_requests[0]
        items.append({**items[0], "questionItemId": json.dumps(["request_user_input_async", request.call_id, 1])})
        capture = replace(capture, async_question_requests=(replace(request, questions=request.questions * 2),),
            user_turns=(*capture.user_turns[:-1], replace(turn, text=self.envelope(items))))
        self.assert_response(capture, bundle, False)

    def test_unknown_wrapper_never_gains_plain_authority(self):
        capture, bundle = self.async_response_fixture(structured=True)
        decision = capture.successful_calls("decision_record")[0]
        turn = capture.user_turns[-1]
        text = '<unknown>Even an exact caller match is not user authority</unknown>'
        decision = replace(decision, arguments={**decision.arguments, "user_turn": text})
        changed = replace(capture, user_turns=(*capture.user_turns[:-1], replace(turn, text=text)))
        self.assertFalse(h.current_host_response_transport(changed, decision)["equivalent"])

    def test_request_capture_rejects_missing_failed_malformed_and_duplicate_requests(self):
        def request(events):
            return next(e for e in events if e.get("payload", {}).get("name") == "request_user_input_async")
        def output(events):
            return next(e for e in events if e.get("payload", {}).get("type") == "function_call_output")
        mutations = [
            lambda es: es.remove(request(es)),
            lambda es: es.remove(output(es)),
            lambda es: es.insert(es.index(request(es)), deepcopy(request(es))),
            lambda es: es.insert(es.index(output(es)), deepcopy(output(es))),
            lambda es: request(es)["payload"].update(arguments='{bad JSON}'),
            lambda es: request(es)["payload"].update(arguments='{"questions":[]}'),
            lambda es: request(es)["payload"].update(arguments='{"questions":[{"title":""}]}'),
            lambda es: request(es)["payload"]["internal_chat_message_metadata_passthrough"].update(turn_id="other-task"),
            lambda es: output(es)["payload"].update(output='{"accepted":false}'),
            lambda es: output(es)["payload"].update(output='{"accepted":1}'),
            lambda es: output(es)["payload"].update(output='{"accepted":false,"accepted":true}'),
            lambda es: output(es)["payload"].update(output='{bad JSON}'),
            lambda es: output(es)["payload"]["internal_chat_message_metadata_passthrough"].update(turn_id="other-task"),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                capture, bundle = self.async_response_fixture(structured=True, mutate=mutate)
                self.assertEqual(capture.async_question_requests, ())
                self.assert_response(capture, bundle, False)

    def test_reply_from_other_thread_is_rejected_by_capture_owner(self):
        def change(events):
            user = next(e["payload"] for e in events if e.get("payload", {}).get("item", {}).get("type") == "UserMessage")
            user["thread_id"] = "another-session"
        with self.assertRaises(h.EvidenceError):
            self.async_response_fixture(structured=True, mutate=change)

    def test_structured_text_uses_existing_directional_transport_contract(self):
        capture, bundle = self.async_response_fixture(structured=True)
        turn = capture.user_turns[-1]
        items = json.loads(turn.text.split('>', 1)[1].rsplit('<', 1)[0])
        for field in ("answer", "question"):
            changed = deepcopy(items)
            changed[0][field] += " \t\r\n"
            self.assert_response(replace(capture, user_turns=(*capture.user_turns[:-1],
                replace(turn, text=self.envelope(changed)))), bundle, True)

    def test_structured_conflicting_scope_and_duplicate_witnesses_remain_hard(self):
        capture, bundle = self.async_response_fixture(structured=True)
        decision = capture.successful_calls("decision_record")[0]
        for field, value in (("work_scope", "project_wide"), ("work_item_id", "another-work")):
            altered = replace(decision, arguments={**decision.arguments, field: value})
            self.assert_response(replace(capture, tool_calls=tuple(altered if c is decision else c
                for c in capture.tool_calls)), bundle, False)
        self.assert_response(replace(capture, tool_calls=(*capture.tool_calls, decision)), bundle, False)
        for table in ("sources", "question_response_sources", "decisions", "question_decision_history_witnesses"):
            tables = deepcopy(bundle.tables)
            tables[table] = tables[table] * 2
            self.assert_response(capture, replace(bundle, tables=tables), False)
        tables = deepcopy(bundle.tables)
        original = tables["question_decision_history_witnesses"][0]
        tables["question_decision_history_witnesses"] = (*tables["question_decision_history_witnesses"],
            {**original, "root_decision_id": "conflicting-decision"})
        self.assert_response(capture, replace(bundle, tables=tables), False)

    def test_raw_reply_before_request_and_after_decision_are_rejected(self):
        def before_request(events):
            reply = next(e for e in events if e.get("payload", {}).get("item", {}).get("type") == "UserMessage")
            events.remove(reply)
            index = next(i for i, e in enumerate(events) if e.get("payload", {}).get("name") == "request_user_input_async")
            events.insert(index, reply)
        def after_decision(events):
            reply = next(e for e in events if e.get("payload", {}).get("item", {}).get("type") == "UserMessage")
            events.remove(reply)
            index = next(i for i, e in enumerate(events) if e.get("payload", {}).get("invocation", {}).get("tool") == "decision_record")
            events.insert(index + 1, reply)
        for mutate in (before_request, after_decision):
            capture, bundle = self.async_response_fixture(structured=True, mutate=mutate)
            self.assert_response(capture, bundle, False)

    def test_prefixed_envelope_cannot_gain_plain_authority(self):
        capture, bundle = self.async_response_fixture(structured=True)
        turn = capture.user_turns[-1]
        decision = capture.successful_calls("decision_record")[0]
        text = "prefix " + turn.text
        changed = replace(capture, user_turns=(*capture.user_turns[:-1], replace(turn, text=text)))
        call = replace(decision, arguments={**decision.arguments, "user_turn": text})
        self.assertFalse(h.current_host_response_transport(changed, call)["equivalent"])

    def test_two_decisions_from_one_multi_item_event_count_once(self):
        capture, bundle = self.async_response_fixture(structured=True)
        turn = capture.user_turns[-1]
        first = capture.successful_calls("decision_record")[0]
        first_facts = h.decision_facts(capture, bundle)
        old_question = first.arguments["question_id"]
        old_source = first.result["user_response_source_id"]
        old_decision = first_facts[1]
        new_question, new_source, new_decision = "a1" * 16, "a2" * 16, "a3" * 16
        answer, question = "Keep the additional report available.", "Should the additional report remain available?"
        items = json.loads(turn.text.split('>', 1)[1].rsplit('<', 1)[0])
        items.insert(0, {"questionItemId": json.dumps(["request_user_input_async", "async-request", 1]),
            "answer": answer, "question": question})
        request = capture.async_question_requests[0]
        second = replace(first, sequence=first.sequence + 1, completion_sequence=first.completion_sequence + 1,
            call_id="second-decision", arguments={**first.arguments, "question_id": new_question, "user_turn": answer},
            result={**first.result, "user_response_source_id": new_source})
        calls = []
        for call in capture.tool_calls:
            if call.operation == "inquiry_frontier" and call.outcome == "succeeded":
                presented = next(q for q in call.result["questions"] if q["identity"] == old_question)
                call = replace(call, result={**call.result, "questions": [*call.result["questions"],
                    {**presented, "identity": new_question}]})
            calls.append(call)
        capture = replace(capture, user_turns=(*capture.user_turns[:-1], replace(turn, text=self.envelope(items))),
            async_question_requests=(replace(request, questions=(*request.questions, question)),),
            tool_calls=(*calls, second))
        tables = deepcopy(bundle.tables)
        for table in ("sources", "question_response_sources", "decisions", "question_revisions", "question_decision_history_witnesses"):
            original = next(row for row in tables[table] if (row.get("id") == old_source if table == "sources"
                else row.get("question_id") == old_question))
            added = {key: (new_question if value == old_question else new_source if value == old_source
                else new_decision if value == old_decision else value) for key, value in original.items()}
            if table == "sources":
                added["locator"] = answer
            tables[table] = (*tables[table], added)
        bundle = replace(bundle, tables=tables)
        facts = self.assert_response(capture, bundle, True)
        self.assertEqual(len(facts[-1]), 2)

    def test_async_reply_in_same_task_has_exact_current_schema_authority(self):
        capture, bundle = self.async_response_fixture()
        decision = capture.successful_calls("decision_record")[0]
        self.assertEqual(len(capture.user_turns), 2)
        self.assertEqual(len({t.turn_id for t in capture.user_turns}), 1)
        self.assertEqual(len({t.user_turn_id for t in capture.user_turns}), 2)
        latest = capture.user_turns[-1]
        self.assertEqual(capture.turn_for_call(decision), latest)
        facts = h.decision_facts(capture, bundle)
        self.assertTrue(facts[0])
        self.assert_response(capture, bundle, True)
        transport = facts[-1][facts[1]]["current_host_response_transport"]
        self.assertEqual(transport["captured_user_turn_id"], latest.user_turn_id)
        self.assertEqual(transport["captured_turn_sequence"], latest.sequence)
        self.assertEqual(transport["raw_capture_sha256"], capture.source_sha256)

    def test_async_reply_cannot_reuse_old_future_cross_task_or_ambiguous_authority(self):
        capture, bundle = self.async_response_fixture()
        decision = capture.successful_calls("decision_record")[0]
        latest = capture.user_turns[-1]
        for turns in (
            capture.user_turns[:-1],
            (*capture.user_turns[:-1], replace(latest, sequence=decision.sequence + 1)),
            (*capture.user_turns[:-1], replace(latest, turn_id="other-task")),
            (*capture.user_turns, replace(latest, user_turn_id="ambiguous-client")),
            (*capture.user_turns, replace(latest, sequence=decision.sequence - 1,
                user_turn_id="newer-client", text="A new response replaces the earlier answer")),
        ):
            self.assertFalse(h.decision_facts(replace(capture, user_turns=turns), bundle)[0])
        for table in ("sources", "question_response_sources", "decisions",
                      "question_decision_history_witnesses", "question_revisions"):
            tables = deepcopy(bundle.tables)
            tables[table] = ()
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])
        for table, field, value in (
            ("question_response_sources", "question_revision", 99),
            ("question_decision_history_witnesses", "root_decision_id", "f" * 32),
            ("question_decision_history_witnesses", "response_source_id", "f" * 32),
            ("question_decision_history_witnesses", "response_authority", "agent"),
            ("decisions", "user_authority", "agent"),
            ("question_revisions", "material_scope", "malformed"),
        ):
            tables = deepcopy(bundle.tables)
            for row in tables[table]:
                row[field] = value
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])

    def test_internal_host_session_is_not_raw_codex_session(self):
        capture, bundle = self.fixture()
        self.assertTrue(h.decision_facts(capture, bundle)[0])
        tables = deepcopy(bundle.tables)
        for source in tables["sources"]:
            if source["source_kind"] == "current_host_user_turn":
                source["detail_two"] = "independent-host-adapter-session"
        changed = replace(bundle, tables=tables)
        self.assertTrue(h.decision_facts(capture, changed)[0])
        goal = capture.successful_calls("context_record")[0]
        self.assertTrue(h.goal_facts(capture, changed, capture.user_turns[0].text,
            goal.result["context_item_id"])[0])

    def test_exact_decision_links_reject_substitution(self):
        capture, bundle = self.fixture()
        decision = capture.successful_calls("decision_record")[0]
        for field, value in (("question_revision", 99), ("question_id", "ff" * 16),
            ("presentation_receipt_id", "unrelated-receipt"),
            ("user_turn", "The agent recommends stable behavior."),
            ("user_turn", "Keep concise output, please.")):
            with self.subTest(field=field, value=value):
                altered = replace(decision, arguments={**decision.arguments, field: value})
                changed = replace(capture, tool_calls=tuple(altered if c is decision else c for c in capture.tool_calls))
                self.assertFalse(h.decision_facts(changed, bundle)[0])
        for field, value in (("actor_kind", "agent"), ("detail_one", "other-host"),
            ("project_id", "ff" * 16), ("source_kind", "file"),
            ("locator", "unrelated user turn")):
            tables = deepcopy(bundle.tables)
            source = next(s for s in tables["sources"] if s["id"] == decision.result["user_response_source_id"])
            source[field] = value
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])
        for altered in (
            replace(decision, turn_id=capture.user_turns[0].turn_id),
            replace(decision, result={**decision.result, "user_response_source_id": "ff" * 16}),
        ):
            changed = replace(capture, tool_calls=tuple(altered if c is decision else c for c in capture.tool_calls))
            self.assertFalse(h.decision_facts(changed, bundle)[0])
        response = capture.turn_for_call(decision)
        changed = replace(capture, user_turns=tuple(replace(t, text="A different response") if t is response else t
            for t in capture.user_turns))
        self.assertFalse(h.decision_facts(changed, bundle)[0])
        self.assertFalse(h.decision_facts(replace(capture, tool_calls=(*capture.tool_calls, decision)), bundle)[0])
        for table in ("question_response_sources", "question_decision_history_witnesses", "decisions"):
            tables = deepcopy(bundle.tables)
            tables[table] = (*tables[table], *tables[table])
            self.assertFalse(h.decision_facts(capture, replace(bundle, tables=tables))[0])

    def test_actual_capture_and_canonical_source_linkage(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for suffix, semantic_change, expected in ((" \t\r\n", False, True), ("", True, False)):
                descriptor = h.real_session_fixture("volicord", 1, "0" * 40, root)
                path = root / descriptor["evidence"]["captures"]["work"]["file"]
                capture = h.load_codex_capture(path)
                decision = capture.successful_calls("decision_record")[0]
                caller = decision.arguments["user_turn"]
                raw = caller.replace("concise", "verbose") if semantic_change else caller + suffix
                events = [json.loads(line) for line in path.read_text().splitlines()]
                changed = 0
                for event in events:
                    payload = event.get("payload", {})
                    if payload.get("type") == "user_message" and payload.get("message") == caller:
                        payload["message"] = raw
                        changed += 1
                self.assertEqual(changed, 1)
                path.write_text("".join(json.dumps(event) + "\n" for event in events))
                capture = h.load_codex_capture(path)
                bundle = h.load_canonical_bundle(root / descriptor["evidence"]["canonical_bundle"]["file"])
                facts = h.decision_facts(capture, bundle)
                self.assertEqual(facts[0], expected)
                checkpoint = h.terminal_checkpoint_call(capture)
                baseline = h.selected_checkpoint_baseline_call(capture, checkpoint)
                observations = h.work_blocker_behavior_observations(capture, "explicit_user_owned_decision", baseline,
                    min(x.sequence for x in h.meaningful_work_path_observations(capture)))
                self.assertEqual(observations[2], expected)
                if expected:
                    evidence = next(iter(facts[-1].values()))["current_host_response_transport"]
                    self.assertEqual(evidence["raw_capture_sha256"], h.sha256(path))
                    self.assertEqual(evidence["raw_host_text_sha256"], hashlib.sha256(raw.encode()).hexdigest())
                    self.assertEqual(evidence["canonical_source_text_sha256"], hashlib.sha256(caller.encode()).hexdigest())
                    self.assertGreater(evidence["terminal_ascii_whitespace_removed_count"], 0)


def check_decision_transport_regressions():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(DecisionTransportTests))
    if not result.wasSuccessful():
        raise AssertionError("Decision transport regressions failed")


if __name__ == "__main__":
    check_decision_transport_regressions()
