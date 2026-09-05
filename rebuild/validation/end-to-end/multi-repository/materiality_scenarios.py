"""Installed-MCP Materiality behavior matrix; deterministic, not a model-quality claim.

The evaluator owns scenario expectations. No private campaign labels or expected
answers are sent to an authenticated model. V11's connection probe stays separate.
"""
from __future__ import annotations

import argparse
import copy
import json
import os
from pathlib import Path
import tempfile
from typing import Any


SCENARIOS = (
    "tuple-precedent", "failure-semantics", "accepted-contract",
    "applicable-decision", "exact-delegation", "mechanical-fact", "private-helper",
)


def qualify(api: Any, binary: Path, env: dict[str, str], root: Path) -> dict[str, Any]:
    results = {}
    for name in SCENARIOS:
        directory = root / name
        repository = directory / "repository"
        repository.mkdir(parents=True)
        source = repository / "codec.py"
        source.write_text(
            "def decode_unchecked(value):\n    return (True, value)\n\n"
            "def protocol_tag():\n    return 7\n",
            encoding="utf-8",
        )
        (repository / "CONTRACT.md").write_text(
            "Existing decode_unchecked callers retain their tuple. A new inspection API "
            "must preserve those callers. Private organization is unrestricted if results "
            "and the fixed resource budget are preserved.\n"
            + ("The new inspect API MUST return a tagged result and MUST NOT throw for malformed input.\n"
               if name == "accepted-contract" else ""), encoding="utf-8",
        )
        host = api.Mcp(binary, env | {"VOLICORD_RUNTIME_DIR": str(directory / "runtime")})
        trace = []
        work = repository / "implementation.py"

        def call(tool: str, arguments: dict[str, Any], *, accepted: bool = True) -> dict[str, Any]:
            value, ok = host.tool(tool, arguments)
            trace.append({"sequence": len(trace), "tool": tool, "action": arguments.get("action"),
                          "accepted": ok, "work_exists": work.exists(),
                          "workflow": (value or {}).get("workflow", {}).get("stage")})
            if ok != accepted:
                raise AssertionError(f"{name}: {tool}: {value}")
            return value or {}

        try:
            host.initialize()
            project = call("project_initialize", {"repository": str(repository)})["project_id"]
            goal_text = "Add an inspection API while preserving existing callers."
            if name == "exact-delegation":
                goal_text += " I delegate the new API return and failure contract to you."
            elif name == "mechanical-fact":
                goal_text = "Record the exact protocol tag already present in the repository."
            elif name == "private-helper":
                goal_text = "Reorganize the private helper while preserving all caller behavior."
            goal = call("context_record", {"project_id": project, "user_turn": goal_text,
                                          "role": "goal", "statement": goal_text})
            analysis = call("repository_analyze", {"project_id": project})
            source_id = analysis["repository_source_id"]
            if name == "tuple-precedent":
                alternatives = [("optional", "Return an optional tuple; absence requires a None check"),
                                ("tuple", "Always return a tuple; callers always unpack it"),
                                ("tagged", "Return a tagged result; callers inspect the variant")]
            elif name == "mechanical-fact":
                alternatives = [("observed", "Record the observed protocol tag 7"),
                                ("different", "Record 8, contradicting the current source")]
            elif name == "private-helper":
                alternatives = [("inline", "Keep the private check inline with the same public results"),
                                ("helper", "Extract the identical check into a private helper")]
            else:
                alternatives = [("tagged", "Return a discriminated failure; callers inspect the result"),
                                ("throw", "Throw an exception; callers must catch the failure")]
            effects = ["implementation_internal"] if name == "private-helper" else [
                "public_api_shape_or_semantics", "failure_or_error_semantics"]
            choice = {
                "choice_id": "inspection-contract", "summary": goal_text,
                "affected_scope": ["implementation.py"],
                "alternatives": [{"alternative_id": key, "summary": consequence,
                    "technical_consequences": [consequence],
                    "material_decomposition": {"state": "materially_atomic",
                        "rationale": "This bounded fixture fixes other behavior; this alternative leaves no subordinate material fork."}}
                    for key, consequence in alternatives],
                "technical_consequences": [text for _, text in alternatives],
                "source_ids": [source_id], "effect_categories": effects,
                "relationship": {"state": "independent"}, "evidence_state": "sufficient",
            }
            discovery = call("engineering_choice_discovery", {
                "project_id": project, "goal_context_id": goal["context_item_id"],
                "baseline_analysis_snapshot_id": analysis["analysis_snapshot_id"],
                "source_operation": "bounded repository investigation", "summary": goal_text,
                "choices": [choice], "material_boundary_review": api.material_boundary_review([choice], [source_id]),
            })
            accounts = api.alternative_accounting(choice["choice_id"], [a[0] for a in alternatives], source_id)
            judgment = {
                "choice_id": choice["choice_id"], "disposition": "unresolved_user_owned_outcome",
                "basis_summary": "Existing callers constrain compatibility but do not select the new API contract.",
                "authority_counterfactual": "Compatible representations remain materially different for new callers; no exact contract, Decision or delegation selects one.",
                "materially_varying_outcomes": choice["technical_consequences"],
                "contains_user_owned_outcome": True, "user_owned_outcomes": ["new caller return and failure contract"],
                "ownership_rationale": "Caller inspection versus exception handling and public return shape are product semantics, regardless of where the code is implemented.",
                "ownership_source_ids": [source_id], "alternative_accounting": accounts,
                "learning_value": {"state": "routine", "rationale": "User authority is independent of learning participation."},
            }
            request = {
                "action": "record", "project_id": project,
                "engineering_choice_discovery_candidate_id": discovery["discovery_candidate_id"],
                "rationale": "Assess the exact outcome after inspecting current source and contract.",
                "behavioral_context_basis": {"context_item_ids": [], "completeness_rationale": "Only this Goal affects the bounded fixture."},
                "learning_participation": {"state": "inactive"}, "judgments": [judgment],
            }

            def settle(kind: str, reference: str | None = None) -> None:
                judgment["disposition"] = "repository_or_environment_fact" if kind == "unique_mechanical_fact" else "settled_authority"
                role = {"kind": kind}
                if kind == "accepted_contract":
                    role["contract_reference"] = reference
                    judgment["contract_basis"] = [reference]
                elif kind == "applicable_decision":
                    role["decision_id"] = reference
                    judgment["decision_ids"] = [reference]
                judgment["basis_summary"] = "The exact current contract, Decision or mechanical observation settles the complete dimension."
                judgment["authority_counterfactual"] = "Applying the cited exact requirement leaves only the selected outcome; each other alternative contradicts that requirement."
                if kind == "unique_mechanical_fact":
                    judgment["user_owned_outcomes"] = ["accuracy of the recorded public protocol value"]
                    judgment["ownership_rationale"] = "Recording a different public value would misstate the product protocol; the current source uniquely establishes the fact without a new user choice."
                judgment.update({"authority_coverage": choice["summary"],
                    "unique_outcome_rationale": "The exact current requirement excludes every other listed outcome.",
                    "authority_source_evidence": [{"source_id": source_id, "role": role,
                        "rationale": "CONTRACT.md explicitly mandates tagged results without throwing." if kind == "accepted_contract"
                        else "The current Decision selects this exact displayed contract." if kind == "applicable_decision"
                        else "codec.py protocol_tag returns the literal 7; recording any other value contradicts the observed source."}]})
                for index, account in enumerate(accounts):
                    account["status"] = "selected" if index == 0 else {
                        "accepted_contract": "eliminated_by_accepted_contract",
                        "applicable_decision": "eliminated_by_applicable_decision",
                        "unique_mechanical_fact": "eliminated_by_repository_or_environment_fact",
                    }[kind]
                    if index and kind != "unique_mechanical_fact":
                        account["contract_reference" if kind == "accepted_contract" else "decision_id"] = reference
                    account["rationale"] = "The cited exact requirement selects this outcome or excludes this incompatible alternative."

            if name in {"tuple-precedent", "failure-semantics"}:
                false_claim = copy.deepcopy(request)
                bad = false_claim["judgments"][0]
                if name == "failure-semantics":
                    bad.update({"disposition": "agent_owned_implementation_choice", "contains_user_owned_outcome": False,
                                "user_owned_outcomes": [], "bounded_implementation_discretion_rationale": "The agent implements the failure handling internally."})
                else:
                    bad.update({"disposition": "settled_authority", "contract_basis": ["decode_unchecked tuple convention"],
                        "authority_coverage": "new public result representation", "unique_outcome_rationale": "Follow the existing tuple precedent.",
                        "authority_source_evidence": [{"source_id": source_id, "role": {"kind": "repository_precedent"},
                            "rationale": "Existing decode_unchecked returns a tuple, but no accepted clause requires it for the new API."}]})
                    for index, account in enumerate(bad["alternative_accounting"]):
                        account["status"] = "selected" if index == 0 else "eliminated_by_accepted_contract"
                        if index:
                            account["contract_reference"] = "decode_unchecked tuple convention"
                call("materiality_review", false_claim, accepted=False)
                assert not work.exists()
            elif name == "accepted-contract":
                settle("accepted_contract", "CONTRACT.md: new inspect API MUST clause")
            elif name == "exact-delegation":
                judgment.update({"disposition": "delegated_implementation_choice",
                    "basis_summary": "The current Goal explicitly delegates this exact new API return and failure contract.",
                    "authority_counterfactual": "Callers observe materially different outcomes, but the verbatim current-task delegation explicitly covers this dimension and scope.",
                    "delegation_statement": "I delegate the new API return and failure contract to you",
                    "delegated_scope": ["implementation.py"]})
            elif name == "mechanical-fact":
                settle("unique_mechanical_fact")
            elif name == "private-helper":
                judgment.update({"disposition": "agent_owned_implementation_choice", "contains_user_owned_outcome": False,
                    "user_owned_outcomes": [],
                    "basis_summary": "The current source and contract leave private helper organization unconstrained.",
                    "authority_counterfactual": "Both private arrangements preserve caller behavior and the fixed budget; no user-owned product policy varies.",
                    "ownership_rationale": "Only private organization differs; neither callers nor operators observe a product-policy difference.",
                    "bounded_implementation_discretion_rationale": "Only a private extraction changes; public results and the fixed resource budget remain identical.",
                    "discretion_counterfactuals": [{"choice_id": choice["choice_id"], "alternative_id": key,
                        "externally_observable": False, "observation_rationale": consequence,
                        "source_id": source_id, "source_supported_boundary": "CONTRACT.md permits private organization when public behavior and the fixed budget are preserved."}
                        for key, consequence in alternatives]})
            review = call("materiality_review", request)
            measurement_start = 0
            needs_question = name in {"tuple-precedent", "failure-semantics", "applicable-decision"}
            if needs_question:
                assert review["workflow"]["blocks_ordinary_work"] is True
                assert review["workflow"]["stage"] == "question_candidate"
                candidate = call("candidate_manage", {
                    "action": "submit_question_from_materiality", "project_id": project,
                    "review_candidate_id": review["review_candidate_id"], "dimension_id": choice["choice_id"],
                    "research_state": "ready_to_ask", "research_state_basis": "Current source inspection leaves the public semantics open.",
                    "retention_basis": "Through the current response", "bounded_summary": choice["summary"],
                    "prompt": "Which public inspection contract should callers use?", "why_now": "Implementation would otherwise commit a user-owned contract.",
                    "alternatives": [{"key": key, "label": key, "consequence": text} for key, text in alternatives],
                    "recommendation_key": alternatives[0][0], "recommendation_rationale": "This is a recommendation pending the user's choice.",
                    "duplicate_basis": "No current Question or Decision settles this dimension.", "presentation_order": 1,
                })
                promoted = call("candidate_manage", {"action": "promote_question", "project_id": project, "candidate_id": candidate["candidate_id"]})
                frontier = call("inquiry_frontier", {"project_id": project})
                displayed = next(q for q in frontier["questions"] if q["identity"] == promoted["question_id"])
                assert len(displayed["alternatives"]) == len(alternatives)
                response = {"project_id": project, "question_id": promoted["question_id"],
                    "question_revision": displayed["revision"], "alternative_key": alternatives[0][0],
                    "user_turn": f"Choose {alternatives[0][0]} for this displayed public inspection contract."}
                call("decision_record", response, accepted=False)
                assert not work.exists()
                response["presentation_receipt_id"] = displayed["presentation_receipt_id"]
                decision = call("decision_record", response)
                assert decision["all_succeeded"] is True
                canonical = call("canonical_inspect", {"project_id": project})
                record = next(r for r in canonical["records"] if r["kind"] == "decision")
                decision_id = record["identity"]
                request.pop("engineering_choice_discovery_candidate_id")
                request.pop("behavioral_context_basis")
                request.update({"action": "revise", "review_candidate_id": review["review_candidate_id"]})
                if name == "applicable-decision":
                    measurement_start = len(trace)
                    settle("applicable_decision", decision_id)
                    decision_source = decision["user_response_source_id"]
                    judgment["authority_source_evidence"][0]["source_id"] = decision_source
                    for account in accounts:
                        account["source_ids"].append(decision_source)
                else:
                    judgment["resolution_decision_id"] = decision_id
                    judgment["alternative_accounting"] = api.alternative_accounting(choice["choice_id"], [a[0] for a in alternatives], source_id, resolution_decision_id=decision_id)
                review = call("materiality_review", request)
            ready = call("materiality_review", {"action": "inspect", "project_id": project,
                "review_candidate_id": review["review_candidate_id"], "goal_context_id": goal["context_item_id"],
                "baseline_analysis_snapshot_id": analysis["analysis_snapshot_id"], "paths": ["implementation.py"],
                "coupled_artifact_review": api.coupled_artifact_review(["implementation.py"])})
            assert ready["workflow"]["stage"] == "ready_for_work", ready
            frontier = call("inquiry_frontier", {"project_id": project})
            assert frontier["questions"] == []
            assert not work.exists()
            implementation = (
                "def inspect(value):\n    return None if value is None else (True, value)\n"
                if name == "tuple-precedent" else
                "PROTOCOL_TAG = 7\n" if name == "mechanical-fact" else
                "def _identity(value):\n    return value\n" if name == "private-helper" else
                "def inspect(value):\n    return {'kind': 'error'} if value is None else {'kind': 'value', 'value': value}\n"
            )
            work.write_text(implementation, encoding="utf-8")
            trace.append({"sequence": len(trace), "tool": "ordinary_write", "work_exists": True})
            assert all(not event["work_exists"] for event in trace[:-1])
            question_events = [event for event in trace[measurement_start:]
                               if event.get("action") == "promote_question" and event.get("accepted")]
            expected = 1 if name in {"tuple-precedent", "failure-semantics"} else 0
            assert len(question_events) == expected
            results[name] = {"status": "passed", "question_count": len(question_events),
                             "measurement_start_sequence": measurement_start,
                             "operation_chronology": trace}
        except (AssertionError, KeyError, RuntimeError, StopIteration) as error:
            results[name] = {"status": "failed", "error": str(error), "operation_chronology": trace}
        finally:
            host.close()
    return {"status": "passed" if all(r["status"] == "passed" for r in results.values()) else "failed",
            "measurement": "deterministic installed MCP; authenticated model behavior is a separate qualification", "scenarios": results}


def main() -> int:
    import harness as api
    parser = argparse.ArgumentParser()
    parser.add_argument("--mcp-binary", type=Path, required=True)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="volicord-materiality-") as directory:
        result = qualify(api, args.mcp_binary.resolve(), os.environ.copy(), Path(directory))
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
