//! Shared Markdown parsing for links, anchors, and bilingual structure checks.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::fmt;
use std::ops::Range;

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum MarkdownUnitKind {
    Heading,
    Paragraph,
    ListItem,
    TableCell,
    DefinitionTitle,
    Definition,
    Callout,
    Footnote,
    CodeBlock,
}

impl fmt::Display for MarkdownUnitKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Heading => "heading",
            Self::Paragraph => "paragraph",
            Self::ListItem => "list item",
            Self::TableCell => "table cell",
            Self::DefinitionTitle => "definition title",
            Self::Definition => "definition",
            Self::Callout => "callout",
            Self::Footnote => "footnote",
            Self::CodeBlock => "code block",
        })
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct HeadingCoordinate {
    pub(crate) level: u8,
    pub(crate) ordinal: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum MeaningUnitCoordinate {
    ListItem(usize),
    TableRow(usize),
    TableCell(usize),
    Definition(usize),
    DefinitionPart(usize),
    Callout(usize),
    Footnote(usize),
    CodeExample(usize),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct MeaningUnitKey {
    pub(crate) heading_path: Vec<HeadingCoordinate>,
    pub(crate) block_ordinal: Option<usize>,
    pub(crate) kind: MarkdownUnitKind,
    pub(crate) coordinates: Vec<MeaningUnitCoordinate>,
}

impl fmt::Display for MeaningUnitKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.heading_path.is_empty() {
            formatter.write_str("preamble")?;
        } else {
            formatter.write_str("heading ")?;
            for (index, heading) in self.heading_path.iter().enumerate() {
                if index > 0 {
                    formatter.write_str("/")?;
                }
                write!(formatter, "h{}:{}", heading.level, heading.ordinal)?;
            }
        }
        if let Some(block) = self.block_ordinal {
            write!(formatter, " > block {block}")?;
        }
        write!(formatter, " > {}", self.kind)?;
        for coordinate in &self.coordinates {
            match coordinate {
                MeaningUnitCoordinate::ListItem(index) => write!(formatter, " {index}")?,
                MeaningUnitCoordinate::TableRow(index) => write!(formatter, " row {index}")?,
                MeaningUnitCoordinate::TableCell(index) => write!(formatter, " cell {index}")?,
                MeaningUnitCoordinate::Definition(index) => write!(formatter, " entry {index}")?,
                MeaningUnitCoordinate::DefinitionPart(index) => {
                    write!(formatter, " definition {index}")?
                }
                MeaningUnitCoordinate::Callout(index) => write!(formatter, " callout {index}")?,
                MeaningUnitCoordinate::Footnote(index) => write!(formatter, " footnote {index}")?,
                MeaningUnitCoordinate::CodeExample(index) => write!(formatter, " example {index}")?,
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum MarkdownLiteralKind {
    Inline,
    Fenced,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownLiteral {
    pub(crate) kind: MarkdownLiteralKind,
    pub(crate) language: Option<String>,
    pub(crate) attributes: Vec<String>,
    pub(crate) line: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownUnit {
    pub(crate) key: MeaningUnitKey,
    pub(crate) line: usize,
    pub(crate) owner_source: Option<String>,
    pub(crate) literals: Vec<MarkdownLiteral>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownSection {
    pub(crate) heading_level: Option<u8>,
    pub(crate) line: usize,
    pub(crate) heading: String,
    pub(crate) heading_path: Vec<HeadingCoordinate>,
    pub(crate) units: Vec<MarkdownUnit>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct MarkdownStructure {
    pub(crate) sections: Vec<MarkdownSection>,
}

impl MarkdownStructure {
    pub(crate) fn units(&self) -> impl Iterator<Item = &MarkdownUnit> {
        self.sections.iter().flat_map(|section| &section.units)
    }

    pub(crate) fn line_for_heading_path(&self, heading_path: &[HeadingCoordinate]) -> usize {
        self.sections
            .iter()
            .find(|section| section.heading_path == heading_path)
            .map_or(1, |section| section.line)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MarkdownOwnerRegion {
    pub(crate) range: Range<usize>,
    pub(crate) source_id: String,
}

#[derive(Debug)]
struct ListState {
    next_item: usize,
    current_item: Option<usize>,
}

#[derive(Debug, Default)]
struct TableState {
    row: usize,
    cell: usize,
}

#[derive(Debug, Default)]
struct DefinitionState {
    entry: usize,
    definition: usize,
}

#[derive(Debug, Default)]
struct CalloutState {
    coordinate: usize,
    next_child: usize,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum RootBlockEnd {
    Paragraph,
    List,
    Table,
    DefinitionList,
    BlockQuote,
    Footnote,
    CodeBlock,
}

#[derive(Debug)]
struct RootBlock {
    ordinal: usize,
    end: RootBlockEnd,
    next_code_example: usize,
    owner_source: Option<String>,
}

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_DEFINITION_LIST
}

pub(crate) fn structure(
    contents: &str,
    owner_regions: &[MarkdownOwnerRegion],
) -> MarkdownStructure {
    let parse_contents = mask_code_span_pipes(contents);
    let newline_offsets = contents
        .bytes()
        .enumerate()
        .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset))
        .collect::<Vec<_>>();
    let mut sections = vec![MarkdownSection {
        heading_level: None,
        line: 1,
        heading: "document preamble".to_owned(),
        heading_path: Vec::new(),
        units: Vec::new(),
    }];
    let mut heading_ordinals = [0_usize; 6];
    let mut current_heading_path = Vec::new();
    let mut active_heading_unit = None;
    let mut active_units = Vec::new();
    let mut paragraph_unit_started = Vec::new();
    let mut list_stack = Vec::<ListState>::new();
    let mut table = None::<TableState>;
    let mut definition = None::<DefinitionState>;
    let mut callout_stack = Vec::<CalloutState>::new();
    let mut footnote_ordinal = 0;
    let mut in_code_block = false;
    let mut code_block_language = None;
    let mut code_block_attributes = Vec::new();
    let mut block_ordinal = 0;
    let mut root_block = None::<RootBlock>;
    let mut pending_owner_source = None::<String>;

    for (event, range) in Parser::new_ext(&parse_contents, options()).into_offset_iter() {
        let line = source_line(&newline_offsets, &range);
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                pending_owner_source = None;
                let level = heading_level(level);
                let level_index = usize::from(level - 1);
                heading_ordinals[level_index] += 1;
                heading_ordinals[level_index + 1..].fill(0);
                current_heading_path = heading_ordinals
                    .iter()
                    .enumerate()
                    .take(level_index + 1)
                    .filter(|(_, ordinal)| **ordinal > 0)
                    .map(|(index, ordinal)| HeadingCoordinate {
                        level: u8::try_from(index + 1).expect("Markdown heading level fits u8"),
                        ordinal: *ordinal,
                    })
                    .collect();
                sections.push(MarkdownSection {
                    heading_level: Some(level),
                    line,
                    heading: String::new(),
                    heading_path: current_heading_path.clone(),
                    units: Vec::new(),
                });
                block_ordinal = 0;
                active_heading_unit = Some(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: None,
                        kind: MarkdownUnitKind::Heading,
                        coordinates: Vec::new(),
                    },
                    line,
                    range.start,
                    owner_regions,
                    None,
                ));
            }
            Event::End(TagEnd::Heading(_)) => active_heading_unit = None,
            Event::Start(Tag::Paragraph) => {
                let started = if active_units.is_empty() {
                    let ordinal = ensure_root_block(
                        &mut root_block,
                        &mut block_ordinal,
                        RootBlockEnd::Paragraph,
                        &mut pending_owner_source,
                    );
                    active_units.push(push_unit(
                        &mut sections,
                        MeaningUnitKey {
                            heading_path: current_heading_path.clone(),
                            block_ordinal: Some(ordinal),
                            kind: MarkdownUnitKind::Paragraph,
                            coordinates: Vec::new(),
                        },
                        line,
                        range.start,
                        owner_regions,
                        root_block
                            .as_ref()
                            .and_then(|root| root.owner_source.as_deref()),
                    ));
                    true
                } else {
                    false
                };
                paragraph_unit_started.push(started);
            }
            Event::End(TagEnd::Paragraph) => {
                if paragraph_unit_started.pop().unwrap_or(false) {
                    active_units.pop();
                }
                finish_root_block(&mut root_block, RootBlockEnd::Paragraph);
            }
            Event::Start(Tag::List(_)) => {
                ensure_root_block(
                    &mut root_block,
                    &mut block_ordinal,
                    RootBlockEnd::List,
                    &mut pending_owner_source,
                );
                list_stack.push(ListState {
                    next_item: 0,
                    current_item: None,
                });
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                if list_stack.is_empty() {
                    finish_root_block(&mut root_block, RootBlockEnd::List);
                }
            }
            Event::Start(Tag::Item) => {
                let Some(list) = list_stack.last_mut() else {
                    continue;
                };
                list.next_item += 1;
                list.current_item = Some(list.next_item);
                let coordinates = list_stack
                    .iter()
                    .filter_map(|list| list.current_item)
                    .map(MeaningUnitCoordinate::ListItem)
                    .collect();
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(current_root_ordinal(&root_block)),
                        kind: MarkdownUnitKind::ListItem,
                        coordinates,
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::Item) => {
                active_units.pop();
                if let Some(list) = list_stack.last_mut() {
                    list.current_item = None;
                }
            }
            Event::Start(Tag::Table(_)) => {
                ensure_root_block(
                    &mut root_block,
                    &mut block_ordinal,
                    RootBlockEnd::Table,
                    &mut pending_owner_source,
                );
                table = Some(TableState::default());
            }
            Event::End(TagEnd::Table) => {
                table = None;
                finish_root_block(&mut root_block, RootBlockEnd::Table);
            }
            Event::Start(Tag::TableHead) => {
                let table = table.as_mut().expect("table head belongs to a table");
                table.row += 1;
                table.cell = 0;
            }
            Event::End(TagEnd::TableHead) => {}
            Event::Start(Tag::TableRow) => {
                let table = table.as_mut().expect("table row belongs to a table");
                table.row += 1;
                table.cell = 0;
            }
            Event::End(TagEnd::TableRow) => {}
            Event::Start(Tag::TableCell) => {
                let table = table.as_mut().expect("table cell belongs to a table");
                table.cell += 1;
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(current_root_ordinal(&root_block)),
                        kind: MarkdownUnitKind::TableCell,
                        coordinates: vec![
                            MeaningUnitCoordinate::TableRow(table.row),
                            MeaningUnitCoordinate::TableCell(table.cell),
                        ],
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::TableCell) => {
                active_units.pop();
            }
            Event::Start(Tag::DefinitionList) => {
                ensure_root_block(
                    &mut root_block,
                    &mut block_ordinal,
                    RootBlockEnd::DefinitionList,
                    &mut pending_owner_source,
                );
                definition = Some(DefinitionState::default());
            }
            Event::End(TagEnd::DefinitionList) => {
                definition = None;
                finish_root_block(&mut root_block, RootBlockEnd::DefinitionList);
            }
            Event::Start(Tag::DefinitionListTitle) => {
                let definition = definition
                    .as_mut()
                    .expect("definition title belongs to a definition list");
                definition.entry += 1;
                definition.definition = 0;
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(current_root_ordinal(&root_block)),
                        kind: MarkdownUnitKind::DefinitionTitle,
                        coordinates: vec![MeaningUnitCoordinate::Definition(definition.entry)],
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::DefinitionListTitle) => {
                active_units.pop();
            }
            Event::Start(Tag::DefinitionListDefinition) => {
                let definition = definition
                    .as_mut()
                    .expect("definition belongs to a definition list");
                definition.definition += 1;
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(current_root_ordinal(&root_block)),
                        kind: MarkdownUnitKind::Definition,
                        coordinates: vec![
                            MeaningUnitCoordinate::Definition(definition.entry),
                            MeaningUnitCoordinate::DefinitionPart(definition.definition),
                        ],
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::DefinitionListDefinition) => {
                active_units.pop();
            }
            Event::Start(Tag::BlockQuote(_)) => {
                ensure_root_block(
                    &mut root_block,
                    &mut block_ordinal,
                    RootBlockEnd::BlockQuote,
                    &mut pending_owner_source,
                );
                let coordinate = if let Some(parent) = callout_stack.last_mut() {
                    parent.next_child += 1;
                    parent.next_child
                } else {
                    1
                };
                callout_stack.push(CalloutState {
                    coordinate,
                    next_child: 0,
                });
                let coordinates = callout_stack
                    .iter()
                    .map(|callout| MeaningUnitCoordinate::Callout(callout.coordinate))
                    .collect();
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(current_root_ordinal(&root_block)),
                        kind: MarkdownUnitKind::Callout,
                        coordinates,
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                active_units.pop();
                callout_stack.pop();
                if callout_stack.is_empty() {
                    finish_root_block(&mut root_block, RootBlockEnd::BlockQuote);
                }
            }
            Event::Start(Tag::FootnoteDefinition(_)) => {
                footnote_ordinal += 1;
                let ordinal = ensure_root_block(
                    &mut root_block,
                    &mut block_ordinal,
                    RootBlockEnd::Footnote,
                    &mut pending_owner_source,
                );
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(ordinal),
                        kind: MarkdownUnitKind::Footnote,
                        coordinates: vec![MeaningUnitCoordinate::Footnote(footnote_ordinal)],
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                active_units.pop();
                finish_root_block(&mut root_block, RootBlockEnd::Footnote);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let was_nested = root_block.is_some();
                let ordinal = ensure_root_block(
                    &mut root_block,
                    &mut block_ordinal,
                    RootBlockEnd::CodeBlock,
                    &mut pending_owner_source,
                );
                let coordinates = if was_nested {
                    let root = root_block.as_mut().expect("root block exists");
                    root.next_code_example += 1;
                    let mut coordinates = active_units
                        .last()
                        .and_then(|index| {
                            sections.last().expect("section exists").units.get(*index)
                        })
                        .map(|unit| unit.key.coordinates.clone())
                        .unwrap_or_default();
                    coordinates.push(MeaningUnitCoordinate::CodeExample(root.next_code_example));
                    coordinates
                } else {
                    Vec::new()
                };
                in_code_block = true;
                match kind {
                    CodeBlockKind::Indented => {
                        code_block_language = None;
                        code_block_attributes.clear();
                    }
                    CodeBlockKind::Fenced(info) => {
                        let mut parts = info.split_whitespace();
                        code_block_language = parts
                            .next()
                            .filter(|language| !language.is_empty())
                            .map(|language| language.to_ascii_lowercase());
                        code_block_attributes = parts.map(str::to_owned).collect();
                    }
                }
                active_units.push(push_unit(
                    &mut sections,
                    MeaningUnitKey {
                        heading_path: current_heading_path.clone(),
                        block_ordinal: Some(ordinal),
                        kind: MarkdownUnitKind::CodeBlock,
                        coordinates,
                    },
                    line,
                    range.start,
                    owner_regions,
                    root_block
                        .as_ref()
                        .and_then(|root| root.owner_source.as_deref()),
                ));
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
                code_block_language = None;
                code_block_attributes.clear();
                active_units.pop();
                finish_root_block(&mut root_block, RootBlockEnd::CodeBlock);
            }
            Event::Code(code) => {
                let unit_index = active_heading_unit.or_else(|| active_units.last().copied());
                if let Some(unit_index) = unit_index {
                    sections.last_mut().expect("section exists").units[unit_index]
                        .literals
                        .push(MarkdownLiteral {
                            kind: MarkdownLiteralKind::Inline,
                            language: None,
                            attributes: Vec::new(),
                            line,
                            text: restore_masked_pipes(&code),
                        });
                }
                if active_heading_unit.is_some() {
                    append_heading_text(&mut sections, &code);
                }
            }
            Event::Text(text) if in_code_block => {
                if let Some(unit_index) = active_units.last().copied() {
                    sections.last_mut().expect("section exists").units[unit_index]
                        .literals
                        .push(MarkdownLiteral {
                            kind: MarkdownLiteralKind::Fenced,
                            language: code_block_language.clone(),
                            attributes: code_block_attributes.clone(),
                            line,
                            text: restore_masked_pipes(&text),
                        });
                }
            }
            Event::Text(text) if active_heading_unit.is_some() => {
                append_heading_text(&mut sections, &text);
            }
            _ => {}
        }
    }

    MarkdownStructure { sections }
}

fn mask_code_span_pipes(contents: &str) -> String {
    let mut bytes = contents.as_bytes().to_vec();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] != b'`' {
            cursor += 1;
            continue;
        }
        let delimiter_start = cursor;
        while cursor < bytes.len() && bytes[cursor] == b'`' {
            cursor += 1;
        }
        let delimiter_length = cursor - delimiter_start;
        let content_start = cursor;
        let mut closing = None;
        while cursor < bytes.len() {
            if bytes[cursor] != b'`' {
                cursor += 1;
                continue;
            }
            let run_start = cursor;
            while cursor < bytes.len() && bytes[cursor] == b'`' {
                cursor += 1;
            }
            if cursor - run_start == delimiter_length {
                closing = Some(run_start);
                break;
            }
        }
        let Some(closing) = closing else {
            break;
        };
        for byte in &mut bytes[content_start..closing] {
            if *byte == b'|' {
                *byte = b'\x1f';
            }
        }
    }
    String::from_utf8(bytes).expect("masking ASCII pipe bytes preserves UTF-8")
}

fn restore_masked_pipes(contents: &str) -> String {
    contents.replace('\u{1f}', "|")
}

fn append_heading_text(sections: &mut [MarkdownSection], text: &str) {
    let heading = &mut sections.last_mut().expect("heading section exists").heading;
    if !heading.is_empty() {
        heading.push(' ');
    }
    heading.push_str(text);
}

fn push_unit(
    sections: &mut [MarkdownSection],
    key: MeaningUnitKey,
    line: usize,
    offset: usize,
    owner_regions: &[MarkdownOwnerRegion],
    declared_owner: Option<&str>,
) -> usize {
    let section = sections.last_mut().expect("section exists");
    let index = section.units.len();
    section.units.push(MarkdownUnit {
        key,
        line,
        owner_source: declared_owner.map(str::to_owned).or_else(|| {
            owner_regions
                .iter()
                .find(|region| region.range.contains(&offset))
                .map(|region| region.source_id.clone())
        }),
        literals: Vec::new(),
    });
    index
}

fn ensure_root_block(
    root: &mut Option<RootBlock>,
    block_ordinal: &mut usize,
    end: RootBlockEnd,
    pending_owner_source: &mut Option<String>,
) -> usize {
    if root.is_none() {
        *block_ordinal += 1;
        *root = Some(RootBlock {
            ordinal: *block_ordinal,
            end,
            next_code_example: 0,
            owner_source: pending_owner_source.take(),
        });
    }
    current_root_ordinal(root)
}

fn current_root_ordinal(root: &Option<RootBlock>) -> usize {
    root.as_ref()
        .expect("Markdown unit has a root block")
        .ordinal
}

fn finish_root_block(root: &mut Option<RootBlock>, end: RootBlockEnd) {
    if root.as_ref().is_some_and(|root| root.end == end) {
        *root = None;
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

pub(crate) fn contains_exact_identifier(haystack: &str, identifier: &str) -> bool {
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
    fn assigns_deterministic_paragraph_and_nested_list_keys() {
        let parsed = structure(
            "# Example\n\nFirst.\n\n- Parent `state`\n  - Child `ready`\n",
            &[],
        );
        let keys = parsed
            .units()
            .map(|unit| unit.key.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            [
                "heading h1:1 > heading",
                "heading h1:1 > block 1 > paragraph",
                "heading h1:1 > block 2 > list item 1",
                "heading h1:1 > block 2 > list item 1 1",
            ]
        );
    }

    #[test]
    fn assigns_each_table_cell_its_own_coordinate() {
        let parsed = structure(
            "# Example\n\n| Field | Value |\n|---|---|\n| `status` | `ready` |\n",
            &[],
        );
        let keys = parsed
            .units()
            .filter(|unit| unit.key.kind == MarkdownUnitKind::TableCell)
            .map(|unit| unit.key.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            keys,
            [
                "heading h1:1 > block 1 > table cell row 1 cell 1",
                "heading h1:1 > block 1 > table cell row 1 cell 2",
                "heading h1:1 > block 1 > table cell row 2 cell 1",
                "heading h1:1 > block 1 > table cell row 2 cell 2",
            ]
        );
    }

    #[test]
    fn table_cells_preserve_code_after_union_pipes() {
        let parsed = structure(
            "# Example\n\n| Field | Value |\n|---|---|\n| `state` | `StateRecordRef | null` with `record_kind=state` |\n",
            &[],
        );
        let literals = parsed
            .units()
            .find(|unit| unit.key.to_string() == "heading h1:1 > block 1 > table cell row 2 cell 2")
            .expect("value cell")
            .literals
            .iter()
            .map(|literal| literal.text.as_str())
            .collect::<Vec<_>>();

        assert_eq!(literals, ["StateRecordRef | null", "record_kind=state"]);
    }

    #[test]
    fn owner_regions_annotate_generated_units() {
        let markdown = "# Manual\n\n<!-- begin -->\n## `volicord inspect`\n\n```text\n--report\n```\n<!-- end -->\n";
        let start = markdown.find("<!-- begin -->").expect("begin");
        let end = markdown.find("<!-- end -->").expect("end") + "<!-- end -->".len();
        let parsed = structure(
            markdown,
            &[MarkdownOwnerRegion {
                range: start..end,
                source_id: "administrative_cli".to_owned(),
            }],
        );

        assert!(parsed
            .units()
            .filter(|unit| unit.key.heading_path.len() == 2)
            .all(|unit| unit.owner_source.as_deref() == Some("administrative_cli")));
    }

    #[test]
    fn exact_identifier_matching_includes_simple_lowercase_values() {
        assert!(contains_exact_identifier("status is ready", "ready"));
        assert!(!contains_exact_identifier("status is unready", "ready"));
    }
}
