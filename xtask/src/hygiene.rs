use crate::cli_docs::{is_closing_fence, opening_fence};
use crate::diagnostics::ValidationIssue;
use crate::doc_index::DocIndex;
use crate::repository::repo_relative;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const PUBLIC_LANGUAGE_SOURCE_ROOTS: &[&str] = &[
    "crates/volicord-cli/src/connection_command.rs",
    "crates/volicord-cli/src/connection_command",
    "crates/volicord-cli/src/doctor_command.rs",
    "crates/volicord-cli/src/guard_command.rs",
    "crates/volicord-cli/src/guard_command",
    "crates/volicord-cli/src/guard_integration",
    "crates/volicord-cli/src/host_integration",
    "crates/volicord-cli/src/setup_command.rs",
    "crates/volicord-cli/src/setup_command",
    "crates/volicord-cli/src/user_command.rs",
    "crates/volicord-mcp/src",
];
const PUBLIC_UNQUALIFIED_SECURITY_WORDS: &[&str] = &["safe", "secure", "protected"];
const PUBLIC_AMBIGUOUS_HOST_SUPPORT_PHRASES: &[PublicHostSupportPhrase] = &[
    PublicHostSupportPhrase::ascii("supported agent hosts"),
    PublicHostSupportPhrase::ascii("supported agent host"),
    PublicHostSupportPhrase::ascii("supported agent-hosts"),
    PublicHostSupportPhrase::ascii("supported agent-host"),
    PublicHostSupportPhrase::ascii("supported managed hosts"),
    PublicHostSupportPhrase::ascii("supported managed host"),
    PublicHostSupportPhrase::ascii("supported managed-hosts"),
    PublicHostSupportPhrase::ascii("supported managed-host"),
    PublicHostSupportPhrase::ascii("supported-hosts"),
    PublicHostSupportPhrase::ascii("supported-host"),
    PublicHostSupportPhrase::ascii("supported hosts"),
    PublicHostSupportPhrase::ascii("supported host"),
    PublicHostSupportPhrase::ascii("supported agent connections"),
    PublicHostSupportPhrase::ascii("supported agent connection"),
    PublicHostSupportPhrase::ascii("support for agent connection"),
    PublicHostSupportPhrase::ascii("agent connection support"),
    PublicHostSupportPhrase::ascii("agent connections are supported"),
    PublicHostSupportPhrase::ascii("agent connection is supported"),
    PublicHostSupportPhrase::ascii("supported managed connection hosts"),
    PublicHostSupportPhrase::ascii("supported managed connection host"),
    PublicHostSupportPhrase::ascii("host support for"),
    PublicHostSupportPhrase::ascii("agent hosts are supported"),
    PublicHostSupportPhrase::ascii("agent host is supported"),
    PublicHostSupportPhrase::ascii("agent-hosts are supported"),
    PublicHostSupportPhrase::ascii("agent-host is supported"),
    PublicHostSupportPhrase::ascii("managed hosts are supported"),
    PublicHostSupportPhrase::ascii("managed host is supported"),
    PublicHostSupportPhrase::ascii("managed-hosts are supported"),
    PublicHostSupportPhrase::ascii("managed-host is supported"),
    PublicHostSupportPhrase::ascii("hosts are supported"),
    PublicHostSupportPhrase::ascii("host is supported"),
    PublicHostSupportPhrase::ascii("supported codex"),
    PublicHostSupportPhrase::ascii("supported claude code"),
    PublicHostSupportPhrase::ascii("supports codex"),
    PublicHostSupportPhrase::ascii("supports claude code"),
    PublicHostSupportPhrase::ascii("supports both codex and claude code"),
    PublicHostSupportPhrase::ascii("codex and claude code are supported"),
    PublicHostSupportPhrase::ascii("codex and claude code are fully supported"),
    PublicHostSupportPhrase::ascii("codex is supported"),
    PublicHostSupportPhrase::ascii("codex is fully supported"),
    PublicHostSupportPhrase::ascii("codex support is available"),
    PublicHostSupportPhrase::ascii("claude code is supported"),
    PublicHostSupportPhrase::ascii("claude code is fully supported"),
    PublicHostSupportPhrase::ascii("claude code support is available"),
    PublicHostSupportPhrase::ascii("supported record profile"),
    PublicHostSupportPhrase::ascii("supported detective profile"),
    PublicHostSupportPhrase::ascii("supported `record` profile"),
    PublicHostSupportPhrase::ascii("supported `detective` profile"),
    PublicHostSupportPhrase::ascii("supported record host configuration"),
    PublicHostSupportPhrase::ascii("supported detective host configuration"),
    PublicHostSupportPhrase::ascii("supports the record profile"),
    PublicHostSupportPhrase::ascii("supports the detective profile"),
    PublicHostSupportPhrase::ascii("supports record profile"),
    PublicHostSupportPhrase::ascii("supports detective profile"),
    PublicHostSupportPhrase::ascii("supports the `record` profile"),
    PublicHostSupportPhrase::ascii("supports the `detective` profile"),
    PublicHostSupportPhrase::ascii("supports `record` profile"),
    PublicHostSupportPhrase::ascii("supports `detective` profile"),
    PublicHostSupportPhrase::ascii("supports `--profile record`"),
    PublicHostSupportPhrase::ascii("supports `--profile detective`"),
    PublicHostSupportPhrase::ascii("record profile is supported"),
    PublicHostSupportPhrase::ascii("detective profile is supported"),
    PublicHostSupportPhrase::ascii("`record` profile is supported"),
    PublicHostSupportPhrase::ascii("`detective` profile is supported"),
    PublicHostSupportPhrase::ascii("record host configuration is supported"),
    PublicHostSupportPhrase::ascii("detective host configuration is supported"),
    PublicHostSupportPhrase::ascii("record and detective profiles are supported"),
    PublicHostSupportPhrase::ascii("`record` and `detective` profiles are supported"),
    PublicHostSupportPhrase::ascii("supports record and detective profiles"),
    PublicHostSupportPhrase::ascii("supports the record and detective profiles"),
    PublicHostSupportPhrase::korean("지원되는 에이전트 호스트"),
    PublicHostSupportPhrase::korean("지원하는 에이전트 호스트"),
    PublicHostSupportPhrase::korean("지원되는 관리 호스트"),
    PublicHostSupportPhrase::korean("지원하는 관리 호스트"),
    PublicHostSupportPhrase::korean("지원되는 호스트"),
    PublicHostSupportPhrase::korean("지원하는 호스트"),
    PublicHostSupportPhrase::korean("지원 호스트"),
    PublicHostSupportPhrase::korean("에이전트 호스트가 지원됩니다"),
    PublicHostSupportPhrase::korean("관리 호스트가 지원됩니다"),
    PublicHostSupportPhrase::korean("호스트가 지원됩니다"),
    PublicHostSupportPhrase::korean("지원되는 Agent Connection"),
    PublicHostSupportPhrase::korean("Agent Connection 지원"),
    PublicHostSupportPhrase::korean("지원되는 에이전트 연결"),
    PublicHostSupportPhrase::korean("Agent Connection이 지원됩니다"),
    PublicHostSupportPhrase::korean("Agent Connection은 지원됩니다"),
    PublicHostSupportPhrase::korean("Agent Connection을 지원합니다"),
    PublicHostSupportPhrase::korean("에이전트 연결이 지원됩니다"),
    PublicHostSupportPhrase::korean("에이전트 연결을 지원합니다"),
    PublicHostSupportPhrase::korean("지원되는 Codex"),
    PublicHostSupportPhrase::korean("지원하는 Codex"),
    PublicHostSupportPhrase::korean("지원되는 Claude Code"),
    PublicHostSupportPhrase::korean("지원하는 Claude Code"),
    PublicHostSupportPhrase::korean("Codex를 지원합니다"),
    PublicHostSupportPhrase::korean("Claude Code를 지원합니다"),
    PublicHostSupportPhrase::korean("Codex가 지원됩니다"),
    PublicHostSupportPhrase::korean("Claude Code가 지원됩니다"),
    PublicHostSupportPhrase::korean("Codex와 Claude Code를 지원합니다"),
    PublicHostSupportPhrase::korean("Codex와 Claude Code가 지원됩니다"),
    PublicHostSupportPhrase::korean("지원되는 `record` 프로필"),
    PublicHostSupportPhrase::korean("지원되는 `detective` 프로필"),
    PublicHostSupportPhrase::korean("지원되는 record 프로필"),
    PublicHostSupportPhrase::korean("지원되는 detective 프로필"),
    PublicHostSupportPhrase::korean("`record` 프로필이 지원됩니다"),
    PublicHostSupportPhrase::korean("`detective` 프로필이 지원됩니다"),
    PublicHostSupportPhrase::korean("record 프로필이 지원됩니다"),
    PublicHostSupportPhrase::korean("detective 프로필이 지원됩니다"),
    PublicHostSupportPhrase::korean("`record` 프로필을 지원합니다"),
    PublicHostSupportPhrase::korean("`detective` 프로필을 지원합니다"),
    PublicHostSupportPhrase::korean("record·detective 프로필을 지원합니다"),
    PublicHostSupportPhrase::korean("`record`·`detective` 프로필을 지원합니다"),
    PublicHostSupportPhrase::korean("record 및 detective 프로필을 지원합니다"),
    PublicHostSupportPhrase::korean("`record` 및 `detective` 프로필을 지원합니다"),
    PublicHostSupportPhrase::korean("지원되는 detective 호스트 설정"),
    PublicHostSupportPhrase::korean("detective 호스트 설정이 지원됩니다"),
    PublicHostSupportPhrase::korean("`--profile record`를 지원합니다"),
    PublicHostSupportPhrase::korean("`--profile detective`를 지원합니다"),
];
const PUBLIC_DOCUMENT_DISALLOWED_TERMS: &[PublicDocumentDisallowedTerm] = &[
    PublicDocumentDisallowedTerm {
        term: "write-readiness",
        replacement: "write-ticket",
    },
    PublicDocumentDisallowedTerm {
        term: "write readiness",
        replacement: "write ticket",
    },
];
struct PublicDocumentDisallowedTerm {
    term: &'static str,
    replacement: &'static str,
}

