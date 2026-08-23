#!/usr/bin/env python3
"""Strict bounded Volicord CLI fake for validation self-tests.

This fake deliberately models only the current argv shapes exercised by the
Dogfood campaign and repeated-resource rehearsal. Any other shape is a usage
error so removed commands cannot produce a false successful self-test.
"""

from __future__ import annotations

import json
import os
from pathlib import Path
import sys


DOCUMENT_KINDS = {
    "project-architecture-guide",
    "decision-report",
    "implementation-plan",
    "handoff-resume",
}


def usage(message: str) -> int:
    print(f"strict fake CLI usage error: {message}", file=sys.stderr)
    return 2


def parse_globals(arguments: list[str]) -> tuple[dict[str, object], list[str]]:
    options: dict[str, object] = {}
    index = 0
    while index < len(arguments) and arguments[index].startswith("--"):
        option = arguments[index]
        if option in {"--runtime", "--repository", "--project", "--locale"}:
            if option in options or index + 1 >= len(arguments):
                raise ValueError(f"missing or duplicate global option {option}")
            options[option] = arguments[index + 1]
            index += 2
        elif option == "--json":
            if option in options:
                raise ValueError("duplicate global option --json")
            options[option] = True
            index += 1
        else:
            raise ValueError(f"unsupported global option {option}")
    return options, arguments[index:]


def require_selection(options: dict[str, object]) -> None:
    if "--repository" not in options and "--project" not in options:
        raise ValueError("a repository or Project selection is required")


def parse_document(arguments: list[str]) -> dict[str, str]:
    if len(arguments) != 8 or arguments[0] != "export":
        raise ValueError("document export shape mismatch")
    kind = arguments[1]
    if kind not in DOCUMENT_KINDS:
        raise ValueError(f"unsupported document kind {kind}")
    if arguments[2] != "--format" or arguments[3] not in {"markdown", "html"}:
        raise ValueError("document format shape mismatch")
    if arguments[4] != "--output" or arguments[6] != "--language":
        raise ValueError("document option ordering mismatch")
    if not arguments[5] or not arguments[7]:
        raise ValueError("document output and language are required")
    return {
        "kind": kind,
        "format": arguments[3],
        "output": arguments[5],
        "language": arguments[7],
    }


def parse(arguments: list[str]) -> tuple[dict[str, object], str, dict[str, str]]:
    options, command = parse_globals(arguments)
    if not command:
        raise ValueError("missing top-level command")
    require_selection(options)
    top = command[0]
    rest = command[1:]
    details: dict[str, str] = {}
    if top == "codex" and rest in (["enable"], ["disable"]):
        details["action"] = rest[0]
    elif top == "context" and len(rest) == 3 and rest[:2] == ["export", "--output"]:
        details["output"] = rest[2]
    elif top == "document":
        details = parse_document(rest)
    elif top == "analyze" and not rest:
        pass
    elif top == "doctor" and rest in (["repair"], ["reindex"]):
        details["action"] = rest[0]
    else:
        raise ValueError(f"unsupported command shape: {' '.join(command)}")
    return options, top, details


def main() -> int:
    try:
        _options, command, details = parse(sys.argv[1:])
    except ValueError as error:
        return usage(str(error))
    if command == "codex":
        print(json.dumps({"project_trust": "user_controlled", "changed": True}))
    elif command == "document":
        destination = Path(details["output"])
        try:
            with destination.open("xb") as output:
                output.write(b"fixed no-replace document\n")
        except FileExistsError:
            return 17
        if os.environ.get("PHASE8_FAKE_DOCUMENT_FAIL_AFTER_CREATE") == "1":
            return 19
        print(json.dumps({"operation": "document_export"}))
    else:
        print("{}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
