"""Deterministic real-Git collection regressions, never naturalistic evidence."""
import copy
from pathlib import Path
from types import SimpleNamespace
import subprocess
import tempfile
import unittest

import campaign
import harness
import repository_state as state


class RepositoryStateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.repo = self.root / "repository"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        (self.repo / "tracked.txt").write_bytes(b"baseline\n")
        (self.repo / ".gitignore").write_bytes(b"ignored\n")
        self.git("add", ".")
        self.git("commit", "-qm", "baseline")
        self.baseline = self.git("rev-parse", "HEAD").strip().decode()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, check=True,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout

    def mapping(self):
        journeys, mapped = {}, {}
        for kind in harness.CLASSES:
            journeys[campaign.journey_id(kind)] = {"repository_path": str(self.repo),
                "repository_revision": self.baseline}
        for slot in harness.current_session_slots():
            mapped[slot] = SimpleNamespace(capture=SimpleNamespace(git_revision=self.baseline))
        return {"journeys": journeys}, mapped

    def test_dirty_exact_bytes_determinism_and_no_git_mutation(self):
        (self.repo / "tracked.txt").write_bytes(b"staged\x00bytes\n")
        self.git("add", "tracked.txt")
        (self.repo / "tracked.txt").write_bytes(b"unstaged\x00bytes\n")
        (self.repo / "untracked space.txt").write_bytes(b"untracked\xff\n")
        index_before = (self.repo / ".git/index").read_bytes()
        status_before = self.git("status", "--porcelain=v1", "-z")
        captured, patches = state.observe(self.repo)
        self.assertEqual(state.observe(self.repo), (captured, patches))
        self.assertEqual(captured["tracked"][-1]["sha256"], state.digest(b"unstaged\x00bytes\n"))
        self.assertEqual(captured["untracked"][0]["sha256"], state.digest(b"untracked\xff\n"))
        self.assertEqual(captured["status"][-2], {"path": "tracked.txt", "index": "M", "worktree": "M"})
        state.verify_retained(captured, patches)
        c, mapped = self.mapping()
        result = campaign.verify_journey_revision_chronology(c, mapped)
        self.assertEqual(result["journey-volicord"]["repository_state"], captured)
        self.assertEqual(self.git("rev-parse", "HEAD").strip().decode(), self.baseline)
        self.assertEqual((self.repo / ".git/index").read_bytes(), index_before)
        self.assertEqual(self.git("status", "--porcelain=v1", "-z"), status_before)

    def test_clean_and_naturally_committed_descendant(self):
        c, mapped = self.mapping()
        self.assertTrue(campaign.verify_journey_revision_chronology(c, mapped)["journey-volicord"]["workspace_clean"])
        (self.repo / "tracked.txt").write_bytes(b"committed task\n")
        self.git("commit", "-qam", "ordinary task")
        result = campaign.verify_journey_revision_chronology(c, mapped)
        self.assertNotEqual(result["journey-volicord"]["final_revision"], self.baseline)
        self.assertTrue(result["journey-volicord"]["workspace_clean"])

    def test_non_descendant_and_reversed_session_history(self):
        c, mapped = self.mapping()
        self.git("checkout", "--orphan", "unrelated")
        self.git("commit", "-qm", "unrelated root")
        with self.assertRaisesRegex(campaign.IntegrityError, "descend"):
            campaign.verify_journey_revision_chronology(c, mapped)
        self.git("checkout", "-q", self.baseline)
        (self.repo / "tracked.txt").write_bytes(b"later\n")
        self.git("commit", "-qam", "later")
        later = self.git("rev-parse", "HEAD").strip().decode()
        slots = [slot for slot in mapped if slot[0] == "volicord"]
        mapped[slots[0]].capture.git_revision = later
        with self.assertRaisesRegex(campaign.IntegrityError, "chronological"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_tracked_and_untracked_tamper_before_publication(self):
        (self.repo / "untracked").write_bytes(b"one")
        captured, patches = state.observe(self.repo)
        for name in ("untracked", "tracked.txt"):
            original = (self.repo / name).read_bytes()
            (self.repo / name).write_bytes(b"tampered")
            with self.assertRaisesRegex(state.StateError, "changed before publication"):
                state.verify(self.repo, captured)
            (self.repo / name).write_bytes(original)
        damaged = copy.deepcopy(captured)
        damaged["untracked"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(state.StateError, "fingerprint"):
            state.verify_retained(damaged, patches)
        with self.assertRaisesRegex(state.StateError, "diff"):
            state.verify_retained(captured, {**patches, "unstaged": b"altered"})

    def test_path_escape_wrong_root_and_missing_file_fail(self):
        (self.repo / "escape").symlink_to(self.root / "outside")
        with self.assertRaisesRegex(state.StateError, "escape"):
            state.observe(self.repo)
        (self.repo / "escape").unlink()
        (self.repo / "nested").mkdir()
        with self.assertRaisesRegex(state.StateError, "wrong repository identity"):
            state.observe(self.repo / "nested")
        with self.assertRaisesRegex(state.StateError, "missing"):
            state.file_state(self.repo, "absent")


def check_repository_state_regressions():
    result = unittest.TextTestRunner(verbosity=1).run(unittest.defaultTestLoader.loadTestsFromTestCase(RepositoryStateTests))
    if not result.wasSuccessful():
        raise AssertionError("repository-state regressions failed")


if __name__ == "__main__":
    unittest.main()
