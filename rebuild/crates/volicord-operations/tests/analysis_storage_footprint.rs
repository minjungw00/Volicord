use std::fs;
use tempfile::TempDir;
use volicord_operations::{LocalOperations, RuntimeLayout};

struct Fixture {
    _temporary: TempDir,
    operations: LocalOperations,
    repository: std::path::PathBuf,
}

fn fixture(file_count: usize) -> Result<Fixture, Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    fs::create_dir_all(repository.join("src"))?;
    for index in 0..file_count {
        fs::write(
            repository.join("src").join(format!("module_{index}.py")),
            format!("def value_{index}():\n    return {index}\n"),
        )?;
    }
    let operations = LocalOperations::new(RuntimeLayout::new(temporary.path().join("runtime"))?);
    Ok(Fixture {
        _temporary: temporary,
        operations,
        repository,
    })
}

#[test]
fn reports_graph_scale_dominance_repeat_delta_and_allocated_bytes(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(48)?;
    let project = fixture
        .operations
        .initialize_project("Storage footprint", Some(&fixture.repository))?
        .project
        .id;
    let first = fixture
        .operations
        .analyze(project, Vec::new())?
        .value
        .ok_or("first analysis")?;
    let second = fixture
        .operations
        .analyze(project, Vec::new())?
        .value
        .ok_or("second analysis")?;
    let third = fixture
        .operations
        .analyze(project, Vec::new())?
        .value
        .ok_or("third analysis")?;
    assert_ne!(first.analysis.identity, second.analysis.identity);
    assert_ne!(second.analysis.identity, third.analysis.identity);
    let manifest: serde_json::Value = serde_json::from_slice(&fs::read(&first.stored_at)?)?;
    assert!(manifest.get("metadata").is_none());
    let metadata_hash = manifest["metadata_blob"].as_str().ok_or("metadata blob")?;
    let metadata_blob = first
        .stored_at
        .parent()
        .ok_or("analysis directory")?
        .join("blobs")
        .join(format!("{metadata_hash}.metadata"));
    let metadata_bytes = fs::metadata(&metadata_blob)?.len();
    let source_metadata_bytes = serde_json::to_vec(
        &volicord_repository_intelligence::AnalysisMetadata::from(&first.analysis),
    )?
    .len() as u64;
    assert!(
        metadata_bytes * 2 < source_metadata_bytes,
        "metadata must be compressed before durable publication"
    );

    let report = fixture.operations.analysis_storage_footprint(project)?;
    eprintln!("{}", serde_json::to_string_pretty(&report)?);
    assert_eq!(report.snapshot_count, 3);
    assert_eq!(report.snapshots.len(), 3);
    assert_eq!(report.reusable_content_overlap_millionths, Some(1_000_000));
    let repeated = report.unchanged_repeat_delta_bytes.ok_or("repeat delta")?;
    assert!(
        repeated > 0,
        "distinct retained snapshots must have measurable growth: {repeated}"
    );
    assert!(
        report.logical_bytes
            <= report
                .snapshots
                .iter()
                .map(|item| item.logical_bytes)
                .sum::<u64>(),
        "content-addressed blobs must be counted once in the aggregate"
    );
    #[cfg(unix)]
    assert!(report.physical_bytes.is_some());
    for snapshot in &report.snapshots {
        assert!(snapshot.entity_count >= 48);
        assert!(snapshot.relation_count >= snapshot.entity_count);
        let graph = snapshot
            .sections
            .iter()
            .filter(|section| {
                matches!(
                    section.name.as_str(),
                    "structural_facts" | "semantic_results"
                )
            })
            .map(|section| section.logical_bytes)
            .sum::<u64>();
        assert!(
            graph * 2 > snapshot.logical_bytes,
            "graph sections must dominate the fixture"
        );
        assert!(snapshot.bytes_per_graph_item > 0);
    }
    let full_graph_json = report.snapshots[0]
        .sections
        .iter()
        .map(|section| section.logical_bytes)
        .sum::<u64>();
    assert!(
        repeated * 2 < full_graph_json,
        "repeat delta {repeated} must not approach another full snapshot {full_graph_json}"
    );
    Ok(())
}

