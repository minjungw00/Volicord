use crate::diagnostics::ValidationIssue;
use crate::doc_index::TERMINOLOGY_MAP_PATH;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

const OPERATION_CATEGORY_DOC_PATHS: &[&str] = &[
    "docs/en/reference/api/schema-value-sets.md",
    "docs/ko/reference/api/schema-value-sets.md",
];
const OPERATION_CATEGORY_ANCHOR: &str = "operation-category-values";
const OPERATION_CATEGORY_TERM_KEY: &str = "operation_category";
const SURFACE_STABILITY_LABELS: &[&str] = &["stable", "beta", "internal", "diagnostic"];
const EN_ARCHITECTURE_DESIGN_H2_SCHEMA: &[&str] = &[
    "Purpose",
    "Design",
    "Invariants",
    "Responsibility boundaries",
    "Execution flow",
    "Failure behavior",
    "Scope exclusions",
    "Implementation routes",
    "Reference owners",
];
const KO_ARCHITECTURE_DESIGN_H2_SCHEMA: &[&str] = &[
    "목적",
    "설계",
    "불변 조건",
    "책임 경계",
    "실행 흐름",
    "실패 동작",
    "범위 제외",
    "구현 경로",
    "참조 담당 문서",
];
const EN_PROHIBITED_ARCHITECTURE_DESIGN_HEADINGS: &[&str] = &[
    "Context",
    "Decision",
    "Consequences",
    "Rejected alternatives",
    "Migration notes",
    "Before and after",
    "Review findings",
    "Change history",
    "Decision chronology",
    "Review history",
    "Migration narrative",
    "Migration narratives",
    "Release recommendations",
];
const KO_PROHIBITED_ARCHITECTURE_DESIGN_HEADINGS: &[&str] = &[
    "맥락",
    "결정",
    "결과",
    "거부한 대안",
    "마이그레이션 메모",
    "이전과 이후",
    "검토 결과",
    "변경 이력",
    "결정 연대기",
    "검토 이력",
    "마이그레이션 설명",
    "릴리스 권고",
];
const REQUIRED_SURFACE_STABILITY_DOCS: &[SurfaceStabilityRequirement] = &[
    SurfaceStabilityRequirement {
        path: "docs/en/reference/admin-cli.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/admin-cli.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/api/methods.md",
        required_labels: &["stable"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/api/methods.md",
        required_labels: &["stable"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/mcp-transport.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/mcp-transport.md",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/conformance.md",
        required_labels: &["stable", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/conformance.md",
        required_labels: &["stable", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/en/reference/storage-ddl.md",
        required_labels: &["stable", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        path: "docs/ko/reference/storage-ddl.md",
        required_labels: &["stable", "internal", "diagnostic"],
    },
];

struct SurfaceStabilityRequirement {
    path: &'static str,
    required_labels: &'static [&'static str],
}

pub(crate) fn validate_architecture_design_documents(
    root: &Path,
    errors: &mut Vec<ValidationIssue>,
) {
    for (directory, expected_h2, prohibited_headings) in [
        (
            "docs/en/architecture-guide/design",
            EN_ARCHITECTURE_DESIGN_H2_SCHEMA,
            EN_PROHIBITED_ARCHITECTURE_DESIGN_HEADINGS,
        ),
        (
            "docs/ko/architecture-guide/design",
            KO_ARCHITECTURE_DESIGN_H2_SCHEMA,
            KO_PROHIBITED_ARCHITECTURE_DESIGN_HEADINGS,
        ),
    ] {
        let prohibited: BTreeSet<_> = prohibited_headings
            .iter()
            .map(|heading| normalize_heading(heading))
            .collect();

        let directory_path = root.join(directory);
        if !directory_path.exists() {
            continue;
        }
        let entries = match fs::read_dir(&directory_path) {
            Ok(entries) => entries,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    directory,
                    "architecture_design.read_directory",
                    format!("failed to read current architecture-design directory: {error}"),
                ));
                continue;
            }
        };
        let mut filenames = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    errors.push(ValidationIssue::new(
                        directory,
                        "architecture_design.read_directory",
                        format!(
                            "failed to read an entry in the current architecture-design directory: {error}"
                        ),
                    ));
                    continue;
                }
            };
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|extension| extension == "md") {
                filenames.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        filenames.sort();

        for filename in filenames {
            let relative_path = format!("{directory}/{filename}");
            let path = root.join(&relative_path);
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    errors.push(ValidationIssue::new(
                        relative_path,
                        "architecture_design.read",
                        format!("failed to read current architecture-design document: {error}"),
                    ));
                    continue;
                }
            };
            let headings = markdown_headings(&contents);

            for heading in &headings {
                if prohibited.contains(&normalize_heading(&heading.text)) {
                    errors.push(ValidationIssue::at_line(
                        &relative_path,
                        "architecture_design.prohibited_heading",
                        Some(heading.line),
                        format!(
                            "current architecture-design documents cannot use transitional heading `{}`",
                            heading.text
                        ),
                    ));
                }
            }

            if filename == "README.md" {
                continue;
            }

            let h1_count = headings.iter().filter(|heading| heading.level == 1).count();
            if h1_count != 1 {
                errors.push(ValidationIssue::new(
                    &relative_path,
                    "architecture_design.title_schema",
                    format!(
                        "current architecture-design documents require exactly one H1 title; found {h1_count}"
                    ),
                ));
            }

            let actual_h2 = headings
                .iter()
                .filter(|heading| heading.level == 2)
                .map(|heading| heading.text.as_str())
                .collect::<Vec<_>>();
            if actual_h2 != expected_h2 {
                errors.push(ValidationIssue::new(
                    &relative_path,
                    "architecture_design.section_schema",
                    format!(
                        "H2 sequence must be exactly {}; found {}",
                        format_heading_sequence(expected_h2.iter().copied()),
                        format_heading_sequence(actual_h2.iter().copied()),
                    ),
                ));
            }
        }
    }
}

