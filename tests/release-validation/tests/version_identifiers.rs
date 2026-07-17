use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

const ALLOWLIST_PATH: &str = "tests/release-validation/contracts/version-identifier-allowlist.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Allowlist {
    entries: Vec<AllowlistEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AllowlistEntry {
    path: String,
    needle: String,
    classification: String,
    reason: String,
}

#[test]
fn numeric_version_identifiers_are_explicitly_classified() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("release-validation package must live under tests/");
    let allowlist_text = fs::read_to_string(root.join(ALLOWLIST_PATH)).expect("read allowlist");
    let allowlist: Allowlist =
        serde_json::from_str(&allowlist_text).expect("strict allowlist JSON");
    assert!(!allowlist.entries.is_empty(), "allowlist must be explicit");

    for entry in &allowlist.entries {
        assert!(!entry.path.is_empty(), "allowlist path must not be empty");
        assert!(
            !entry.needle.is_empty(),
            "allowlist needle must not be empty"
        );
        assert!(
            matches!(
                entry.classification.as_str(),
                "external_standard"
                    | "third_party_api"
                    | "release_tag_example"
                    | "maintenance_metadata"
                    | "negative_test"
                    | "negative_documentation_example"
            ),
            "unknown allowlist classification: {}",
            entry.classification
        );
        assert!(
            !entry.reason.is_empty(),
            "allowlist reason must not be empty"
        );
    }

    let mut files = Vec::new();
    collect_source_files(root, root, &mut files).expect("collect repository sources");
    let mut matched_entries = BTreeSet::new();
    let mut unexpected = Vec::new();

    for path in files {
        let relative = repository_path(root, &path);
        if relative == ALLOWLIST_PATH {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_index, line) in text.lines().enumerate() {
            for (start, end) in numeric_version_markers(line) {
                let mut classified = false;
                for (entry_index, entry) in allowlist.entries.iter().enumerate() {
                    if entry.path == relative
                        && needle_covers(entry.needle.as_str(), line, start, end)
                    {
                        matched_entries.insert(entry_index);
                        classified = true;
                    }
                }
                if !classified {
                    unexpected.push(format!("{relative}:{}: {}", line_index + 1, line.trim()));
                }
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "unclassified numeric version identifier occurrence(s):\n{}",
        unexpected.join("\n")
    );

    let unused = allowlist
        .entries
        .iter()
        .enumerate()
        .filter(|(index, _)| !matched_entries.contains(index))
        .map(|(_, entry)| format!("{}: {}", entry.path, entry.needle))
        .collect::<Vec<_>>();
    assert!(
        unused.is_empty(),
        "stale numeric-version allowlist entry or entries:\n{}",
        unused.join("\n")
    );
}

fn collect_source_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name == ".git" || name == "target" || (name.starts_with('.') && name != ".github") {
                continue;
            }
            collect_source_files(root, &path, files)?;
        } else if file_type.is_file() && is_source_file(root, &path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_source_file(root: &Path, path: &Path) -> bool {
    let relative = repository_path(root, path);
    if matches!(
        relative.as_str(),
        "Cargo.toml" | "Cargo.lock" | "README.md" | "AGENTS.md"
    ) {
        return true;
    }
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some(
            "rs" | "toml" | "lock" | "md" | "yaml" | "yml" | "json" | "sql" | "sh" | "ps1" | "txt"
        )
    )
}

fn repository_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("repository path")
        .to_string_lossy()
        .replace('\\', "/")
}

fn numeric_version_markers(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut markers = Vec::new();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if matches!(bytes[index], b'v' | b'V')
            && bytes[index + 1].is_ascii_digit()
            && (index == 0 || !bytes[index - 1].is_ascii_alphanumeric())
        {
            let mut end = index + 2;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end == bytes.len() || !bytes[end].is_ascii_alphanumeric() {
                markers.push((index, end));
                index = end;
                continue;
            }
        }
        index += 1;
    }
    markers
}

fn needle_covers(needle: &str, line: &str, marker_start: usize, marker_end: usize) -> bool {
    line.match_indices(needle).any(|(needle_start, _)| {
        needle_start <= marker_start && marker_end <= needle_start + needle.len()
    })
}

#[test]
fn marker_scanner_distinguishes_versions_from_algorithm_names() {
    let contract_suffix = format!("contract-{}{}", 'v', 2);
    let release_tag = format!("release {}{}.4", 'v', 12);
    assert_eq!(numeric_version_markers(&contract_suffix), vec![(9, 11)]);
    assert_eq!(numeric_version_markers(&release_tag), vec![(8, 11)]);
    assert!(numeric_version_markers("fnv1a64").is_empty());
    assert!(numeric_version_markers("version").is_empty());
}
