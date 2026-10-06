"""Authored changed-surface and exact-candidate controls, never human judgments."""
import copy
import json
import unittest

import human_observation_plan as p
import viewer_observation
from viewer_observation_self_test import display_fixture, manifest_fixture, subject_fixture


class ChangedSurfaceTests(unittest.TestCase):
    def test_multi_work_requires_two_distinct_campaign_works(self):
        manifest = manifest_fixture()
        subjects = subject_fixture(manifest)
        contexts = {}
        for locale in ("en", "ko"):
            work = display_fixture(manifest, locale)
            work["context"].update(view={"view": "work"}, selected_work="1" * 32)
            other_render = copy.deepcopy(work)
            other_render["context"].update(render_id="4" * 32, canonical_read_fingerprint="5" * 64)
            contexts[locale] = [work, other_render]
        blocks = self.inventory()["observation_blocks"]
        readiness = p.block_readiness(blocks, contexts, subjects)
        for locale in contexts:
            self.assertEqual(next(b["state"] for b in readiness[locale] if b["id"] == "multi-work"), "insufficient_evidence")
            contexts[locale][1]["context"]["selected_work"] = "2" * 32
        readiness = p.block_readiness(blocks, contexts, subjects)
        for locale in contexts:
            self.assertEqual(next(b["state"] for b in readiness[locale] if b["id"] == "multi-work"), "ready")
        contexts["en"][1]["context"]["selected_work"] = "b" * 32
        with self.assertRaisesRegex(ValueError, "outside campaign subjects"):
            p.block_readiness(blocks, contexts, subjects)
        with self.assertRaisesRegex(ValueError, "outside campaign subjects"):
            p.prepared_claims({"subjects": subjects, "readiness": readiness}, "en", contexts["en"])

    def inventory(self):
        return json.loads(p.FIXTURE.read_bytes())

    def test_bounded_actual_changes_do_not_reuse_26_answers_or_invalidate_setup(self):
        inventory = self.inventory()
        paths = {path for b in inventory["observation_blocks"] for path in b["affected_paths"]}
        for path in paths:
            self.assertTrue((p.ROOT / path).is_file(), path)
        value = p.plan("f" * 40, {"volicord-viewer": "a" * 64}, paths, inventory)
        self.assertEqual(len(value["observation_blocks"]), 8)
        self.assertEqual({b["id"] for b in value["observation_blocks"]}, {
            "overview", "multi-work", "work", "decision", "code-analysis",
            "color-grouping", "keyboard-presentation", "input-paint"})
        for block in value["observation_blocks"]:
            self.assertEqual(block["locales"], ["en", "ko"])
            self.assertEqual(block["human_evidence"], "unobserved")
            self.assertFalse({"answer", "verdict", "assessment"} & set(block))
        self.assertFalse(value["question_count_available"])
        self.assertFalse(value["phase_9_ready"])
        self.assertIn("setup/resource", " ".join(value["unaffected_historical_claims"]))
        self.assertIn("Actual execution", " ".join(value["separate_evidence"]))
        narrow = p.plan("f" * 40, {}, ["rebuild/crates/volicord-projections/src/understanding.rs"], inventory)
        self.assertEqual([b["id"] for b in narrow["observation_blocks"]], ["code-analysis"])
        with self.assertRaises(ValueError):
            p.plan(inventory["diagnostic_candidate"], {}, paths, inventory)

    def test_every_direct_claim_has_a_live_formal_destination(self):
        import harness
        import qualitative_review
        rubric = qualitative_review.rubric(harness.load_definition())
        claims = {c for b in self.inventory()["observation_blocks"] for c in b["claims"]}
        self.assertEqual(claims, set(rubric["criteria"]["live_viewer"]))
        for claim in claims:
            self.assertEqual(qualitative_review.required_surfaces({"name": claim, "group": "live_viewer"}),
                ["live_viewer_observation"])
        for name in ("completed_current_remaining_work", "next_step", "code_behavior",
                     "architecture_components_flow", "information_hierarchy_and_cognitive_burden"):
            self.assertIn(name, rubric["criteria"]["viewer_snapshot"])
            self.assertEqual(qualitative_review.required_surfaces({"name": name, "group": "viewer_snapshot"}),
                ["viewer_snapshot"])

    def test_missing_optional_code_context_leaves_overview_and_work_ready(self):
        inventory = self.inventory()
        manifest = manifest_fixture("f" * 40)
        subjects = subject_fixture(manifest)
        contexts = {}
        for locale in ("en", "ko"):
            overview = display_fixture(manifest, locale)
            work = copy.deepcopy(overview)
            work["context"].update(view={"view": "work"}, selected_work="1"*32)
            contexts[locale] = [overview, work]
        ready = p.block_readiness(inventory["observation_blocks"], contexts, subjects)
        for locale in contexts:
            states = {b["id"]: b["state"] for b in ready[locale]}
            self.assertEqual(states["overview"], "ready")
            self.assertEqual(states["work"], "ready")
            self.assertEqual(states["code-analysis"], "insufficient_evidence")
            self.assertEqual(states["multi-work"], "insufficient_evidence")
        for locale in contexts:
            decisions = copy.deepcopy(contexts[locale][0])
            decisions["context"].update(view={"view": "decisions"})
            contexts[locale].append(decisions)
        ready = p.block_readiness(inventory["observation_blocks"], contexts, subjects)
        self.assertEqual(next(b["state"] for b in ready["en"] if b["id"] == "color-grouping"), "ready")
        self.assertIn("not_color_only", p.prepared_claims({"readiness": ready, "subjects": subjects}, "en", contexts["en"]))
        self.assertNotIn("multiple_work_comprehension", p.prepared_claims({"readiness": ready, "subjects": subjects}, "en", contexts["en"]))
        with self.assertRaisesRegex(ValueError, "prepared block"):
            p.require_claim_context({"readiness": ready, "subjects": subjects}, contexts, "en", "code_behavior_comprehension", "violated")

    def test_changed_candidate_requires_new_context_and_no_screen_fixes_missing_execution(self):
        manifest = manifest_fixture("f" * 40)
        subjects = subject_fixture(manifest)
        old = display_fixture(manifest, "en")
        old["candidate_head"] = self.inventory()["diagnostic_candidate"]
        with self.assertRaisesRegex(ValueError, "capture binding"):
            viewer_observation.for_manifest(manifest, old, "en", subjects)
        current = display_fixture(manifest, "en")
        viewer_observation.for_manifest(manifest, current, "en", subjects)
        # A current browser receipt has no execution, authority or human verdict field.
        self.assertFalse({"authority", "execution_evidence", "assessment"} & set(current))

    def test_required_contexts_precede_questions_and_plan_cannot_drop_a_surface(self):
        inventory = self.inventory()
        candidate = p.git("rev-parse", "HEAD")
        changed = p.git("diff", "--name-only", inventory["diagnostic_candidate"], candidate).splitlines()
        value = p.plan(candidate, {"volicord-viewer": "a" * 64}, changed, inventory)
        value["inputs"] = {str(path.relative_to(p.ROOT)): p.digest(path)
            for path in (p.Path(p.__file__), p.FIXTURE, p.ROOT / "rebuild/docs/design/qualitative-review.md")}
        manifest = manifest_fixture(candidate)
        subjects = subject_fixture(manifest)
        contexts = {locale: [display_fixture(manifest, locale)] for locale in ("en", "ko")}
        readiness = p.require_contexts(value, contexts, candidate, "a" * 64, subjects)
        self.assertEqual(next(b["state"] for b in readiness["en"] if b["id"] == "overview"), "ready")
        self.assertEqual(next(b["state"] for b in readiness["en"] if b["id"] == "decision"), "insufficient_evidence")
        for locale in contexts:
            for view in ({"view": "work", "work": "1" * 32}, {"view": "work", "work": "2" * 32},
                         {"view": "decisions"}, {"view": "code"}, {"view": "tools", "tool": "status"}):
                display = display_fixture(manifest, locale)
                display["context"]["view"] = view
                if view["view"] == "work":
                    display["context"]["selected_work"] = view["work"]
                if view["view"] == "decisions":
                    display["context"]["selected_decision"] = "3" * 32
                contexts[locale].append(display)
        p.require_contexts(value, contexts, candidate, "a" * 64, subjects)
        tampered = copy.deepcopy(value)
        tampered["observation_blocks"].pop()
        with self.assertRaisesRegex(ValueError, "altered"):
            p.require_contexts(tampered, contexts, candidate, "a" * 64, subjects)


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ChangedSurfaceTests))
    if not result.wasSuccessful():
        raise AssertionError("changed-surface Human preparation regressions failed")


if __name__ == "__main__":
    unittest.main()
