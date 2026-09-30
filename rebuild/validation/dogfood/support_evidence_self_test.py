"""Synthetic current producer/consumer regressions; no Naturalistic replay."""
from copy import deepcopy
from dataclasses import replace
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import harness as h
import machine_findings as m


class SupportEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        parent = Path(cls.temp.name)
        binary = parent / "bin/volicord"
        fixtures.write_fake_binary(binary)
        with patch.object(h, "git_clean", return_value=True):
            cls.root, captures, bundles = fixtures.prepared_batch(parent, "support", binary)
            c.collect_batch(cls.root, captures, exporter=fixtures.batch_exporter(bundles),
                documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)
        cls.manifest = c.load_evidence_set(cls.root)

    @classmethod
    def tearDownClass(cls):
        cls.temp.cleanup()

    def inputs(self, kind="volicord", work="A"):
        state = self.manifest["works"][h.work_slot_id(kind, work)]
        d = c.read_json(self.root / "tasks/descriptors" / f"{state['work_slot_id']}.json")
        context = {"candidate_head": self.manifest["candidate_head"],
            "journey": deepcopy(self.manifest["journeys"][h.journey_id(kind)]),
            "final_entries": deepcopy(self.manifest["journey_final_evidence"]),
            "work_entries": deepcopy(self.manifest["work_evidence"])}
        bundle = h.load_canonical_bundle(self.root / d["evidence"]["canonical_bundle"]["file"])
        return d["evidence"], context, bundle

    def evaluate(self, kind="volicord", work="A", mutate=None, json_mutate=None):
        evidence, context, bundle = self.inputs(kind, work)
        if mutate:
            mutate(evidence, context)
        original = h.verified_json_evidence
        def changed(reference, directory):
            path, value = original(reference, directory)
            if json_mutate and value is not None:
                value = deepcopy(value)
                json_mutate(value)
            return path, value
        with patch.object(h, "verified_json_evidence", side_effect=changed):
            return h.campaign_support_evidence(evidence, self.root, bundle,
                project_id=bundle.project_id, candidate_revision=self.manifest["candidate_head"],
                kind=kind, work_label=work, journey_context=context)

    def test_five_works_share_three_valid_journey_projections(self):
        files = set()
        for kind, work in h.current_work_slots():
            checks, basis = self.evaluate(kind, work)
            self.assertTrue(all(checks.values()), (kind, work, checks))
            self.assertEqual(basis["projection_evidence_identity"], "confirmed_pass")
            files.add(self.inputs(kind, work)[0]["viewer_snapshot"]["file"])
        self.assertEqual(len(files), 3)
        self.assertEqual({self.inputs("volicord", w)[0]["canonical_bundle"]["file"]
                          for w in "ABC"}, {self.inputs()[0]["canonical_bundle"]["file"]})

    def test_activation_conflicts_and_malformed_evidence_are_hard(self):
        cases = [("start_session_start_activation_observed", None),
                 ("resume_session_start_activation_observed", None),
                 ("kind", "wrong-kind"), ("journey_id", "journey-small-python"),
                 ("work_slot_id", "journey-volicord-work-b"),
                 ("repository_class", "small-python"),
                 ("repository_config_present", False),
                 ("repository_ownership_manifest_present", False)]
        for field, value in cases:
            with self.subTest(field=field):
                def mutate(summary):
                    if summary.get("kind") == "phase8_dogfood_activation_summary":
                        summary[field] = value
                checks, _ = self.evaluate(json_mutate=mutate)
                self.assertFalse(checks["bounded_runtime_and_activation_evidence"])
        for work in "BC":
            checks, _ = self.evaluate(work=work, json_mutate=lambda s: s.update(
                resume_session_start_activation_observed=True) if s.get("kind") ==
                "phase8_dogfood_activation_summary" else None)
            self.assertFalse(checks["bounded_runtime_and_activation_evidence"])
            self.assertEqual(h.session_roles("volicord", work), ("start",))
        for key in ("activation_summary", "runtime_summary"):
            checks, _ = self.evaluate(mutate=lambda e, _: e[key].update(sha256="0" * 64))
            self.assertFalse(checks["bounded_runtime_and_activation_evidence"])
        for field, value in (("content_included", True), ("runtime_home_bytes", -1),
            ("derived_analysis_bytes", "128"), ("managed_file_inventory", [{}])):
            checks, _ = self.evaluate(json_mutate=lambda s: s.update({field:value})
                if s.get("kind") == "phase8_bounded_runtime_summary" else None)
            self.assertFalse(checks["bounded_runtime_and_activation_evidence"])
        self.assertEqual(m.disposition("bounded_runtime_and_activation_evidence",
            "confirmed_violation"), m.Disposition.HARD)

    def test_malformed_json_and_missing_canonical_history_are_hard(self):
        path = self.root / "malformed-summary.json"
        path.write_text("{invalid")
        try:
            checks, _ = self.evaluate(mutate=lambda e, _: e.update(
                activation_summary={"file": path.name, "sha256": h.sha256(path)}))
            self.assertFalse(checks["bounded_runtime_and_activation_evidence"])
        finally:
            path.unlink()
        evidence, context, bundle = self.inputs()
        for table in ("context_items", "checkpoints"):
            tables = deepcopy(bundle.tables)
            tables[table] = ()
            checks, basis = h.campaign_support_evidence(evidence, self.root,
                replace(bundle, tables=tables), project_id=bundle.project_id,
                candidate_revision=self.manifest["candidate_head"], kind="volicord",
                work_label="A", journey_context=context)
            self.assertEqual(basis["projection_evidence_identity"], "confirmed_violation")
        for field, value in (("sha256", "0" * 64),
                             ("relative_evidence_path", "../escape"), ("bytes", -1)):
            def mutate(summary):
                if summary.get("kind") == "phase8_generated_document_evidence_summary":
                    summary["documents"]["decision-report"]["formats"]["html"][field] = value
            _, basis = self.evaluate(json_mutate=mutate)
            self.assertEqual(basis["projection_evidence_identity"], "confirmed_violation")

    def test_projection_conflicts_are_hard_including_sibling_representation(self):
        def final(context):
            return next(e for e in context["final_entries"] if e["journey_id"] == "journey-volicord")
        cases = [lambda e,cx: final(cx)["represented_work_slot_ids"].remove("journey-volicord-work-b"),
            lambda e,cx: final(cx).update(project_id="wrong"),
            lambda e,cx: cx.update(candidate_head="f" * 40),
            lambda e,cx: final(cx).update(projection_source_work_slot_id="journey-volicord-work-b"),
            lambda e,cx: final(cx).update(journey_id="journey-small-python"),
            lambda e,cx: final(cx)["ordered_work_item_ids"].reverse(),
            lambda e,cx: cx["work_entries"][0].update(repository_class="small-python"),
            lambda e,cx: e["generated_documents"].update(sha256="0" * 64),
            lambda e,cx: e["viewer_snapshot"].update(file="../escape"),
            lambda e,cx: e.update(viewer_snapshot=cx["final_entries"][1]["artifact_inventory"]["viewer_snapshot"])]
        for mutate in cases:
            _, basis = self.evaluate(work="B", mutate=mutate)
            self.assertEqual(basis["projection_evidence_identity"], "confirmed_violation")
        for field,value in (("project_id","wrong"),("candidate_head","f"*40),
            ("repository_class","small-python"),("work","B"),("sha256","0"*64),
            ("relative_evidence_path","../escape"),("bytes",-1)):
            def mutate(s):
                if s.get("kind") == "phase8_viewer_snapshot_evidence_summary": s[field] = value
            _, basis = self.evaluate(json_mutate=mutate)
            self.assertEqual(basis["projection_evidence_identity"], "confirmed_violation")
        self.assertEqual(m.disposition("projection_evidence_identity",
            "confirmed_violation"), m.Disposition.HARD)


if __name__ == "__main__":
    unittest.main()
