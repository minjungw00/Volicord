"""Collector/publication/reviewer integration over a real disposable Git target."""
import copy
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import campaign as c
import campaign_self_test as fixtures
import harness
import repository_state
import review_operations


class EvidenceControlTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.parent = Path(self.temporary.name)
        self.seed = self.parent / "seed"
        self.seed.mkdir()
        self.git(self.seed, "init", "-q")
        self.git(self.seed, "config", "user.name", "Fixture")
        self.git(self.seed, "config", "user.email", "fixture@example.invalid")
        (self.seed / "tracked.txt").write_bytes(b"baseline\n")
        (self.seed / ".gitignore").write_bytes(b".codex/\n")
        self.git(self.seed, "add", ".")
        self.git(self.seed, "commit", "-qm", "baseline")
        baseline = self.git(self.seed, "rev-parse", "HEAD").strip().decode()
        clean = patch.object(harness, "git_clean", return_value=True)
        clean.start()
        self.addCleanup(clean.stop)
        revision = patch.object(fixtures, "REVISION", baseline)
        revision.start()
        self.addCleanup(revision.stop)
        binary = self.parent / "candidate/bin/volicord"
        fixtures.write_fake_binary(binary)
        self.root, self.captures, self.bundles = fixtures.prepared_batch(self.parent, "campaign", binary)
        self.repository = Path(c.load_campaign(self.root)["journeys"]["journey-small-python"]["repository_path"])
        shutil.copytree(self.seed, self.repository, dirs_exist_ok=True)
        self.baseline = baseline

    def git(self, repository, *args):
        return subprocess.run(["git", *args], cwd=repository, check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout

    def collect(self):
        return c.collect_batch(self.root, self.captures, exporter=fixtures.batch_exporter(self.bundles),
            documenter=fixtures.documenter, snapshotter=fixtures.snapshotter)

    def dirty(self):
        (self.repository / "tracked.txt").write_bytes(b"staged exact bytes\n")
        self.git(self.repository, "add", "tracked.txt")
        (self.repository / "tracked.txt").write_bytes(b"unstaged exact bytes\n")
        (self.repository / "untracked.txt").write_bytes(b"untracked exact bytes\n")

    def test_dirty_target_collection_publishes_exact_attestation_without_mutation(self):
        self.dirty()
        before, patches = repository_state.observe(self.repository)
        index = (self.repository / ".git/index").read_bytes()
        result = self.collect()
        self.assertEqual(result["collection_state"], "collected")
        manifest = c.load_evidence_set(self.root)
        final = next(item for item in manifest["journey_final_evidence"] if item["journey_id"] == "journey-small-python")
        lineage = final["repository_revision_lineage"]
        self.assertEqual(lineage["repository_state"], before)
        self.assertEqual(lineage["final_revision"], self.baseline)
        self.assertFalse(lineage["workspace_clean"])
        self.assertEqual(repository_state.observe(self.repository), (before, patches))
        self.assertEqual((self.repository / ".git/index").read_bytes(), index)
        for key, content in patches.items():
            self.assertEqual((self.root / lineage["attestation_artifacts"][key]).read_bytes(), content)
        # The portable, bounded observation is citable but makes no actor/verification claim.
        files, evidence_index, *_ = review_operations.select_evidence(self.root, manifest, None, include_raw=False)
        evidence = evidence_index["evidence"]["journey-small-python-repository-state"]
        self.assertEqual(c.read_json(self.root / lineage["attestation_artifacts"]["state"]), before)
        self.assertIn(b'"workspace_clean": false', files[evidence["path"]])
        git_surface = evidence_index["evidence"]["journey-small-python-git-observations"]
        git_evidence = json.loads(files[git_surface["path"]])["observation"]
        self.assertFalse(git_evidence["workspace_clean"])
        self.assertEqual(git_evidence["commits"]["commits"], [])
        self.assertEqual(len(git_evidence["session_git_observations"]), 2)
        self.assertFalse(any(item["surface"] in {"work_capture", "resume_capture"}
                             for item in evidence_index["evidence"].values()))
        git_path = self.root / lineage["attestation_artifacts"]["git_observations"]
        original_git = git_path.read_bytes()
        try:
            damaged = json.loads(original_git)
            damaged["observation"]["workspace_clean"] = True
            c.write_json(git_path, damaged)
            with self.assertRaisesRegex(c.CampaignError, "Git observation binding changed"):
                c.verify_retained_repository_states(self.root, manifest)
            with self.assertRaises(c.CampaignError):
                c.load_evidence_set(self.root)
        finally:
            git_path.write_bytes(original_git)
        (self.repository / "tracked.txt").write_bytes(b"later ordinary task")
        # Historical inspection does not bind mutable current work to old evidence.
        self.assertEqual(c.load_evidence_set(self.root), manifest)

    def test_naturally_committed_descendant_is_collected(self):
        (self.repository / "tracked.txt").write_bytes(b"committed task\n")
        self.git(self.repository, "commit", "-qam", "task")
        result = self.collect()
        final = next(item for item in result["journey_final_evidence"] if item["journey_id"] == "journey-small-python")
        self.assertNotEqual(final["repository_revision_lineage"]["final_revision"], self.baseline)
        self.assertTrue(final["repository_revision_lineage"]["workspace_clean"])

    def test_multi_commit_work_retains_history_without_terminal_check(self):
        for content in (b"first\n", b"second\n"):
            (self.repository / "tracked.txt").write_bytes(content)
            self.git(self.repository, "commit", "-qam", "ordinary task")
        result = self.collect()
        final = next(item for item in result["journey_final_evidence"]
                     if item["journey_id"] == "journey-small-python")
        history = final["repository_revision_lineage"]["commits"]
        self.assertEqual(len(history["commits"]), 2)
        self.assertTrue(all(item["paths"] == ["tracked.txt"] for item in history["commits"]))

    def test_absent_session_git_metadata_remains_unknown_in_collected_evidence(self):
        for path in self.captures:
            events = [json.loads(line) for line in path.read_text().splitlines()]
            if events[0]["payload"]["cwd"] == str(self.repository):
                events[0]["payload"].pop("git", None)
                path.write_text("".join(json.dumps(event) + "\n" for event in events))
        result = self.collect()
        final = next(item for item in result["journey_final_evidence"]
                     if item["journey_id"] == "journey-small-python")
        for item in final["repository_revision_lineage"]["session_git_observations"]:
            self.assertIsNone(item["start_head"])
            self.assertEqual(item["path_correlation_to_final"]["state"], "not_computable")

    def assert_distinct_works_git_collection(self, *, combined_before_collection):
        # Keep the exact Product candidate as this journey's pinned baseline.
        repository = Path(c.load_campaign(self.root)["journeys"]["journey-volicord"]["repository_path"])
        clone = self.parent / "volicord-clone"
        self.git(c.ROOT, "clone", "--quiet", "--shared", "--no-checkout", str(c.ROOT), str(clone))
        shutil.move(str(clone / ".git"), repository / ".git")
        self.git(repository, "reset", "--hard", "HEAD")
        self.git(repository, "config", "user.name", "Fixture")
        self.git(repository, "config", "user.email", "fixture@example.invalid")
        with (repository / ".git/info/exclude").open("a") as stream:
            stream.write("\n/.codex/\n")
        baseline = self.git(repository, "rev-parse", "HEAD").strip().decode()
        for path in self.captures:
            events = [json.loads(line) for line in path.read_text().splitlines()]
            if events[0]["payload"]["cwd"] != str(repository):
                continue
            events[0]["payload"]["git"]["commit_hash"] = baseline
            # No synthetic commit or status commands: actual Git observations
            # are collected, and the captured canonical identities stay intact.
            events = [event for event in events if not event.get("payload", {}).get("call_id", "").startswith(("commit-", "boundary-"))]
            path.write_text("".join(json.dumps(event) + "\n" for event in events))
        (repository / "work-changes.txt").write_bytes(b"several Works before any commit\n")
        mapped = c.map_batch_rollouts(self.root, self.captures)
        dirty = c.collect_journey_git_evidence(c.load_campaign(self.root), mapped)["journey-volicord"]
        self.assertFalse(dirty["workspace_clean"])
        self.assertEqual(dirty["commits"]["commits"], [])
        # Publish once while dirty. The earlier observation remains usable after
        # a later combined commit; no Work is created or merged by that commit.
        if combined_before_collection:
            self.git(repository, "add", "work-changes.txt")
            self.git(repository, "commit", "-qm", "combined changes")
        self.collect()
        manifest = c.load_evidence_set(self.root)
        final = next(item for item in manifest["journey_final_evidence"] if item["journey_id"] == "journey-volicord")
        self.assertEqual(final["repository_revision_lineage"]["workspace_clean"], combined_before_collection)
        self.assertEqual(len(final["repository_revision_lineage"]["commits"]["commits"]), int(combined_before_collection))
        works = [item for item in manifest["work_evidence"] if item["repository_class"] == "volicord"]
        self.assertEqual(len({item["work_item_id"] for item in works}), 3)
        if not combined_before_collection:
            self.git(repository, "add", "work-changes.txt")
            self.git(repository, "commit", "-qm", "combined changes")
        later = c.collect_journey_git_evidence(c.load_campaign(self.root), mapped)["journey-volicord"]
        self.assertEqual(len(later["commits"]["commits"]), 1)
        self.assertTrue(later["workspace_clean"])
        self.assertEqual(c.load_evidence_set(self.root), manifest)
        evaluated = c.evaluate_campaign(self.root)
        evaluation = c.read_json(self.root / evaluated["evaluation"])
        git_findings = [finding for journey in evaluation["journeys"] for finding in journey["findings"]
                        if finding["check"] == "git_history_observation"]
        self.assertTrue(all(finding["disposition"] == "advisory" for finding in git_findings))

    def test_distinct_works_at_dirty_zero_commit_head_are_collected(self):
        self.assert_distinct_works_git_collection(combined_before_collection=False)

    def test_distinct_works_before_later_combined_commit_are_collected(self):
        self.assert_distinct_works_git_collection(combined_before_collection=True)

    def snapshot_campaign(self):
        return {p.relative_to(self.root).as_posix(): p.read_bytes() for p in self.root.rglob("*")
            if p.is_file() and not p.is_relative_to(self.repository)}

    def test_tracked_or_untracked_mutation_before_publication_is_atomic_rejection(self):
        self.dirty()
        publish = c.publish_batch
        for name in ("tracked.txt", "untracked.txt"):
            original = (self.repository / name).read_bytes()
            before = self.snapshot_campaign()
            def tampered_publish(root, stage, baseline):
                (self.repository / name).write_bytes(b"changed after capture")
                return publish(root, stage, baseline)
            with patch.object(c, "publish_batch", side_effect=tampered_publish):
                with self.assertRaisesRegex(c.IntegrityError, "changed before publication"):
                    self.collect()
            self.assertEqual(self.snapshot_campaign(), before)
            self.assertFalse((self.root / "evidence-set.json").exists())
            self.assertFalse((self.root / "batch-publication.json").exists())
            (self.repository / name).write_bytes(original)

    def test_non_descendant_target_rejected_before_any_publication(self):
        before = self.snapshot_campaign()
        self.git(self.repository, "checkout", "--orphan", "unrelated")
        self.git(self.repository, "commit", "-qm", "unrelated root")
        with self.assertRaisesRegex(c.IntegrityError, "descend"):
            self.collect()
        self.assertEqual(self.snapshot_campaign(), before)


def check_evidence_control_regressions():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(EvidenceControlTests))
    if not result.wasSuccessful():
        raise AssertionError("evidence control integration regressions failed")


if __name__ == "__main__":
    unittest.main()
