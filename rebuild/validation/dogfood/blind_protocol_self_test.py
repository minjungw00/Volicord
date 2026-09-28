#!/usr/bin/env python3
"""Generic blind completeness protocol and campaign mutation controls."""
from __future__ import annotations

import copy
import subprocess
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import blind_dimensions
import blind_protocol as b
import campaign as c
import campaign_self_test as fixtures
import harness


class BlindProtocolTests(unittest.TestCase):
    def setUp(self):
        self.slot = "a" * 32
        self.preparation_sha = "b" * 64
        self.discovery_sha = "c" * 64
        self.critique_sha = "d" * 64
        self.critic_run = "e" * 32
        self.adjudicator_run = "f" * 32
        self.assessments = harness.fixture_blind_assessments(
            ["learning_deliberation", "learning_routine_control"])
        self.assessments[0]["outcome_scope"] = "The visible task's bounded implementation fork."
        self.assessments[1]["outcome_scope"] = "Independent recovery behavior after interrupted work."
        self.provisional = {
            "kind": "phase8_provisional_behavior_review", "review_slot_id": self.slot,
            "status": "recorded", "reviewer_role": "campaign_preparation_independent_reviewer",
            "preparation_sha256": self.preparation_sha, "assessments": [copy.deepcopy(self.assessments[0])],
            "classification": "learning_deliberation",
            "materiality_conclusion": "no_user_owned_material_outcome",
            "material_outcome_unavoidable": False,
            "operator_prompt_does_not_disclose_material_outcome": None,
            "basis": "The visible task and owner ground the bounded assessment.",
            "provenance_reference_indices": [0],
        }
        self.discovery = b.discovery_from_provisional(self.provisional)
        self.critique = {"kind": "phase8_blind_completeness_critique", "review_slot_id": self.slot,
            "preparation_sha256": self.preparation_sha, "discovery_sha256": self.discovery_sha,
            "critic_run_id": self.critic_run, "proposals": []}
        self.final = copy.deepcopy(self.provisional)
        self.final["adjudication"] = {"discovery_sha256": self.discovery_sha,
            "critique_sha256": self.critique_sha, "adjudicator_run_id": self.adjudicator_run,
            "dispositions": [], "lineage": [self.lineage(self.assessments[0]["dimension_id"])]}

    def lineage(self, dimension_id, discovery_ids=None, critic_ids=None, relationship="discovery"):
        return {"dimension_id": dimension_id,
            "discovery_dimension_ids": discovery_ids if discovery_ids is not None else [dimension_id],
            "critic_proposal_ids": critic_ids if critic_ids is not None else [],
            "relationship": relationship}

    def proposal(self, identity, kind, targets, assessment=None):
        return {"proposal_id": identity, "proposal_kind": kind,
            "target_dimension_ids": targets, "outcome_scope":
                assessment["outcome_scope"] if assessment else "Contested separate user-visible scope.",
            "basis": "The visible task and pinned owner support inspection of this independent scope.",
            "provenance_reference_indices": [0], "proposed_assessment": assessment}

    def disposition(self, identity, decision, destinations):
        return {"proposal_id": identity, "disposition": decision,
            "basis": "The visible task and pinned owner support this disposition.",
            "provenance_reference_indices": [0], "final_dimension_ids": destinations,
            "accepted_scope": None, "rejected_scope": None}

    def check(self):
        prep = {"kind": "phase8_blind_review_preparation_reference",
            "review_slot_id": self.slot, "sha256": self.preparation_sha}
        self.assertEqual(b.discovery_errors(self.discovery, prep, 1), [])
        self.assertEqual(b.critique_errors(self.critique, self.slot, self.preparation_sha,
            self.discovery_sha, self.critic_run, self.discovery, 1), [])
        self.assertEqual(harness.blind_first_review_errors(prep, self.final, 1), [])
        return b.adjudication_errors(self.final, self.discovery, self.critique,
            self.discovery_sha, self.critique_sha, self.adjudicator_run, 1)

    def test_primary_omission_and_common_blind_spot_recovered_by_critic(self):
        added = copy.deepcopy(self.assessments[1])
        self.critique["proposals"] = [self.proposal("recovery", "missing_scope", [], added)]
        self.final["assessments"].append(added)
        self.final["adjudication"]["dispositions"] = [self.disposition("recovery", "accept", [added["dimension_id"]])]
        self.final["adjudication"]["lineage"].append(
            self.lineage(added["dimension_id"], [], ["recovery"], "accepted_critic"))
        self.assertEqual(self.check(), [])
        self.assertEqual(len(self.final["assessments"]), 2)
        self.assertEqual(blind_dimensions.coverage_errors(harness.fixture_obligation_coverage(self.final["assessments"]),
            self.final, ["learning_deliberation", "learning_routine_control"], 1,
            harness.MAX_REVIEW_TEXT_BYTES), [])
        self.final["adjudication"]["dispositions"].clear()
        self.assertTrue(self.check())  # Proposal cannot be copied automatically.

    def test_false_positive_rejected_and_equivalent_concern_merged(self):
        self.critique["proposals"] = [self.proposal("unsupported", "missing_scope", [], self.assessments[1]),
            self.proposal("same", "duplicate_scope", [self.assessments[0]["dimension_id"]])]
        self.final["adjudication"]["dispositions"] = [
            self.disposition("unsupported", "reject", []),
            self.disposition("same", "merge_equivalent", [self.assessments[0]["dimension_id"]])]
        self.final["adjudication"]["lineage"][0]["critic_proposal_ids"] = ["same"]
        self.final["adjudication"]["lineage"][0]["relationship"] = "merged_equivalent"
        self.assertEqual(self.check(), [])
        self.assertEqual(len(self.final["assessments"]), 1)
        self.assertNotIn("fixture-dimension-1", {a["dimension_id"] for a in self.final["assessments"]})

    def test_partial_acceptance_separates_supported_and_unsupported_scope(self):
        added = copy.deepcopy(self.assessments[1])
        self.critique["proposals"] = [self.proposal("mixed", "split_scope",
            [self.assessments[0]["dimension_id"]], added)]
        self.final["assessments"].append(added)
        disposition = self.disposition("mixed", "partially_accept", [added["dimension_id"]])
        self.final["adjudication"]["dispositions"] = [disposition]
        self.final["adjudication"]["lineage"].append(
            self.lineage(added["dimension_id"], [self.assessments[0]["dimension_id"]],
                ["mixed"], "accepted_critic"))
        self.assertTrue(self.check())
        disposition["accepted_scope"] = "The visible recovery outcome is separately material."
        disposition["rejected_scope"] = "The proposed extra presentation detail lacks material support."
        self.assertEqual(self.check(), [])

    def test_both_miss_scope_then_reveal_still_finds_gap(self):
        self.assertEqual(self.check(), [])
        rows = harness.fixture_obligation_coverage(self.final["assessments"])
        rows.append({"obligation": "learning_routine_control", "dimension_id": None,
            "reviewer_outcome_scope": None, "evaluator_outcome_scope": "A separately material recovery outcome.",
            "status": "blind_coverage_gap", "applicability_resolution": "not_applicable",
            "basis": "The revealed obligation has no pre-reveal assessment.",
            "provenance_reference_indices": [0]})
        errors = blind_dimensions.coverage_errors(rows, self.final,
            ["learning_deliberation", "learning_routine_control"], 1, harness.MAX_REVIEW_TEXT_BYTES)
        self.assertTrue(any("blind_coverage_gap" in error for error in errors))

    def test_explicit_negative_survives_and_is_not_an_omission(self):
        negative = self.discovery["assessments"][0]
        negative["applicability"] = "not_applicable"
        self.final["assessments"][0] = copy.deepcopy(negative)
        self.assertEqual(self.check(), [])
        rows = harness.fixture_obligation_coverage(self.final["assessments"])
        rows[0]["status"] = "applicability_disagreement"
        rows[0]["applicability_resolution"] = "unresolved_conflict"
        self.assertEqual(blind_dimensions.coverage_errors(rows, self.final,
            ["learning_deliberation"], 1, harness.MAX_REVIEW_TEXT_BYTES), [])
        rows[0]["status"] = "blind_coverage_gap"
        self.assertTrue(blind_dimensions.coverage_errors(rows, self.final,
            ["learning_deliberation"], 1, harness.MAX_REVIEW_TEXT_BYTES))

    def test_private_material_and_invalid_lineage_rejected(self):
        self.discovery["evaluation_basis"] = {"secret": "private"}
        self.assertTrue(b.discovery_errors(self.discovery,
            {"kind": "phase8_blind_review_preparation_reference", "review_slot_id": self.slot,
             "sha256": self.preparation_sha}, 1))
        self.discovery.pop("evaluation_basis")
        self.critique["proposals"] = [self.proposal("hidden", "authority", [self.assessments[0]["dimension_id"]])]
        self.critique["proposals"][0]["basis"] = "See evaluator/qualification-profile.json"
        self.assertTrue(b.critique_errors(self.critique, self.slot, self.preparation_sha,
            self.discovery_sha, self.critic_run, self.discovery, 1))
        self.critique["proposals"] = []
        self.final["adjudication"]["lineage"][0]["critic_proposal_ids"] = ["unrecorded"]
        self.assertTrue(self.check())


