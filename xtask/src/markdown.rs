//! Shared Markdown parsing for links, anchors, and bilingual structure checks.

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownSection {
    pub(crate) heading_level: Option<u8>,
    pub(crate) line: usize,
    pub(crate) heading_identifiers: BTreeSet<String>,
    pub(crate) units: Vec<MarkdownUnit>,
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
        heading_identifiers: BTreeSet::new(),
        units: Vec::new(),
    }];
    let mut in_heading = false;
    let mut in_code_block = false;
    let mut active_unit = None;

    for (event, range) in Parser::new_ext(contents, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                finish_unit(&mut active_unit, &mut sections);
                sections.push(MarkdownSection {
                    heading_level: Some(heading_level(level)),
                    line: source_line(&newline_offsets, &range),
                    heading_identifiers: BTreeSet::new(),
                    units: Vec::new(),
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
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
                start_unit(
                    MarkdownUnitKind::CodeBlock,
                    source_line(&newline_offsets, &range),
                    &mut active_unit,
                );
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                finish_if_kind(MarkdownUnitKind::CodeBlock, &mut active_unit, &mut sections);
            }
            Event::Code(code) => {
                let identifiers = exact_identifier_mentions(&code, exact_identifiers);
                record_identifiers(&mut sections, active_unit.as_mut(), in_heading, identifiers);
            }
            Event::Text(text) if in_code_block => {
                record_identifiers(
                    &mut sections,
                    active_unit.as_mut(),
                    in_heading,
                    exact_identifier_mentions(&text, exact_identifiers),
                );
            }
            _ => {}
        }
    }
    finish_unit(&mut active_unit, &mut sections);

    MarkdownStructure { sections }
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
) -> BTreeSet<String> {
    exact_identifiers
        .iter()
        .filter(|identifier| contains_exact_identifier(literal, identifier))
        .cloned()
        .collect()
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
    fn extracts_only_catalog_identifiers_from_meaning_units() {
        let identifiers = ["Ready", "ready", "status"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let parsed = identifier_structure(
            "# `Ready`\n\nThe `status` is `ready`; `unmapped` is ignored.\n",
            &identifiers,
        );

        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(
            parsed.sections[1].heading_identifiers,
            ["Ready".to_owned()].into_iter().collect()
        );
        assert_eq!(
            parsed.sections[1].units[0].identifiers,
            ["ready".to_owned(), "status".to_owned()]
                .into_iter()
                .collect()
        );
    }

    #[test]
    fn does_not_match_identifiers_inside_larger_tokens() {
        let identifiers = ["complete".to_owned()].into_iter().collect();
        let parsed = identifier_structure("# Example\n\n`completed`\n", &identifiers);
        assert!(parsed.sections[1].units[0].identifiers.is_empty());
    }
}
