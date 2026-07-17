use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RustTokenKind {
    Identifier,
    Punctuation(u8),
    Literal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RustToken<'a> {
    kind: RustTokenKind,
    text: &'a str,
    start: usize,
    end: usize,
}

const FORBIDDEN_HOST_IMPLEMENTATION_MARKERS: &[(&str, &str)] = &[
    ("codex", "Codex-specific implementation knowledge"),
    ("claude", "removed host implementation knowledge"),
    (".codex", "host configuration path knowledge"),
    ("hooks.json", "host configuration filename knowledge"),
    ("config.toml", "host configuration filename knowledge"),
    ("_hook", "host hook command knowledge"),
    ("_final-output", "removed host command knowledge"),
    ("managed_marker", "generated wrapper marker knowledge"),
    ("wrapper_path", "generated wrapper path knowledge"),
    (
        "shell_command",
        "host shell-command representation knowledge",
    ),
];

#[test]
fn production_core_contains_no_host_adapter_implementation_knowledge() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    inspect_source_tree(&source, &mut violations);
    assert!(
        violations.is_empty(),
        "Core must consume typed host receipts without adapter implementation knowledge:\n{}",
        violations.join("\n")
    );
}

fn inspect_source_tree(path: &Path, violations: &mut Vec<String>) {
    let mut entries = fs::read_dir(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("failed to enumerate {}: {error}", path.display()));
    entries.sort_by_key(std::fs::DirEntry::file_name);

    for entry in entries {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if entry.file_name() != "tests" {
                inspect_source_tree(&entry_path, violations);
            }
            continue;
        }
        if entry_path.extension().and_then(|value| value.to_str()) != Some("rs") {
            continue;
        }
        let source = fs::read_to_string(&entry_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", entry_path.display()));
        let production_source = mask_inline_cfg_test_modules(&source);
        for (line_index, line) in production_source.lines().enumerate() {
            let normalized = line.to_ascii_lowercase();
            for (marker, meaning) in FORBIDDEN_HOST_IMPLEMENTATION_MARKERS {
                if normalized.contains(marker) {
                    violations.push(format!(
                        "{}:{} contains `{marker}` ({meaning})",
                        entry_path.display(),
                        line_index + 1
                    ));
                }
            }
        }
    }
}

fn mask_inline_cfg_test_modules(source: &str) -> String {
    let tokens = lex_rust(source);
    let mut excluded_ranges = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let Some(after_cfg) = exact_cfg_test_attribute_end(&tokens, index) else {
            index += 1;
            continue;
        };
        let Some(open_brace) = inline_module_open_brace(&tokens, after_cfg) else {
            index = after_cfg;
            continue;
        };
        let close_brace = matching_brace(&tokens, open_brace).unwrap_or_else(|| {
            panic!(
                "inline #[cfg(test)] module starting at byte {} has no closing brace",
                tokens[index].start
            )
        });
        excluded_ranges.push((tokens[index].start, tokens[close_brace].end));
        index = close_brace + 1;
    }

    let mut masked = source.as_bytes().to_vec();
    for (start, end) in excluded_ranges {
        for byte in &mut masked[start..end] {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(masked).expect("masking source with ASCII whitespace preserves valid UTF-8")
}

fn exact_cfg_test_attribute_end(tokens: &[RustToken<'_>], start: usize) -> Option<usize> {
    let expected = [
        RustTokenKind::Punctuation(b'#'),
        RustTokenKind::Punctuation(b'['),
        RustTokenKind::Identifier,
        RustTokenKind::Punctuation(b'('),
        RustTokenKind::Identifier,
        RustTokenKind::Punctuation(b')'),
        RustTokenKind::Punctuation(b']'),
    ];
    let candidate = tokens.get(start..start + expected.len())?;
    if candidate
        .iter()
        .zip(expected)
        .all(|(token, kind)| token.kind == kind)
        && candidate[2].text == "cfg"
        && candidate[4].text == "test"
    {
        Some(start + expected.len())
    } else {
        None
    }
}

fn inline_module_open_brace(tokens: &[RustToken<'_>], mut index: usize) -> Option<usize> {
    while is_punctuation(tokens.get(index), b'#') && is_punctuation(tokens.get(index + 1), b'[') {
        index = matching_delimiter(tokens, index + 1, b'[', b']')? + 1;
    }
    if is_identifier(tokens.get(index), "pub") {
        index += 1;
        if is_punctuation(tokens.get(index), b'(') {
            index = matching_delimiter(tokens, index, b'(', b')')? + 1;
        }
    }
    if !is_identifier(tokens.get(index), "mod")
        || tokens.get(index + 1)?.kind != RustTokenKind::Identifier
        || !is_punctuation(tokens.get(index + 2), b'{')
    {
        return None;
    }
    Some(index + 2)
}

fn matching_brace(tokens: &[RustToken<'_>], open: usize) -> Option<usize> {
    matching_delimiter(tokens, open, b'{', b'}')
}

fn matching_delimiter(
    tokens: &[RustToken<'_>],
    open: usize,
    opening: u8,
    closing: u8,
) -> Option<usize> {
    if !is_punctuation(tokens.get(open), opening) {
        return None;
    }
    let mut depth = 0_u32;
    for (index, token) in tokens.iter().enumerate().skip(open) {
        if token.kind == RustTokenKind::Punctuation(opening) {
            depth += 1;
        } else if token.kind == RustTokenKind::Punctuation(closing) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(index);
            }
        }
    }
    None
}

fn is_identifier(token: Option<&RustToken<'_>>, expected: &str) -> bool {
    token.is_some_and(|token| token.kind == RustTokenKind::Identifier && token.text == expected)
}

fn is_punctuation(token: Option<&RustToken<'_>>, expected: u8) -> bool {
    token.is_some_and(|token| token.kind == RustTokenKind::Punctuation(expected))
}

fn lex_rust(source: &str) -> Vec<RustToken<'_>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"//") {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index = block_comment_end(bytes, index);
            continue;
        }
        if let Some(end) = raw_string_end(bytes, index) {
            tokens.push(RustToken {
                kind: RustTokenKind::Literal,
                text: &source[index..end],
                start: index,
                end,
            });
            index = end;
            continue;
        }
        let quote = match bytes[index..].get(..2) {
            Some([b'b' | b'c', b'"']) => Some(index + 1),
            _ if bytes[index] == b'"' => Some(index),
            _ => None,
        };
        if let Some(quote) = quote {
            let end = quoted_literal_end(bytes, quote, b'"');
            tokens.push(RustToken {
                kind: RustTokenKind::Literal,
                text: &source[index..end],
                start: index,
                end,
            });
            index = end;
            continue;
        }
        if bytes[index] == b'\'' {
            if let Some(end) = char_literal_end(bytes, index) {
                tokens.push(RustToken {
                    kind: RustTokenKind::Literal,
                    text: &source[index..end],
                    start: index,
                    end,
                });
                index = end;
                continue;
            }
        }
        if is_identifier_start(bytes[index]) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_identifier_continue(bytes[index]) {
                index += 1;
            }
            tokens.push(RustToken {
                kind: RustTokenKind::Identifier,
                text: &source[start..index],
                start,
                end: index,
            });
            continue;
        }
        let start = index;
        index += source[start..]
            .chars()
            .next()
            .expect("a non-empty source suffix has a character")
            .len_utf8();
        tokens.push(RustToken {
            kind: RustTokenKind::Punctuation(bytes[start]),
            text: &source[start..index],
            start,
            end: index,
        });
    }
    tokens
}