pub(crate) fn validate_surface_stability_sections(root: &Path, errors: &mut Vec<ValidationIssue>) {
    for requirement in REQUIRED_SURFACE_STABILITY_DOCS {
        let path = root.join(requirement.path);
        if !path.exists() {
            continue;
        }
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    requirement.path,
                    "surface_stability.read",
                    format!("failed to read required surface stability document: {error}"),
                ));
                continue;
            }
        };

        let Some(section) = extract_surface_stability_section(&contents) else {
            errors.push(ValidationIssue::new(
                requirement.path,
                "surface_stability.missing_section",
                "missing required <a id=\"surface-stability\"></a> Surface Stability section",
            ));
            continue;
        };

        if !section.contains("documentation-policy.md#surface-stability-labels") {
            errors.push(ValidationIssue::new(
                requirement.path,
                "surface_stability.missing_link",
                "Surface Stability section must link to the canonical documentation policy vocabulary",
            ));
        }

        let labels = extract_surface_stability_labels(section);
        for label in &labels {
            if !SURFACE_STABILITY_LABELS.contains(&label.as_str()) {
                errors.push(ValidationIssue::new(
                    requirement.path,
                    "surface_stability.invalid_label",
                    format!("Surface Stability section uses unsupported label `{label}`"),
                ));
            }
        }
        for required_label in requirement.required_labels {
            if !labels.contains(*required_label) {
                errors.push(ValidationIssue::new(
                    requirement.path,
                    "surface_stability.missing_label",
                    format!(
                        "Surface Stability section is missing required `{required_label}` label"
                    ),
                ));
            }
        }
    }
}

struct MarkdownHeading {
    level: u8,
    line: usize,
    text: String,
}

