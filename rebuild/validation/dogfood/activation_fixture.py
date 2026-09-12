"""Generate maintained activation evidence through the production CLI hook."""

from functools import lru_cache
import json
from pathlib import Path
import subprocess


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
    result = subprocess.run(
        [str(production_binary()), "--repository", str(repository), "codex", "hook"],
        input=json.dumps({
            "hook_event_name": "SessionStart", "session_id": session_id,
            "cwd": str(repository), "source": source, "model": "fixture",
            "permission_mode": "default", "transcript_path": None,
        }), text=True, capture_output=True, check=True,
    )
    output = json.loads(result.stdout)["hookSpecificOutput"]
    assert output["hookEventName"] == "SessionStart"
    context = output["additionalContext"]
    assert len(context.encode("utf-8")) < 4096
    assert "Record only observed outcomes; retain no raw arguments" in context
    assert "exact transient invocation and numeric exit/termination observable for the same execution through polling" in context
    assert "For verified completion/pause, rerun relevant bounded verification after the final meaningful mutation" in context
    assert "Pre-mutation success, inspection, prior Checkpoint or prose cannot certify later changes" in context
    assert "Use standalone bounded verification for unambiguous terminal evidence" in context
    assert "compound diagnostics may be useful but mixed/ambiguous commands cannot be sole terminal evidence" in context
    assert "No post-mutation requirement for read-only, explanation-only or no-write exploratory continuation" in context
    assert "No-write research/prototype conclusions using execution need a completed bounded scratch experiment" in context
    assert "exact invocation and actual numeric exit/termination from that execution" in context
    assert "Diagnostics may precede it" in context
    assert "afterward normally only read-only inspection/reporting that preserves its evidence basis before Checkpoint" in context
    assert "After later substantive executable diagnostics, establish a later bounded experiment with observed numeric completion" in context
    assert "Checkpoint records no repository changed paths and the terminal experiment evidence, not an earlier superseded run" in context
    assert "Never infer success from prose/output, hide failed/indeterminate experiments, or promote arbitrary successful commands to repository validation" in context
    assert "call decision_record promptly" in context
    assert "existing valid presentation_receipt_id" in context
    assert "exact current user_turn" in context
    assert "No repeat confirmation of unchanged Questions" in context
    assert "Explanation is not a Decision; clarify ambiguous answers" in context
    assert "Changed revisions or stale/invalid receipts require current presentation" in context
    assert "caller-supplied current-host response" in context
    assert "never infer a Decision from recommendation or silence" in context
    assert "Stronger confirmation is only for existing high-risk effects" in context
    assert "Recall is not completion" in context
    assert "Run explicitly requested tests/build/lint/verification to termination before completion; report blockers" in context
    assert "Prior Checkpoint or inspection cannot substitute" in context
    assert "Inspect/explain-only requests need no execution" in context
    assert "Explicit learning/explanation participation in this bounded Goal stays active" in context
    assert "exact current-host Source/verbatim statement even if all dimensions are routine" in context
    assert "Per-dimension learning value is separate: routine detail requires no Learning Deliberation, Question, or Decision" in context
    assert "default inactive; generic coding, agent explanation, or ungrounded keywords cannot activate participation" in context
    return context
