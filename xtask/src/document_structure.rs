use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

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
const REQUIRED_SURFACE_STABILITY_DOCS: &[SurfaceStabilityRequirement] = &[
    SurfaceStabilityRequirement {
        doc_id: "reference.admin-cli",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        doc_id: "reference.api.methods",
        required_labels: &["stable"],
    },
    SurfaceStabilityRequirement {
        doc_id: "reference.mcp-transport",
        required_labels: &["stable", "beta", "internal", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        doc_id: "reference.conformance",
        required_labels: &["stable", "diagnostic"],
    },
    SurfaceStabilityRequirement {
        doc_id: "reference.storage-ddl",
        required_labels: &["stable", "internal", "diagnostic"],
    },
];

struct SurfaceStabilityRequirement {
    doc_id: &'static str,
    required_labels: &'static [&'static str],
}

pub(crate) fn validate_architecture_design_documents(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    for paired in index.paired_documents.values().filter(|paired| {
        paired.doc_id.starts_with("architecture-guide.design.")
            && paired.doc_id != "architecture-guide.design.index"
    }) {
        for (relative_path, expected_h2) in [
            (paired.path_en.as_str(), EN_ARCHITECTURE_DESIGN_H2_SCHEMA),
            (paired.path_ko.as_str(), KO_ARCHITECTURE_DESIGN_H2_SCHEMA),
        ] {
            let path = root.join(relative_path);
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

            for heading in headings.iter().filter(|heading| heading.level > 2) {
                errors.push(ValidationIssue::at_line(
                    relative_path,
                    "architecture_design.unknown_section",
                    Some(heading.line),
                    format!(
                        "current architecture-design schema does not define level-{} section `{}`",
                        heading.level, heading.text
                    ),
                ));
            }

            let h1_count = headings.iter().filter(|heading| heading.level == 1).count();
            if h1_count != 1 {
                errors.push(ValidationIssue::new(
                    relative_path,
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
                    relative_path,
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

pub(crate) fn validate_surface_stability_sections(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    for requirement in REQUIRED_SURFACE_STABILITY_DOCS {
        let Some(owner) = index.paired_documents.get(requirement.doc_id) else {
            continue;
        };
        for relative_path in [&owner.path_en, &owner.path_ko] {
            let path = root.join(relative_path);
            let contents = match fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    errors.push(ValidationIssue::new(
                        relative_path,
                        "surface_stability.read",
                        format!("failed to read required surface stability document: {error}"),
                    ));
                    continue;
                }
            };

            let Some(section) = extract_surface_stability_section(&contents) else {
                errors.push(ValidationIssue::new(
                    relative_path,
                    "surface_stability.missing_section",
                    "missing required <a id=\"surface-stability\"></a> Surface Stability section",
                ));
                continue;
            };

            if !section.contains("documentation-policy.md#surface-stability-labels") {
                errors.push(ValidationIssue::new(
                    relative_path,
                    "surface_stability.missing_link",
                    "Surface Stability section must link to the canonical documentation policy vocabulary",
                ));
            }

            let labels = extract_surface_stability_labels(section);
            for label in &labels {
                if !SURFACE_STABILITY_LABELS.contains(&label.as_str()) {
                    errors.push(ValidationIssue::new(
                        relative_path,
                        "surface_stability.invalid_label",
                        format!("Surface Stability section uses unsupported label `{label}`"),
                    ));
                }
            }
            for required_label in requirement.required_labels {
                if !labels.contains(*required_label) {
                    errors.push(ValidationIssue::new(
                        relative_path,
                        "surface_stability.missing_label",
                        format!(
                            "Surface Stability section is missing required `{required_label}` label"
                        ),
                    ));
                }
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
