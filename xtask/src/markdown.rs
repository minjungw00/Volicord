//! Shared Markdown parsing for links, anchors, and bilingual structure checks.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::BTreeSet;
use std::ops::Range;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum MarkdownUnitKind {
    Paragraph,
    ListItem,
    TableRow,
    CodeBlock,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownUnit {
    pub(crate) kind: MarkdownUnitKind,
    pub(crate) line: usize,
    pub(crate) identifiers: BTreeSet<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum MarkdownLiteralKind {
    Inline,
    Fenced,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownLiteral {
    pub(crate) kind: MarkdownLiteralKind,
    pub(crate) unit_kind: Option<MarkdownUnitKind>,
    pub(crate) language: Option<String>,
    pub(crate) line: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownSection {
    pub(crate) heading_level: Option<u8>,
    pub(crate) line: usize,
    pub(crate) heading: String,
    pub(crate) heading_identifiers: BTreeSet<String>,
    pub(crate) units: Vec<MarkdownUnit>,
    pub(crate) literals: Vec<MarkdownLiteral>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownStructure {
    pub(crate) sections: Vec<MarkdownSection>,
}

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

pub(crate) fn identifier_structure(
    contents: &str,
    exact_identifiers: &BTreeSet<String>,
) -> MarkdownStructure {
    let newline_offsets = contents
        .bytes()
        .enumerate()
        .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset))
        .collect::<Vec<_>>();
    let mut sections = vec![MarkdownSection {
        heading_level: None,
        line: 1,
        heading: "document preamble".to_owned(),
        heading_identifiers: BTreeSet::new(),
        units: Vec::new(),
        literals: Vec::new(),
    }];
    let mut in_heading = false;
    let mut in_code_block = false;
    let mut code_block_language = None;
    let mut active_unit = None;

    for (event, range) in Parser::new_ext(contents, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                finish_unit(&mut active_unit, &mut sections);
                sections.push(MarkdownSection {
                    heading_level: Some(heading_level(level)),
                    line: source_line(&newline_offsets, &range),
                    heading: String::new(),
                    heading_identifiers: BTreeSet::new(),
                    units: Vec::new(),
                    literals: Vec::new(),
                });
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => in_heading = false,
            Event::Start(Tag::Item) => start_unit(
                MarkdownUnitKind::ListItem,
                source_line(&newline_offsets, &range),
                &mut active_unit,
            ),
            Event::End(TagEnd::Item) => {
                finish_if_kind(MarkdownUnitKind::ListItem, &mut active_unit, &mut sections)
            }
            Event::Start(Tag::TableRow) => start_unit(
                MarkdownUnitKind::TableRow,
                source_line(&newline_offsets, &range),
                &mut active_unit,
            ),
            Event::End(TagEnd::TableRow) => {
                finish_if_kind(MarkdownUnitKind::TableRow, &mut active_unit, &mut sections)
            }
            Event::Start(Tag::Paragraph) => start_unit(
                MarkdownUnitKind::Paragraph,
                source_line(&newline_offsets, &range),
                &mut active_unit,
            ),
            Event::End(TagEnd::Paragraph) => {
                finish_if_kind(MarkdownUnitKind::Paragraph, &mut active_unit, &mut sections)
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                in_code_block = true;
                code_block_language = match kind {
                    CodeBlockKind::Indented => None,
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .filter(|language| !language.is_empty())
                        .map(|language| language.to_ascii_lowercase()),
                };
                start_unit(
                    MarkdownUnitKind::CodeBlock,
                    source_line(&newline_offsets, &range),
                    &mut active_unit,
                );
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                code_block_language = None;
                finish_if_kind(MarkdownUnitKind::CodeBlock, &mut active_unit, &mut sections);
            }
            Event::Code(code) => {
                let unit_kind = active_unit.as_ref().map(|unit| unit.kind);
                let identifiers = exact_identifier_mentions(
                    &code,
                    exact_identifiers,
                    unit_kind == Some(MarkdownUnitKind::TableRow),
                );
                record_identifiers(&mut sections, active_unit.as_mut(), in_heading, identifiers);
                if in_heading {
                    append_heading_text(&mut sections, &code);
                }
                sections
                    .last_mut()
                    .expect("section exists")
                    .literals
                    .push(MarkdownLiteral {
                        kind: MarkdownLiteralKind::Inline,
                        unit_kind,
                        language: None,
                        line: source_line(&newline_offsets, &range),
                        text: code.into_string(),
                    });
            }
            Event::Text(text) if in_code_block => {
                let identifiers = code_block_identifier_mentions(
                    &text,
                    code_block_language.as_deref(),
                    exact_identifiers,
                );
                record_identifiers(&mut sections, active_unit.as_mut(), in_heading, identifiers);
                sections
                    .last_mut()
                    .expect("section exists")
                    .literals
                    .push(MarkdownLiteral {
                        kind: MarkdownLiteralKind::Fenced,
                        unit_kind: Some(MarkdownUnitKind::CodeBlock),
                        language: code_block_language.clone(),
                        line: source_line(&newline_offsets, &range),
                        text: text.into_string(),
                    });
            }
            Event::Text(text) if in_heading => append_heading_text(&mut sections, &text),
            _ => {}
        }
    }
    finish_unit(&mut active_unit, &mut sections);

    MarkdownStructure { sections }
}

fn append_heading_text(sections: &mut [MarkdownSection], text: &str) {
    let heading = &mut sections.last_mut().expect("heading section exists").heading;
    if !heading.is_empty() {
        heading.push(' ');
    }
    heading.push_str(text);
}

fn start_unit(kind: MarkdownUnitKind, line: usize, active: &mut Option<MarkdownUnit>) {
    if active.is_none() {
        *active = Some(MarkdownUnit {
            kind,
            line,
            identifiers: BTreeSet::new(),
        });
    }
}

fn finish_if_kind(
    kind: MarkdownUnitKind,
    active: &mut Option<MarkdownUnit>,
    sections: &mut [MarkdownSection],
) {
    if active.as_ref().is_some_and(|unit| unit.kind == kind) {
        finish_unit(active, sections);
    }
}

fn finish_unit(active: &mut Option<MarkdownUnit>, sections: &mut [MarkdownSection]) {
    if let Some(unit) = active.take() {
        sections
            .last_mut()
            .expect("preamble section exists")
            .units
            .push(unit);
    }
}

fn record_identifiers(
    sections: &mut [MarkdownSection],
    active: Option<&mut MarkdownUnit>,
    in_heading: bool,
    identifiers: BTreeSet<String>,
) {
    if in_heading {
        let section = sections.last_mut().expect("heading section exists");
        section.heading_identifiers.extend(identifiers);
    } else if let Some(unit) = active {
        unit.identifiers.extend(identifiers);
    }
}

fn source_line(newline_offsets: &[usize], range: &Range<usize>) -> usize {
    newline_offsets.partition_point(|offset| *offset < range.start) + 1
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn exact_identifier_mentions(
    literal: &str,
    exact_identifiers: &BTreeSet<String>,
    allow_simple_identifiers: bool,
) -> BTreeSet<String> {
    exact_identifiers
        .iter()
        .filter(|identifier| {
            allow_simple_identifiers || is_explicit_contract_identifier(identifier)
        })
        .filter(|identifier| contains_exact_identifier(literal, identifier))
        .cloned()
        .collect()
}

fn code_block_identifier_mentions(
    literal: &str,
    language: Option<&str>,
    exact_identifiers: &BTreeSet<String>,
) -> BTreeSet<String> {
    match language {
        Some("json" | "yaml" | "yml") => {
            let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(literal) else {
                return BTreeSet::new();
            };
            let mut tokens = BTreeSet::new();
            collect_structured_tokens(&value, &mut tokens);
            tokens.intersection(exact_identifiers).cloned().collect()
        }
        Some("bash" | "console" | "sh" | "shell" | "zsh") => {
            exact_identifier_mentions(literal, exact_identifiers, true)
        }
        _ => BTreeSet::new(),
    }
}

fn collect_structured_tokens(value: &serde_yaml::Value, tokens: &mut BTreeSet<String>) {
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if let Some(key) = key.as_str() {
                    tokens.insert(normalize_structured_key(key).to_owned());
                }
                collect_structured_tokens(value, tokens);
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for value in sequence {
                collect_structured_tokens(value, tokens);
            }
        }
        serde_yaml::Value::String(value) if looks_like_structured_identifier(value) => {
            tokens.insert(value.to_owned());
        }
        _ => {}
    }
}

fn normalize_structured_key(key: &str) -> &str {
    key.strip_suffix('?').unwrap_or(key)
}

fn looks_like_structured_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
}

