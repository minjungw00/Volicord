use crate::diagnostics::ValidationIssue;
use crate::doc_index::{DocIndex, PairedDocument};
use crate::markdown;
use crate::repository::{normalize_path, path_to_slash, repo_relative};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

fn is_ignored_link(link: &str) -> bool {
    let trimmed = link.trim();
    trimmed.is_empty() || has_uri_scheme(trimmed)
}

fn has_uri_scheme(link: &str) -> bool {
    let Some(colon_index) = link.find(':') else {
        return false;
    };
    let scheme = &link[..colon_index];
    !scheme.is_empty()
        && scheme.chars().enumerate().all(|(index, character)| {
            if index == 0 {
                character.is_ascii_alphabetic()
            } else {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
            }
        })
}

fn percent_decode(value: &str) -> std::result::Result<String, String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("truncated percent escape".to_string());
            }
            let high =
                hex_value(bytes[index + 1]).ok_or_else(|| "invalid percent escape".to_string())?;
            let low =
                hex_value(bytes[index + 2]).ok_or_else(|| "invalid percent escape".to_string())?;
            decoded.push(high << 4 | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }

    String::from_utf8(decoded).map_err(|error| error.to_string())
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LinkFailure {
    pub(crate) category: &'static str,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SemanticLinkKey {
    target: SemanticLinkTarget,
    fragment: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum SemanticLinkTarget {
    DocId(String),
    RepositoryPath(String),
}

#[derive(Debug, Clone)]
pub(crate) struct MarkdownAnchors {
    pub(crate) anchors: BTreeSet<String>,
}

#[derive(Default)]
pub(crate) struct AnchorCache {
    pub(crate) files: BTreeMap<String, MarkdownAnchors>,
}

pub(crate) fn validate_markdown_links(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    let mut cache = AnchorCache::default();
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md"))
    {
        let absolute_path = root.join(path);
        let contents = match fs::read_to_string(&absolute_path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    path,
                    "link.read",
                    format!("failed to read Markdown file: {error}"),
                ));
                continue;
            }
        };
        for link in markdown_links(&contents) {
            if is_ignored_link(&link) {
                continue;
            }
            if let Err(failure) = validate_local_target(root, path, &link, &mut cache) {
                errors.push(ValidationIssue::new(
                    path,
                    failure.category,
                    failure.message,
                ));
            }
        }
    }
}

pub(crate) fn validate_bilingual_link_parity(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    for paired in index.paired_documents.values() {
        let en_links = match collect_semantic_links(root, index, &paired.path_en) {
            Ok(links) => links,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    &paired.path_en,
                    "bilingual_link.read",
                    error,
                ));
                continue;
            }
        };
        let ko_links = match collect_semantic_links(root, index, &paired.path_ko) {
            Ok(links) => links,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    &paired.path_ko,
                    "bilingual_link.read",
                    error,
                ));
                continue;
            }
        };

        compare_semantic_link_multisets(paired, en_links, ko_links, errors);
    }
}

fn collect_semantic_links(
    root: &Path,
    index: &DocIndex,
    path: &str,
) -> std::result::Result<BTreeMap<SemanticLinkKey, usize>, String> {
    let contents = fs::read_to_string(root.join(path))
        .map_err(|error| format!("failed to read Markdown file: {error}"))?;
    let mut links = BTreeMap::new();
    for link in markdown_reader_links(&contents) {
        if is_ignored_link(&link) {
            continue;
        }
        if let Some(key) = normalize_semantic_link(root, index, path, &link) {
            *links.entry(key).or_insert(0) += 1;
        }
    }
    Ok(links)
}

fn normalize_semantic_link(
    root: &Path,
    index: &DocIndex,
    source: &str,
    link: &str,
) -> Option<SemanticLinkKey> {
    let resolved = resolve_link_target(root, source, link).ok()?;
    let target_absolute = root.join(&resolved.path);
    if !target_absolute.exists() {
        return None;
    }

    let indexed_lookup_path = indexed_target_lookup_path(root, &resolved.path);
    let target = index
        .path_doc_ids
        .get(&indexed_lookup_path)
        .cloned()
        .map(SemanticLinkTarget::DocId)
        .unwrap_or_else(|| SemanticLinkTarget::RepositoryPath(resolved.path));

    Some(SemanticLinkKey {
        target,
        fragment: resolved.fragment,
    })
}

