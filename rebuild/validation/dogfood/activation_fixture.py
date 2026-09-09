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
    assert len(context.encode("utf-8")) < 1536
    return context