fn markdown_headings(contents: &str) -> Vec<MarkdownHeading> {
    let newline_offsets = contents
        .bytes()
        .enumerate()
        .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset))
        .collect::<Vec<_>>();
    let mut headings = Vec::new();
    let mut active: Option<MarkdownHeading> = None;

    for (event, range) in Parser::new_ext(contents, crate::markdown::options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                active = Some(MarkdownHeading {
                    level: markdown_heading_level(level),
                    line: newline_offsets.partition_point(|offset| *offset < range.start) + 1,
                    text: String::new(),
                });
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(heading) = active.as_mut() {
                    heading.text.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(heading) = active.as_mut() {
                    heading.text.push(' ');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut heading) = active.take() {
                    heading.text = heading.text.trim().to_string();
                    headings.push(heading);
                }
            }
            _ => {}
        }
    }

    headings
}

fn markdown_heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn normalize_heading(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_heading_sequence<'a>(headings: impl Iterator<Item = &'a str>) -> String {
    let headings = headings
        .map(|heading| format!("`{heading}`"))
        .collect::<Vec<_>>();
    if headings.is_empty() {
        "no H2 headings".to_string()
    } else {
        headings.join(", ")
    }
}

fn extract_surface_stability_section(contents: &str) -> Option<&str> {
    let marker = "<a id=\"surface-stability\"></a>";
    let start = contents.find(marker)?;
    let after_marker = &contents[start..];
    let mut offset = 0;
    let mut heading_count = 0;

    for line in after_marker.split_inclusive('\n') {
        if line.trim_start().starts_with("## ") {
            heading_count += 1;
            if heading_count == 2 {
                return Some(&after_marker[..offset]);
            }
        }
        offset += line.len();
    }

    Some(after_marker)
}

fn extract_surface_stability_labels(section: &str) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        let cells = markdown_table_cells(trimmed);
        if cells.len() < 2 || is_markdown_table_separator(&cells) {
            continue;
        }
        for label in extract_backtick_values(cells[1]) {
            labels.insert(label);
        }
    }
    labels
}

fn markdown_table_cells(line: &str) -> Vec<&str> {
    line.trim_matches('|').split('|').map(str::trim).collect()
}

fn is_markdown_table_separator(cells: &[&str]) -> bool {
    cells.iter().all(|cell| {
        cell.chars()
            .all(|character| matches!(character, '-' | ':' | ' '))
    })
}

fn extract_backtick_values(contents: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = contents;

    while let Some(start) = remaining.find('`') {
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('`') else {
            break;
        };
        values.push(after_start[..end].to_string());
        remaining = &after_start[end + 1..];
    }

    values
}

pub(crate) fn validate_operation_category_values(root: &Path, errors: &mut Vec<ValidationIssue>) {
    if !OPERATION_CATEGORY_DOC_PATHS
        .iter()
        .any(|path| root.join(path).exists())
    {
        return;
    }

    let mut documented_values = Vec::new();
    for relative_path in OPERATION_CATEGORY_DOC_PATHS {
        let contents = match fs::read_to_string(root.join(relative_path)) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    *relative_path,
                    "operation_category_values.read",
                    format!("failed to read operation category owner: {error}"),
                ));
                continue;
            }
        };
        let Some(section) = extract_anchored_markdown_section(&contents, OPERATION_CATEGORY_ANCHOR)
        else {
            errors.push(ValidationIssue::new(
                *relative_path,
                "operation_category_values.missing_section",
                format!("missing anchored operation category section #{OPERATION_CATEGORY_ANCHOR}"),
            ));
            continue;
        };
        let values = extract_first_column_identifier_values(section);
        if values.is_empty() {
            errors.push(ValidationIssue::new(
                *relative_path,
                "operation_category_values.invalid_table",
                "operation category section must contain a Markdown table with backticked values in its first column",
            ));
            continue;
        }
        documented_values.push((*relative_path, values));
    }

    if let [(en_path, en_values), (ko_path, ko_values)] = documented_values.as_slice() {
        if en_values != ko_values {
            let missing_from_ko = en_values.difference(ko_values).cloned().collect();
            let missing_from_en = ko_values.difference(en_values).cloned().collect();
            errors.push(ValidationIssue::new(
                *ko_path,
                "operation_category_values.language_drift",
                format!(
                    "operation category value sets differ between {en_path} and {ko_path}; missing from Korean owner: {}; missing from English owner: {}",
                    format_backticked_values(&missing_from_ko),
                    format_backticked_values(&missing_from_en),
                ),
            ));
        }
    }

    let mut required_identifiers = BTreeSet::from([OPERATION_CATEGORY_TERM_KEY.to_string()]);
    for (_, values) in &documented_values {
        required_identifiers.extend(values.iter().cloned());
    }
    validate_operation_category_terminology(root, &required_identifiers, errors);
}