fn indexed_target_lookup_path(root: &Path, path: &str) -> String {
    let absolute = root.join(path);
    if absolute.is_dir() {
        let readme = absolute.join("README.md");
        if readme.exists() {
            return repo_relative(root, &readme);
        }
    }
    path.to_string()
}

fn compare_semantic_link_multisets(
    paired: &PairedDocument,
    en_links: BTreeMap<SemanticLinkKey, usize>,
    ko_links: BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationIssue>,
) {
    let mut only_en = multiset_difference(&en_links, &ko_links);
    let mut only_ko = multiset_difference(&ko_links, &en_links);

    report_fragment_mismatches(paired, &mut only_en, &mut only_ko, errors);
    report_target_mismatches(paired, &mut only_en, &mut only_ko, errors);
    report_unpaired_semantic_links(paired, "bilingual_link.only_en", true, only_en, errors);
    report_unpaired_semantic_links(paired, "bilingual_link.only_ko", false, only_ko, errors);
}

fn multiset_difference(
    left: &BTreeMap<SemanticLinkKey, usize>,
    right: &BTreeMap<SemanticLinkKey, usize>,
) -> BTreeMap<SemanticLinkKey, usize> {
    let mut difference = BTreeMap::new();
    for (key, left_count) in left {
        let right_count = right.get(key).copied().unwrap_or(0);
        if *left_count > right_count {
            difference.insert(key.clone(), left_count - right_count);
        }
    }
    difference
}

fn report_fragment_mismatches(
    paired: &PairedDocument,
    only_en: &mut BTreeMap<SemanticLinkKey, usize>,
    only_ko: &mut BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationIssue>,
) {
    let en_keys = only_en.keys().cloned().collect::<Vec<_>>();
    for en_key in en_keys {
        while count_for(only_en, &en_key) > 0 {
            let Some(ko_key) = only_ko
                .keys()
                .find(|ko_key| ko_key.target == en_key.target && ko_key.fragment != en_key.fragment)
                .cloned()
            else {
                break;
            };
            let count = count_for(only_en, &en_key).min(count_for(only_ko, &ko_key));
            consume_count(only_en, &en_key, count);
            consume_count(only_ko, &ko_key, count);
            errors.push(ValidationIssue::new(
                &paired.path_en,
                "bilingual_link.fragment_mismatch",
                format!(
                    "{} has {count} paired local semantic link occurrence(s) to {} but different fragments: English {}, Korean {} ({} <-> {})",
                    paired.doc_id,
                    en_key.target.describe(),
                    describe_fragment(&en_key.fragment),
                    describe_fragment(&ko_key.fragment),
                    paired.path_en,
                    paired.path_ko
                ),
            ));
        }
    }
}

fn report_target_mismatches(
    paired: &PairedDocument,
    only_en: &mut BTreeMap<SemanticLinkKey, usize>,
    only_ko: &mut BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationIssue>,
) {
    let en_keys = only_en.keys().cloned().collect::<Vec<_>>();
    for en_key in en_keys {
        while count_for(only_en, &en_key) > 0 {
            let Some(ko_key) = only_ko
                .keys()
                .find(|ko_key| ko_key.fragment == en_key.fragment && ko_key.target != en_key.target)
                .cloned()
            else {
                break;
            };
            let count = count_for(only_en, &en_key).min(count_for(only_ko, &ko_key));
            consume_count(only_en, &en_key, count);
            consume_count(only_ko, &ko_key, count);
            errors.push(ValidationIssue::new(
                &paired.path_en,
                "bilingual_link.target_mismatch",
                format!(
                    "{} has {count} paired local semantic link occurrence(s) with {} but different normalized targets: English {}, Korean {} ({} <-> {})",
                    paired.doc_id,
                    describe_fragment(&en_key.fragment),
                    en_key.target.describe(),
                    ko_key.target.describe(),
                    paired.path_en,
                    paired.path_ko
                ),
            ));
        }
    }
}

