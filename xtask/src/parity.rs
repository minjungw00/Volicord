use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use crate::markdown;
use crate::terminology::ExactIdentifierCatalog;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub(crate) fn validate_bilingual_structure(
    root: &Path,
    index: &DocIndex,
    catalog: &ExactIdentifierCatalog,
    issues: &mut Vec<ValidationIssue>,
) {
    for paired in index.paired_documents.values() {
        let en = match read_structure(root, &paired.path_en, &catalog.identifiers) {
            Ok(structure) => structure,
            Err(error) => {
                issues.push(ValidationIssue::new(
                    &paired.path_en,
                    "bilingual_structure.read",
                    error,
                ));
                continue;
            }
        };
        let ko = match read_structure(root, &paired.path_ko, &catalog.identifiers) {
            Ok(structure) => structure,
            Err(error) => {
                issues.push(ValidationIssue::new(
                    &paired.path_ko,
                    "bilingual_structure.read",
                    error,
                ));
                continue;
            }
        };

        let en_levels = en
            .sections
            .iter()
            .filter_map(|section| section.heading_level)
            .collect::<Vec<_>>();
        let ko_levels = ko
            .sections
            .iter()
            .filter_map(|section| section.heading_level)
            .collect::<Vec<_>>();
        if en_levels != ko_levels {
            issues.push(ValidationIssue::new(
                &paired.path_ko,
                "bilingual_structure.heading_levels",
                format!(
                    "{} must preserve the English/Korean heading-level sequence; English has {:?}, Korean has {:?}",
                    paired.doc_id, en_levels, ko_levels
                ),
            ));
        }

        for (position, (en_section, ko_section)) in en.sections.iter().zip(&ko.sections).enumerate()
        {
            let en_required = section_identifiers(en_section);
            let ko_required = section_identifiers(ko_section);
            compare_identifier_sets(
                &paired.path_en,
                en_section.line,
                &paired.path_ko,
                ko_section.line,
                position,
                "section",
                &en_required,
                &ko_required,
                issues,
            );
        }
    }
}

fn section_identifiers(section: &markdown::MarkdownSection) -> BTreeSet<String> {
    section
        .units
        .iter()
        .flat_map(|unit| unit.identifiers.iter().cloned())
        .chain(section.heading_identifiers.iter().cloned())
        .collect()
}

fn read_structure(
    root: &Path,
    relative_path: &str,
    exact_identifiers: &BTreeSet<String>,
) -> Result<markdown::MarkdownStructure, String> {
    let contents = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read paired Markdown: {error}"))?;
    Ok(markdown::identifier_structure(&contents, exact_identifiers))
}

#[allow(clippy::too_many_arguments)]
fn compare_identifier_sets(
    en_path: &str,
    en_line: usize,
    ko_path: &str,
    ko_line: usize,
    position: usize,
    unit: &str,
    en_required: &BTreeSet<String>,
    ko_required: &BTreeSet<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    let missing_in_ko = en_required
        .difference(ko_required)
        .cloned()
        .collect::<BTreeSet<_>>();
    if !missing_in_ko.is_empty() {
        issues.push(ValidationIssue::at_line(
            ko_path,
            "identifier_parity.missing",
            Some(ko_line),
            format!(
                "corresponding {unit} unit {position} is missing exact identifier(s) present at {en_path}:{en_line}: {}",
                format_identifiers(&missing_in_ko)
            ),
        ));
    }

    let missing_in_en = ko_required
        .difference(en_required)
        .cloned()
        .collect::<BTreeSet<_>>();
    if !missing_in_en.is_empty() {
        issues.push(ValidationIssue::at_line(
            en_path,
            "identifier_parity.missing",
            Some(en_line),
            format!(
                "corresponding {unit} unit {position} is missing exact identifier(s) present at {ko_path}:{ko_line}: {}",
                format_identifiers(&missing_in_en)
            ),
        ));
    }
}

fn format_identifiers(identifiers: &BTreeSet<String>) -> String {
    identifiers
        .iter()
        .map(|identifier| format!("`{identifier}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_identifier_drift_only_for_catalog_literals() {
        let identifiers = ["queued", "state"].into_iter().map(str::to_owned).collect();
        let en = markdown::identifier_structure(
            "# State\n\nThe `state` is `queued`; `local_note` is ignored.",
            &identifiers,
        );
        let ko = markdown::identifier_structure(
            "# 상태\n\n`state` 값입니다. `local_note`는 무시됩니다.",
            &identifiers,
        );
        let mut issues = Vec::new();
        compare_identifier_sets(
            "docs/en/example.md",
            en.sections[1].line,
            "docs/ko/example.md",
            ko.sections[1].line,
            1,
            "section",
            &en.sections[1].units[0].identifiers,
            &ko.sections[1].units[0].identifiers,
            &mut issues,
        );

        assert_eq!(issues.len(), 1);
        assert!(issues[0].message().contains("`queued`"));
        assert!(!issues[0].message().contains("local_note"));
        assert_eq!(issues[0].line(), Some(1));
    }

    #[test]
    fn accepts_the_same_catalog_identifier_in_a_corresponding_section() {
        let identifiers = ["queued"].into_iter().map(str::to_owned).collect();
        let en = markdown::identifier_structure("# State\n\nThe item is `queued`.", &identifiers);
        let ko = markdown::identifier_structure("# 상태\n\n항목은 `queued`입니다.", &identifiers);
        let mut issues = Vec::new();

        compare_identifier_sets(
            "docs/en/example.md",
            en.sections[1].line,
            "docs/ko/example.md",
            ko.sections[1].line,
            1,
            "section",
            &section_identifiers(&en.sections[1]),
            &section_identifiers(&ko.sections[1]),
            &mut issues,
        );

        assert!(issues.is_empty());
    }
}
