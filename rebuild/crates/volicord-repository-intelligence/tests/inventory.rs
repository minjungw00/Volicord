use std::collections::BTreeSet;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use volicord_context::{
    Availability, CanonicalReadOptions, DeterministicIdGenerator, FixedClock, OperationId,
    Principal, PrincipalKind, ProjectId, SourceDraft, SourceId, SourcePayload, Store,
    TimestampMicros,
};
use volicord_repository_intelligence::{
    analyze_repository, canonical_json, inventory_repository, Capability, CapabilityState,
    EcosystemObservationKind, InventoryClassification, InventoryRequest, Language, ProvenanceClass,
    SemanticAnalysisResult, StructuralAnalysisRequest, StructuralFact,
};

const OBSERVED_AT: i64 = 1_725_000_000_000_000;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../validation/repository-intelligence/polyglot-structural/fixtures")
        .join(name)
}

fn request(root: &Path) -> InventoryRequest<'_> {
    InventoryRequest::new(
        root,
        ProjectId::from_bytes([0x11; 16]),
        SourceId::from_bytes([0x22; 16]),
        OBSERVED_AT,
    )
}

#[test]
fn maintained_fixtures_recognize_all_seven_gate_languages() -> Result<(), Box<dyn Error>> {
    let matrix = [
        ("java", Language::Java),
        ("python", Language::Python),
        ("javascript", Language::JavaScript),
        ("typescript", Language::TypeScript),
        ("c", Language::C),
        ("cpp", Language::Cpp),
        ("rust", Language::Rust),
    ];

    for (name, expected_language) in matrix {
        let (repository, analysis) = inventory_repository(request(&fixture(name)))?;
        assert_eq!(analysis.repository_snapshot, repository.identity);
        assert!(analysis.inventory.languages.contains(&expected_language));
        let structural = analysis
            .capabilities
            .iter()
            .find(|report| {
                report.language.as_ref() == Some(&expected_language)
                    && report.capability == Capability::Structural
            })
            .ok_or("missing structural capability report")?;
        assert_eq!(structural.state, CapabilityState::Unavailable);
        assert!(!structural.coverage.unavailable.is_empty());
        assert!(analysis.structural_facts.is_empty());
        assert!(analysis.semantic_results.is_empty());
        if name == "rust" {
            assert!(analysis
                .inventory
                .ecosystem_observations
                .iter()
                .any(|observation| {
                    observation.area.path == "Cargo.toml"
                        && observation.kind == EcosystemObservationKind::WorkspaceManifest
                }));
        }
    }
    Ok(())
}

#[test]
fn out_of_set_text_language_keeps_inventory_with_honest_fallback() -> Result<(), Box<dyn Error>> {
    let (_, analysis) = inventory_repository(request(&fixture("out_of_set")))?;
    assert!(analysis.inventory.languages.contains(&Language::Go));

    let inventory = capability(&analysis, &Language::Go, Capability::Inventory)?;
    let structural = capability(&analysis, &Language::Go, Capability::Structural)?;
    let semantic = capability(&analysis, &Language::Go, Capability::Semantic)?;
    let ecosystem = capability(&analysis, &Language::Go, Capability::Ecosystem)?;
    assert_eq!(inventory.state, CapabilityState::Available);
    assert_eq!(structural.state, CapabilityState::Unsupported);
    assert_eq!(semantic.state, CapabilityState::Unsupported);
    assert_eq!(ecosystem.state, CapabilityState::Partial);
    assert!(!structural.coverage.unsupported.is_empty());
    assert!(analysis
        .inventory
        .ecosystem_observations
        .iter()
        .any(|observation| observation.area.path == "go.mod"));
    Ok(())
}

