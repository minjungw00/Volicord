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
    assert_ne!(first.analysis.identity, second.analysis.identity);

    let report = fixture.operations.analysis_storage_footprint(project)?;
    eprintln!("{}", serde_json::to_string_pretty(&report)?);
    assert_eq!(report.snapshot_count, 2);
    assert_eq!(report.snapshots.len(), 2);
    assert_eq!(report.reusable_content_overlap_millionths, Some(1_000_000));
    let repeated = report.unchanged_repeat_delta_bytes.ok_or("repeat delta")?;
    assert!(
        repeated > 100_000,
        "fixture must expose meaningful full-copy growth: {repeated}"
    );
    assert_eq!(
        report.logical_bytes,
        report
            .snapshots
            .iter()
            .map(|item| item.logical_bytes)
            .sum::<u64>()
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