class CampaignBlindStageTests(unittest.TestCase):
    def test_cli_stage_help_is_current(self):
        helper = c.ROOT / "rebuild/scripts/dogfood-campaign"
        for operation, input_flag in (("validate-discovery", "--discovery"),
                                      ("record-discovery", "--discovery"),
                                      ("prepare-critique", "--review-slot-id"),
                                      ("validate-critique", "--critique"),
                                      ("record-critique", "--critique"),
                                      ("prepare-adjudication", "--review-slot-id"),
                                      ("validate-provisional-review", "--provisional-review"),
                                      ("record-provisional-review", "--provisional-review")):
            result = subprocess.run([str(helper), operation, "--help"], capture_output=True,
                                    text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(input_flag, result.stdout)

    def test_critic_recovers_primary_omission_in_recorded_final(self):
        with tempfile.TemporaryDirectory(prefix="blind-recovery-") as temporary, \
                patch.object(harness, "git_clean", return_value=True):
            parent = Path(temporary)
            root = parent / "campaign"
            binary = parent / "candidate/bin/volicord"
            fixtures.write_fake_binary(binary)
            fixtures.prepare(root, parent / "sources", binary)
            descriptor, *_ = fixtures.fixture_for(parent / "fixture", "volicord", "A", campaign_root=root)
            descriptor.pop("_evidence_directory", None)
            descriptor.pop("_evidence_file_sha256", None)
            descriptor.pop("evidence", None)
            source = parent / "descriptor.json"
            c.write_json(source, descriptor)
            prepared = c.prepare_review(root, "volicord", "A", source)
            slot, head = prepared["review_slot_id"], c.load_campaign(root)["candidate_head"]
            provisional = copy.deepcopy(descriptor["behavior_review"]["independent_review"]["provisional_review"])
            provisional["preparation_sha256"] = prepared["preparation_sha256"]
            provisional["review_slot_id"] = slot
            self.assertGreaterEqual(len(provisional["assessments"]), 2)
            missed = provisional["assessments"].pop()
            discovery = b.discovery_from_provisional(provisional)
            draft = parent / "discovery.json"
            c.write_json(draft, discovery)
            c.record_discovery(root, head, slot, draft)
            c.prepare_critique(root, head, slot)
            critique = c.read_json(c.blind_stage_path(root, "critique-drafts", slot))
            critique["proposals"].append({"proposal_id": "independent-outcome", "proposal_kind": "missing_scope",
                "target_dimension_ids": [], "outcome_scope": missed["outcome_scope"],
                "basis": "The reviewer-visible owner and task support a separate outcome.",
                "provenance_reference_indices": [0], "proposed_assessment": missed})
            critique_path = parent / "critique.json"
            c.write_json(critique_path, critique)
            c.record_critique(root, head, slot, critique_path)
            staged = c.prepare_adjudication(root, head, slot)
            final = c.read_json(root / staged["draft"])
            final["assessments"].append(missed)
            final["adjudication"]["dispositions"].append({"proposal_id": "independent-outcome",
                "disposition": "accept", "basis": "The visible owner separates this material outcome.",
                "provenance_reference_indices": [0], "final_dimension_ids": [missed["dimension_id"]],
                "accepted_scope": None, "rejected_scope": None})
            final["adjudication"]["lineage"].append({"dimension_id": missed["dimension_id"],
                "discovery_dimension_ids": [], "critic_proposal_ids": ["independent-outcome"],
                "relationship": "accepted_critic"})
            final_path = parent / "final.json"
            c.write_json(final_path, final)
            c.record_provisional_review(root, head, slot, final_path)
            recorded = c.read_json(c.blind_stage_path(root, "provisional", slot))
            self.assertEqual(len(recorded["assessments"]), len(discovery["assessments"]) + 1)
            self.assertEqual(recorded["adjudication"]["lineage"][-1]["critic_proposal_ids"],
                ["independent-outcome"])

    def test_state_tampering_blindness_and_reveal_barrier(self):
        with tempfile.TemporaryDirectory(prefix="blind-stage-") as temporary, \
                patch.object(harness, "git_clean", return_value=True):
            parent = Path(temporary)
            root = parent / "campaign"
            binary = parent / "candidate/bin/volicord"
            fixtures.write_fake_binary(binary)
            fixtures.prepare(root, parent / "sources", binary)
            descriptor, *_ = fixtures.fixture_for(parent / "fixture", "volicord", "A", campaign_root=root)
            descriptor.pop("_evidence_directory", None)
            descriptor.pop("_evidence_file_sha256", None)
            descriptor.pop("evidence", None)
            source = parent / "descriptor.json"
            c.write_json(source, descriptor)
            prepared = c.prepare_review(root, "volicord", "A", source)
            slot = prepared["review_slot_id"]
            head = c.load_campaign(root)["candidate_head"]
            provisional = copy.deepcopy(descriptor["behavior_review"]["independent_review"]["provisional_review"])
            provisional["preparation_sha256"] = prepared["preparation_sha256"]
            provisional["review_slot_id"] = slot
            draft = parent / "review.json"
            c.write_json(draft, provisional)
            with self.assertRaises(c.CampaignError):
                c.record_critique(root, head, slot, draft)
            with self.assertRaises(c.CampaignError):
                c.record_provisional_review(root, head, slot, draft)
            discovery = b.discovery_from_provisional(provisional)
            c.write_json(draft, discovery)
            c.record_discovery(root, head, slot, draft)
            c.write_json(draft, provisional)
            with self.assertRaises(c.CampaignError):
                c.record_provisional_review(root, head, slot, draft)
            fixed_discovery = c.blind_stage_path(root, "discovery", slot)
            original = fixed_discovery.read_bytes()
            c.write_json(draft, dict(discovery, basis=discovery["basis"] + " later edit"))
            self.assertEqual(fixed_discovery.read_bytes(), original)
            fixed_discovery.write_bytes(original + b" ")
            with self.assertRaises(c.CampaignError):
                c.prepare_critique(root, head, slot)
            fixed_discovery.write_bytes(original)
            read_paths = []
            original_read = c.read_json
            def track_critique(path):
                read_paths.append(Path(path).as_posix())
                return original_read(path)
            with patch.object(c, "read_json", side_effect=track_critique):
                c.prepare_critique(root, head, slot)
            self.assertFalse(any("evaluator/" in path for path in read_paths))
            critique_path = parent / "critique.json"
            critique = c.read_json(c.blind_stage_path(root, "critique-drafts", slot))
            critique["proposals"] = [{"proposal_id": "false-positive", "proposal_kind": "missing_scope",
                "target_dimension_ids": [], "outcome_scope": "Unsupported additional outcome.",
                "basis": "The task does not establish this as a separate material outcome.",
                "provenance_reference_indices": [0], "proposed_assessment": None}]
            c.write_json(critique_path, critique)
            c.record_critique(root, head, slot, critique_path)
            c.write_json(draft, provisional)
            with self.assertRaises(c.CampaignError):
                c.record_provisional_review(root, head, slot, draft)
            fixed_critique = c.blind_stage_path(root, "critique", slot)
            original_critique = fixed_critique.read_bytes()
            fixed_critique.write_bytes(original_critique + b" ")
            with self.assertRaises(c.CampaignError):
                c.prepare_adjudication(root, head, slot)
            fixed_critique.write_bytes(original_critique)
            read_paths = []
            original_read = c.read_json
            def tracking(path):
                read_paths.append(Path(path).as_posix())
                return original_read(path)
            with patch.object(c, "read_json", side_effect=tracking):
                staged = c.prepare_adjudication(root, head, slot)
            self.assertFalse(any("evaluator/" in path for path in read_paths))
            final_path = parent / "final.json"
            final = c.read_json(root / staged["draft"])
            c.write_json(final_path, final)
            with self.assertRaises(c.CampaignError):
                c.record_provisional_review(root, head, slot, final_path)
            final["adjudication"]["dispositions"] = [{"proposal_id": "false-positive",
                "disposition": "reject", "basis": "No separate outcome is supported by the visible task.",
                "provenance_reference_indices": [0], "final_dimension_ids": [],
                "accepted_scope": None, "rejected_scope": None}]
            c.write_json(final_path, final)
            c.validate_provisional_review(root, head, slot, final_path)
            c.record_provisional_review(root, head, slot, final_path)
            self.assertEqual(c.load_campaign(root)["provisional_count"], 1)
            with self.assertRaises(c.CampaignError):
                c.reveal_qualification_profile(root, head)
            fixed_final = c.blind_stage_path(root, "provisional", slot)
            original_final = fixed_final.read_bytes()
            fixed_final.write_bytes(original_final + b" ")
            with self.assertRaises(c.CampaignError):
                c.verify_inventory(root)
            fixed_final.write_bytes(original_final)
            c.verify_inventory(root)
            symlink = root / "reviewer/critique-drafts" / ("f" * 32 + ".json")
            symlink.symlink_to(parent / "outside.json")
            with self.assertRaises(c.CampaignError):
                c.blind_stage_path(root, "critique-drafts", "f" * 32)


def check_blind_protocol_regressions():
    suite = unittest.TestSuite([
        unittest.defaultTestLoader.loadTestsFromTestCase(BlindProtocolTests),
        unittest.defaultTestLoader.loadTestsFromTestCase(CampaignBlindStageTests),
    ])
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    if not result.wasSuccessful():
        raise AssertionError("blind completeness protocol regressions failed")


if __name__ == "__main__":
    check_blind_protocol_regressions()