fn report_unpaired_semantic_links(
    paired: &PairedDocument,
    category: &'static str,
    english_surplus: bool,
    links: BTreeMap<SemanticLinkKey, usize>,
    errors: &mut Vec<ValidationIssue>,
) {
    for (key, count) in links {
        let language = if english_surplus { "English" } else { "Korean" };
        let paired_language = if english_surplus { "Korean" } else { "English" };
        errors.push(ValidationIssue::new(
            &paired.path_en,
            category,
            format!(
                "{} has {count} more {language} occurrence(s) of local semantic link to {} than {paired_language} ({} <-> {})",
                paired.doc_id,
                key.describe(),
                paired.path_en,
                paired.path_ko
            ),
        ));
    }
}

fn count_for(links: &BTreeMap<SemanticLinkKey, usize>, key: &SemanticLinkKey) -> usize {
    links.get(key).copied().unwrap_or(0)
}

fn consume_count(
    links: &mut BTreeMap<SemanticLinkKey, usize>,
    key: &SemanticLinkKey,
    count: usize,
) {
    if let Some(current) = links.get_mut(key) {
        *current -= count;
        if *current == 0 {
            links.remove(key);
        }
    }
}

impl SemanticLinkKey {
    fn describe(&self) -> String {
        match &self.fragment {
            Some(fragment) => format!("{}#{fragment}", self.target.describe()),
            None => format!("{} without fragment", self.target.describe()),
        }
    }
}

impl SemanticLinkTarget {
    fn describe(&self) -> String {
        match self {
            SemanticLinkTarget::DocId(doc_id) => format!("target {doc_id}"),
            SemanticLinkTarget::RepositoryPath(path) => format!("repository path {path}"),
        }
    }
}

fn describe_fragment(fragment: &Option<String>) -> String {
    match fragment {
        Some(fragment) => format!("#{fragment}"),
        None => "no fragment".to_string(),
    }
}

fn markdown_links(contents: &str) -> Vec<String> {
    markdown_destinations(contents, true)
}

fn markdown_reader_links(contents: &str) -> Vec<String> {
    markdown_destinations(contents, false)
}

fn markdown_destinations(contents: &str, include_images: bool) -> Vec<String> {
    let mut links = Vec::new();
    let parser = Parser::new_ext(contents, markdown::options());
    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                links.push(dest_url.to_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) if include_images => {
                links.push(dest_url.to_string());
            }
            _ => {}
        }
    }
    links
}

