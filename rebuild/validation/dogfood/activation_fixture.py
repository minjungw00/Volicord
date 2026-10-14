"""Generate maintained activation evidence through the production CLI hook."""

from functools import lru_cache
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


@lru_cache(maxsize=1)
def production_binary() -> Path:
    root = Path(__file__).resolve().parents[3]
    built = subprocess.run(
        ["cargo", "build", "--manifest-path", "rebuild/Cargo.toml", "-p",
         "volicord-operations", "--bin", "volicord", "--message-format=json"],
        cwd=root, text=True, stdout=subprocess.PIPE, check=True,
    )
    artifacts = [json.loads(line) for line in built.stdout.splitlines()]
    return Path(next(
        item["executable"] for item in artifacts
        if item.get("reason") == "compiler-artifact"
        and item.get("target", {}).get("name") == "volicord"
        and item.get("executable")
    ))


def production_context(repository: Path, session_id: str, source: str = "startup") -> str:
    repository = repository.resolve()
    binary = production_binary()
    runtime = repository / ".local/activation-fixture-runtime"
    result = subprocess.run(
        [str(binary), "--runtime", str(runtime), "--repository", str(repository), "codex", "hook"],
        input=json.dumps({
            "hook_event_name": "SessionStart", "session_id": session_id,
            "cwd": str(repository), "source": source, "model": "fixture",
            "permission_mode": "default", "transcript_path": None,
        }), text=True, capture_output=True, check=True,
    )
    output = json.loads(result.stdout)["hookSpecificOutput"]
    assert output["hookEventName"] == "SessionStart"
    context = output["additionalContext"]
    assert "CLI route:" in context and str(binary) in context and str(runtime) in context and str(repository) in context
    guidance_bytes = len("\n".join(line for line in context.splitlines() if not line.startswith("CLI route:")).encode("utf-8"))
    assert guidance_bytes < 4096, f"SessionStart guidance is {guidance_bytes} UTF-8 bytes; must be less than 4096"
    for rule in (
        "project_resolve using the MCP tool",
        "workflow.required_next_action until blocks_ordinary_work is false",
        "MCP work_explanation generate",
        "work_source for permitted relevant Sources",
        "work_explanation read_source",
        "then author and record with exact bindings",
        "Prepare/read/GET never generate or transmit",
        "no automatic generation, server-side model dispatch or generation quota",
        "Missing host access/authority is a blocker",
        "Never infer user authority from agent recommendation",
        "transmit sources without separate exact provider authorization",
    ):
        assert rule in context, f"missing activation boundary: {rule}"
    assert "Record only observed outcomes; retain no raw arguments" in context
    assert "exact transient invocation and numeric exit/termination observable for the same execution through polling" in context
    assert "Verified completion/pause: rerun relevant bounded verification after the final meaningful mutation" in context
    assert "Pre-mutation success, inspection, prior Checkpoint or prose cannot certify later changes" in context
    assert "Use standalone bounded verification for terminal evidence" in context
    assert "mixed/ambiguous compound diagnostics cannot be sole terminal evidence" in context
    assert "Read-only, explanation-only or no-write continuation needs no post-mutation verification" in context
    assert "No-write research/prototype execution needs a completed bounded scratch experiment" in context
    assert "exact invocation and actual numeric exit/termination" in context
    assert "Diagnostics may precede it" in context
    assert "then preserve its basis with read-only inspection/reporting before Checkpoint" in context
    assert "Later substantive diagnostics require a later bounded experiment with numeric completion" in context
    assert "Checkpoint records no changed paths and terminal experiment evidence, never a superseded run" in context
    assert "Never infer success from prose/output, hide failed/indeterminate experiments, or promote arbitrary successful commands to repository validation" in context
    assert "call decision_record promptly" in context
    assert "existing valid presentation_receipt_id" in context
    assert "exact current user_turn" in context
    assert "No repeat confirmation of unchanged Questions" in context
    assert "Explanation is not a Decision; clarify ambiguous answers" in context
    assert "Changed revisions or stale/invalid receipts require current presentation" in context
    assert "caller-supplied current-host response" in context
    assert "Never infer a Decision from recommendation or silence" in context
    assert "stronger confirmation only for existing high-risk effects" in context
    assert "Recall is not completion" in context
    assert "Complete requested tests/build/lint/verification to termination before completion; report blockers" in context
    assert "Inspect/explain-only requests need no execution" in context
    assert "Explicit learning/explanation participation stays active for this bounded Goal" in context
    assert "exact current-host Source/verbatim statement, even for routine dimensions" in context
    assert "Routine learning value requires no Learning Deliberation, Question, or Decision" in context
    assert "default inactive; generic coding, agent explanation or ungrounded keywords cannot activate participation" in context
    return context


class ProductionHookTests(unittest.TestCase):
    """Exercise the installed hook route; guidance itself grants no Runtime access."""

    def setUp(self):
        root = Path(__file__).resolve().parents[3]
        local = root / "rebuild/.local"
        local.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(prefix="activation-", dir=local)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.repository = self.root / "authorized-저장소"
        self.repository.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repository)], check=True)

    def test_normal_sources_are_bounded_without_creating_runtime(self):
        for source in ("startup", "resume", "clear", "compact"):
            with self.subTest(source=source):
                context = production_context(self.repository, "session-학습", source)
                size = len("\n".join(line for line in context.splitlines()
                                    if not line.startswith("CLI route:")).encode("utf-8"))
                print(f"production SessionStart {source}: {size} UTF-8 bytes (<4096)")
                self.assertFalse((self.repository / ".local").exists())

    def test_wrong_repository_and_absent_runtime_access(self):
        child = self.repository / "src"
        child.mkdir()
        # A lexical prefix is not an authorized descendant.
        foreign = self.root / (self.repository.name + "-foreign")
        foreign.mkdir()
        inaccessible = self.root / "runtime-is-a-file"
        inaccessible.write_bytes(b"foreign-runtime-sentinel")
        missing = self.root / "missing-runtime"
        for runtime in (inaccessible, missing):
            for cwd, expected in ((child, True), (foreign, False), (self.root / "absent", False)):
                with self.subTest(runtime=runtime.name, cwd=cwd.name):
                    result = subprocess.run(
                        [str(production_binary()), "--runtime", str(runtime), "--repository",
                         str(self.repository), "codex", "hook"],
                        input=json.dumps({"hook_event_name": "SessionStart", "session_id": "negative",
                                          "cwd": str(cwd), "source": "startup", "model": "fixture",
                                          "permission_mode": "default", "transcript_path": None}),
                        text=True, capture_output=True, check=True,
                    )
                    self.assertEqual(bool(result.stdout), expected)
                    self.assertEqual(result.stderr, "")
                    if expected:
                        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
                        self.assertIn(str(runtime), context)
                        self.assertIn("Missing host access/authority is a blocker", context)
                    self.assertEqual(inaccessible.read_bytes(), b"foreign-runtime-sentinel")
                    self.assertFalse(missing.exists())


def check_activation_regressions():
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(ProductionHookTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if not result.wasSuccessful():
        raise AssertionError("production SessionStart regressions failed")


if __name__ == "__main__":
    unittest.main()