#[test]
fn reports_changed_content_overlap_separately_from_snapshot_growth(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(12)?;
    let project = fixture
        .operations
        .initialize_project("Changed footprint", Some(&fixture.repository))?
        .project
        .id;
    fixture.operations.analyze(project, Vec::new())?;
    fs::write(
        fixture.repository.join("src/module_0.py"),
        "def changed():\n    return 999\n",
    )?;
    fixture.operations.analyze(project, Vec::new())?;
    let report = fixture.operations.analysis_storage_footprint(project)?;
    let overlap = report
        .reusable_content_overlap_millionths
        .ok_or("overlap")?;
    assert!(
        overlap > 500_000 && overlap < 1_000_000,
        "unexpected overlap {overlap}"
    );
    assert_eq!(report.unchanged_repeat_delta_bytes, None);
    Ok(())
}

#[test]
fn shared_storage_survives_restart_retains_history_and_collects_only_orphans(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(24)?;
    let project = fixture
        .operations
        .initialize_project("Storage lifecycle", Some(&fixture.repository))?
        .project
        .id;
    let first = fixture
        .operations
        .analyze(project, Vec::new())?
        .value
        .ok_or("first analysis")?;
    let blob_directory = first
        .stored_at
        .parent()
        .ok_or("analysis parent")?
        .join("blobs");
    let orphan = blob_directory.join(format!("{}.shape", "0".repeat(64)));
    fs::write(&orphan, b"interrupted-unreachable-publication")?;

    let restarted = LocalOperations::new(fixture.operations.layout().clone());
    let second = restarted
        .analyze(project, Vec::new())?
        .value
        .ok_or("second analysis")?;
    assert_ne!(first.analysis.identity, second.analysis.identity);
    assert!(
        first.stored_at.exists(),
        "immutable historical manifest was discarded"
    );
    assert!(
        !orphan.exists(),
        "unreachable crash residue was not reclaimed"
    );
    let report = restarted.analysis_storage_footprint(project)?;
    let repeat = report.unchanged_repeat_delta_bytes.ok_or("repeat delta")?;
    let equivalent_full = report.snapshots[0]
        .sections
        .iter()
        .map(|section| section.logical_bytes)
        .sum::<u64>();
    assert!(repeat * 2 < equivalent_full);

    fs::write(
        fixture.repository.join("src/changed.py"),
        "def changed():\n    return True\n",
    )?;
    let changed = restarted
        .analyze(project, Vec::new())?
        .value
        .ok_or("changed analysis")?;
    assert!(first.stored_at.exists());
    assert!(second.stored_at.exists());
    assert!(changed.stored_at.exists());
    assert!(changed
        .analysis
        .inventory
        .entries
        .iter()
        .any(|entry| entry.area.path == "src/changed.py"));
    let cache_directory = changed
        .stored_at
        .parent()
        .ok_or("analysis parent")?
        .join("cache");
    let cache_names = fs::read_dir(cache_directory)?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(cache_names.len(), 2, "read cache must remain bounded");
    assert!(cache_names
        .iter()
        .any(|name| name.starts_with(&first.analysis.identity.to_string())));
    assert!(cache_names
        .iter()
        .any(|name| name.starts_with(&changed.analysis.identity.to_string())));
    Ok(())
}