#[test]
fn snapshot_identity_and_serialization_are_path_independent_and_repeatable(
) -> Result<(), Box<dyn Error>> {
    let first_binding = tempfile::tempdir()?;
    let second_binding = tempfile::tempdir()?;
    copy_tree(&fixture("polyglot"), first_binding.path())?;
    copy_tree(&fixture("polyglot"), second_binding.path())?;

    let (first_repository, first_analysis) = inventory_repository(request(first_binding.path()))?;
    let (repeated_repository, repeated_analysis) =
        inventory_repository(request(first_binding.path()))?;
    let (other_repository, other_analysis) = inventory_repository(request(second_binding.path()))?;

    assert_eq!(first_repository.identity, repeated_repository.identity);
    assert_eq!(first_repository.identity, other_repository.identity);
    assert_eq!(first_analysis.identity, repeated_analysis.identity);
    assert_eq!(first_analysis.identity, other_analysis.identity);
    assert_eq!(
        canonical_json(&first_repository)?,
        canonical_json(&other_repository)?
    );
    assert_eq!(
        canonical_json(&first_analysis)?,
        canonical_json(&other_analysis)?
    );
    let serialized = String::from_utf8(canonical_json(&first_analysis)?)?;
    assert!(!serialized.contains(&first_binding.path().display().to_string()));
    assert!(!serialized.contains(&second_binding.path().display().to_string()));

    fs::write(
        first_binding.path().join("python/formatter.py"),
        "def format_greeting(name):\n    return name.upper()\n",
    )?;
    let (changed_repository, changed_analysis) =
        inventory_repository(request(first_binding.path()))?;
    assert_ne!(first_repository.identity, changed_repository.identity);
    assert_ne!(first_analysis.identity, changed_analysis.identity);
    Ok(())
}

#[test]
fn exclusions_binary_vendor_generated_and_ignored_scopes_remain_visible(
) -> Result<(), Box<dyn Error>> {
    let repository = tempfile::tempdir()?;
    fs::create_dir_all(repository.path().join("vendor/lib"))?;
    fs::create_dir_all(repository.path().join("target/debug"))?;
    fs::create_dir_all(repository.path().join("private"))?;
    fs::write(repository.path().join("main.py"), "print('ok')\n")?;
    fs::write(repository.path().join("image.bin"), [0_u8, 1, 2, 3])?;
    fs::write(repository.path().join("ignored.log"), "ignored\n")?;
    fs::write(repository.path().join(".gitignore"), "*.log\n")?;
    fs::write(repository.path().join("vendor/lib/code.js"), "export {};\n")?;
    fs::write(
        repository.path().join("target/debug/out.rs"),
        "fn generated() {}\n",
    )?;
    fs::write(
        repository.path().join("private/secret.txt"),
        "not inventoried\n",
    )?;

    let mut inventory_request = request(repository.path());
    inventory_request.excluded_paths = vec!["private".to_owned()];
    let (snapshot, analysis) = inventory_repository(inventory_request)?;

    assert_classification(&analysis, "ignored.log", InventoryClassification::Ignored)?;
    assert_classification(&analysis, "vendor", InventoryClassification::Vendor)?;
    assert_classification(&analysis, "target", InventoryClassification::Generated)?;
    assert_classification(&analysis, "private", InventoryClassification::Excluded)?;
    assert_classification(&analysis, "image.bin", InventoryClassification::Binary)?;
    assert!(snapshot
        .excluded_areas
        .iter()
        .any(|area| area.path == "ignored.log"));
    let overall = analysis
        .capabilities
        .iter()
        .find(|report| report.language.is_none() && report.capability == Capability::Inventory)
        .ok_or("missing repository inventory capability")?;
    assert!(overall.coverage.excluded.len() >= 4);
    Ok(())
}