struct PublicHostSupportPhrase {
    phrase: &'static str,
    require_end_boundary: bool,
}

impl PublicHostSupportPhrase {
    const fn ascii(phrase: &'static str) -> Self {
        Self {
            phrase,
            require_end_boundary: true,
        }
    }

    const fn korean(phrase: &'static str) -> Self {
        Self {
            phrase,
            require_end_boundary: false,
        }
    }
}

pub(crate) fn validate_public_language_claims(root: &Path, errors: &mut Vec<ValidationIssue>) {
    if !PUBLIC_LANGUAGE_SOURCE_ROOTS
        .iter()
        .any(|relative| root.join(relative).exists())
    {
        return;
    }

    let mut sources = BTreeSet::new();
    for relative in PUBLIC_LANGUAGE_SOURCE_ROOTS {
        collect_public_language_source_root(root, relative, &mut sources, errors);
    }

    for relative in sources {
        let path = root.join(&relative);
        let contents = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    relative.clone(),
                    "public_language.read",
                    format!("failed to read public output source: {error}"),
                ));
                continue;
            }
        };
        for (index, line) in contents.lines().enumerate() {
            if let Some(phrase) = ambiguous_host_support_phrase(line) {
                errors.push(ValidationIssue::new(
                    relative.clone(),
                    "public_language.ambiguous_host_support_claim",
                    format!(
                        "line {} uses ambiguous host, profile, or connection support wording `{phrase}` in public output source; name the accepted `HOST` value, built-in adapter, configuration or environment boundary, or exact feature and `support_status`; only `verified` establishes current feature support",
                        index + 1
                    ),
                ));
            }
            for word in PUBLIC_UNQUALIFIED_SECURITY_WORDS {
                if contains_ascii_word_ignore_case(line, word) {
                    errors.push(ValidationIssue::new(
                        relative.clone(),
                        "public_language.security_claim",
                        format!(
                            "line {} uses unqualified `{word}` in public output source; use explicit guarantee disclosure wording",
                            index + 1
                        ),
                    ));
                }
            }
        }
    }
}

