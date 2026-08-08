#!/usr/bin/env python3
"""Disposable V01 structural normalization prototype.

This experiment deliberately uses only the Python standard library. It tests a
common lexical framework plus language adapters, and optionally replaces the
Python adapter with the native ``ast`` parser to exercise a hybrid boundary.
It is not production Repository Intelligence code.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import os
from pathlib import Path
import re
import resource
import shutil
import subprocess
import sys
import tempfile
import time
from typing import Any, Iterable


REPOSITORY_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_FIXTURE_ROOT = REPOSITORY_ROOT / "rebuild" / "validation" / "fixtures" / "v01"
LANGUAGE_SUFFIXES = {
    ".java": "java",
    ".py": "python",
    ".js": "javascript",
    ".ts": "typescript",
    ".c": "c",
    ".h": "c",
    ".cpp": "cpp",
    ".cc": "cpp",
    ".cxx": "cpp",
    ".hpp": "cpp",
    ".hh": "cpp",
    ".rs": "rust",
    ".go": "go",
}
CONFIG_NAMES = {
    "Cargo.toml",
    "CMakeLists.txt",
    "go.mod",
    "package.json",
    "pom.xml",
    "pyproject.toml",
    "system.json",
    "tsconfig.json",
}
GATE_LANGUAGES = ("java", "python", "javascript", "typescript", "c", "cpp", "rust")
DECLARATION_KINDS = {
    "class",
    "interface",
    "trait",
    "struct",
    "enum",
    "type",
    "function",
    "method",
    "field",
    "test",
}


def sha256_bytes(content: bytes) -> str:
    return hashlib.sha256(content).hexdigest()


def directory_hash(directory: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(item for item in directory.rglob("*") if item.is_file()):
        relative = path.relative_to(directory).as_posix().encode("utf-8")
        content = path.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def stable_id(fixture: str, path: str, kind: str, qualified_name: str) -> str:
    basis = "\0".join((fixture, path, kind, qualified_name)).encode("utf-8")
    return f"v01:{hashlib.sha256(basis).hexdigest()[:24]}"


def line_range(lines: list[str], start: int, end: int | None = None) -> dict[str, int]:
    actual_end = start if end is None else end
    end_column = len(lines[actual_end - 1].rstrip("\n")) + 1 if lines else 1
    return {"start_line": start, "start_column": 1, "end_line": actual_end, "end_column": end_column}


def brace_end(lines: list[str], start_line: int) -> tuple[int, bool]:
    depth = 0
    seen = False
    for line_number in range(start_line, len(lines) + 1):
        line = lines[line_number - 1]
        for character in line:
            if character == "{":
                depth += 1
                seen = True
            elif character == "}":
                depth -= 1
                if seen and depth == 0:
                    return line_number, True
    return len(lines), not seen


def entity(
    fixture: str,
    language: str,
    path: str,
    kind: str,
    name: str,
    qualified_name: str,
    source_range: dict[str, int],
    fact_kind: str = "parser_confirmed",
    extensions: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "id": stable_id(fixture, path, kind, qualified_name),
        "fixture_id": fixture,
        "language": language,
        "path": path,
        "kind": kind,
        "name": name,
        "qualified_name": qualified_name,
        "range": source_range,
        "fact_kind": fact_kind,
        "extensions": extensions or {},
    }


def relation(
    fixture: str,
    language: str,
    path: str,
    kind: str,
    source: str,
    target: str,
    line: int,
    fact_kind: str = "parser_confirmed",
) -> dict[str, Any]:
    identity = "\0".join((fixture, language, path, kind, source, target, str(line)))
    return {
        "id": f"v01r:{hashlib.sha256(identity.encode('utf-8')).hexdigest()[:24]}",
        "fixture_id": fixture,
        "language": language,
        "path": path,
        "kind": kind,
        "source": source,
        "target": target,
        "line": line,
        "fact_kind": fact_kind,
    }


def text_without_line_comment(line: str) -> str:
    return re.sub(r"//.*$", "", line)


def parse_python(
    fixture: str, relative: str, source: str, native_ast: bool
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[str], str]:
    lines = source.splitlines(keepends=True)
    module_name = relative.removesuffix(".py").replace("/", ".")
    entities = [entity(fixture, "python", relative, "module", module_name, module_name, line_range(lines, 1, max(1, len(lines))))]
    relations: list[dict[str, Any]] = []
    unsupported = ["dynamic dispatch and runtime monkey-patching are not resolved"]
    status = "available"
    if not native_ast:
        class_ranges: list[tuple[int, int, str]] = []
        for number, line in enumerate(lines, start=1):
            class_match = re.match(r"class\s+(\w+)", line)
            if class_match:
                name = class_match.group(1)
                end = len(lines)
                for later in range(number + 1, len(lines) + 1):
                    candidate = lines[later - 1]
                    if candidate.strip() and not candidate.startswith((" ", "\t")):
                        end = later - 1
                        break
                class_ranges.append((number, end, name))
                entities.append(entity(fixture, "python", relative, "class", name, f"{module_name}.{name}", line_range(lines, number, end)))
            function_match = re.match(r"(\s*)def\s+(\w+)\s*\(", line)
            if function_match:
                indentation, name = function_match.groups()
                owner = next((item[2] for item in class_ranges if item[0] < number <= item[1]), None)
                kind = "test" if name.startswith("test_") or "/tests/" in f"/{relative}" else "method" if indentation else "function"
                qualified = f"{module_name}.{owner}.{name}" if owner else f"{module_name}.{name}"
                entities.append(entity(fixture, "python", relative, kind, name, qualified, line_range(lines, number)))
            import_match = re.match(r"(?:from\s+([\w.]+)\s+)?import\s+([\w.]+)", line)
            if import_match:
                target = import_match.group(1) or import_match.group(2)
                relations.append(relation(fixture, "python", relative, "imports", module_name, target, number))
            field_match = re.search(r"self\.(\w+)\s*=", line)
            if field_match:
                owner = next((item[2] for item in class_ranges if item[0] < number <= item[1]), None)
                if owner:
                    name = field_match.group(1)
                    entities.append(entity(fixture, "python", relative, "field", name, f"{module_name}.{owner}.{name}", line_range(lines, number)))
        return entities, relations, unsupported, status

    try:
        tree = ast.parse(source, filename=relative)
    except SyntaxError as error:
        return entities, relations, unsupported + [f"partial parse after syntax error at line {error.lineno}"], "partial"
    parents: dict[ast.AST, ast.AST] = {}
    for parent in ast.walk(tree):
        for child in ast.iter_child_nodes(parent):
            parents[child] = parent
    for node in ast.walk(tree):
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            targets = [alias.name for alias in node.names]
            if isinstance(node, ast.ImportFrom) and node.module:
                targets = [node.module]
            for target in targets:
                relations.append(relation(fixture, "python", relative, "imports", module_name, target, node.lineno))
        elif isinstance(node, ast.ClassDef):
            qualified = f"{module_name}.{node.name}"
            entities.append(entity(fixture, "python", relative, "class", node.name, qualified, line_range(lines, node.lineno, node.end_lineno)))
            for base in node.bases:
                target = ast.unparse(base)
                relations.append(relation(fixture, "python", relative, "inherits", qualified, target, node.lineno))
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            parent = parents.get(node)
            owner = parent.name if isinstance(parent, ast.ClassDef) else None
            kind = "test" if node.name.startswith("test_") or "/tests/" in f"/{relative}" else "method" if owner else "function"
            qualified = f"{module_name}.{owner}.{node.name}" if owner else f"{module_name}.{node.name}"
            entities.append(entity(fixture, "python", relative, kind, node.name, qualified, line_range(lines, node.lineno, node.end_lineno), extensions={"async": isinstance(node, ast.AsyncFunctionDef)}))
            for child in ast.walk(node):
                if isinstance(child, ast.Call):
                    relations.append(relation(fixture, "python", relative, "calls_syntactically", qualified, ast.unparse(child.func), child.lineno))
        elif isinstance(node, (ast.Assign, ast.AnnAssign)):
            parent = parents.get(node)
            function_parent = parent
            while function_parent is not None and not isinstance(function_parent, (ast.FunctionDef, ast.AsyncFunctionDef)):
                function_parent = parents.get(function_parent)
            targets = node.targets if isinstance(node, ast.Assign) else [node.target]
            for target_node in targets:
                if isinstance(target_node, ast.Attribute) and isinstance(target_node.value, ast.Name) and target_node.value.id == "self":
                    owner_node = parents.get(function_parent) if function_parent else None
                    if isinstance(owner_node, ast.ClassDef):
                        qualified = f"{module_name}.{owner_node.name}.{target_node.attr}"
                        entities.append(entity(fixture, "python", relative, "field", target_node.attr, qualified, line_range(lines, node.lineno)))
    return entities, relations, unsupported, status


def parse_brace_language(
    fixture: str, language: str, relative: str, source: str
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[str], str]:
    lines = source.splitlines(keepends=True)
    entities: list[dict[str, Any]] = []
    relations: list[dict[str, Any]] = []
    unsupported_by_language = {
        "java": "annotation processing, reflection, and generated sources are not resolved",
        "javascript": "dynamic properties, prototype mutation, and runtime module resolution are not resolved",
        "typescript": "type-level evaluation and declaration merging are not resolved",
        "c": "macro expansion and conditional compilation are not resolved",
        "cpp": "template instantiation, macros, and conditional compilation are not resolved",
        "rust": "macro expansion, cfg-disabled code, and trait resolution are not resolved",
    }
    unsupported = [unsupported_by_language[language]]
    stripped_source = "\n".join(text_without_line_comment(line) for line in lines)
    status = "partial" if stripped_source.count("{") != stripped_source.count("}") else "available"
    if status == "partial":
        unsupported.append("unbalanced braces produced a partial structural result")

    package = ""
    namespace_ranges: list[tuple[int, int, str]] = []
    type_ranges: list[tuple[int, int, str, str]] = []
    function_ranges: list[tuple[int, int, str]] = []

    for number, original_line in enumerate(lines, start=1):
        line = text_without_line_comment(original_line).strip()
        if not line:
            continue
        if language == "java":
            match = re.match(r"package\s+([\w.]+)\s*;", line)
            if match:
                package = match.group(1)
                entities.append(entity(fixture, language, relative, "package", package, package, line_range(lines, number)))
            match = re.match(r"import\s+(?:static\s+)?([\w.*]+)\s*;", line)
            if match:
                relations.append(relation(fixture, language, relative, "imports", package or relative, match.group(1), number))
        elif language in {"javascript", "typescript"}:
            match = re.match(r"import\s+.+?\s+from\s+[\"']([^\"']+)[\"']", line)
            if match:
                relations.append(relation(fixture, language, relative, "imports", relative, match.group(1), number))
            match = re.match(r"export\s+.+?from\s+[\"']([^\"']+)[\"']", line)
            if match:
                relations.append(relation(fixture, language, relative, "exports", relative, match.group(1), number))
        elif language in {"c", "cpp"}:
            match = re.match(r"#\s*include\s*[<\"]([^>\"]+)[>\"]", line)
            if match:
                relations.append(relation(fixture, language, relative, "includes", relative, match.group(1), number))
        elif language == "rust":
            match = re.match(r"(?:pub\s+)?use\s+([^;]+);", line)
            if match:
                relations.append(relation(fixture, language, relative, "imports", relative, match.group(1).strip(), number))

        if language == "cpp":
            match = re.match(r"namespace\s+(\w+)\s*\{", line)
            if match:
                end, _ = brace_end(lines, number)
                namespace_ranges.append((number, end, match.group(1)))
                entities.append(entity(fixture, language, relative, "namespace", match.group(1), match.group(1), line_range(lines, number, end)))
        if language == "rust":
            match = re.match(r"(?:pub\s+)?mod\s+(\w+)\s*\{", line)
            if match:
                end, _ = brace_end(lines, number)
                name = match.group(1)
                entities.append(entity(fixture, language, relative, "module", name, name, line_range(lines, number, end)))

        type_match: re.Match[str] | None = None
        kind = ""
        if language in {"java", "javascript", "typescript"}:
            type_match = re.search(r"\b(class|interface|enum|type)\s+(\w+)", line)
            if type_match:
                kind = type_match.group(1)
        elif language in {"c", "cpp"}:
            type_match = re.search(r"\b(?:typedef\s+)?(struct|enum|class)\s+(?:class\s+)?(\w+)", line)
            if type_match:
                kind = type_match.group(1)
        elif language == "rust":
            type_match = re.search(r"\b(trait|struct|enum|type)\s+(\w+)", line)
            if type_match:
                kind = type_match.group(1)
        if language == "cpp" and re.match(r"using\s+(\w+)\s*=", line):
            using = re.match(r"using\s+(\w+)\s*=", line)
            assert using is not None
            name = using.group(1)
            entities.append(entity(fixture, language, relative, "type", name, name, line_range(lines, number)))
        if type_match:
            name = type_match.group(2)
            owner = next((item[2] for item in namespace_ranges if item[0] <= number <= item[1]), package)
            qualified = f"{owner}.{name}" if owner else name
            end, _ = brace_end(lines, number)
            if kind == "type" and "{" not in line:
                end = number
            else:
                type_ranges.append((number, end, qualified, kind))
            entities.append(entity(fixture, language, relative, kind, name, qualified, line_range(lines, number, end)))
            inheritance = re.search(r"\bextends\s+([\w.]+)", line)
            implementation = re.search(r"\bimplements\s+([\w., ]+)", line)
            cpp_base = re.search(r":\s*(?:public\s+)?(\w+)", line) if language == "cpp" else None
            if inheritance:
                relations.append(relation(fixture, language, relative, "inherits", qualified, inheritance.group(1), number))
            if implementation:
                for target in implementation.group(1).split(","):
                    relations.append(relation(fixture, language, relative, "implements", qualified, target.strip(), number))
            if cpp_base:
                relations.append(relation(fixture, language, relative, "inherits", qualified, cpp_base.group(1), number))
        if language == "rust":
            implementation = re.match(r"impl(?:\s+(\w+)\s+for)?\s+(\w+)\s*\{", line)
            if implementation:
                end, _ = brace_end(lines, number)
                implemented_trait, implemented_type = implementation.groups()
                type_ranges.append((number, end, implemented_type, "impl"))
                if implemented_trait:
                    relations.append(relation(fixture, language, relative, "implements", implemented_type, implemented_trait, number))

        function_match: re.Match[str] | None = None
        inside_function = any(start < number <= end for start, end, _ in function_ranges)
        if language == "java" and not inside_function:
            function_match = re.search(r"(?:[\w<>\[\].]+\s+)+(\w+)\s*\([^;]*\)\s*(?:\{|;)", line)
            owner_range = next((item for item in reversed(type_ranges) if item[0] <= number <= item[1]), None)
            if function_match is None and owner_range:
                constructor_name = re.escape(owner_range[2].split(".")[-1])
                constructor = re.match(rf"({constructor_name})\s*\(", line)
                function_match = constructor
        elif language in {"javascript", "typescript"} and not inside_function:
            ending = r"(?:\{|;)" if language == "typescript" else r"\{"
            function_match = re.search(r"(?:function\s+)?(\w+)\s*\([^)]*\)\s*(?::\s*[^={;]+)?\s*" + ending, line)
            if re.match(r"(?:if|for|while|switch|catch)\b", line):
                function_match = None
        elif language == "c" and not inside_function:
            function_match = re.search(r"(?:[A-Za-z_]\w*[\s*]+)+(\w+)\s*\([^;]*\)\s*(?:\{|;)", line)
        elif language == "cpp" and not inside_function:
            function_match = re.match(r"(?:[~\w:<>&*]+\s+)*([~\w]+::[~\w]+)\s*\([^;]*\)\s*(?:const\s*)?(?:override\s*)?(?::[^{}]*)?(?:\{|;)", line)
            if function_match is None:
                function_match = re.match(r"(?:[~\w:<>&*]+\s+)+([~\w]+)\s*\([^;]*\)\s*(?:const\s*)?(?:override\s*)?(?:\{|;)", line)
        elif language == "rust" and not inside_function:
            function_match = re.search(r"\bfn\s+(\w+)\s*\([^)]*\)", line)
        if function_match:
            name = function_match.group(1).split("::")[-1]
            if name not in {"if", "for", "while", "switch", "return", "sizeof"}:
                owner_range = next((item for item in reversed(type_ranges) if item[0] <= number <= item[1]), None)
                owner = owner_range[2] if owner_range else package or relative
                is_test = (
                    name.startswith("test_")
                    or name.startswith("test")
                    or "/test" in f"/{relative}"
                    or (language == "rust" and any("#[test]" in item for item in lines[max(0, number - 3) : number]))
                    or (language == "java" and any("@Test" in item for item in lines[max(0, number - 3) : number]))
                )
                kind = "test" if is_test else "method" if owner_range or "::" in function_match.group(1) else "function"
                qualified = f"{owner}.{name}"
                end, _ = brace_end(lines, number)
                if ";" in line and "{" not in line:
                    end = number
                function_ranges.append((number, end, qualified))
                entities.append(entity(fixture, language, relative, kind, name, qualified, line_range(lines, number, end)))

        field_match: re.Match[str] | None = None
        if language == "java":
            field_match = re.match(r"(?:private|public|protected)\s+(?:final\s+)?[\w<>\[\].]+\s+(\w+)\s*;", line)
        elif language == "typescript":
            field_match = re.search(r"(?:private|public|protected)\s+(?:readonly\s+)?(\w+)\s*:", line)
            if field_match is None and next((item for item in type_ranges if item[0] < number < item[1] and item[3] in {"class", "interface"}), None):
                field_match = re.match(r"(\w+)\??\s*:", line)
        elif language == "javascript":
            field_match = re.search(r"this\.(\w+)\s*=", line)
        elif language in {"c", "cpp"}:
            if next((item for item in type_ranges if item[0] < number < item[1]), None):
                field_match = re.match(r"(?:const\s+)?[\w:<>&*\s]+?\b(\w+_?)\s*;", line)
        elif language == "rust":
            if next((item for item in type_ranges if item[0] < number < item[1] and item[3] == "struct"), None):
                field_match = re.match(r"(?:pub\s+)?(\w+)\s*:", line)
        if field_match:
            name = field_match.group(1)
            owner_range = next((item for item in reversed(type_ranges) if item[0] <= number <= item[1]), None)
            if owner_range:
                entities.append(entity(fixture, language, relative, "field", name, f"{owner_range[2]}.{name}", line_range(lines, number)))

        if language in {"javascript", "typescript"} and line.startswith("export "):
            exported = re.search(r"\b(?:class|interface|enum|type|function|const)\s+(\w+)", line)
            if exported:
                relations.append(relation(fixture, language, relative, "exports", relative, exported.group(1), number))

    call_pattern = re.compile(r"\b([A-Za-z_]\w*(?:::\w+|\.\w+)*)\s*[!(]?\(")
    ignored_calls = {"if", "for", "while", "switch", "catch", "return", "sizeof"}
    for number, original_line in enumerate(lines, start=1):
        caller = next((item[2] for item in reversed(function_ranges) if item[0] <= number <= item[1]), None)
        if caller is None:
            continue
        for call in call_pattern.finditer(text_without_line_comment(original_line)):
            target = call.group(1)
            if target.split(".")[-1].split("::")[-1] in ignored_calls:
                continue
            if target == caller.split(".")[-1]:
                continue
            relations.append(relation(fixture, language, relative, "calls_syntactically", caller, target, number))
    return entities, relations, unsupported, status


def add_inventory(
    fixture: str, fixture_root: Path, relative: str, content: bytes
) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    path = fixture_root / relative
    lines = content.decode("utf-8").splitlines(keepends=True)
    language = LANGUAGE_SUFFIXES.get(path.suffix, "text")
    file_entity = entity(fixture, language, relative, "file", path.name, relative, line_range(lines, 1, max(1, len(lines))), "inventory")
    entities = [file_entity]
    relations = [relation(fixture, language, relative, "contains", fixture, file_entity["id"], 1, "inventory")]
    if path.name in CONFIG_NAMES:
        config = entity(fixture, language, relative, "configuration", path.name, relative, line_range(lines, 1, max(1, len(lines))), "inventory")
        entities.append(config)
        relations.append(relation(fixture, language, relative, "configures", config["id"], fixture, 1, "inventory"))
        package_name: str | None = None
        text = content.decode("utf-8")
        if path.name in {"package.json", "system.json", "tsconfig.json"}:
            try:
                parsed_json = json.loads(text)
                package_name = parsed_json.get("name") if isinstance(parsed_json, dict) else None
            except json.JSONDecodeError:
                package_name = None
        elif path.name in {"Cargo.toml", "pyproject.toml"}:
            package_match = re.search(r"(?m)^name\s*=\s*[\"']([^\"']+)[\"']", text)
            package_name = package_match.group(1) if package_match else None
        elif path.name == "CMakeLists.txt":
            package_match = re.search(r"(?m)^project\(([^ )]+)", text)
            package_name = package_match.group(1) if package_match else None
        elif path.name == "pom.xml":
            package_match = re.search(r"<artifactId>([^<]+)</artifactId>", text)
            package_name = package_match.group(1) if package_match else None
        elif path.name == "go.mod":
            package_match = re.search(r"(?m)^module\s+(\S+)", text)
            package_name = package_match.group(1) if package_match else None
        if package_name:
            package_entity = entity(fixture, language, relative, "package", package_name, package_name, line_range(lines, 1), "inventory")
            entities.append(package_entity)
            relations.append(relation(fixture, language, relative, "declares", config["id"], package_entity["id"], 1, "inventory"))
    if path.suffix.lower() in {".md", ".rst", ".txt"}:
        document = entity(fixture, language, relative, "document", path.name, relative, line_range(lines, 1, max(1, len(lines))), "inventory")
        entities.append(document)
        relations.append(relation(fixture, language, relative, "contains", file_entity["id"], document["id"], 1, "inventory"))
    return entities, relations


def analyze_file(
    fixture: str, fixture_directory: Path, path: Path, python_mode: str
) -> dict[str, Any]:
    relative = path.relative_to(fixture_directory).as_posix()
    content = path.read_bytes()
    inventory_entities, inventory_relations = add_inventory(fixture, fixture_directory, relative, content)
    language = LANGUAGE_SUFFIXES.get(path.suffix, "text")
    if language not in GATE_LANGUAGES:
        return {
            "path": relative,
            "language": language,
            "content_sha256": sha256_bytes(content),
            "structural_state": "unavailable",
            "analyzer": None,
            "entities": inventory_entities,
            "relations": inventory_relations,
            "unsupported": ["no V01 structural adapter for this text language"],
        }
    source = content.decode("utf-8")
    if language == "python":
        parsed_entities, parsed_relations, unsupported, state = parse_python(
            fixture, relative, source, native_ast=python_mode == "native"
        )
        analyzer = "python-ast-normalizer" if python_mode == "native" else "common-lexer/python-normalizer"
    else:
        parsed_entities, parsed_relations, unsupported, state = parse_brace_language(
            fixture, language, relative, source
        )
        analyzer = f"common-lexer/{language}-normalizer"
    file_id = inventory_entities[0]["id"]
    for parsed_entity in parsed_entities:
        parsed_relations.append(
            relation(
                fixture,
                language,
                relative,
                "declares",
                file_id,
                parsed_entity["id"],
                parsed_entity["range"]["start_line"],
            )
        )
        if parsed_entity["kind"] == "test":
            parsed_relations.append(
                relation(
                    fixture,
                    language,
                    relative,
                    "tests",
                    parsed_entity["qualified_name"],
                    fixture,
                    parsed_entity["range"]["start_line"],
                )
            )
    return {
        "path": relative,
        "language": language,
        "content_sha256": sha256_bytes(content),
        "structural_state": state,
        "analyzer": analyzer,
        "entities": inventory_entities + parsed_entities,
        "relations": inventory_relations + parsed_relations,
        "unsupported": unsupported,
    }


def cache_key(fixture: str, relative: str, content_hash: str, python_mode: str) -> str:
    return "\0".join((fixture, relative, content_hash, python_mode))


def analyze(
    fixture_root: Path,
    python_mode: str = "native",
    fail_language: str | None = None,
    cache_path: Path | None = None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    started = time.perf_counter_ns()
    before_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    cache: dict[str, Any] = {}
    if cache_path and cache_path.is_file():
        cache = json.loads(cache_path.read_text(encoding="utf-8"))
    updated_cache: dict[str, Any] = {}
    reused = 0
    analyzed = 0
    fixtures: list[dict[str, Any]] = []
    for fixture_directory in sorted(path for path in fixture_root.iterdir() if path.is_dir()):
        fixture = fixture_directory.name
        files: list[dict[str, Any]] = []
        failures: list[dict[str, str]] = []
        for path in sorted(item for item in fixture_directory.rglob("*") if item.is_file()):
            relative = path.relative_to(fixture_directory).as_posix()
            language = LANGUAGE_SUFFIXES.get(path.suffix, "text")
            if fail_language and language == fail_language:
                failures.append({"language": language, "path": relative, "reason": "injected analyzer failure"})
                inventory_entities, inventory_relations = add_inventory(fixture, fixture_directory, relative, path.read_bytes())
                files.append({
                    "path": relative,
                    "language": language,
                    "content_sha256": sha256_bytes(path.read_bytes()),
                    "structural_state": "failed",
                    "analyzer": f"injected/{language}",
                    "entities": inventory_entities,
                    "relations": inventory_relations,
                    "unsupported": ["analyzer result unavailable after injected failure"],
                })
                continue
            content_hash = sha256_bytes(path.read_bytes())
            key = cache_key(fixture, relative, content_hash, python_mode)
            if key in cache:
                result = cache[key]
                reused += 1
            else:
                result = analyze_file(fixture, fixture_directory, path, python_mode)
                analyzed += 1
            updated_cache[key] = result
            files.append(result)
        repository_entity = entity(fixture, "repository", ".", "repository", fixture, fixture, {"start_line": 1, "start_column": 1, "end_line": 1, "end_column": 1}, "inventory")
        all_entities = [repository_entity] + [item for file in files for item in file["entities"]]
        all_relations = [item for file in files for item in file["relations"]]
        all_unsupported = [
            {"language": file["language"], "path": file["path"], "detail": detail}
            for file in files
            for detail in file["unsupported"]
        ]
        languages = sorted({file["language"] for file in files})
        capabilities = []
        for language in languages:
            language_files = [file for file in files if file["language"] == language]
            states = {file["structural_state"] for file in language_files}
            structural = "failed" if states == {"failed"} else "partial" if "failed" in states or "partial" in states else "unavailable" if states == {"unavailable"} else "available"
            capabilities.append({"language": language, "inventory": "available", "structural": structural})
        fixtures.append({
            "fixture_id": fixture,
            "snapshot_sha256": directory_hash(fixture_directory),
            "capabilities": capabilities,
            "files": [{key: file[key] for key in ("path", "language", "content_sha256", "structural_state", "analyzer")} for file in files],
            "entities": sorted(all_entities, key=lambda item: item["id"]),
            "relations": sorted(all_relations, key=lambda item: item["id"]),
            "unsupported": sorted(all_unsupported, key=lambda item: (item["path"], item["detail"])),
            "failures": sorted(failures, key=lambda item: item["path"]),
            "interpretations": [],
        })
    graph = {
        "schema_version": 1,
        "experiment": "V01",
        "model": "normalized-structural-experiment",
        "python_adapter_mode": python_mode,
        "fixtures": fixtures,
    }
    serialized = (json.dumps(graph, indent=2, sort_keys=True) + "\n").encode("utf-8")
    elapsed = time.perf_counter_ns() - started
    after_rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
    metrics = {
        "schema_version": 1,
        "duration_ms": round(elapsed / 1_000_000, 3),
        "output_bytes": len(serialized),
        "peak_rss_kib": after_rss,
        "peak_rss_delta_kib": max(0, after_rss - before_rss),
        "analyzed_file_count": analyzed,
        "reused_file_count": reused,
        "fixture_count": len(fixtures),
        "entity_count": sum(len(fixture["entities"]) for fixture in fixtures),
        "relation_count": sum(len(fixture["relations"]) for fixture in fixtures),
    }
    if cache_path:
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        cache_path.write_text(json.dumps(updated_cache, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return graph, metrics


def tool_version(executable: str, arguments: Iterable[str]) -> dict[str, Any]:
    path = shutil.which(executable)
    if path is None:
        return {"executable": executable, "available": False, "reason": "executable not found on PATH"}
    try:
        completed = subprocess.run([path, *arguments], cwd=REPOSITORY_ROOT, capture_output=True, text=True, check=False, timeout=10)
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"executable": executable, "available": False, "path": path, "reason": str(error)}
    output = (completed.stdout + completed.stderr).strip().splitlines()
    return {
        "executable": executable,
        "available": completed.returncode == 0,
        "path": path,
        "exit_code": completed.returncode,
        "version": output[0] if output else "no version output",
        "reason": None if completed.returncode == 0 else "version command returned nonzero",
    }


def syntax_probe(name: str, argv: list[str], cwd: Path) -> dict[str, Any]:
    executable = shutil.which(argv[0])
    if executable is None:
        return {"name": name, "executed": False, "reason": f"{argv[0]} executable not found on PATH"}
    completed = subprocess.run(
        [executable, *argv[1:]],
        cwd=cwd,
        capture_output=True,
        text=True,
        check=False,
        timeout=20,
    )
    return {
        "name": name,
        "executed": True,
        "argv": [executable, *argv[1:]],
        "working_directory": str(cwd),
        "exit_code": completed.returncode,
        "stdout": completed.stdout,
        "stderr": completed.stderr,
        "structural_output": False,
        "reason": "syntax diagnostics only; installed compiler does not expose the normalized declaration graph through this command",
    }


def probe_candidates() -> dict[str, Any]:
    tools = {
        "python": tool_version(sys.executable, ("--version",)),
        "tree-sitter": tool_version("tree-sitter", ("--version",)),
        "javac": tool_version("javac", ("-version",)),
        "node": tool_version("node", ("--version",)),
        "tsc": tool_version("tsc", ("--version",)),
        "gcc": tool_version("gcc", ("--version",)),
        "g++": tool_version("g++", ("--version",)),
        "rustc": tool_version("rustc", ("--version",)),
    }
    local_v01 = REPOSITORY_ROOT / "rebuild" / ".local" / "v01"
    local_v01.mkdir(parents=True, exist_ok=True)
    probe_root = Path(tempfile.mkdtemp(prefix="native-probe-", dir=local_v01))
    java_output = probe_root / "java"
    java_output.mkdir(parents=True)
    probes = [
        syntax_probe(
            "java-javac",
            ["javac", "-d", str(java_output), "src/main/java/example/Greeter.java"],
            DEFAULT_FIXTURE_ROOT / "java",
        ),
        syntax_probe("javascript-node-check", ["node", "--check", "src/index.js"], DEFAULT_FIXTURE_ROOT / "javascript"),
        syntax_probe("typescript-tsc", ["tsc", "--noEmit", "-p", "tsconfig.json"], DEFAULT_FIXTURE_ROOT / "typescript"),
        syntax_probe(
            "c-gcc",
            ["gcc", "-fsyntax-only", "-Iinclude", "src/greeter.c", "tests/test_greeter.c"],
            DEFAULT_FIXTURE_ROOT / "c",
        ),
        syntax_probe(
            "cpp-g++",
            ["g++", "-std=c++17", "-fsyntax-only", "-Iinclude", "src/greeter.cpp", "tests/test_greeter.cpp"],
            DEFAULT_FIXTURE_ROOT / "cpp",
        ),
        syntax_probe(
            "rust-rustc",
            [
                "rustc",
                "--edition=2021",
                "--crate-name",
                "v01_fixture",
                "--crate-type",
                "lib",
                "--emit",
                "metadata",
                "-o",
                str(probe_root / "v01_fixture.rmeta"),
                "crates/greeter/src/lib.rs",
            ],
            DEFAULT_FIXTURE_ROOT / "rust",
        ),
    ]
    python_files = sorted((DEFAULT_FIXTURE_ROOT / "python").rglob("*.py"))
    try:
        for path in python_files:
            ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        probes.append({
            "name": "python-ast",
            "executed": True,
            "exit_code": 0,
            "file_count": len(python_files),
            "structural_output": True,
            "reason": "native AST declarations and ranges were normalized by the hybrid prototype",
        })
    except SyntaxError as error:
        probes.append({"name": "python-ast", "executed": True, "exit_code": 1, "structural_output": False, "reason": str(error)})
    return {
        "schema_version": 1,
        "approaches": [
            {
                "class": "common parser framework with language-specific normalization",
                "executable": True,
                "implementation": "common lexical parser and seven language normalizers",
                "limit": "not a production parser and does not expand macros or resolve semantics",
            },
            {
                "class": "language-native, compiler-front-end, or direct-parser sources",
                "executable": True,
                "implementation": "Python ast is structurally normalized; available compilers are syntax-diagnostic probes only",
                "limit": "no common machine-readable declaration output was available from installed JavaScript, Java, C/C++, or Rust tools",
            },
            {
                "class": "hybrid boundary",
                "executable": True,
                "implementation": "Python ast adapter plus common lexical adapters for the other six gate languages",
                "limit": "fixture evidence only; production grammar integrations remain unselected",
            },
        ],
        "tools": tools,
        "native_and_direct_probes": probes,
        "external_transmission": False,
    }


def write_output(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main(arguments: list[str]) -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)
    analyze_parser = subparsers.add_parser("analyze")
    analyze_parser.add_argument("--fixture-root", type=Path, default=DEFAULT_FIXTURE_ROOT)
    analyze_parser.add_argument("--output", type=Path, required=True)
    analyze_parser.add_argument("--metrics", type=Path, required=True)
    analyze_parser.add_argument("--cache", type=Path)
    analyze_parser.add_argument("--python-mode", choices=("common", "native"), default="native")
    analyze_parser.add_argument("--fail-language", choices=GATE_LANGUAGES)
    probe_parser = subparsers.add_parser("probe-candidates")
    probe_parser.add_argument("--output", type=Path, required=True)
    options = parser.parse_args(arguments)
    if options.command == "probe-candidates":
        write_output(options.output, probe_candidates())
        print(options.output)
        return 0
    graph, metrics = analyze(options.fixture_root, options.python_mode, options.fail_language, options.cache)
    write_output(options.output, graph)
    write_output(options.metrics, metrics)
    print(json.dumps(metrics, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