pub(crate) fn is_explicit_contract_identifier(identifier: &str) -> bool {
    identifier.chars().any(|character| {
        matches!(character, '_' | '-' | '.' | ' ') || character.is_ascii_uppercase()
    })
}

fn contains_exact_identifier(haystack: &str, identifier: &str) -> bool {
    if identifier.is_empty() {
        return false;
    }
    for (start, _) in haystack.match_indices(identifier) {
        let end = start + identifier.len();
        let before = haystack[..start].chars().next_back();
        let after = haystack[end..].chars().next();
        if !before.is_some_and(is_identifier_character)
            && !after.is_some_and(is_identifier_character)
        {
            return true;
        }
    }
    false
}

fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_explicit_catalog_identifiers_from_inline_meaning_units() {
        let identifiers = ["Ready", "ready", "state_version", "status"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let parsed = identifier_structure(
            "# `Ready`\n\nThe `state_version` has `status` `ready`; `unmapped` is ignored.\n",
            &identifiers,
        );

        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(
            parsed.sections[1].heading_identifiers,
            ["Ready".to_owned()].into_iter().collect()
        );
        assert_eq!(
            parsed.sections[1].units[0].identifiers,
            ["state_version".to_owned()].into_iter().collect()
        );
    }

    #[test]
    fn does_not_match_identifiers_inside_larger_tokens() {
        let identifiers = ["complete".to_owned()].into_iter().collect();
        let parsed = identifier_structure("# Example\n\n`completed`\n", &identifiers);
        assert!(parsed.sections[1].units[0].identifiers.is_empty());
    }

    #[test]
    fn structured_optional_marker_preserves_the_exact_field_identifier() {
        let identifiers = ["continuity_page".to_owned()].into_iter().collect();
        let parsed = identifier_structure(
            "# Example\n\n```yaml\ncontinuity_page?: object\n```\n",
            &identifiers,
        );

        assert_eq!(
            parsed.sections[1].units[0].identifiers,
            ["continuity_page".to_owned()].into_iter().collect()
        );
    }
}