fn collect_public_language_source_root(
    root: &Path,
    relative: &str,
    sources: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    let path = root.join(relative);
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) => {
            errors.push(ValidationIssue::new(
                relative,
                "public_language.read",
                format!("failed to inspect public output source: {error}"),
            ));
            return;
        }
    };

    if metadata.is_dir() {
        collect_public_language_source_dir(root, &path, sources, errors);
    } else if metadata.is_file() && is_rust_source_path(&path) {
        sources.insert(relative.to_string());
    }
}

fn collect_public_language_source_dir(
    root: &Path,
    dir: &Path,
    sources: &mut BTreeSet<String>,
    errors: &mut Vec<ValidationIssue>,
) {
    let mut entries = Vec::new();
    let read_dir = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            errors.push(ValidationIssue::new(
                repo_relative(root, dir),
                "public_language.read",
                format!("failed to read public output source directory: {error}"),
            ));
            return;
        }
    };

    for entry in read_dir {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => {
                errors.push(ValidationIssue::new(
                    repo_relative(root, dir),
                    "public_language.read",
                    format!("failed to read public output source directory entry: {error}"),
                ));
            }
        }
    }
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    repo_relative(root, &path),
                    "public_language.read",
                    format!("failed to inspect public output source: {error}"),
                ));
                continue;
            }
        };

        if file_type.is_dir() {
            collect_public_language_source_dir(root, &path, sources, errors);
        } else if file_type.is_file() && is_rust_source_path(&path) {
            sources.insert(repo_relative(root, &path));
        }
    }
}