#[test]
fn repeated_polyglot_observations_keep_graph_locality_and_immutable_history(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = fixture(0)?;
    for file in 0..12 {
        for extension in ["rs", "ts", "py"] {
            let mut body = String::new();
            for item in 0..12 {
                body.push_str(&match extension {
                    "rs" => format!("pub fn value_{item}() -> i32 {{ {item} }}\npub fn call_{item}() -> i32 {{ value_{item}() }}\n"),
                    "ts" => format!("export function value_{item}(): number {{ return {item}; }}\nexport function call_{item}(): number {{ return value_{item}(); }}\n"),
                    _ => format!("def value_{item}():\n    return {item}\ndef call_{item}():\n    return value_{item}()\n"),
                });
            }
            fs::write(
                fixture
                    .repository
                    .join(format!("src/module_{file}.{extension}")),
                body,
            )?;
        }
    }
    let project = fixture
        .operations
        .initialize_project("Repeated observations", Some(&fixture.repository))?
        .project
        .id;
    let mut retained = Vec::new();
    let mut sources = std::collections::BTreeSet::new();
    let mut expected = None;
    let mut warmup = 0;
    for cycle in 0..8 {
        let operations = LocalOperations::new(fixture.operations.layout().clone());
        let output = operations
            .analyze(project, Vec::new())?
            .value
            .ok_or("analysis output")?;
        let analysis = &output.analysis;
        assert!(sources.insert(analysis.repository_source.identity()));
        let facts = analysis
            .structural_facts
            .iter()
            .map(|fact| {
                assert_eq!(fact.entity.source, analysis.repository_source);
                assert_eq!(fact.entity.analysis_snapshot, analysis.identity);
                serde_json::json!([
                    fact.entity.area,
                    fact.entity.kind,
                    fact.entity.display_name,
                    fact.entity.qualified_name
                ])
            })
            .collect::<Vec<_>>();
        let ranks = analysis
            .structural_facts
            .iter()
            .enumerate()
            .map(|(rank, fact)| (fact.entity.identity.as_str(), rank))
            .collect::<std::collections::HashMap<_, _>>();
        let semantics = analysis
            .semantic_results
            .iter()
            .map(|result| {
                assert_eq!(result.relation.analysis_snapshot, analysis.identity);
                let target = match &result.relation.target {
                    volicord_repository_intelligence::RelationTarget::ResolvedEntity(id) => {
                        serde_json::json!(ranks.get(id.as_str()))
                    }
                    volicord_repository_intelligence::RelationTarget::Unresolved(value) => {
                        serde_json::json!(value)
                    }
                };
                serde_json::json!([
                    ranks.get(result.relation.source_entity.as_str()),
                    result.relation.kind,
                    target
                ])
            })
            .collect::<Vec<_>>();
        assert!(!semantics.is_empty());
        let current = (facts, semantics);
        if let Some(expected) = &expected {
            assert_eq!(&current, expected);
        } else {
            expected = Some(current);
        }
        for (path, bytes) in &retained {
            assert_eq!(&fs::read(path)?, bytes);
        }
        retained.push((output.stored_at.clone(), fs::read(&output.stored_at)?));
        if cycle == 0 {
            warmup = operations
                .analysis_storage_footprint(project)?
                .logical_bytes;
        }
    }
    let restarted = LocalOperations::new(fixture.operations.layout().clone());
    // Footprint reads every historical durable payload, independently of the two read caches.
    let report = restarted.analysis_storage_footprint(project)?;
    assert_eq!(report.snapshot_count, 8);
    assert_eq!(report.reusable_content_overlap_millionths, Some(1_000_000));
    assert!(report.snapshots.iter().all(|snapshot| snapshot.entity_count
        == report.snapshots[0].entity_count
        && snapshot.relation_count == report.snapshots[0].relation_count));
    eprintln!(
        "{}",
        serde_json::json!({"warmup_logical_bytes":warmup,"retained_logical_bytes":report.logical_bytes,"post_warmup_growth_bytes":report.logical_bytes-warmup,"unchanged_repeat_delta_bytes":report.unchanged_repeat_delta_bytes,"snapshot_count":report.snapshot_count})
    );
    fs::write(
        fixture.repository.join("src/module_0.py"),
        "def changed():\n    return 999\n",
    )?;
    let changed = restarted
        .analyze(project, Vec::new())?
        .value
        .ok_or("changed output")?;
    assert!(changed.analysis.structural_facts.iter().any(|fact| fact
        .entity
        .display_name
        .as_deref()
        == Some("changed")));
    for (path, bytes) in &retained {
        assert_eq!(&fs::read(path)?, bytes);
    }
    assert_eq!(
        restarted
            .analysis_storage_footprint(project)?
            .snapshot_count,
        9
    );
    Ok(())
}
