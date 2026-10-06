"""Authored changed-surface and exact-candidate controls, never human judgments."""
import copy
import json
import unittest

import human_observation_plan as p
import viewer_observation
from viewer_observation_self_test import display_fixture


class ChangedSurfaceTests(unittest.TestCase):
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

    def test_changed_candidate_requires_new_context_and_no_screen_fixes_missing_execution(self):
        manifest = {"candidate_head": "f" * 40,
            "candidate_artifacts": {"volicord-viewer": {"sha256": "a" * 64}},
            "journeys": {"journey-volicord": {"runtime_home": "/synthetic/runtime", "project_id": "a" * 32}}}
        old = display_fixture(manifest, "en")
        old["candidate_head"] = self.inventory()["diagnostic_candidate"]
        with self.assertRaisesRegex(ValueError, "capture binding"):
            viewer_observation.for_manifest(manifest, old, "en")
        current = display_fixture(manifest, "en")
        viewer_observation.for_manifest(manifest, current, "en")
        # A current browser receipt has no execution, authority or human verdict field.
        self.assertFalse({"authority", "execution_evidence", "assessment"} & set(current))

    def test_required_contexts_precede_questions_and_plan_cannot_drop_a_surface(self):
        inventory = self.inventory()
        candidate = p.git("rev-parse", "HEAD")
        changed = p.git("diff", "--name-only", inventory["diagnostic_candidate"], candidate).splitlines()
        value = p.plan(candidate, {"volicord-viewer": "a" * 64}, changed, inventory)
        value["inputs"] = {str(path.relative_to(p.ROOT)): p.digest(path)
            for path in (p.Path(p.__file__), p.FIXTURE, p.ROOT / "rebuild/docs/design/qualitative-review.md")}
        manifest = {"candidate_head": candidate, "candidate_artifacts": {"volicord-viewer": {"sha256": "a" * 64}},
            "journeys": {"journey-volicord": {"runtime_home": "/synthetic/runtime", "project_id": "a" * 32}}}
        contexts = {locale: [display_fixture(manifest, locale)] for locale in ("en", "ko")}
        with self.assertRaisesRegex(ValueError, "before asking"):
            p.require_contexts(value, contexts, candidate, "a" * 64)
        for locale in contexts:
            for view in ({"view": "work", "work": "a" * 32}, {"view": "work", "work": "b" * 32},
                         {"view": "decisions"}, {"view": "code"}, {"view": "tools", "tool": "status"}):
                display = display_fixture(manifest, locale)
                display["context"]["view"] = view
                if view["view"] == "work":
                    display["context"]["selected_work"] = view["work"]
                if view["view"] == "decisions":
                    display["context"]["selected_decision"] = "d" * 32
                contexts[locale].append(display)
        p.require_contexts(value, contexts, candidate, "a" * 64)
        tampered = copy.deepcopy(value)
        tampered["observation_blocks"].pop()
        with self.assertRaisesRegex(ValueError, "altered"):
            p.require_contexts(tampered, contexts, candidate, "a" * 64)


def run_contract_tests():
    result = unittest.TextTestRunner().run(unittest.defaultTestLoader.loadTestsFromTestCase(ChangedSurfaceTests))
    if not result.wasSuccessful():
        raise AssertionError("changed-surface Human preparation regressions failed")


if __name__ == "__main__":
    unittest.main()