fn is_rust_source_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("rs")
}

pub(crate) fn validate_public_document_language(
    root: &Path,
    index: &DocIndex,
    errors: &mut Vec<ValidationIssue>,
) {
    for path in index
        .indexed_paths
        .iter()
        .filter(|path| path.ends_with(".md"))
    {
        let contents = match fs::read_to_string(root.join(path)) {
            Ok(contents) => contents,
            Err(error) => {
                errors.push(ValidationIssue::new(
                    path,
                    "public_language.read",
                    format!("failed to read Markdown file: {error}"),
                ));
                continue;
            }
        };

        let mut active_fence = None;
        for (index, line) in contents.lines().enumerate() {
            if let Some(fence) = active_fence.as_ref() {
                if is_closing_fence(line, fence) {
                    active_fence = None;
                }
                continue;
            }
            if let Some(fence) = opening_fence(line) {
                active_fence = Some(fence);
                continue;
            }

            let lower = line.to_ascii_lowercase();
            if let Some(phrase) = ambiguous_host_support_phrase(line) {
                errors.push(ValidationIssue::new(
                    path,
                    "public_language.ambiguous_host_support_claim",
                    format!(
                        "line {} uses ambiguous host, profile, or connection support wording `{phrase}` in public documentation; name the accepted `HOST` value, built-in adapter, configuration or environment boundary, or exact feature and `support_status`; only `verified` establishes current feature support",
                        index + 1
                    ),
                ));
            }
            for disallowed in PUBLIC_DOCUMENT_DISALLOWED_TERMS {
                if lower.contains(disallowed.term) {
                    errors.push(ValidationIssue::new(
                        path,
                        "public_language.write_ticket_term",
                        format!(
                            "line {} uses `{}` in public documentation; use `{}` terminology for the write-ticket concept",
                            index + 1,
                            disallowed.term,
                            disallowed.replacement
                        ),
                    ));
                }
            }
        }
    }
}

fn ambiguous_host_support_phrase(line: &str) -> Option<&'static str> {
    let lower = line.to_ascii_lowercase();
    PUBLIC_AMBIGUOUS_HOST_SUPPORT_PHRASES
        .iter()
        .find(|candidate| {
            contains_phrase_with_token_boundaries(
                &lower,
                candidate.phrase,
                candidate.require_end_boundary,
            )
        })
        .map(|candidate| candidate.phrase)
}

fn contains_phrase_with_token_boundaries(
    line: &str,
    phrase: &str,
    require_end_boundary: bool,
) -> bool {
    let phrase = phrase.to_ascii_lowercase();
    let mut search_from = 0;

    while let Some(offset) = line[search_from..].find(&phrase) {
        let start = search_from + offset;
        let end = start + phrase.len();
        let before = line[..start].chars().next_back();
        let after = line[end..].chars().next();
        if !is_public_language_token_char(before)
            && (!require_end_boundary || !is_public_language_token_char(after))
        {
            return true;
        }
        search_from = end;
    }

    false
}

fn is_public_language_token_char(character: Option<char>) -> bool {
    character.is_some_and(|character| character.is_alphanumeric() || character == '_')
}

fn contains_ascii_word_ignore_case(line: &str, word: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let word = word.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let needle = word.as_bytes();
    if needle.is_empty() || bytes.len() < needle.len() {
        return false;
    }
    for start in 0..=bytes.len() - needle.len() {
        if &bytes[start..start + needle.len()] != needle {
            continue;
        }
        let before = start
            .checked_sub(1)
            .and_then(|index| bytes.get(index))
            .copied();
        let after = bytes.get(start + needle.len()).copied();
        if !is_ascii_word_byte(before) && !is_ascii_word_byte(after) {
            return true;
        }
    }
    false
}

fn is_ascii_word_byte(byte: Option<u8>) -> bool {
    matches!(byte, Some(b'a'..=b'z' | b'0'..=b'9' | b'_'))
}
