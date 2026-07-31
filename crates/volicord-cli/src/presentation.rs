//! Semantics-neutral primitives for deterministic human CLI output.
//!
//! The vocabulary is intentionally broader than the first migrated commands.

#![cfg_attr(not(test), allow(dead_code))]

use std::fmt::{self, Write};

const INDENT: &str = "  ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetailLevel {
    Compact,
    Verbose,
}

impl DetailLevel {
    const fn includes(self, minimum: Self) -> bool {
        matches!(
            (self, minimum),
            (Self::Verbose, _) | (Self::Compact, Self::Compact)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum YesNo {
    Yes,
    No,
}

impl From<bool> for YesNo {
    fn from(value: bool) -> Self {
        if value {
            Self::Yes
        } else {
            Self::No
        }
    }
}

impl fmt::Display for YesNo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Yes => "yes",
            Self::No => "no",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HumanValue {
    Text(String),
    YesNo(YesNo),
    None,
    Count(usize),
}

impl HumanValue {
    pub(crate) fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }
}

impl fmt::Display for HumanValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(value) => formatter.write_str(value),
            Self::YesNo(value) => write!(formatter, "{value}"),
            Self::None => formatter.write_str("none"),
            Self::Count(value) => write!(formatter, "{value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    label: String,
    value: HumanValue,
    minimum_detail: DetailLevel,
}

impl Field {
    pub(crate) fn new(label: impl Into<String>, value: HumanValue) -> Self {
        Self {
            label: label.into(),
            value,
            minimum_detail: DetailLevel::Compact,
        }
    }

    pub(crate) fn verbose(label: impl Into<String>, value: HumanValue) -> Self {
        Self {
            label: label.into(),
            value,
            minimum_detail: DetailLevel::Verbose,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Section {
    heading: String,
    body: Vec<Element>,
}

impl Section {
    pub(crate) fn new(heading: impl Into<String>, body: Vec<Element>) -> Self {
        Self {
            heading: heading.into(),
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NestedRecord {
    heading: String,
    fields: Vec<Field>,
}

impl NestedRecord {
    pub(crate) fn new(heading: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            heading: heading.into(),
            fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BulletList {
    items: Vec<String>,
}

impl BulletList {
    pub(crate) fn new(items: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CollectionItem {
    heading: String,
    fields: Vec<Field>,
}

impl CollectionItem {
    pub(crate) fn new(heading: impl Into<String>, fields: Vec<Field>) -> Self {
        Self {
            heading: heading.into(),
            fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionHint {
    action: String,
}

impl ActionHint {
    pub(crate) fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Element {
    Field(Field),
    Section(Section),
    NestedRecord(NestedRecord),
    Bullets(BulletList),
    CollectionItem(CollectionItem),
    ActionHint(ActionHint),
}

impl Element {
    fn is_group(&self) -> bool {
        matches!(
            self,
            Self::Section(_) | Self::NestedRecord(_) | Self::Bullets(_) | Self::CollectionItem(_)
        )
    }
}

impl From<Field> for Element {
    fn from(value: Field) -> Self {
        Self::Field(value)
    }
}

impl From<Section> for Element {
    fn from(value: Section) -> Self {
        Self::Section(value)
    }
}

impl From<NestedRecord> for Element {
    fn from(value: NestedRecord) -> Self {
        Self::NestedRecord(value)
    }
}

impl From<BulletList> for Element {
    fn from(value: BulletList) -> Self {
        Self::Bullets(value)
    }
}

impl From<CollectionItem> for Element {
    fn from(value: CollectionItem) -> Self {
        Self::CollectionItem(value)
    }
}

impl From<ActionHint> for Element {
    fn from(value: ActionHint) -> Self {
        Self::ActionHint(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Document {
    headline: String,
    detail: DetailLevel,
    body: Vec<Element>,
}

impl Document {
    pub(crate) fn new(headline: impl Into<String>, body: Vec<Element>) -> Self {
        Self {
            headline: headline.into(),
            detail: DetailLevel::Compact,
            body,
        }
    }

    pub(crate) fn verbose(headline: impl Into<String>, body: Vec<Element>) -> Self {
        Self {
            headline: headline.into(),
            detail: DetailLevel::Verbose,
            body,
        }
    }

    pub(crate) fn render(&self) -> String {
        let mut lines = vec![human_text(&self.headline)];
        if !self.body.is_empty() {
            lines.push(String::new());
            render_elements(&self.body, self.detail, 0, &mut lines);
        }
        while lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        let mut output = lines.join("\n");
        output.push('\n');
        output
    }
}

fn render_elements(
    elements: &[Element],
    detail: DetailLevel,
    depth: usize,
    lines: &mut Vec<String>,
) {
    let mut rendered_any = false;
    let mut previous_was_group = false;
    for element in elements {
        if !element_visible(element, detail) {
            continue;
        }
        let is_group = element.is_group();
        if rendered_any && (is_group || previous_was_group) {
            push_blank_line(lines);
        }
        render_element(element, detail, depth, lines);
        rendered_any = true;
        previous_was_group = is_group;
    }
}

fn element_visible(element: &Element, detail: DetailLevel) -> bool {
    match element {
        Element::Field(field) => detail.includes(field.minimum_detail),
        _ => true,
    }
}

fn render_element(element: &Element, detail: DetailLevel, depth: usize, lines: &mut Vec<String>) {
    match element {
        Element::Field(field) => render_field(field, detail, depth, lines),
        Element::Section(section) => {
            push_line(lines, depth, &section.heading);
            render_elements(&section.body, detail, depth + 1, lines);
        }
        Element::NestedRecord(record) => {
            push_line(lines, depth, &record.heading);
            for field in &record.fields {
                render_field(field, detail, depth + 1, lines);
            }
        }
        Element::Bullets(list) => {
            for item in &list.items {
                let mut line = String::new();
                line.push_str(&INDENT.repeat(depth));
                line.push_str("- ");
                line.push_str(&human_text(item));
                lines.push(line);
            }
        }
        Element::CollectionItem(item) => {
            push_line(lines, depth, &item.heading);
            for field in &item.fields {
                render_field(field, detail, depth + 1, lines);
            }
        }
        Element::ActionHint(hint) => {
            let value = HumanValue::text(&hint.action);
            render_field(&Field::new("Next action", value), detail, depth, lines);
        }
    }
}

fn render_field(field: &Field, detail: DetailLevel, depth: usize, lines: &mut Vec<String>) {
    if !detail.includes(field.minimum_detail) {
        return;
    }
    let mut line = INDENT.repeat(depth);
    write!(
        line,
        "{}: {}",
        human_text(&field.label),
        human_text(&field.value.to_string())
    )
    .expect("writing to a String cannot fail");
    lines.push(line);
}

fn push_line(lines: &mut Vec<String>, depth: usize, value: &str) {
    lines.push(format!("{}{}", INDENT.repeat(depth), human_text(value)));
}

fn push_blank_line(lines: &mut Vec<String>) {
    if lines.last().is_some_and(|line| !line.is_empty()) {
        lines.push(String::new());
    }
}

fn human_text(value: &str) -> String {
    let mut rendered = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            rendered.extend(character.escape_default());
        } else {
            rendered.push(character);
        }
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_vocabulary_is_deterministic_and_tab_free() {
        let document = Document::verbose(
            "Report",
            vec![
                Field::new("Enabled", HumanValue::YesNo(YesNo::Yes)).into(),
                Field::new("Optional", HumanValue::None).into(),
                Field::new("Items", HumanValue::Count(2)).into(),
                Field::verbose("Detail", HumanValue::text("full")).into(),
                Section::new(
                    "Section",
                    vec![
                        NestedRecord::new(
                            "Record",
                            vec![Field::new("Path", HumanValue::text("/a/long/path"))],
                        )
                        .into(),
                        BulletList::new(["first", "second\titem"]).into(),
                    ],
                )
                .into(),
                CollectionItem::new(
                    "Entry",
                    vec![Field::new("Status", HumanValue::text("active"))],
                )
                .into(),
                ActionHint::new("Run `command --flag`.").into(),
            ],
        );

        let output = document.render();
        assert!(output.contains("Enabled: yes"));
        assert!(output.contains("Optional: none"));
        assert!(output.contains("Items: 2"));
        assert!(output.contains("  - second\\titem"));
        assert!(!output.contains('\t'));
        assert!(output.ends_with('\n'));
        assert!(!output.ends_with("\n\n"));
    }

    #[test]
    fn compact_output_omits_verbose_only_fields() {
        let output = Document::new(
            "Report",
            vec![
                Field::new("Visible", HumanValue::YesNo(YesNo::No)).into(),
                Field::verbose("Hidden", HumanValue::text("value")).into(),
            ],
        )
        .render();

        assert_eq!(output, "Report\n\nVisible: no\n");
    }

    #[test]
    fn headline_only_has_exactly_one_trailing_newline() {
        assert_eq!(
            Document::new("No records are available.", Vec::new()).render(),
            "No records are available.\n"
        );
    }
}