fn extract_anchored_markdown_section<'a>(contents: &'a str, anchor: &str) -> Option<&'a str> {
    let marker = format!("<a id=\"{anchor}\"></a>");
    let section_start = contents.find(&marker)? + marker.len();
    let remaining = &contents[section_start..];
    let section_end = remaining.find("\n<a id=\"").unwrap_or(remaining.len());
    Some(&remaining[..section_end])
}

fn extract_first_column_identifier_values(section: &str) -> BTreeSet<String> {
    let mut values = BTreeSet::new();
    for line in section.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
            continue;
        }
        let cells = markdown_table_cells(trimmed);
        if cells.is_empty() || is_markdown_table_separator(&cells) {
            continue;
        }
        let identifiers = extract_backtick_values(cells[0]);
        if let [identifier] = identifiers.as_slice() {
            values.insert(identifier.clone());
        }
    }
    values
}

fn validate_operation_category_terminology(
    root: &Path,
    required_identifiers: &BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    let contents = match fs::read_to_string(root.join(TERMINOLOGY_MAP_PATH)) {
        Ok(contents) => contents,
        Err(error) => {
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "operation_category_values.terminology_read",
                format!("failed to read terminology map: {error}"),
            ));
            return;
        }
    };
    let value: Value = match serde_yaml::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            errors.push(ValidationIssue::new(
                TERMINOLOGY_MAP_PATH,
                "operation_category_values.terminology_yaml",
                format!("failed to parse terminology map YAML: {error}"),
            ));
            return;
        }
    };

    let preserved = value
        .as_mapping()
        .and_then(|top| mapping_get(top, "terms"))
        .and_then(Value::as_mapping)
        .and_then(|terms| mapping_get(terms, OPERATION_CATEGORY_TERM_KEY))
        .and_then(Value::as_mapping)
        .and_then(|entry| mapping_get(entry, "preserve_as_identifier"))
        .and_then(sequence_strings);
    let Some(preserved) = preserved else {
        errors.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "operation_category_values.terminology_shape",
            format!(
                "terms.{OPERATION_CATEGORY_TERM_KEY}.preserve_as_identifier must be a sequence of strings"
            ),
        ));
        return;
    };
    let preserved: BTreeSet<_> = preserved.into_iter().collect();
    let missing: BTreeSet<_> = required_identifiers
        .difference(&preserved)
        .cloned()
        .collect();
    if !missing.is_empty() {
        errors.push(ValidationIssue::new(
            TERMINOLOGY_MAP_PATH,
            "operation_category_values.terminology_missing",
            format!(
                "terms.{OPERATION_CATEGORY_TERM_KEY}.preserve_as_identifier is missing {}",
                format_backticked_values(&missing)
            ),
        ));
    }
}

fn format_backticked_values(values: &BTreeSet<String>) -> String {
    if values.is_empty() {
        return "none".to_string();
    }
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn sequence_strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_sequence()?
        .iter()
        .map(|item| item.as_str().map(str::to_owned))
        .collect()
}
