"""Deterministic real-Git collection regressions, never naturalistic evidence."""
import copy
from dataclasses import replace
import json
import os
import socket
from pathlib import Path
from types import SimpleNamespace
import subprocess
import tempfile
import unittest
from unittest import mock

import campaign
import harness
import repository_state as state
from codex_events import CommandObservation, command_role, normalized_file_changes


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
            mapped[slot] = SimpleNamespace(capture=SimpleNamespace(git_revision=self.baseline,
                path_observations=(), commands=(), cwd=self.repo, session_id=str(slot),
                source_sha256=state.digest(str(slot).encode())))
        return {"journeys": journeys}, mapped

    def worktree_bytes(self):
        return {str(path.relative_to(self.repo)): path.read_bytes()
            for path in self.repo.rglob("*") if path.is_file() and ".git" not in path.relative_to(self.repo).parts}

    def assert_replacement(self, historical, replacements, *, staged, excluded=()):
        head_before = self.git("rev-parse", "HEAD")
        index_before = (self.repo / ".git/index").read_bytes()
        status_before = self.git("status", "--porcelain=v1", "-z", "--untracked-files=all")
        files_before = self.worktree_bytes()
        read_bytes = Path.read_bytes
        def boundary_read(path):
            self.assertNotIn(path.relative_to(self.repo).as_posix(), excluded,
                "ignored replacement bytes must not be read by attestation")
            return read_bytes(path)
        with mock.patch.object(Path, "read_bytes", autospec=True, side_effect=boundary_read):
            captured, patches = state.observe(self.repo)
            state.verify(self.repo, captured)
        tracked = {item["path"]: item for item in captured["tracked"]}
        untracked = {item["path"]: item for item in captured["untracked"]}
        statuses = {item["path"]: item for item in captured["status"]}
        indexed = {item["path"] for item in captured["index"]}
        for name in historical:
            self.assertEqual(tracked[name], {"path": name, "state": "deleted"})
            self.assertEqual(statuses[name], {"path": name,
                "index": "D" if staged else " ", "worktree": " " if staged else "D"})
            self.assertEqual(name in indexed, not staged)
        for name, content in replacements.items():
            self.assertEqual((tracked if staged else untracked)[name],
                {"path": name, "state": "file", "bytes": len(content), "sha256": state.digest(content)})
            self.assertEqual(statuses[name], {"path": name,
                "index": "A" if staged else "?", "worktree": " " if staged else "?"})
            self.assertEqual(name in indexed, staged)
        for name in excluded:
            self.assertNotIn(name, untracked)
            self.assertNotIn(name, indexed)
            if name not in historical:
                self.assertNotIn(name, tracked)
                self.assertNotIn(name, statuses)
            self.assertNotIn(files_before[name], patches["staged"] + patches["unstaged"])
        self.assertFalse(captured["workspace_clean"])
        self.assertEqual(captured["schema_version"], 1)
        self.assertIn(b"deleted file mode 100644", patches["staged" if staged else "unstaged"])
        if staged:
            if replacements:
                self.assertIn(b"new file mode 100644", patches["staged"])
            self.assertEqual(patches["unstaged"], b"")
        else:
            self.assertEqual(patches["staged"], b"")
        self.assertEqual(state.observe(self.repo), (captured, patches))
        state.verify(self.repo, captured)
        retained = json.loads(state.encoded(captured))
        state.verify_retained(retained, patches)
        with self.assertRaisesRegex(campaign.IntegrityError, "uncommitted final"):
            c, mapped = self.mapping()
            campaign.verify_journey_revision_chronology(c, mapped)
        lineage = {"repository_state": captured, "final_revision": captured["final_head"]}
        evidence = self.root / "retained"
        evidence.mkdir()
        binding = {"state": "repository-state.json", "staged": "staged.patch", "unstaged": "unstaged.patch"}
        (evidence / binding["state"]).write_bytes(state.encoded(retained))
        for key, patch in patches.items():
            (evidence / binding[key]).write_bytes(patch)
        manifest = {"journeys": {"journey-volicord": {"repository_path": str(self.repo)}},
            "journey_final_evidence": [{"journey_id": "journey-volicord",
                "repository_revision_lineage": {**lineage, "attestation_artifacts": binding}}]}
        (evidence / "evidence-set.json").write_bytes(state.encoded(manifest))
        with self.assertRaisesRegex(campaign.CampaignError, "not clean"):
            campaign.verify_final_repository_states_for_publication(evidence)
        self.assertEqual(self.git("rev-parse", "HEAD"), head_before)
        self.assertEqual((self.repo / ".git/index").read_bytes(), index_before)
        self.assertEqual(self.git("status", "--porcelain=v1", "-z", "--untracked-files=all"), status_before)
        self.assertEqual(self.worktree_bytes(), files_before)
        for name in excluded:
            original = (self.repo / name).read_bytes()
            (self.repo / name).write_bytes(original + b"ignored change")
            self.assertEqual(state.observe(self.repo), (captured, patches))
            state.verify(self.repo, captured)
            with self.assertRaisesRegex(campaign.CampaignError, "not clean"):
                campaign.verify_final_repository_states_for_publication(evidence)
            (self.repo / name).write_bytes(original)
        name, original = next(iter(replacements.items()), (".gitignore", b"ignored\n"))
        (self.repo / name).write_bytes(original + b"tampered")
        with self.assertRaisesRegex(state.StateError, "changed before publication"):
            state.verify(self.repo, captured)
        with self.assertRaisesRegex(campaign.CampaignError, "not clean"):
            campaign.verify_final_repository_states_for_publication(evidence)
        # Historical verification remains independent of the mutated live target.
        state.verify_retained(retained, patches)
        with self.assertRaisesRegex(campaign.CampaignError, "not clean"):
            campaign.verify_retained_repository_states(evidence, manifest)
        (self.repo / name).write_bytes(original)
        damaged = copy.deepcopy(retained)
        next(item for item in damaged["tracked"] if item["path"] in historical)["state"] = "file"
        with self.assertRaisesRegex(state.StateError, "fingerprint"):
            state.verify_retained(damaged, patches)
        key = "staged" if staged else "unstaged"
        with self.assertRaisesRegex(state.StateError, "diff"):
            state.verify_retained(retained, {**patches, key: patches[key] + b"tampered"})

    def replace_file_with_directory(self, *, staged):
        (self.repo / "tracked.txt").unlink()
        (self.repo / "tracked.txt/nested").mkdir(parents=True)
        replacements = {"tracked.txt/first.bin": b"replacement\x00\xff\n",
            "tracked.txt/nested/second.txt": b"another replacement\n"}
        for name, content in replacements.items():
            (self.repo / name).write_bytes(content)
        if staged:
            self.git("add", "-A")
        self.assert_replacement(["tracked.txt"], replacements, staged=staged)

    def replace_directory_with_file(self, *, staged):
        (self.repo / "config/nested").mkdir(parents=True)
        historical = ["config/settings.json", "config/nested/other.json"]
        for name in historical:
            (self.repo / name).write_bytes(b'{"baseline": true}\n')
        self.git("add", "config")
        self.git("commit", "-qm", "tracked directory")
        for name in historical:
            (self.repo / name).unlink()
        (self.repo / "config/nested").rmdir()
        (self.repo / "config").rmdir()
        replacements = {"config": b"replacement\x00\xff\n"}
        (self.repo / "config").write_bytes(replacements["config"])
        if staged:
            self.git("add", "-A")
        self.assert_replacement(historical, replacements, staged=staged)

    def test_unstaged_file_to_directory(self):
        self.replace_file_with_directory(staged=False)

    def test_staged_file_to_directory(self):
        self.replace_file_with_directory(staged=True)

    def test_unstaged_directory_to_file(self):
        self.replace_directory_with_file(staged=False)

    def test_staged_directory_to_file(self):
        self.replace_directory_with_file(staged=True)

    def replace_file_with_excluded_directory(self, *, staged, ignored):
        (self.repo / "tracked.txt").unlink()
        (self.repo / "tracked.txt").mkdir()
        excluded = []
        if ignored:
            (self.repo / "tracked.txt/ignored/nested").mkdir(parents=True)
            excluded = ["tracked.txt/ignored/first.bin", "tracked.txt/ignored/nested/second.txt"]
            for name in excluded:
                (self.repo / name).write_bytes(b"ignored replacement\x00\xff\n")
        if staged:
            self.git("add", "-u")
        self.assert_replacement(["tracked.txt"], {}, staged=staged, excluded=excluded)

    def test_unstaged_file_to_empty_directory(self):
        self.replace_file_with_excluded_directory(staged=False, ignored=False)

    def test_staged_file_to_empty_directory(self):
        self.replace_file_with_excluded_directory(staged=True, ignored=False)

    def test_unstaged_file_to_ignored_directory(self):
        self.replace_file_with_excluded_directory(staged=False, ignored=True)

    def test_staged_file_to_ignored_directory(self):
        self.replace_file_with_excluded_directory(staged=True, ignored=True)

    def replace_directory_with_ignored_file(self, *, staged):
        (self.repo / "config/nested").mkdir(parents=True)
        historical = ["config/settings.json", "config/nested/other.json"]
        for name in historical:
            (self.repo / name).write_bytes(b"historical settings\n")
        self.git("add", "config")
        self.git("commit", "-qm", "tracked directory")
        (self.repo / ".git/info/exclude").write_text("/config\n")
        for name in historical:
            (self.repo / name).unlink()
        (self.repo / "config/nested").rmdir()
        (self.repo / "config").rmdir()
        (self.repo / "config").write_bytes(b"ignored replacement\x00\xff\n")
        if staged:
            self.git("add", "-u")
        self.assert_replacement(historical, {}, staged=staged, excluded=["config"])

    def test_unstaged_directory_to_ignored_file(self):
        self.replace_directory_with_ignored_file(staged=False)

    def test_staged_directory_to_ignored_file(self):
        self.replace_directory_with_ignored_file(staged=True)

    def test_staged_deletion_with_ignored_recreated_leaf(self):
        self.git("rm", "-q", "tracked.txt")
        (self.repo / ".git/info/exclude").write_text("/tracked.txt\n")
        (self.repo / "tracked.txt").write_bytes(b"ignored recreated leaf\n")
        self.assert_replacement(["tracked.txt"], {}, staged=True, excluded=["tracked.txt"])

    def replace_leaf_with_symlink(self, *, staged, ignored):
        targets = ("ignored/first", "ignored/second-longer-target")
        (self.repo / "ignored").mkdir()
        for target in targets:
            (self.repo / target).write_bytes(b"excluded target content\n")
        if staged:
            self.git("rm", "-q", "tracked.txt")
        else:
            (self.repo / "tracked.txt").unlink()
        if ignored:
            (self.repo / ".git/info/exclude").write_text("/tracked.txt\n")
        (self.repo / "tracked.txt").symlink_to(targets[0])
        return targets

    def test_staged_deletion_with_ignored_internal_symlink(self):
        targets = self.replace_leaf_with_symlink(staged=True, ignored=True)
        head_before = self.git("rev-parse", "HEAD")
        index_before = (self.repo / ".git/index").read_bytes()
        status_before = self.git("status", "--porcelain=v1", "-z", "--untracked-files=all")
        files_before = self.worktree_bytes()
        captured, patches = state.observe(self.repo)
        self.assertEqual(next(item for item in captured["tracked"] if item["path"] == "tracked.txt"),
            {"path": "tracked.txt", "state": "deleted"})
        self.assertEqual(captured["untracked"], [])
        self.assertNotIn("tracked.txt", {item["path"] for item in captured["index"]})
        self.assertEqual(captured["status"], [{"path": "tracked.txt", "index": "D", "worktree": " "}])
        self.assertEqual(captured["boundary"],
            "HEAD_index_tracked_and_nonignored_untracked; ignored_content_excluded")
        self.assertIn(b"deleted file mode 100644", patches["staged"])
        self.assertEqual(patches["unstaged"], b"")
        read_bytes, digest = Path.read_bytes, state.digest
        def boundary_read(path):
            self.assertNotIn(path.relative_to(self.repo).as_posix(), ("tracked.txt", *targets))
            return read_bytes(path)
        def boundary_digest(content):
            self.assertNotIn(content, [os.fsencode(target) for target in targets] + [b"excluded target content\n"])
            return digest(content)
        with mock.patch.object(Path, "read_bytes", autospec=True, side_effect=boundary_read), \
                mock.patch.object(state, "digest", side_effect=boundary_digest):
            self.assertEqual(state.observe(self.repo), (captured, patches))
            state.verify(self.repo, captured)
        state.verify_retained(json.loads(state.encoded(captured)), patches)
        c, mapped = self.mapping()
        with self.assertRaisesRegex(campaign.IntegrityError, "uncommitted final"):
            campaign.verify_journey_revision_chronology(c, mapped)
        self.assertEqual(self.git("rev-parse", "HEAD"), head_before)
        self.assertEqual((self.repo / ".git/index").read_bytes(), index_before)
        self.assertEqual(self.git("status", "--porcelain=v1", "-z", "--untracked-files=all"), status_before)
        self.assertEqual(self.worktree_bytes(), files_before)
        self.assertEqual(os.readlink(self.repo / "tracked.txt"), targets[0])

    def test_ignored_symlink_target_change_preserves_fingerprint_but_dirty_collection_rejects(self):
        targets = self.replace_leaf_with_symlink(staged=True, ignored=True)
        captured, patches = state.observe(self.repo)
        lineage = {"repository_state": captured, "final_revision": captured["final_head"]}
        evidence = self.root / "retained"
        evidence.mkdir()
        binding = {"state": "repository-state.json", "staged": "staged.patch", "unstaged": "unstaged.patch"}
        (evidence / binding["state"]).write_bytes(state.encoded(captured))
        for key, patch in patches.items():
            (evidence / binding[key]).write_bytes(patch)
        manifest = {"journeys": {"journey-volicord": {"repository_path": str(self.repo)}},
            "journey_final_evidence": [{"journey_id": "journey-volicord",
                "repository_revision_lineage": {**lineage, "attestation_artifacts": binding}}]}
        (evidence / "evidence-set.json").write_bytes(state.encoded(manifest))
        for target in (targets[1], targets[0]):
            with self.subTest(target=target):
                (self.repo / "tracked.txt").unlink()
                (self.repo / "tracked.txt").symlink_to(target)
                observed, observed_patches = state.observe(self.repo)
                self.assertEqual(observed["fingerprint"], captured["fingerprint"])
                self.assertEqual((observed, observed_patches), (captured, patches))
                state.verify(self.repo, captured)
                state.verify_retained(captured, patches)
                with self.assertRaisesRegex(campaign.CampaignError, "not clean"):
                    campaign.verify_final_repository_states_for_publication(evidence)
                with self.assertRaisesRegex(campaign.CampaignError, "not clean"):
                    campaign.verify_retained_repository_states(evidence, manifest)

    def test_nonignored_same_path_symlink_is_still_attested(self):
        targets = self.replace_leaf_with_symlink(staged=True, ignored=False)
        captured, patches = state.observe(self.repo)
        expected = {"path": "tracked.txt", "state": "symlink", "bytes": len(os.fsencode(targets[0])),
            "sha256": state.digest(os.fsencode(targets[0]))}
        self.assertIn(expected, captured["tracked"])
        self.assertEqual(captured["untracked"], [expected])
        state.verify(self.repo, captured)
        state.verify_retained(captured, patches)
        (self.repo / "tracked.txt").unlink()
        (self.repo / "tracked.txt").symlink_to(targets[1])
        with self.assertRaisesRegex(state.StateError, "changed before publication"):
            state.verify(self.repo, captured)

    def test_ignored_unstaged_symlink_is_type_change_not_deleted_leaf(self):
        # Git still owns the index leaf: an ignore rule does not turn its
        # unstaged file-to-symlink type change into a historical deletion.
        targets = self.replace_leaf_with_symlink(staged=False, ignored=True)
        self.assertEqual(self.git("status", "--porcelain=v1", "-z"), b" T tracked.txt\0")
        captured, patches = state.observe(self.repo)
        self.assertIn({"path": "tracked.txt", "state": "symlink", "bytes": len(os.fsencode(targets[0])),
            "sha256": state.digest(os.fsencode(targets[0]))}, captured["tracked"])
        self.assertEqual(captured["untracked"], [])
        state.verify(self.repo, captured)
        state.verify_retained(captured, patches)

    def test_ignored_current_index_leaf_is_still_attested(self):
        (self.repo / ".git/info/exclude").write_text("/tracked.txt\n")
        content = b"current tracked content\n"
        (self.repo / "tracked.txt").write_bytes(content)
        captured, patches = state.observe(self.repo)
        self.assertIn({"path": "tracked.txt", "state": "file", "bytes": len(content),
            "sha256": state.digest(content)}, captured["tracked"])
        state.verify(self.repo, captured)
        state.verify_retained(captured, patches)

    def test_ordinary_add_delete_and_rename(self):
        (self.repo / "deleted.txt").write_bytes(b"deleted later\n")
        self.git("add", "deleted.txt")
        self.git("commit", "-qm", "deletion fixture")
        (self.repo / "deleted.txt").unlink()
        self.git("mv", "tracked.txt", "renamed.txt")
        (self.repo / "added.txt").write_bytes(b"addition\n")
        self.git("add", "-A")
        self.assert_replacement(["deleted.txt", "tracked.txt"],
            {"renamed.txt": b"baseline\n", "added.txt": b"addition\n"}, staged=True)

    def test_staged_deletion_with_recreated_current_leaf(self):
        self.git("rm", "-q", "tracked.txt")
        content = b"recreated current file\n"
        (self.repo / "tracked.txt").write_bytes(content)
        captured, patches = state.observe(self.repo)
        expected = {"path": "tracked.txt", "state": "file", "bytes": len(content),
            "sha256": state.digest(content)}
        self.assertIn(expected, captured["tracked"])
        self.assertEqual(captured["untracked"], [expected])
        state.verify(self.repo, captured)
        state.verify_retained(captured, patches)

    def test_current_symlink_and_executable_exact_bytes(self):
        (self.repo / "link").symlink_to("tracked.txt")
        (self.repo / "tracked.txt").chmod(0o755)
        self.git("add", "link", "tracked.txt")
        (self.repo / "untracked-link").symlink_to("tracked.txt")
        captured, patches = state.observe(self.repo)
        tracked = {item["path"]: item for item in captured["tracked"]}
        self.assertEqual(tracked["link"], {"path": "link", "state": "symlink",
            "bytes": len(b"tracked.txt"), "sha256": state.digest(b"tracked.txt")})
        self.assertEqual(tracked["tracked.txt"], {"path": "tracked.txt", "state": "executable",
            "bytes": len(b"baseline\n"), "sha256": state.digest(b"baseline\n")})
        self.assertEqual(captured["untracked"], [{**tracked["link"], "path": "untracked-link"}])
        state.verify(self.repo, captured)
        state.verify_retained(captured, patches)

    def test_directories_need_git_proven_deletion(self):
        (self.repo / "tracked.txt").unlink()
        (self.repo / "tracked.txt").mkdir()
        real_git = state.git
        def without_deletion(repository, *arguments, **options):
            if "status" in arguments:
                return b""
            return real_git(repository, *arguments, **options)
        with mock.patch.object(state, "git", side_effect=without_deletion):
            with self.assertRaisesRegex(state.StateError, "unsupported"):
                state.observe(self.repo)
        (self.repo / "tracked.txt/child").write_bytes(b"child")
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.file_state(self.repo, "tracked.txt", missing_allowed=True)
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.file_state(self.repo, "tracked.txt", replacement_leaves={"tracked.txt/child"})
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.file_state(self.repo, "tracked.txt", missing_allowed=True, replacement_leaves={"elsewhere"})

    def test_non_directory_parent_requires_deleted_git_leaf_and_attestation(self):
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.file_state(self.repo, "tracked.txt/child", missing_allowed=True)
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.file_state(self.repo, "tracked.txt/child", replacement_leaves={"tracked.txt"})

    def test_historical_directory_symlink_is_not_traversed(self):
        (self.repo / "config").mkdir()
        (self.repo / "config/settings.json").write_bytes(b"settings")
        self.git("add", "config")
        self.git("commit", "-qm", "directory fixture")
        (self.repo / "config/settings.json").unlink()
        (self.repo / "config").rmdir()
        (self.repo / "inside").mkdir()
        (self.repo / "inside/settings.json").write_bytes(b"replacement")
        for target in (self.repo / "inside", self.root / "outside"):
            with self.subTest(target=target):
                (self.repo / "config").symlink_to(target)
                with self.assertRaisesRegex(state.StateError, "escape"):
                    state.observe(self.repo)
                (self.repo / "config").unlink()

    def test_special_entry_is_not_a_deleted_leaf_or_replacement(self):
        (self.repo / "tracked.txt").unlink()
        os.mkfifo(self.repo / "tracked.txt")
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.observe(self.repo)
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.file_state(self.repo, "tracked.txt/child", missing_allowed=True,
                replacement_leaves={"tracked.txt"})
        (self.repo / "tracked.txt").unlink()
        (self.repo / "tracked.txt").mkdir()
        os.mkfifo(self.repo / "tracked.txt/child")
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.observe(self.repo)

    def assert_ignored_special_replacement_rejected(self, *, ancestor):
        historical = "tracked.txt"
        if ancestor:
            (self.repo / historical).unlink()
            (self.repo / historical).mkdir()
            historical = "tracked.txt/child"
            (self.repo / historical).write_bytes(b"baseline\n")
            self.git("add", "-A")
            self.git("commit", "-qm", "tracked descendant")
        (self.repo / ".git/info/exclude").write_text("/tracked.txt\n")
        for staged in (False, True):
            for special in ("fifo", "socket"):
                with self.subTest(staged=staged, special=special, ancestor=ancestor):
                    (self.repo / historical).unlink()
                    if ancestor:
                        (self.repo / "tracked.txt").rmdir()
                    if staged:
                        self.git("update-index", "--force-remove", historical)
                    if special == "fifo":
                        os.mkfifo(self.repo / "tracked.txt")
                    else:
                        with socket.socket(socket.AF_UNIX) as endpoint:
                            endpoint.bind(str(self.repo / "tracked.txt"))
                    # A special object at the exact index leaf is a Git
                    # modification; a blocking special ancestor proves deletion.
                    status = ("D  " if staged else " D " if ancestor else " M ") + historical + "\0"
                    self.assertIn(status.encode(), self.git("status", "--porcelain=v1", "-z"))
                    with self.assertRaisesRegex(state.StateError, "unsupported"):
                        state.observe(self.repo)
                    (self.repo / "tracked.txt").unlink()
                    if ancestor:
                        (self.repo / "tracked.txt").mkdir()
                    (self.repo / historical).write_bytes(b"baseline\n")
                    self.git("add", "-f", historical)

    def test_ignored_special_leaf_is_not_deletion(self):
        self.assert_ignored_special_replacement_rejected(ancestor=False)

    def test_ignored_special_ancestor_is_not_deletion(self):
        self.assert_ignored_special_replacement_rejected(ancestor=True)

    def test_ignored_external_symlink_is_still_rejected(self):
        self.git("rm", "-q", "tracked.txt")
        (self.repo / ".git/info/exclude").write_text("/tracked.txt\n")
        outside = self.root / "outside"
        outside.write_bytes(b"external content")
        for target in (outside, "../outside", "../missing-outside"):
            with self.subTest(target=target):
                (self.repo / "tracked.txt").symlink_to(target)
                with self.assertRaisesRegex(state.StateError, "escape"):
                    state.observe(self.repo)
                (self.repo / "tracked.txt").unlink()

    def test_replacement_directory_with_nonignored_special_and_current_file_fails(self):
        (self.repo / "tracked.txt").unlink()
        (self.repo / "tracked.txt/nested").mkdir(parents=True)
        (self.repo / "tracked.txt/current").write_bytes(b"current replacement\n")
        os.mkfifo(self.repo / "tracked.txt/nested/fifo")
        with self.assertRaisesRegex(state.StateError, "unsupported"):
            state.observe(self.repo)

    def test_unmerged_index_is_rejected(self):
        object_id = self.git("rev-parse", "HEAD:tracked.txt").strip().decode()
        subprocess.run(["git", "update-index", "--index-info"], cwd=self.repo, check=True,
            input=f"100644 {object_id} 1\tconflict.txt\n100644 {object_id} 2\tconflict.txt\n".encode(),
            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        with self.assertRaisesRegex(state.StateError, "unmerged index"):
            state.observe(self.repo)

    def test_submodule_index_is_rejected(self):
        self.git("update-index", "--add", "--cacheinfo", "160000", self.baseline, "module")
        with self.assertRaisesRegex(state.StateError, "submodule"):
            state.observe(self.repo)

    def test_deleted_head_submodule_is_not_a_directory_replacement(self):
        self.git("update-index", "--add", "--cacheinfo", "160000", self.baseline, "module")
        self.git("commit", "-qm", "submodule fixture")
        self.git("update-index", "--force-remove", "module")
        (self.repo / "module").mkdir()
        (self.repo / "module/current.txt").write_bytes(b"replacement")
        with self.assertRaisesRegex(state.StateError, "submodule"):
            state.observe(self.repo)

    def test_concurrent_status_or_head_mutation_is_rejected(self):
        real_git = state.git
        for commit in (False, True):
            with self.subTest(commit=commit):
                def mutate(repository, *arguments, **options):
                    output = real_git(repository, *arguments, **options)
                    if arguments[0] == "diff" and "--cached" not in arguments:
                        (self.repo / "concurrent").write_bytes(b"concurrent change")
                        if commit:
                            self.git("add", "concurrent")
                            self.git("commit", "-qm", "concurrent commit")
                    return output
                with mock.patch.object(state, "git", side_effect=mutate):
                    with self.assertRaisesRegex(state.StateError, "changed during attestation"):
                        state.observe(self.repo)
                if not commit:
                    (self.repo / "concurrent").unlink()

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
        with self.assertRaisesRegex(campaign.IntegrityError, "uncommitted final"):
            campaign.verify_journey_revision_chronology(c, mapped)
        self.assertEqual(self.git("rev-parse", "HEAD").strip().decode(), self.baseline)
        self.assertEqual((self.repo / ".git/index").read_bytes(), index_before)
        self.assertEqual(self.git("status", "--porcelain=v1", "-z"), status_before)

    def mark_change(self, mapped, kind, work, role, *, completed=False, paths=("tracked.txt",)):
        capture = mapped[(kind, work, role)].capture
        capture.path_observations = (SimpleNamespace(sequence=1, paths=paths),)
        capture.successful_calls = lambda operation: ([SimpleNamespace(
            arguments={"work_state": "completed" if completed else "paused"})]
            if operation == "checkpoint_record" else [])

    def boundary_check(self, mapped, kind, work, role, *, sequence=10):
        capture = mapped[(kind, work, role)].capture
        result = subprocess.run(campaign.WORK_BOUNDARY_CHECK, shell=True, cwd=self.repo,
            check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        capture.commands = (CommandObservation(sequence, sequence + 1, "fixture-turn", 0,
            {"cmd": campaign.WORK_BOUNDARY_CHECK, "workdir": str(self.repo)},
            result.returncode, "exited", result.stdout.decode(), False,
            "fixture-boundary-check", "completed"),)

    def commit_change(self, content):
        (self.repo / "tracked.txt").write_bytes(content)
        self.git("commit", "-qam", "fixture Work")
        return self.git("rev-parse", "HEAD").strip().decode()

    def test_completed_work_boundaries_and_clean_collection(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "volicord", "A", "start")
        self.mark_change(mapped, "volicord", "A", "resume")
        self.mark_change(mapped, "volicord", "B", "start")
        self.mark_change(mapped, "volicord", "C", "start")
        a = self.commit_change(b"A complete\n")
        self.boundary_check(mapped, "volicord", "A", "resume")
        mapped[("volicord", "B", "start")].capture.git_revision = a
        b = self.commit_change(b"B complete\n")
        self.boundary_check(mapped, "volicord", "B", "start")
        mapped[("volicord", "C", "start")].capture.git_revision = b
        self.commit_change(b"C complete\n")
        self.boundary_check(mapped, "volicord", "C", "start")
        result = campaign.verify_journey_revision_chronology(c, mapped)["journey-volicord"]
        self.assertTrue(result["workspace_clean"])
        self.assertEqual(len(result["work_commit_boundaries"]), 4)
        self.assertEqual([item["committed_boundary_revision"] for item in result["work_commit_boundaries"]],
            [a, a, b, result["final_revision"]])

    def test_completed_a_committed_before_no_change_resume_and_b(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "volicord", "A", "start", completed=True)
        a = self.commit_change(b"A complete before resume\n")
        self.boundary_check(mapped, "volicord", "A", "start")
        mapped[("volicord", "A", "resume")].capture.git_revision = a
        mapped[("volicord", "B", "start")].capture.git_revision = a
        mapped[("volicord", "C", "start")].capture.git_revision = a
        result = campaign.verify_journey_revision_chronology(c, mapped)["journey-volicord"]
        self.assertEqual(len(result["work_commit_boundaries"]), 1)
        self.assertEqual(result["final_revision"], a)

    def test_dirty_a_cannot_cross_into_b_even_if_committed_later(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "volicord", "A", "start")
        self.commit_change(b"A and B bundled\n")
        with self.assertRaisesRegex(campaign.IntegrityError, "Work A start.*before Work B"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_b_and_c_cannot_be_bundled_into_final_commit(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "volicord", "B", "start")
        self.mark_change(mapped, "volicord", "C", "start")
        self.commit_change(b"B and C bundled\n")
        with self.assertRaisesRegex(campaign.IntegrityError, "Work B start.*before Work C"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_same_work_dirty_continuation_and_final_commit(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "small-python", "A", "start")
        self.mark_change(mapped, "small-python", "A", "resume")
        self.commit_change(b"same Work completed\n")
        self.boundary_check(mapped, "small-python", "A", "resume")
        result = campaign.verify_journey_revision_chronology(c, mapped)
        self.assertEqual(len(result["journey-small-python"]["work_commit_boundaries"]), 2)
        c, mapped = self.mapping()
        self.mark_change(mapped, "small-python", "A", "start", completed=True)
        with self.assertRaisesRegex(campaign.IntegrityError, "dirty start lacks incomplete state"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_final_dirty_mutation_is_rejected(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "polyglot-medium", "A", "resume")
        (self.repo / "tracked.txt").write_bytes(b"uncommitted\n")
        with self.assertRaisesRegex(campaign.IntegrityError, "uncommitted final"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def changed_b(self, paths=("tracked.txt",)):
        c, mapped = self.mapping()
        self.mark_change(mapped, "volicord", "B", "start", completed=True, paths=paths)
        return c, mapped

    def close_b(self, mapped):
        boundary = self.git("rev-parse", "HEAD").strip().decode()
        mapped[("volicord", "C", "start")].capture.git_revision = boundary
        self.boundary_check(mapped, "volicord", "B", "start")
        return boundary

    def test_two_paths_fully_committed_and_retained_proofs(self):
        c, mapped = self.changed_b(("tracked.txt", "new path\nZ.txt"))
        (self.repo / "tracked.txt").write_bytes(b"B\n")
        (self.repo / "new path\nZ.txt").write_bytes(b"Z\n")
        self.git("add", "--", "tracked.txt", "new path\nZ.txt")
        self.git("commit", "-qm", "B complete")
        boundary = self.close_b(mapped)
        proof = campaign.verify_journey_revision_chronology(c, mapped)["journey-volicord"]["work_commit_boundaries"][0]
        self.assertEqual(proof["proven_committed_paths"], sorted(("tracked.txt", "new path\nZ.txt")))
        self.assertEqual(proof["boundary_kind"], "distinct_work_transition")
        self.assertEqual(proof["session_revision"], self.baseline)
        self.assertEqual(proof["cleanliness"]["head"], boundary)
        self.assertEqual(proof["cleanliness"]["source_sha256"],
            mapped[("volicord", "B", "start")].capture.source_sha256)

    def test_unrelated_only_commit_and_later_work_commit_rejected(self):
        c, mapped = self.changed_b()
        (self.repo / "tracked.txt").write_bytes(b"B dirty\n")
        (self.repo / "unrelated.txt").write_bytes(b"unrelated\n")
        self.git("add", "--", "unrelated.txt")
        self.git("commit", "-qm", "unrelated only")
        self.close_b(mapped)
        self.git("commit", "-qam", "B bundled with C later")
        with self.assertRaisesRegex(campaign.IntegrityError, "observed paths absent.*tracked.txt"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_partial_path_commit_before_next_work_rejected(self):
        c, mapped = self.changed_b(("tracked.txt", "Z.txt"))
        self.commit_change(b"X committed\n")
        (self.repo / "Z.txt").write_bytes(b"Z dirty\n")
        self.close_b(mapped)
        self.git("add", "--", "Z.txt")
        self.git("commit", "-qm", "Z too late")
        with self.assertRaisesRegex(campaign.IntegrityError, "observed paths absent.*Z.txt"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_residual_hunk_at_completed_boundary_rejected_even_after_later_commit(self):
        c, mapped = self.changed_b()
        self.commit_change(b"committed hunk\n")
        (self.repo / "tracked.txt").write_bytes(b"committed hunk\nresidual hunk\n")
        self.close_b(mapped)
        self.git("commit", "-qam", "residual too late")
        with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_clean_check_before_last_mutation_rejected(self):
        c, mapped = self.changed_b()
        self.commit_change(b"B\n")
        self.close_b(mapped)
        mapped[("volicord", "B", "start")].capture.path_observations += (
            SimpleNamespace(sequence=20, paths=("tracked.txt",)),)
        with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_missing_wrong_head_cwd_and_indeterminate_cleanliness_rejected(self):
        c, mapped = self.changed_b()
        self.commit_change(b"B\n")
        self.close_b(mapped)
        capture = mapped[("volicord", "B", "start")].capture
        commands = capture.commands
        for field, value in (("output", self.baseline + "\n"), ("exit_code", None),
                ("evidence_state", "indeterminate"), ("termination", None),
                ("parsed_command", {"cmd": campaign.WORK_BOUNDARY_CHECK, "workdir": str(self.root)}),
                ("parsed_command", {"cmd": campaign.WORK_BOUNDARY_CHECK + " | cat"}),
                ("parsed_command", {"cmd": "git status --porcelain=v1 --untracked-files=no"})):
            with self.subTest(field=field, value=value):
                capture.commands = (replace(commands[0], **{field: value}),)
                with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
                    campaign.verify_journey_revision_chronology(c, mapped)
        capture.commands = ()
        with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_later_or_overlapping_command_cannot_follow_cleanliness(self):
        c, mapped = self.changed_b()
        self.commit_change(b"B\n")
        self.close_b(mapped)
        capture = mapped[("volicord", "B", "start")].capture
        clean = capture.commands[0]
        for sequence, completion in ((20, 21), (2, 11)):
            capture.commands = (clean, replace(clean, sequence=sequence,
                completion_sequence=completion, parsed_command={"cmd": "arbitrary writer"}))
            with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
                campaign.verify_journey_revision_chronology(c, mapped)

    def test_final_clean_check_cannot_bind_an_earlier_head(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "polyglot-medium", "A", "resume")
        self.commit_change(b"first hunk\n")
        self.boundary_check(mapped, "polyglot-medium", "A", "resume")
        self.commit_change(b"final mutation committed after check\n")
        with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_clean_reset_or_full_revert_is_not_commit_proof(self):
        c, mapped = self.changed_b()
        # Raw observation remains, but content was fully restored. No semantic
        # inference can substitute for a net tree delta at that exact path.
        (self.repo / "unrelated.txt").write_bytes(b"Y\n")
        self.git("add", "--", "unrelated.txt")
        self.git("commit", "-qm", "unrelated with reverted X")
        self.close_b(mapped)
        with self.assertRaisesRegex(campaign.IntegrityError, "observed paths absent"):
            campaign.verify_journey_revision_chronology(c, mapped)

    def test_rename_add_delete_and_literal_path_coverage(self):
        added = ":(glob)*.txt"
        self.git("mv", "tracked.txt", "renamed space.txt")
        (self.repo / added).write_bytes(b"literal addition\n")
        (self.repo / ".gitignore").unlink()
        self.git("add", "-A")
        self.git("commit", "-qm", "rename addition deletion")
        changes = normalized_file_changes({"tracked.txt": {"type": "update",
            "unified_diff": "@@\n-baseline\n+renamed\n", "move_path": "renamed space.txt"}}, self.repo)
        self.assertEqual(changes[0], ("renamed space.txt", "tracked.txt"))
        paths = (*changes[0], added, ".gitignore")
        c, mapped = self.changed_b(paths)
        self.close_b(mapped)
        proof = campaign.verify_journey_revision_chronology(c, mapped)["journey-volicord"]["work_commit_boundaries"][0]
        self.assertEqual(proof["proven_committed_paths"], sorted(paths))

    def test_genuine_no_change_resume_needs_no_commit_or_status_command(self):
        c, mapped = self.mapping()
        result = campaign.verify_journey_revision_chronology(c, mapped)
        for lineage in result.values():
            self.assertEqual(lineage["final_revision"], self.baseline)
            self.assertEqual(lineage["work_commit_boundaries"], [])

    def test_boundary_check_is_inspection_and_never_product_validation(self):
        self.assertEqual(command_role({"cmd": campaign.WORK_BOUNDARY_CHECK}), "inspection")
        self.assertEqual(command_role({"cmd": "git -c alias.status=writer status"}), "unknown")
        self.assertEqual(command_role({"cmd": "git add -- tracked.txt && git commit -m 'Work'"}),
            "repository_maintenance")
        c, mapped = self.mapping()
        capture = mapped[("volicord", "B", "start")].capture
        capture.commands = (CommandObservation(2, 3, "turn", 0,
            {"cmd": "git add -- tracked.txt && git commit -m 'Work'"},
            0, "exited", "committed\n", False, "commit", "completed"),)
        self.assertFalse(harness.meaningful_resume_validation(capture, 0)["qualified"])

    def test_incomplete_start_and_resume_commit_distinct_paths_before_next_work(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "volicord", "A", "start")
        (self.repo / "tracked.txt").write_bytes(b"start X dirty\n")
        self.assertTrue(self.git("status", "--porcelain=v1"))
        self.assertEqual(mapped[("volicord", "A", "resume")].capture.git_revision, self.baseline)
        self.mark_change(mapped, "volicord", "A", "resume", completed=True, paths=("Z.txt",))
        (self.repo / "Z.txt").write_bytes(b"resume Z\n")
        self.git("add", "--", "tracked.txt", "Z.txt")
        self.git("commit", "-qm", "A X and Z complete")
        a = self.git("rev-parse", "HEAD").strip().decode()
        self.boundary_check(mapped, "volicord", "A", "resume")
        for label in ("B", "C"):
            mapped[("volicord", label, "start")].capture.git_revision = a
        lineage = campaign.verify_journey_revision_chronology(c, mapped)["journey-volicord"]
        self.assertEqual([p["proven_committed_paths"] for p in lineage["work_commit_boundaries"]],
            [["tracked.txt"], ["Z.txt"]])
        self.assertTrue(lineage["same_work_continuations"][0]["dirty_continuation_permitted"])
        self.assertEqual({p["cleanliness"]["session_id"] for p in lineage["work_commit_boundaries"]},
            {mapped[("volicord", "A", "resume")].capture.session_id})

    def test_incomplete_start_no_write_resume_still_proves_completed_work(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "small-python", "A", "start")
        self.commit_change(b"A completed during resume\n")
        self.boundary_check(mapped, "small-python", "A", "resume")
        proof = campaign.verify_journey_revision_chronology(c, mapped)["journey-small-python"]["work_commit_boundaries"][0]
        self.assertEqual(proof["boundary_kind"], "final_collection")
        self.assertEqual(proof["cleanliness"]["last_meaningful_mutation_sequence"], 0)

    def test_completed_start_residual_hunk_cannot_cross_checkpoint_free_resume(self):
        c, mapped = self.mapping()
        self.mark_change(mapped, "small-python", "A", "start", completed=True)
        a = self.commit_change(b"committed hunk\n")
        (self.repo / "tracked.txt").write_bytes(b"committed hunk\nresidual\n")
        self.boundary_check(mapped, "small-python", "A", "start")
        mapped[("small-python", "A", "resume")].capture.git_revision = a
        self.git("commit", "-qam", "residual after resume started")
        with self.assertRaisesRegex(campaign.IntegrityError, "terminal clean Git status"):
            campaign.verify_journey_revision_chronology(c, mapped)

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