#[cfg(unix)]
#[test]
fn unavailable_entry_does_not_erase_successful_inventory() -> Result<(), Box<dyn Error>> {
    use std::os::unix::fs::symlink;

    let repository = tempfile::tempdir()?;
    fs::write(repository.path().join("main.rs"), "fn main() {}\n")?;
    symlink("missing-target", repository.path().join("broken-link"))?;

    let (snapshot, analysis) = inventory_repository(request(repository.path()))?;
    assert!(analysis
        .inventory
        .entries
        .iter()
        .any(|entry| entry.area.path == "main.rs"));
    assert!(snapshot
        .unavailable_areas
        .iter()
        .any(|area| area.path == "broken-link"));
    let overall = analysis
        .capabilities
        .iter()
        .find(|report| report.language.is_none() && report.capability == Capability::Inventory)
        .ok_or("missing repository inventory capability")?;
    assert_eq!(overall.state, CapabilityState::Partial);
    assert!(!overall.coverage.included.is_empty());
    assert!(!overall.coverage.unavailable.is_empty());
    Ok(())
}

#[test]
fn repository_analysis_has_no_canonical_mutation_authority() -> Result<(), Box<dyn Error>> {
    let runtime = tempfile::tempdir()?;
    let mut store = Store::open_with(
        runtime.path().join("context.sqlite3"),
        DeterministicIdGenerator::new([[0x41; 16], [0x42; 16]]),
        FixedClock::new(TimestampMicros::from_unix_micros(100)),
    )?;
    let project = store
        .create_project(OperationId::from_bytes([1; 16]), "Inventory purity")?
        .value;
    let source = store
        .record_source(
            OperationId::from_bytes([2; 16]),
            project.id,
            SourceDraft {
                expected_project_revision: project.revision,
                payload: SourcePayload::RepositorySnapshot {
                    revision: "fixture-basis".to_owned(),
                },
                actor: Principal {
                    kind: PrincipalKind::Repository,
                    identity: "test-repository".to_owned(),
                },
                observer: Some(Principal {
                    kind: PrincipalKind::Agent,
                    identity: "test-agent".to_owned(),
                }),
                availability: Availability::Available,
            },
        )?
        .value;
    let before = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;

    let rust_fixture = fixture("rust");
    let inventory_request =
        InventoryRequest::new(&rust_fixture, project.id, source.id, OBSERVED_AT);
    let (_, analysis) = analyze_repository(StructuralAnalysisRequest::new(inventory_request))?;
    let after = store.read_canonical_basis(project.id, CanonicalReadOptions::default())?;

    assert_eq!(before, after);
    assert!(after.active_questions.is_empty());
    assert!(after.active_decisions.is_empty());
    assert!(after.context_items.is_empty());
    assert!(after.latest_checkpoint.is_none());
    assert_eq!(analysis.project.0, project.id);
    assert_eq!(analysis.repository_source.0, source.id);
    Ok(())
}

#[test]
fn fact_result_and_interpretation_are_distinct_types_and_classes() {
    assert_ne!(
        std::any::TypeId::of::<StructuralFact>(),
        std::any::TypeId::of::<SemanticAnalysisResult>()
    );
    let classes = BTreeSet::from([
        ProvenanceClass::StructuralFact,
        ProvenanceClass::SemanticResult,
        ProvenanceClass::SemanticAnnotation,
        ProvenanceClass::AgentInterpretation,
    ]);
    assert_eq!(classes.len(), 4);
}

fn capability<'a>(
    analysis: &'a volicord_repository_intelligence::AnalysisSnapshot,
    language: &Language,
    capability: Capability,
) -> Result<&'a volicord_repository_intelligence::CapabilityReport, Box<dyn Error>> {
    analysis
        .capabilities
        .iter()
        .find(|report| {
            report.language.as_ref() == Some(language) && report.capability == capability
        })
        .ok_or_else(|| "missing capability report".into())
}

fn assert_classification(
    analysis: &volicord_repository_intelligence::AnalysisSnapshot,
    path: &str,
    expected: InventoryClassification,
) -> Result<(), Box<dyn Error>> {
    let entry = analysis
        .inventory
        .entries
        .iter()
        .find(|entry| entry.area.path == path)
        .ok_or("missing inventory entry")?;
    assert!(entry.classifications.contains(&expected));
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
        } else {
            fs::copy(source_path, destination_path)?;
        }
    }
    Ok(())
}
