use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const REQUIRED_INVARIANTS: &[&str] = &[
    "project-scope-isolation",
    "source-provenance",
    "user-decision-authority",
    "exact-question-revision",
    "question-response-history",
    "decision-supersession-lineage",
    "decision-correction-semantics",
    "context-item-role-provenance",
    "checkpoint-verification-state-dimensions",
    "relation-direction",
    "tombstone-content-exclusion",
    "forgetting-closure",
    "operation-dependency-cleanup",
    "portable-local-state-exclusion",
    "deterministic-bundle-state",
    "deterministic-read-basis",
];

fn first_code_span(value: &str) -> Option<&str> {
    let (_, suffix) = value.split_once('`')?;
    suffix.split_once('`').map(|(value, _)| value)
}

fn rust_test_source(manifest: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut paths = fs::read_dir(manifest.join("tests"))?
        .map(|entry| entry.map(|value| value.path()))
        .collect::<Result<Vec<PathBuf>, _>>()?;
    paths.sort();
    let mut source = String::new();
    for path in paths {
        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            source.push_str(&fs::read_to_string(path)?);
        }
    }
    Ok(source)
}

#[test]
fn invariant_catalog_rows_have_semantic_owners_and_executable_anchors(
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let catalog = fs::read_to_string(manifest.join("INVARIANTS.md"))?;
    let tests = rust_test_source(manifest)?;
    let required = REQUIRED_INVARIANTS.iter().copied().collect::<BTreeSet<_>>();
    let mut observed = BTreeSet::new();

    for line in catalog.lines() {
        if !line.starts_with("| `") || line.starts_with("| `---") {
            continue;
        }
        let columns = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        assert_eq!(columns.len(), 11, "catalog row has the wrong column count");
        let identifier = first_code_span(columns[0]).ok_or("missing semantic identifier")?;
        assert!(
            identifier
                .bytes()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == b'-'),
            "invariant identifier is not a semantic slug: {identifier}"
        );
        assert!(
            observed.insert(identifier),
            "duplicate invariant {identifier}"
        );
        assert!(
            !columns[2].is_empty(),
            "{identifier} has no production owner"
        );
        assert!(
            !columns[3].is_empty(),
            "{identifier} has no direct-command or construction owner"
        );
        assert!(
            columns[5].contains("state")
                || columns[5].contains("group")
                || columns[5].contains("invariant"),
            "{identifier} has no full-state enforcement statement"
        );
        for (kind, column) in [("direct", columns[8]), ("portable", columns[9])] {
            let anchor = first_code_span(column)
                .ok_or_else(|| format!("{identifier} has no {kind} Rust test anchor"))?;
            assert!(
                tests.contains(&format!("fn {anchor}(")),
                "{identifier} references missing {kind} Rust test {anchor}"
            );
        }
        assert!(
            !columns[6].is_empty() && !columns[7].is_empty(),
            "{identifier} does not state portable or forgetting applicability"
        );
    }

    assert_eq!(observed, required, "catalog invariant inventory changed");
    assert!(catalog.contains("exactly one content-free witness"));
    assert!(!catalog.contains("bundle-wide forgotten-Decision existence fallback"));
    assert!(tests
        .contains("fn canonical_invariant_mutation_matrix_rejects_every_portable_write_boundary("));
    for mutation in [
        "witness-removal",
        "unrelated-decision-tombstone",
        "wrong-witness-question",
        "wrong-witness-revision",
        "wrong-witness-outcome",
        "wrong-root-decision",
        "duplicate-root-witness",
        "missing-active-root",
        "forgotten-root-without-matching-tombstone",
        "unrelated-response-source",
        "non-user-response-authority",
        "terminal-without-role-history",
        "role-on-open-question",
        "role-on-non-decision-question",
        "tombstone-plus-active-content",
        "decision-authority",
        "decision-question-revision",
        "decision-alternative",
        "local-binding-in-portable-state",
    ] {
        assert!(tests.contains(mutation), "missing mutation case {mutation}");
    }
    Ok(())
}

#[test]
fn every_full_state_admission_path_names_the_central_boundary(
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let portable = fs::read_to_string(manifest.join("src/portable.rs"))?;
    let merge = fs::read_to_string(manifest.join("src/merge.rs"))?;
    let store = fs::read_to_string(manifest.join("src/store.rs"))?;
    let owner = fs::read_to_string(manifest.join("src/canonical_state.rs"))?;

    assert!(portable.contains("canonical_state::validate_payload"));
    assert_eq!(
        merge.matches("canonical_state::validate_payload").count(),
        3,
        "ExplicitMerged, generated target, and replacement must use one boundary"
    );
    assert!(store.contains("canonical_state::validate_project_state(transaction, project_id)"));
    assert!(owner.contains("pub(crate) fn validate_payload"));
    assert!(owner.contains("pub(crate) fn validate_project_state"));
    for source in [&portable, &merge, &store] {
        assert!(
            !source.contains("validate_tables"),
            "superseded validator entry point remains"
        );
    }
    Ok(())
}
