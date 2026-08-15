#!/usr/bin/env python3
"""Run the clean Linux/Codex portion of the maintained V08 journey."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import shutil
import stat
import subprocess
import sys
import tempfile
import time
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
INSTALLER = ROOT / "rebuild/install.sh"
EXPECTED_TOOLS = [
    "project_initialize",
    "project_health",
    "recall",
    "repository_understanding",
    "repository_analyze",
    "inquiry_frontier",
    "decision_record",
    "context_record",
    "checkpoint_record",
    "canonical_inspect",
    "canonical_mutate",
    "candidate_inspect",
    "candidate_manage",
    "privacy_status",
    "background_semantic_operation",
    "document_preview",
    "guarded_interaction",
]


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def require_codex_stdio_registration(
    configuration: dict[str, Any],
    *,
    command: Path,
    arguments: list[str],
    environment: dict[str, str],
    context: str,
) -> None:
    require(configuration.get("name") == "volicord", f"{context} server name mismatch")
    expected_transport = {
        "type": "stdio",
        "command": str(command),
        "args": arguments,
        "env": environment,
        "env_vars": [],
        "cwd": None,
    }
    require(
        configuration.get("transport") == expected_transport,
        f"{context} transport mismatch: {configuration.get('transport')!r}",
    )


def run(
    arguments: list[str], env: dict[str, str], *, expected: int = 0
) -> subprocess.CompletedProcess[str]:
    print(f"$ {shlex.join(arguments)}", flush=True)
    result = subprocess.run(
        arguments,
        cwd=ROOT,
        env=env,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.stdout:
        print(result.stdout, end="", file=sys.stdout)
    if result.stderr:
        print(result.stderr, end="", file=sys.stderr)
    if result.returncode != expected:
        raise RuntimeError(
            f"expected exit {expected}, got {result.returncode}: {shlex.join(arguments)}"
        )
    return result


def rpc(
    process: subprocess.Popen[str], request_id: int, method: str, params: dict[str, Any]
) -> dict[str, Any]:
    require(process.stdin is not None and process.stdout is not None, "MCP pipes unavailable")
    message = {"jsonrpc": "2.0", "id": request_id, "method": method, "params": params}
    process.stdin.write(json.dumps(message, separators=(",", ":")) + "\n")
    process.stdin.flush()
    response_line = process.stdout.readline()
    require(bool(response_line), f"MCP host ended before responding to {method}")
    response = json.loads(response_line)
    require(response.get("id") == request_id, f"MCP response identity mismatch for {method}")
    require("error" not in response, f"MCP protocol error for {method}: {response}")
    return response


def tool(
    process: subprocess.Popen[str], request_id: int, name: str, arguments: dict[str, Any]
) -> dict[str, Any]:
    response = rpc(
        process,
        request_id,
        "tools/call",
        {"name": name, "arguments": arguments},
    )
    result = response["result"]
    require(result["isError"] is False, f"{name} failed: {result}")
    return result["structuredContent"]


def tool_result(
    process: subprocess.Popen[str], request_id: int, name: str, arguments: dict[str, Any]
) -> dict[str, Any]:
    return rpc(
        process,
        request_id,
        "tools/call",
        {"name": name, "arguments": arguments},
    )["result"]


def schema_variants(schema: dict[str, Any]) -> list[dict[str, Any]]:
    variants = schema.get("oneOf")
    return variants if isinstance(variants, list) else [schema]


def assert_concrete_schema(name: str, schema: dict[str, Any]) -> None:
    variants = schema_variants(schema)
    require(bool(variants), f"{name} advertises no usable input shape")
    for variant in variants:
        require(variant.get("type") == "object", f"{name} input is not an object")
        require(
            variant.get("additionalProperties") is False,
            f"{name} input permits undocumented properties",
        )
        properties = variant.get("properties")
        required = variant.get("required")
        require(isinstance(properties, dict) and properties, f"{name} has no properties")
        require(isinstance(required, list), f"{name} has no required-field declaration")
        require(set(required) <= set(properties), f"{name} requires an undocumented property")
        for field, child in properties.items():
            require(isinstance(child, dict), f"{name}.{field} schema is not concrete")
            require(isinstance(child.get("description"), str), f"{name}.{field} is undescribed")
            require(
                child.get("type") in {"string", "integer", "array"},
                f"{name}.{field} uses an unsupported client-visible type",
            )
            if child.get("type") == "array":
                require(isinstance(child.get("items"), dict), f"{name}.{field} has no item schema")


def schema_error(schema: dict[str, Any], value: Any, path: str = "arguments") -> str | None:
    variants = schema.get("oneOf")
    if isinstance(variants, list):
        matches = [variant for variant in variants if schema_error(variant, value, path) is None]
        return None if len(matches) == 1 else f"{path} must match exactly one shape"
    kind = schema.get("type")
    if kind == "object":
        if not isinstance(value, dict):
            return f"{path} must be an object"
        properties = schema.get("properties", {})
        if schema.get("additionalProperties") is False:
            unknown = next((field for field in value if field not in properties), None)
            if unknown is not None:
                return f"{path}.{unknown} is not allowed"
        for field in schema.get("required", []):
            if field not in value:
                return f"{path}.{field} is required"
        for field, child in value.items():
            if field in properties:
                error = schema_error(properties[field], child, f"{path}.{field}")
                if error:
                    return error
        return None
    if kind == "string":
        if not isinstance(value, str):
            return f"{path} must be a string"
        if len(value) < schema.get("minLength", 0):
            return f"{path} is too short"
        if len(value) > schema.get("maxLength", len(value)):
            return f"{path} is too long"
        if "enum" in schema and value not in schema["enum"]:
            return f"{path} is not an allowed value"
        if "pattern" in schema and re.fullmatch(schema["pattern"], value) is None:
            return f"{path} does not match its pattern"
        return None
    if kind == "integer":
        if not isinstance(value, int) or isinstance(value, bool):
            return f"{path} must be an integer"
        if value < schema.get("minimum", value):
            return f"{path} is too small"
        return None
    if kind == "array":
        if not isinstance(value, list):
            return f"{path} must be an array"
        for index, item in enumerate(value):
            error = schema_error(schema["items"], item, f"{path}[{index}]")
            if error:
                return error
        return None
    return f"{path} uses an unsupported schema"


def value_from_schema(schema: dict[str, Any], context: dict[str, Any]) -> Any:
    kind = schema.get("type")
    description = str(schema.get("description", "")).lower()
    if kind == "string":
        if "enum" in schema:
            return schema["enum"][0]
        pattern = schema.get("pattern")
        if pattern == "^[0-9a-fA-F]{32}$":
            if "guarded confirmation" in description:
                return context["confirmation_request_id"]
            return context["project_id"]
        if pattern == "^sha256:[0-9a-f]{64}$":
            return context["effect_fingerprint"]
        if "user turn" in description:
            return "Explicit V08 schema-driven user turn"
        if "goal" in description:
            return "Validate discoverable MCP contracts"
        if "next meaningful step" in description:
            return "Continue Phase 7 validation"
        minimum = int(schema.get("minLength", 1))
        return "schema-client".ljust(minimum, "x")
    if kind == "integer":
        if "request revision" in description:
            return context["request_revision"]
        return int(schema.get("minimum", 0))
    if kind == "array":
        return []
    if kind == "object":
        return {
            field: value_from_schema(schema["properties"][field], context)
            for field in schema.get("required", [])
        }
    raise AssertionError(f"cannot interpret advertised schema: {schema}")


def arguments_from_schema(
    schema: dict[str, Any], context: dict[str, Any], *, fullest: bool = False
) -> dict[str, Any]:
    variants = schema_variants(schema)
    variant = max(variants, key=lambda value: len(value.get("required", []))) if fullest else variants[0]
    value = value_from_schema(variant, context)
    require(isinstance(value, dict), "advertised tool input did not produce an object")
    require(schema_error(schema, value) is None, f"schema-generated arguments are invalid: {value}")
    return value


def start_host(binary: Path, env: dict[str, str]) -> subprocess.Popen[str]:
    return subprocess.Popen(
        [str(binary)],
        cwd=ROOT,
        env=env,
        text=True,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


def stop_host(process: subprocess.Popen[str]) -> None:
    require(process.stdin is not None, "MCP stdin unavailable")
    process.stdin.close()
    return_code = process.wait(timeout=10)
    stderr = process.stderr.read() if process.stderr is not None else ""
    require(return_code == 0, f"MCP host did not clean up at EOF: {return_code}: {stderr}")
    require(process.poll() == 0, "MCP host remains live after EOF")


def initialize_host(process: subprocess.Popen[str], request_id: int) -> list[dict[str, Any]]:
    initialized = rpc(
        process,
        request_id,
        "initialize",
        {"protocolVersion": "2025-06-18", "capabilities": {}},
    )
    require(initialized["result"]["serverInfo"]["name"] == "volicord", "wrong MCP server")
    catalog = rpc(process, request_id + 1, "tools/list", {})["result"]["tools"]
    names = [entry["name"] for entry in catalog]
    require(names == EXPECTED_TOOLS, "high-level MCP catalog changed")
    return catalog


def exercise_discovered_tool_contracts(
    process: subprocess.Popen[str], catalog: list[dict[str, Any]], project_id: str,
    guarded: dict[str, Any]
) -> dict[str, Any]:
    context = {
        "project_id": project_id,
        "confirmation_request_id": guarded["confirmation_request_identity"],
        "request_revision": guarded["request_revision"],
        "effect_fingerprint": guarded["effect_fingerprint"],
    }
    by_name = {entry["name"]: entry for entry in catalog}
    request_id = 100
    for name, entry in by_name.items():
        schema = entry.get("inputSchema")
        require(isinstance(schema, dict), f"{name} has no advertised inputSchema")
        assert_concrete_schema(name, schema)

        unknown = {"unexpected_v08_field": True}
        require(schema_error(schema, unknown) is not None, f"{name} locally accepted an unknown field")
        rejected = tool_result(process, request_id, name, unknown)
        request_id += 1
        require(rejected["isError"] is True, f"{name} server accepted an unknown field")
        require(
            f"invalid {name} arguments" in rejected["structuredContent"]["error"],
            f"{name} unknown-field failure bypassed advertised validation",
        )

        variant = max(schema_variants(schema), key=lambda value: len(value.get("required", [])))
        required = variant.get("required", [])
        if required:
            complete = value_from_schema(variant, context)
            missing = dict(complete)
            missing.pop(required[-1])
            require(schema_error(schema, missing) is not None, f"{name} locally accepted a missing field")
            rejected = tool_result(process, request_id, name, missing)
            request_id += 1
            require(rejected["isError"] is True, f"{name} server accepted a missing field")
            require(
                f"invalid {name} arguments" in rejected["structuredContent"]["error"],
                f"{name} missing-field failure bypassed advertised validation",
            )

    recall_args = arguments_from_schema(by_name["recall"]["inputSchema"], context)
    recall = tool(process, request_id, "recall", recall_args)
    request_id += 1
    require(recall["project_id"] == project_id and recall["read_only"] is True, "schema-built Recall failed")

    checkpoint_args = arguments_from_schema(by_name["checkpoint_record"]["inputSchema"], context)
    checkpoint = tool(process, request_id, "checkpoint_record", checkpoint_args)
    request_id += 1
    require(checkpoint.get("checkpoint_id"), "schema-built Checkpoint failed")

    guarded_args = arguments_from_schema(
        by_name["guarded_interaction"]["inputSchema"], context, fullest=True
    )
    guarded_result = tool(process, request_id, "guarded_interaction", guarded_args)
    require(
        guarded_result["confirmation_request_id"] == context["confirmation_request_id"]
        and guarded_result["request_revision"] == context["request_revision"],
        "schema-built Guarded response lost exact identity or revision",
    )
    return {
        "advertised_tools": len(catalog),
        "schema_invalid_calls": request_id - 100,
        "schema_built_calls": ["recall", "checkpoint_record", "guarded_interaction"],
    }


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def portable_tables(path: Path) -> tuple[dict[str, list[dict[str, Any]]], dict[str, str]]:
    envelope = json.loads(path.read_text(encoding="utf-8"))
    payload = envelope["payload"]
    tables: dict[str, list[dict[str, Any]]] = {}
    for table in payload["tables"]:
        columns = table["columns"]
        rows = []
        for encoded_row in table["rows"]:
            decoded = []
            for value in encoded_row:
                decoded.append(None if value["type"] == "null" else value["value"])
            rows.append(dict(zip(columns, decoded, strict=True)))
        tables[table["name"]] = rows
    return tables, payload["lineage"]


def repository_sources(path: Path) -> list[dict[str, Any]]:
    tables, _ = portable_tables(path)
    return [row for row in tables["sources"] if row["source_kind"] == "repository_snapshot"]


def analysis_lineage(
    path: Path,
    project_id: str,
    analysis_snapshot: str,
    bundle: Path,
) -> dict[str, str]:
    analysis = json.loads(path.read_text(encoding="utf-8"))
    source = analysis["repository_source"]
    source_basis = source["basis"]
    require(analysis["identity"] == analysis_snapshot, "CLI and stored Analysis Snapshot disagree")
    require(source["project"] == project_id, "Analysis repository Source belongs to another Project")
    require(source_basis["kind"] == "snapshot", "Analysis repository Source lacks snapshot basis")
    rows = [row for row in repository_sources(bundle) if row["id"] == source["identity"]]
    require(len(rows) == 1, "Analysis repository Source is absent or ambiguous in canonical state")
    require(
        rows[0]["snapshot_basis"] == source_basis["value"],
        "Analysis repository Source basis disagrees with canonical provenance",
    )
    return {
        "analysis_snapshot": analysis["identity"],
        "repository_snapshot": analysis["repository_snapshot"],
        "repository_source": source["identity"],
        "source_basis": source_basis["value"],
    }


def assert_only_repository_observation_added(
    before: Path, after: Path, current_source: str
) -> None:
    before_tables, before_lineage = portable_tables(before)
    after_tables, after_lineage = portable_tables(after)
    require(before_tables.keys() == after_tables.keys(), "portable table set changed during analysis")
    for name in before_tables:
        if name != "sources":
            require(
                before_tables[name] == after_tables[name],
                f"analysis changed user-owned canonical table {name}",
            )
    before_sources = before_tables["sources"]
    after_sources = after_tables["sources"]
    require(
        all(row in after_sources for row in before_sources),
        "analysis rewrote or removed a historical canonical Source",
    )
    additions = [row for row in after_sources if row not in before_sources]
    require(len(additions) == 1, "analysis canonical delta was not one repository observation")
    require(
        additions[0]["id"] == current_source
        and additions[0]["source_kind"] == "repository_snapshot",
        "analysis canonical delta was not its current repository Source",
    )
    require(before_lineage != after_lineage, "portable lineage did not record canonical provenance change")


def exercise_analysis_recovery(
    cli: Path, env: dict[str, str], temporary: Path, runtime: Path
) -> dict[str, Any]:
    first_repository = temporary / "repair-repository"
    second_repository = temporary / "unrelated-repository"
    (first_repository / "src").mkdir(parents=True)
    second_repository.mkdir()
    (first_repository / "src/main.py").write_text("VALUE = 1\n", encoding="utf-8")
    (second_repository / "main.go").write_text("package main\n", encoding="utf-8")
    first = json.loads(
        run(
            [str(cli), "project", "init", "Repair Project", "--repository", str(first_repository)],
            env,
        ).stdout
    )["project_id"]
    second = json.loads(
        run(
            [str(cli), "project", "init", "Unrelated Project", "--repository", str(second_repository)],
            env,
        ).stdout
    )["project_id"]
    first_analysis = json.loads(run([str(cli), "analyze", first], env).stdout)
    second_analysis = json.loads(run([str(cli), "analyze", second], env).stdout)
    first_path = Path(first_analysis["stored_at"])
    second_path = Path(second_analysis["stored_at"])
    first_value = json.loads(first_path.read_text(encoding="utf-8"))
    second_bytes = second_path.read_bytes()

    stable_source = json.loads(run(
        [
            str(cli),
            "canonical",
            "user-source",
            first,
            "v08-recovery",
            "canonical-preservation",
            "Preserve this canonical state across derived recovery",
        ],
        env,
    ).stdout)
    run(
        [
            str(cli),
            "checkpoint",
            "record",
            first,
            "pause",
            stable_source["identity"],
            "Preserve user-owned recovery meaning",
            "Repair and reindex from fresh repository observations",
        ],
        env,
    )
    disposable_source = json.loads(run(
        [
            str(cli),
            "canonical",
            "user-source",
            first,
            "v08-recovery",
            "forgetting-state",
            "Forget this disposable source before recovery",
        ],
        env,
    ).stdout)
    run(
        [
            str(cli),
            "canonical",
            "forget",
            first,
            "source",
            disposable_source["identity"],
            stable_source["identity"],
        ],
        env,
    )
    before_bundle = temporary / "repair-before.json"
    after_repair_bundle = temporary / "repair-after.json"
    after_reindex_bundle = temporary / "reindex-after.json"
    second_bundle = temporary / "unrelated-before.json"
    run([str(cli), "portable", "export", first, str(before_bundle)], env)
    run([str(cli), "portable", "export", second, str(second_bundle)], env)
    second_bundle_bytes = second_bundle.read_bytes()
    initial_lineage = analysis_lineage(
        first_path,
        first,
        first_analysis["analysis_snapshot"],
        before_bundle,
    )
    require(
        initial_lineage["repository_snapshot"] == first_analysis["repository_snapshot"],
        "initial Repository Snapshot identity disagrees across the CLI boundary",
    )

    (first_repository / "src/repair-current.py").write_text(
        "REPAIRED_CURRENT = True\n", encoding="utf-8"
    )
    first_path.write_bytes(b"{ controlled corrupt derived analysis")
    degraded = json.loads(run([str(cli), "health", first], env).stdout)
    require(degraded["state"] == "degraded", "corrupt analysis was not observable as degraded")
    require(
        any(
            issue["kind"] == "corrupt" and issue["scope"] == f"derived_analysis:{first}"
            for issue in degraded["issues"]
        ),
        "corrupt Project analysis scope was not diagnosed",
    )

    repaired = json.loads(run([str(cli), "repair", first, "derived-analysis"], env).stdout)
    require(repaired["kind"] == "derivedanalysisrepair", "repair used the wrong recovery kind")
    require(repaired["discarded_entries"] == 1, "repair did not discard the corrupt owned entry")
    require(Path(repaired["stored_at"]).is_file(), "repair did not publish a fresh analysis")
    repaired_path = Path(repaired["stored_at"])
    repaired_value = json.loads(repaired_path.read_text(encoding="utf-8"))
    require(repaired_value, "repair output is unreadable")
    require(
        "src/repair-current.py" in json.dumps(repaired_value),
        "repair did not read current repository content",
    )
    require(json.loads(run([str(cli), "health", first], env).stdout)["state"] == "healthy", "repair did not restore health")
    run([str(cli), "portable", "export", first, str(after_repair_bundle)], env)
    repaired_lineage = analysis_lineage(
        repaired_path,
        first,
        repaired["analysis_snapshot"],
        after_repair_bundle,
    )
    require(
        repaired_lineage["repository_snapshot"] != initial_lineage["repository_snapshot"]
        and repaired_lineage["analysis_snapshot"] != initial_lineage["analysis_snapshot"]
        and repaired_lineage["repository_source"] != initial_lineage["repository_source"],
        "repair reused stale repository provenance",
    )
    assert_only_repository_observation_added(
        before_bundle, after_repair_bundle, repaired_lineage["repository_source"]
    )
    require(
        any(
            row["id"] == initial_lineage["repository_source"]
            and row["snapshot_basis"] == initial_lineage["source_basis"]
            for row in repository_sources(after_repair_bundle)
        ),
        "repair rewrote the initial repository Source instead of preserving history",
    )
    require(second_path.read_bytes() == second_bytes, "repair changed another Project's derived state")
    run([str(cli), "portable", "export", second, str(temporary / "unrelated-after-repair.json")], env)
    require(
        (temporary / "unrelated-after-repair.json").read_bytes() == second_bundle_bytes,
        "repair changed another Project's canonical state",
    )

    (first_repository / "src/current.py").write_text("CURRENT = True\n", encoding="utf-8")
    reindexed = json.loads(run([str(cli), "reindex", first], env).stdout)
    require(reindexed["kind"] == "derivedrebuild", "reindex used the wrong recovery kind")
    require(
        reindexed["analysis_snapshot"] != repaired["analysis_snapshot"],
        "reindex did not force a fresh Analysis Snapshot",
    )
    reindexed_path = Path(reindexed["stored_at"])
    require(reindexed_path.is_file(), "reindex did not publish replacement analysis")
    reindexed_value = json.loads(reindexed_path.read_text(encoding="utf-8"))
    require(
        "src/current.py" in json.dumps(reindexed_value),
        "reindex did not observe current authoritative repository input",
    )
    run([str(cli), "portable", "export", first, str(after_reindex_bundle)], env)
    reindexed_lineage = analysis_lineage(
        reindexed_path,
        first,
        reindexed["analysis_snapshot"],
        after_reindex_bundle,
    )
    require(
        reindexed_lineage["repository_snapshot"] != repaired_lineage["repository_snapshot"]
        and reindexed_lineage["repository_source"] != repaired_lineage["repository_source"],
        "reindex reused repair's repository provenance",
    )
    assert_only_repository_observation_added(
        after_repair_bundle, after_reindex_bundle, reindexed_lineage["repository_source"]
    )
    require(
        first_value["repository_source"]["identity"]
        == initial_lineage["repository_source"],
        "captured initial analysis provenance changed in memory",
    )
    require(second_path.read_bytes() == second_bytes, "reindex changed another Project's derived state")
    run([str(cli), "portable", "export", second, str(temporary / "unrelated-after-reindex.json")], env)
    require(
        (temporary / "unrelated-after-reindex.json").read_bytes() == second_bundle_bytes,
        "reindex changed another Project's canonical state",
    )

    analysis_root = runtime / "derived" / "analysis"
    project_analysis = analysis_root / first
    require(
        [path for path in project_analysis.iterdir() if path.is_file()] == [reindexed_path],
        "Project derived replacement did not leave exactly one current snapshot",
    )
    require(
        all(path.is_dir() for path in analysis_root.iterdir()),
        "recovery required an old top-level derived-storage file path",
    )
    unsupported_before = (
        after_reindex_bundle.read_bytes(),
        reindexed_path.read_bytes(),
        second_path.read_bytes(),
        second_bundle_bytes,
    )
    unsupported = run([str(cli), "repair", first, "canonical"], env, expected=1)
    require("unsupported repair scope" in unsupported.stderr, "unsupported repair appeared successful")
    run([str(cli), "portable", "export", first, str(temporary / "unsupported-after.json")], env)
    run([str(cli), "portable", "export", second, str(temporary / "unrelated-after-unsupported.json")], env)
    unsupported_after = (
        (temporary / "unsupported-after.json").read_bytes(),
        reindexed_path.read_bytes(),
        second_path.read_bytes(),
        (temporary / "unrelated-after-unsupported.json").read_bytes(),
    )
    require(unsupported_before == unsupported_after, "unsupported repair mutated owned state")
    return {
        "canonical_delta": "repository observations only",
        "cross_project_isolation": True,
        "initial_lineage": initial_lineage,
        "repair_lineage": repaired_lineage,
        "reindex_lineage": reindexed_lineage,
        "user_owned_canonical_meaning_preserved": True,
        "unsupported_scope_rejected": True,
    }


def main() -> int:
    codex = shutil.which("codex")
    if codex is None:
        print(json.dumps({"status": "skipped", "reason": "real codex executable unavailable"}))
        return 77

    base_env = os.environ.copy()
    original_home = Path.home()
    with tempfile.TemporaryDirectory(prefix="volicord-v08-") as directory:
        temporary = Path(directory)
        home = temporary / "home"
        prefix = temporary / "prefix"
        runtime = temporary / "runtime"
        repository = temporary / "repository"
        legacy = temporary / "legacy-runtime"
        codex_home = home / ".codex"
        for path in (home, repository, legacy, codex_home):
            path.mkdir(parents=True)
        (repository / "README.md").write_text("# V08 clean repository\n", encoding="utf-8")
        legacy_sentinel = legacy / "DO-NOT-READ"
        legacy_sentinel.write_text("legacy sentinel\n", encoding="utf-8")
        legacy_before = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
        incompatible_host = temporary / "legacy-volicord-host"
        incompatible_host.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
        incompatible_host.chmod(0o700)

        env = base_env | {
            "HOME": str(home),
            "XDG_DATA_HOME": str(home / ".local/share"),
            "CODEX_HOME": str(codex_home),
            "VOLICORD_HOME": str(legacy),
            "VOLICORD_RUNTIME_DIR": str(runtime),
            "PATH": f"{prefix / 'bin'}:{base_env.get('PATH', '')}",
        }
        env.setdefault("CARGO_HOME", str(original_home / ".cargo"))
        env.setdefault("RUSTUP_HOME", str(original_home / ".rustup"))
        require(not runtime.exists(), "replacement runtime existed before install")

        version = run([codex, "--version"], env).stdout.strip()
        require(version.startswith("codex-cli "), "unexpected Codex executable")
        run(
            [
                codex,
                "mcp",
                "add",
                "volicord",
                "--env",
                f"VOLICORD_HOME={legacy}",
                "--env",
                "VOLICORD_LEGACY_MARKER=incompatible-registration",
                "--",
                str(incompatible_host),
                "_host-launch",
                "legacy.sock",
            ],
            env,
        )
        incompatible_registration = json.loads(
            run([codex, "mcp", "get", "volicord", "--json"], env).stdout
        )
        require_codex_stdio_registration(
            incompatible_registration,
            command=incompatible_host,
            arguments=["_host-launch", "legacy.sock"],
            environment={
                "VOLICORD_HOME": str(legacy),
                "VOLICORD_LEGACY_MARKER": "incompatible-registration",
            },
            context="pre-install incompatible Codex registration",
        )
        install = run(
            [
                str(INSTALLER),
                "--prefix",
                str(prefix),
                "--runtime-dir",
                str(runtime),
                "--setup-codex",
            ],
            env,
        )
        require("Installed Volicord executables" in install.stdout, "install result missing")

        binaries = [prefix / "bin" / name for name in ("volicord", "volicord-viewer", "volicord-mcp")]
        for binary in binaries:
            mode = binary.stat().st_mode
            require(mode & stat.S_IXUSR != 0, f"binary is not owner-executable: {binary.name}")
            require(os.access(binary, os.X_OK), f"binary is not executable: {binary.name}")
            require(shutil.which(binary.name, path=env["PATH"]) == str(binary), f"PATH misses {binary.name}")
        runtime_files = {path.name for path in runtime.iterdir()}
        require(
            {"canonical.sqlite3", "candidates.sqlite3", "privacy.sqlite3", "guarded.sqlite3"}
            <= runtime_files,
            "clean replacement runtime schemas were not initialized",
        )

        codex_get = json.loads(run([codex, "mcp", "get", "volicord", "--json"], env).stdout)
        require_codex_stdio_registration(
            codex_get,
            command=prefix / "bin" / "volicord-mcp",
            arguments=[],
            environment={"VOLICORD_RUNTIME_DIR": str(runtime)},
            context="replacement Codex registration",
        )
        codex_list = json.loads(run([codex, "mcp", "list", "--json"], env).stdout)
        listed_registrations = [entry for entry in codex_list if entry.get("name") == "volicord"]
        require(len(listed_registrations) == 1, "Codex did not discover exactly one Volicord entry")
        require_codex_stdio_registration(
            listed_registrations[0],
            command=prefix / "bin" / "volicord-mcp",
            arguments=[],
            environment={"VOLICORD_RUNTIME_DIR": str(runtime)},
            context="listed replacement Codex registration",
        )

        cli = prefix / "bin" / "volicord"
        initialized = json.loads(
            run(
                [str(cli), "project", "init", "V08 Project", "--repository", str(repository)],
                env,
            ).stdout
        )
        project_id = initialized["project_id"]
        require(initialized["binding"]["path"] == str(repository.resolve()), "Project binding mismatch")

        host = start_host(prefix / "bin" / "volicord-mcp", env)
        catalog = initialize_host(host, 1)
        health = tool(host, 3, "project_health", {"project_id": project_id})
        require(health["connection"] == "connected", "MCP connection not reported connected")
        require(health["capability_state"] == "healthy", "clean Project is not healthy")
        recall = tool(host, 4, "recall", {"project_id": project_id})
        require(recall["project_id"] == project_id and recall["read_only"] is True, "Recall mismatch")
        checkpoint = tool(
            host,
            5,
            "checkpoint_record",
            {
                "project_id": project_id,
                "user_turn": "Pause the V08 clean integration journey",
                "goal": "Validate Linux and Codex integration",
                "next_step": "Restart the host",
                "known_limits": ["V11 remains independent"],
            },
        )
        require(checkpoint.get("checkpoint_id"), "Checkpoint call did not create identity")

        expiration = str(time.time_ns() // 1_000 + 600_000_000)
        guarded = json.loads(
            run(
                [
                    str(cli),
                    "guarded",
                    "request",
                    project_id,
                    "external-publication",
                    "publish schema fixture",
                    "registry/schema-fixture",
                    "publish a public schema fixture",
                    "public artifact",
                    expiration,
                    "release:schema-fixture",
                ],
                env,
            ).stdout
        )
        schema_evidence = exercise_discovered_tool_contracts(host, catalog, project_id, guarded)
        stop_host(host)

        restarted = start_host(prefix / "bin" / "volicord-mcp", env)
        initialize_host(restarted, 10)
        restarted_health = tool(restarted, 12, "project_health", {"project_id": project_id})
        require(restarted_health["capability_state"] == "healthy", "restart did not reconnect")
        stop_host(restarted)

        unavailable_repository = temporary / "repository-unavailable"
        repository.rename(unavailable_repository)
        degraded_host = start_host(prefix / "bin" / "volicord-mcp", env)
        initialize_host(degraded_host, 20)
        degraded = tool(degraded_host, 22, "project_health", {"project_id": project_id})
        require(degraded["connection"] == "connected", "degradation was misreported as disconnect")
        require(degraded["capability_state"] == "degraded", "missing repository not degraded")
        require(degraded["repository_available"] is False, "missing repository reported available")
        stop_host(degraded_host)
        unavailable_repository.rename(repository)

        try:
            subprocess.Popen([str(temporary / "missing-mcp")], env=env)
        except FileNotFoundError:
            connection_failure = "launch_failed"
        else:
            raise AssertionError("missing MCP executable unexpectedly launched")

        recall_before = json.loads(run([str(cli), "recall", project_id], env).stdout)
        canonical = runtime / "canonical.sqlite3"
        canonical_size = canonical.stat().st_size
        run(
            [
                str(INSTALLER),
                "--prefix",
                str(prefix),
                "--runtime-dir",
                str(runtime),
                "--uninstall",
            ],
            env,
        )
        require(not any(binary.exists() for binary in binaries), "uninstall left a product binary")
        require(canonical.exists() and canonical.stat().st_size == canonical_size, "uninstall changed canonical data")
        removed_registration = run([codex, "mcp", "get", "volicord", "--json"], env, expected=1)
        require(removed_registration.returncode == 1, "uninstall left Codex registration")

        run(
            [
                str(INSTALLER),
                "--prefix",
                str(prefix),
                "--runtime-dir",
                str(runtime),
                "--setup-codex",
            ],
            env,
        )
        recall_after = json.loads(run([str(cli), "recall", project_id], env).stdout)
        require(recall_after == recall_before, "reinstall changed canonical Recall")
        reinstalled_registration = json.loads(
            run([codex, "mcp", "get", "volicord", "--json"], env).stdout
        )
        require_codex_stdio_registration(
            reinstalled_registration,
            command=prefix / "bin" / "volicord-mcp",
            arguments=[],
            environment={"VOLICORD_RUNTIME_DIR": str(runtime)},
            context="reinstalled Codex registration",
        )

        recovery_evidence = exercise_analysis_recovery(cli, env, temporary, runtime)

        legacy_after = (legacy_sentinel.stat().st_mtime_ns, sha256(legacy_sentinel))
        require(legacy_after == legacy_before, "clean journey touched the legacy runtime sentinel")

        print(
            json.dumps(
                {
                    "binaries": [binary.name for binary in binaries],
                    "codex": version,
                    "codex_registration": "discovered",
                    "connection_failure": connection_failure,
                    "degraded_capability": degraded["capability_state"],
                    "legacy_runtime": "untouched",
                    "mcp_tools": len(catalog),
                    "mcp_schema_contract": schema_evidence,
                    "process_cleanup": "passed",
                    "project_id": project_id,
                    "repair_reindex": recovery_evidence,
                    "reinstall_preserved_recall": True,
                    "runtime_schemas": sorted(runtime_files),
                    "stale_codex_registration_replaced": True,
                    "status": "passed",
                },
                indent=2,
                sort_keys=True,
            )
        )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"V08 harness failed: {error}", file=sys.stderr)
        raise SystemExit(1)
