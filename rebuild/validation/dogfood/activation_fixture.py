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
    assert len(context.encode("utf-8")) < 2560
    assert "Checkpoint verification that was not actually observed" in context
    assert "actual numeric exit/termination observable in host-visible results" in context
    assert "long-running/polled commands, retain the execution identity and actual terminal outcome" in context
    assert "prose success is not execution evidence" in context
    assert "call decision_record promptly" in context
    assert "existing valid presentation_receipt_id" in context
    assert "exact current user_turn" in context
    assert "Do not re-present an unchanged Question merely for confirmation" in context
    assert "Explanation requests before selection are not Decisions" in context
    assert "clarify genuinely ambiguous answers" in context
    assert "Changed revisions or stale/invalid receipts require current presentation" in context
    assert "caller-supplied current-host response" in context
    assert "never infer a Decision from recommendation or silence" in context
    assert "Stronger confirmation is only for existing high-risk effects" in context
    return context
