use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use crate::markdown;
use std::fs;
use std::path::Path;

pub(crate) fn validate_bilingual_structure(
    root: &Path,
    index: &DocIndex,
    issues: &mut Vec<ValidationIssue>,
) {
    for paired in index.paired_documents.values() {
        let en = match read_structure(root, &paired.path_en) {
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
        let ko = match read_structure(root, &paired.path_ko) {
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
    }
}

fn read_structure(root: &Path, relative_path: &str) -> Result<markdown::MarkdownStructure, String> {
    let contents = fs::read_to_string(root.join(relative_path))
        .map_err(|error| format!("failed to read paired Markdown: {error}"))?;
    Ok(markdown::structure(&contents, &[]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_parser_ignores_identifier_contents() {
        let en = markdown::structure("# State\n\nThe item is `queued`.", &[]);
        let ko = markdown::structure("# 상태\n\n항목은 `local_note`입니다.", &[]);

        assert_eq!(
            en.sections
                .iter()
                .filter_map(|section| section.heading_level)
                .collect::<Vec<_>>(),
            ko.sections
                .iter()
                .filter_map(|section| section.heading_level)
                .collect::<Vec<_>>()
        );
    }
}