fn block_comment_end(bytes: &[u8], start: usize) -> usize {
    let mut index = start + 2;
    let mut depth = 1_u32;
    while index < bytes.len() {
        if bytes[index..].starts_with(b"/*") {
            depth += 1;
            index += 2;
        } else if bytes[index..].starts_with(b"*/") {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += 1;
        }
    }
    panic!("unterminated block comment starting at byte {start}")
}

fn raw_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut prefix = start;
    if matches!(bytes.get(prefix), Some(b'b' | b'c')) {
        prefix += 1;
    }
    if bytes.get(prefix) != Some(&b'r') {
        return None;
    }
    let mut delimiter = prefix + 1;
    while bytes.get(delimiter) == Some(&b'#') {
        delimiter += 1;
    }
    if bytes.get(delimiter) != Some(&b'"') {
        return None;
    }
    let hash_count = delimiter - prefix - 1;
    let mut index = delimiter + 1;
    while index < bytes.len() {
        if bytes[index] == b'"'
            && bytes.get(index + 1..index + 1 + hash_count) == Some(&bytes[prefix + 1..delimiter])
        {
            return Some(index + 1 + hash_count);
        }
        index += 1;
    }
    panic!("unterminated raw string literal starting at byte {start}")
}

fn quoted_literal_end(bytes: &[u8], quote: usize, delimiter: u8) -> usize {
    let mut index = quote + 1;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index = (index + 2).min(bytes.len());
        } else if bytes[index] == delimiter {
            return index + 1;
        } else {
            index += 1;
        }
    }
    panic!("unterminated quoted literal starting at byte {quote}")
}

fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start + 1;
    if bytes.get(index) == Some(&b'\\') {
        index += 2;
    } else {
        index += 1;
    }
    (bytes.get(index) == Some(&b'\'')).then_some(index + 1)
}

fn is_identifier_start(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphabetic()
}

fn is_identifier_continue(byte: u8) -> bool {
    is_identifier_start(byte) || byte.is_ascii_digit()
}

#[test]
fn inline_cfg_test_modules_are_masked_without_changing_line_coordinates() {
    let source = r##"
const BEFORE: &str = "wrapper_path";
#[cfg(test)]
#[allow(dead_code)]
pub(crate) mod tests {
    const TEST_ONLY: &str = "codex";
    const BRACES: &str = r#"not module braces: { }"#;
    /* nested comment { /* and another */ } */
    mod nested { const VALUE: char = '}'; }
}
const AFTER: &str = "shell_command";
"##;

    let masked = mask_inline_cfg_test_modules(source);

    assert_eq!(masked.lines().count(), source.lines().count());
    assert!(masked.contains("wrapper_path"));
    assert!(masked.contains("shell_command"));
    assert!(!masked.contains("codex"));
    assert!(!masked.contains("TEST_ONLY"));
}

#[test]
fn cfg_test_text_in_production_literals_and_comments_is_not_a_skip_directive() {
    let source = r##"
const ATTRIBUTE_TEXT: &str = "#[cfg(test)] mod tests { codex }";
// #[cfg(test)] mod tests { claude }
const PRODUCTION_MARKER: &str = "config.toml";
"##;

    assert_eq!(mask_inline_cfg_test_modules(source), source);
}
