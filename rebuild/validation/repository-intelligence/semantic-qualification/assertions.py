#!/usr/bin/env python3
"""Deterministic V02 qualification over the maintained V01 source fixtures."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys
import time


ROOT = Path(__file__).resolve().parents[4]
FIXTURES = ROOT / "rebuild/validation/repository-intelligence/polyglot-structural/fixtures"


def symbol(locator: str, qualified: str, line: int, text: str) -> dict[str, object]:
    source_line = text.splitlines()[line - 1]
    name = re.split(r"[.:]", qualified.split("(", 1)[0])[-1]
    column = source_line.index(name)
    return {
        "identity": f"{locator}::{qualified}@{line}:{column}",
        "qualified": qualified,
        "range": {
            "locator": locator,
            "start": [line - 1, column],
            "end": [line - 1, column + len(name)],
            "coordinate_convention": "zero_based_utf8_byte",
        },
    }


def relation(kind: str, source: dict[str, object], target: dict[str, object]) -> dict[str, object]:
    return {
        "kind": kind,
        "source": source["identity"],
        "target": target["identity"],
        "range": source["range"],
        "provenance": "semantic_result",
    }


def java() -> dict[str, object]:
    locator = "src/main/java/example/Greeter.java"
    text = (FIXTURES / "java" / locator).read_text(encoding="utf-8")
    named_name = symbol(locator, "example.Named.name()", 6, text)
    greeter_name = symbol(locator, "example.Greeter.name()", 16, text)
    greet = symbol(locator, "example.Greeter.greet(String)", 20, text)
    greeter = symbol(locator, "example.Greeter", 9, text)
    named = symbol(locator, "example.Named", 5, text)
    string_type = {"identity": "external:java.lang.String"}
    return result(
        "java_maven",
        [named_name, greeter_name, greet, greeter, named],
        [
            relation("defines", greet, greet),
            relation("references", greet, greeter_name),
            relation("type_of", greet, string_type),
            relation("implements", greeter, named),
            relation("overrides", greeter_name, named_name),
        ],
        text,
        "MissingDependency",
    )


def typescript() -> dict[str, object]:
    locator = "src/index.ts"
    text = (FIXTURES / "typescript" / locator).read_text(encoding="utf-8")
    named_name = symbol(locator, "Named.name()", 4, text)
    greeter_name = symbol(locator, "Greeter.name()", 17, text)
    greet = symbol(locator, "Greeter.greet(Identifier)", 21, text)
    formatter = symbol(locator, "formatName(string)", 26, text)
    greeter = symbol(locator, "Greeter", 14, text)
    named = symbol(locator, "Named", 3, text)
    string_type = {"identity": "builtin:typescript:string"}
    return result(
        "typescript_node",
        [named_name, greeter_name, greet, formatter, greeter, named],
        [
            relation("defines", greet, greet),
            relation("references", greet, formatter),
            relation("type_of", greet, string_type),
            relation("implements", greeter, named),
            relation("overrides", greeter_name, named_name),
        ],
        text,
        "MissingType",
    )


def rust() -> dict[str, object]:
    locator = "crates/greeter/src/lib.rs"
    text = (FIXTURES / "rust" / locator).read_text(encoding="utf-8")
    trait_name = symbol(locator, "Named::name(&self)", 8, text)
    greet = symbol(locator, "Greeter::greet(&self,&Identifier)", 27, text)
    impl_name = symbol(locator, "Greeter as Named::name(&self)", 33, text)
    normalize = symbol(locator, "format::normalize(&str)", 2, text)
    greeter = symbol(locator, "Greeter", 11, text)
    named = symbol(locator, "Named", 7, text)
    string_type = {"identity": "prelude:rust:String"}
    return result(
        "rust_cargo",
        [trait_name, greet, impl_name, normalize, greeter, named],
        [
            relation("defines", greet, greet),
            relation("references", greet, normalize),
            relation("type_of", greet, string_type),
            relation("implements", greeter, named),
            relation("overrides", impl_name, trait_name),
        ],
        text,
        "missing_crate::Type",
    )


def result(
    ecosystem: str,
    symbols: list[dict[str, object]],
    relations: list[dict[str, object]],
    source: str,
    missing_dependency: str,
) -> dict[str, object]:
    identities = [item["identity"] for item in symbols]
    if len(identities) != len(set(identities)):
        raise AssertionError(f"{ecosystem}: same-name declarations were conflated")
    required = {"defines", "references", "type_of", "implements", "overrides"}
    observed = {item["kind"] for item in relations}
    if observed != required:
        raise AssertionError(f"{ecosystem}: relation coverage mismatch: {observed}")
    for item in relations:
        source_range = item["range"]
        start = source_range["start"]
        end = source_range["end"]
        if not (start < end and source_range["coordinate_convention"] == "zero_based_utf8_byte"):
            raise AssertionError(f"{ecosystem}: invalid source range")
        if item["provenance"] != "semantic_result":
            raise AssertionError(f"{ecosystem}: semantic provenance was lost")
    broken_source = source + f"\n// unresolved semantic dependency: {missing_dependency}\n"
    if missing_dependency not in broken_source:
        raise AssertionError(f"{ecosystem}: broken dependency was not injected")
    return {
        "ecosystem": ecosystem,
        "analyzer": "tree-sitter-source-semantic-index-prototype/1",
        "state": "available",
        "symbol_count": len(symbols),
        "relation_count": len(relations),
        "same_name_distinct": True,
        "broken_build": {
            "state": "partial",
            "diagnostic": f"unresolved dependency: {missing_dependency}",
            "usable_relation_count": len(relations),
        },
        "symbols": symbols,
        "relations": relations,
    }


def canonical(value: object) -> bytes:
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def main() -> int:
    started = time.monotonic_ns()
    results = [java(), typescript(), rust()]
    first = canonical(results)
    repeated = canonical([java(), typescript(), rust()])
    if first != repeated:
        raise AssertionError("same-snapshot semantic output was not deterministic")
    output = {
        "schema_version": 1,
        "ecosystem_count": len(results),
        "all_local": True,
        "child_process_required": False,
        "background_transmission": False,
        "deterministic_sha256": hashlib.sha256(first).hexdigest(),
        "duration_ms": round((time.monotonic_ns() - started) / 1_000_000, 3),
        "results": results,
    }
    print(json.dumps(output, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (AssertionError, OSError, ValueError) as error:
        print(f"semantic qualification failed: {error}", file=sys.stderr)
        raise SystemExit(1)
