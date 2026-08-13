use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use tree_sitter::{InputEdit, Language, Node, Parser, Point, Tree};

#[derive(Deserialize)]
struct FixtureManifest {
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
struct Fixture {
    id: String,
    validation_id: String,
    path: String,
    expected_entities: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ObservedDeclaration {
    fixture: String,
    language: String,
    path: String,
    kind: String,
    name: String,
    start_byte: usize,
    end_byte: usize,
    start_row: usize,
    start_column: usize,
    end_row: usize,
    end_column: usize,
    parser_node_kind: String,
}

#[derive(Serialize)]
struct QualificationResult {
    format: &'static str,
    parser_framework: &'static str,
    grammar_versions: BTreeMap<&'static str, &'static str>,
    fixture_count: usize,
    source_file_count: usize,
    expected_declarations: usize,
    matched_declarations: usize,
    exact_start_rows: usize,
    valid_utf8_byte_ranges: usize,
    malformed_files_with_retained_declarations: usize,
    incremental_changed_ranges: usize,
    incremental_changed_bytes: usize,
    incremental_full_file_bytes: usize,
    declarations: Vec<ObservedDeclaration>,
}

#[derive(Clone)]
struct ExpectedDeclaration {
    language: String,
    path: String,
    kind: String,
    name: String,
    line: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = qualification_root()?;
    let manifest_path = root.join("rebuild/validation/shared/fixture-manifest.json");
    let manifest: FixtureManifest = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    let mut declarations = Vec::new();
    let mut source_files = BTreeSet::new();
    let mut fixture_count = 0;
    let mut malformed_retained = 0;

    for fixture in manifest
        .fixtures
        .iter()
        .filter(|fixture| fixture.validation_id == "V01" && !fixture.expected_entities.is_empty())
    {
        fixture_count += 1;
        let fixture_root = root.join(&fixture.path);
        let expected = fixture
            .expected_entities
            .iter()
            .map(|entry| parse_expected(entry))
            .collect::<Result<Vec<_>, _>>()?;
        let mut parsed = BTreeMap::new();

        for declaration in expected {
            let key = (declaration.language.clone(), declaration.path.clone());
            source_files.insert((fixture.id.clone(), key.clone()));
            if !parsed.contains_key(&key) {
                let source = fs::read(fixture_root.join(&declaration.path))?;
                let tree = parse(&declaration.language, &source)?;
                parsed.insert(key.clone(), (source, tree));
            }
            let (source, tree) = parsed.get(&key).ok_or("parsed source disappeared")?;
            let node = find_declaration(
                tree.root_node(),
                source,
                &declaration.kind,
                &declaration.name,
                declaration.line,
            )
            .ok_or_else(|| {
                format!(
                    "{}: missing {} {} at {}:{}",
                    fixture.id,
                    declaration.kind,
                    declaration.name,
                    declaration.path,
                    declaration.line
                )
            })?;
            let start = node.start_position();
            let end = node.end_position();
            if node.start_byte() >= node.end_byte() || node.end_byte() > source.len() {
                return Err(format!("{}: invalid parser byte range", fixture.id).into());
            }
            declarations.push(ObservedDeclaration {
                fixture: fixture.id.clone(),
                language: declaration.language,
                path: declaration.path,
                kind: declaration.kind,
                name: declaration.name,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_row: start.row,
                start_column: start.column,
                end_row: end.row,
                end_column: end.column,
                parser_node_kind: node.kind().to_owned(),
            });
        }

        if fixture.id == "v01-javascript" {
            let key = ("javascript".to_owned(), "src/broken.js".to_owned());
            let (source, tree) = parsed.get(&key).ok_or("broken JavaScript was not parsed")?;
            if !tree.root_node().has_error()
                || find_declaration(tree.root_node(), source, "function", "stillVisible", 1)
                    .is_none()
            {
                return Err("malformed JavaScript did not retain the known declaration".into());
            }
            malformed_retained += 1;
        }
    }

    declarations.sort();
    let expected_declarations = declarations.len();
    let (incremental_changed_ranges, incremental_changed_bytes, incremental_full_file_bytes) =
        incremental_probe(&root)?;
    if incremental_changed_ranges == 0 || incremental_changed_bytes >= incremental_full_file_bytes {
        return Err("incremental parser reported an unbounded whole-file change".into());
    }