fn validate_local_target(
    root: &Path,
    source: &str,
    link: &str,
    cache: &mut AnchorCache,
) -> std::result::Result<(), LinkFailure> {
    let resolved = resolve_link_target(root, source, link).map_err(|message| LinkFailure {
        category: "link.invalid_target",
        message,
    })?;

    let target_absolute = root.join(&resolved.path);
    if !target_absolute.exists() {
        return Err(LinkFailure {
            category: "link.missing_target",
            message: format!("link {link} resolves to missing target {}", resolved.path),
        });
    }

    if let Some(fragment) = resolved.fragment {
        let anchor_path = if target_absolute.is_dir() {
            let readme = target_absolute.join("README.md");
            if readme.exists() {
                repo_relative(root, &readme)
            } else {
                return Err(LinkFailure {
                    category: "link.missing_fragment",
                    message: format!(
                        "link {link} has fragment #{fragment}, but {} is a directory without README.md",
                        resolved.path
                    ),
                });
            }
        } else {
            resolved.path.clone()
        };

        if !anchor_path.ends_with(".md") {
            return Err(LinkFailure {
                category: "link.missing_fragment",
                message: format!(
                    "link {link} has fragment #{fragment}, but {anchor_path} is not Markdown"
                ),
            });
        }

        let anchors = cache
            .anchors_for(root, &anchor_path)
            .map_err(|message| LinkFailure {
                category: "link.read",
                message,
            })?;
        if !anchors.contains_fragment(&fragment) {
            return Err(LinkFailure {
                category: "link.missing_fragment",
                message: format!(
                    "link {link} resolves to {anchor_path} without fragment #{fragment}"
                ),
            });
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct ResolvedLink {
    path: String,
    fragment: Option<String>,
}

fn resolve_link_target(
    root: &Path,
    source: &str,
    link: &str,
) -> std::result::Result<ResolvedLink, String> {
    let (path_part, fragment) = split_link(link);
    let path_part = percent_decode(&path_part)
        .map_err(|error| format!("link {link} has invalid percent encoding: {error}"))?;
    let fragment = fragment
        .map(|fragment| {
            percent_decode(&fragment).map(|decoded| decoded.trim_start_matches('#').to_string())
        })
        .transpose()
        .map_err(|error| format!("link {link} has invalid fragment percent encoding: {error}"))?;

    let source_parent = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    let joined = if path_part.is_empty() {
        root.join(source)
    } else if let Some(stripped) = path_part.strip_prefix('/') {
        root.join(stripped)
    } else {
        root.join(source_parent).join(path_part)
    };
    let normalized = normalize_path(&joined);
    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| format!("link {link} resolves outside the repository"))?;

    Ok(ResolvedLink {
        path: path_to_slash(relative),
        fragment,
    })
}

pub(crate) fn split_link(link: &str) -> (String, Option<String>) {
    let without_query = link.split('?').next().unwrap_or(link);
    match without_query.split_once('#') {
        Some((path, fragment)) => (path.to_string(), Some(fragment.to_string())),
        None => (without_query.to_string(), None),
    }
}

impl AnchorCache {
    pub(crate) fn anchors_for(
        &mut self,
        root: &Path,
        path: &str,
    ) -> std::result::Result<&MarkdownAnchors, String> {
        if !self.files.contains_key(path) {
            let contents = fs::read_to_string(root.join(path))
                .map_err(|error| format!("failed to read Markdown target {path}: {error}"))?;
            let anchors = collect_markdown_anchors(&contents);
            self.files.insert(path.to_string(), anchors);
        }
        Ok(self.files.get(path).expect("anchor cache entry inserted"))
    }
}

impl MarkdownAnchors {
    pub(crate) fn contains_fragment(&self, fragment: &str) -> bool {
        self.anchors.contains(fragment)
            || fragment
                .strip_prefix("user-content-")
                .is_some_and(|stripped| self.anchors.contains(stripped))
    }
}

fn collect_markdown_anchors(contents: &str) -> MarkdownAnchors {
    let mut anchors = BTreeSet::new();
    let mut slug_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut heading_text = String::new();
    let mut in_heading = false;

    for event in Parser::new_ext(contents, markdown::options()) {
        match event {
            Event::Start(Tag::Heading { id, .. }) => {
                in_heading = true;
                heading_text.clear();
                if let Some(id) = id {
                    anchors.insert(id.to_string());
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let base = slugify_heading(&heading_text);
                if !base.is_empty() {
                    let count = slug_counts.entry(base.clone()).or_insert(0);
                    let anchor = if *count == 0 {
                        base
                    } else {
                        format!("{base}-{count}")
                    };
                    *count += 1;
                    anchors.insert(anchor);
                }
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                for id in html_anchor_ids(&html) {
                    anchors.insert(id);
                }
            }
            _ => {}
        }
    }

    MarkdownAnchors { anchors }
}

fn slugify_heading(heading: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in heading.trim().chars() {
        for lower in character.to_lowercase() {
            if lower.is_alphanumeric() {
                slug.push(lower);
                previous_dash = false;
            } else if lower.is_whitespace() || lower == '-' {
                if !previous_dash && !slug.is_empty() {
                    slug.push('-');
                    previous_dash = true;
                }
            } else if lower == '_' {
                slug.push(lower);
                previous_dash = false;
            }
        }
    }

    slug.trim_matches('-').to_string()
}

fn html_anchor_ids(html: &str) -> Vec<String> {
    let mut ids = Vec::new();
    ids.extend(html_attribute_values(html, "id"));
    if html.trim_start().to_ascii_lowercase().starts_with("<a") {
        ids.extend(html_attribute_values(html, "name"));
    }
    ids
}

fn html_attribute_values(html: &str, attribute: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut values = Vec::new();
    let mut search_start = 0;
    let needle = format!("{attribute}=");

    while let Some(offset) = lower[search_start..].find(&needle) {
        let value_start = search_start + offset + needle.len();
        let Some(quote) = html[value_start..].chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            search_start = value_start;
            continue;
        }
        let content_start = value_start + quote.len_utf8();
        let Some(end_offset) = html[content_start..].find(quote) else {
            break;
        };
        values.push(html[content_start..content_start + end_offset].to_string());
        search_start = content_start + end_offset + quote.len_utf8();
    }

    values
}