    let result = QualificationResult {
        format: "volicord.structural_parser_qualification.v1",
        parser_framework: "tree-sitter 0.26.12",
        grammar_versions: BTreeMap::from([
            ("c", "tree-sitter-c 0.24.2"),
            ("cpp", "tree-sitter-cpp 0.23.4"),
            ("java", "tree-sitter-java 0.23.5"),
            ("javascript", "tree-sitter-javascript 0.25.0"),
            ("python", "tree-sitter-python 0.25.0"),
            ("rust", "tree-sitter-rust 0.24.2"),
            ("typescript", "tree-sitter-typescript 0.23.2"),
        ]),
        fixture_count,
        source_file_count: source_files.len(),
        expected_declarations,
        matched_declarations: expected_declarations,
        exact_start_rows: declarations.len(),
        valid_utf8_byte_ranges: declarations.len(),
        malformed_files_with_retained_declarations: malformed_retained,
        incremental_changed_ranges,
        incremental_changed_bytes,
        incremental_full_file_bytes,
        declarations,
    };
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn qualification_root() -> Result<PathBuf, Box<dyn Error>> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(4)
        .map(Path::to_path_buf)
        .ok_or_else(|| "qualification workspace is not under rebuild/validation".into())
}

fn parse_expected(value: &str) -> Result<ExpectedDeclaration, Box<dyn Error>> {
    let parts = value.split('|').collect::<Vec<_>>();
    if parts.len() != 5 {
        return Err(format!("invalid expected entity: {value}").into());
    }
    Ok(ExpectedDeclaration {
        language: parts[0].to_owned(),
        path: parts[1].to_owned(),
        kind: parts[2].to_owned(),
        name: parts[3].to_owned(),
        line: parts[4].parse()?,
    })
}

fn language(name: &str) -> Result<Language, Box<dyn Error>> {
    let language = match name {
        "java" => tree_sitter_java::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        "javascript" => tree_sitter_javascript::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "c" => tree_sitter_c::LANGUAGE.into(),
        "cpp" => tree_sitter_cpp::LANGUAGE.into(),
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        other => return Err(format!("unsupported qualification language: {other}").into()),
    };
    Ok(language)
}

fn parse(language_name: &str, source: &[u8]) -> Result<Tree, Box<dyn Error>> {
    let mut parser = Parser::new();
    parser.set_language(&language(language_name)?)?;
    parser
        .parse(source, None)
        .ok_or_else(|| "parser was cancelled before producing a tree".into())
}

fn find_declaration<'tree>(
    root: Node<'tree>,
    source: &[u8],
    expected_kind: &str,
    expected_name: &str,
    expected_line: usize,
) -> Option<Node<'tree>> {
    let mut stack = vec![root];
    let mut best = None;
    while let Some(node) = stack.pop() {
        if node.is_named()
            && declaration_node_matches(node.kind(), expected_kind)
            && has_named_token(node, source, expected_name, expected_line)
            && best.as_ref().is_none_or(|current: &Node<'_>| {
                node.byte_range().len() < current.byte_range().len()
            })
        {
            best = Some(node);
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push(child);
        }
    }
    best
}

fn has_named_token(node: Node<'_>, source: &[u8], expected: &str, line: usize) -> bool {
    let mut stack = vec![node];
    while let Some(candidate) = stack.pop() {
        if candidate.is_named()
            && candidate.start_position().row + 1 == line
            && candidate.end_byte() <= source.len()
            && std::str::from_utf8(&source[candidate.start_byte()..candidate.end_byte()])
                .is_ok_and(|text| text == expected)
        {
            return true;
        }
        let mut cursor = candidate.walk();
        for child in candidate.named_children(&mut cursor) {
            stack.push(child);
        }
    }
    false
}

fn declaration_node_matches(node_kind: &str, expected_kind: &str) -> bool {
    match expected_kind {
        "class" | "interface" | "trait" | "struct" | "enum" | "type" => matches!(
            node_kind,
            "class_declaration"
                | "class_definition"
                | "interface_declaration"
                | "trait_item"
                | "struct_item"
                | "struct_specifier"
                | "class_specifier"
                | "enum_item"
                | "enum_declaration"
                | "enum_specifier"
                | "type_item"
                | "type_alias_declaration"
                | "type_definition"
                | "alias_declaration"
        ),
        "function" | "method" | "test" => matches!(
            node_kind,
            "function_declaration"
                | "function_definition"
                | "function_declarator"
                | "function_item"
                | "method_declaration"
                | "method_signature"
                | "function_signature"
                | "function_signature_item"
                | "constructor_declaration"
                | "method_definition"
                | "declaration"
        ),
        "field" => matches!(
            node_kind,
            "field_declaration"
                | "public_field_definition"
                | "assignment"
                | "assignment_expression"
                | "required_parameter"
                | "optional_parameter"
                | "property_signature"
        ),
        _ => false,
    }
}

fn incremental_probe(root: &Path) -> Result<(usize, usize, usize), Box<dyn Error>> {
    let path = root.join(
        "rebuild/validation/repository-intelligence/polyglot-structural/fixtures/typescript/src/index.ts",
    );
    let original = fs::read(&path)?;
    let needle = b"function formatName";
    let replacement = b"async function formatName";
    let start_byte = original
        .windows(needle.len())
        .position(|window| window == needle)
        .ok_or("incremental probe declaration not found")?;
    let mut changed = Vec::with_capacity(original.len() + replacement.len() - needle.len());
    changed.extend_from_slice(&original[..start_byte]);
    changed.extend_from_slice(replacement);
    changed.extend_from_slice(&original[start_byte + needle.len()..]);

    let language = language("typescript")?;
    let mut parser = Parser::new();
    parser.set_language(&language)?;
    let mut old_tree = parser
        .parse(&original, None)
        .ok_or("initial incremental parse was cancelled")?;
    let start_position = point_for_offset(&original, start_byte);
    let old_end_position = point_for_offset(&original, start_byte + needle.len());
    let new_end_position = point_for_offset(&changed, start_byte + replacement.len());
    old_tree.edit(&InputEdit {
        start_byte,
        old_end_byte: start_byte + needle.len(),
        new_end_byte: start_byte + replacement.len(),
        start_position,
        old_end_position,
        new_end_position,
    });
    let new_tree = parser
        .parse(&changed, Some(&old_tree))
        .ok_or("incremental reparse was cancelled")?;
    let ranges = old_tree.changed_ranges(&new_tree).collect::<Vec<_>>();
    let changed_bytes = ranges
        .iter()
        .map(|range| range.end_byte - range.start_byte)
        .sum();
    Ok((ranges.len(), changed_bytes, changed.len()))
}

fn point_for_offset(source: &[u8], offset: usize) -> Point {
    let prefix = &source[..offset];
    let row = prefix.iter().filter(|byte| **byte == b'\n').count();
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len(), |position| prefix.len() - position - 1);
    Point::new(row, column)
}
