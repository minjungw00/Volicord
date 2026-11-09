//! Synthetic retained-history controls, never naturalistic or human evidence.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use std::{error::Error, fs, path::Path, process::Command};
use volicord_context::{ContextItemRole, TimestampMicros};
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_privacy::{ManagedDerivedDraft, ManagedDerivedKind, PrivacyStore};

fn read_cli(
    root: &Path,
    runtime: &Path,
    project: &str,
    args: &[&str],
    proof: Option<&Path>,
    label: &str,
) -> Result<Value, Box<dyn Error>> {
    let binary = env!("CARGO_BIN_EXE_volicord");
    let mut command = if let Some(proof) = proof {
        fs::create_dir(proof.join(label))?;
        let mut command = Command::new("/usr/bin/time");
        command.args(["-v", "-o"]);
        command.arg(proof.join(label).join("resources.txt"));
        command.arg(binary);
        command
    } else {
        Command::new(binary)
    };
    let started = std::time::Instant::now();
    let output = command
        .current_dir(root)
        .arg("--runtime")
        .arg(runtime)
        .args(["--project", project, "--json"])
        .args(args)
        .output()?;
    if let Some(proof) = proof {
        let directory = proof.join(label);
        fs::write(directory.join("stdout.json"), &output.stdout)?;
        fs::write(directory.join("stderr.txt"), &output.stderr)?;
        fs::write(
            directory.join("result.json"),
            serde_json::to_vec_pretty(&json!({
                "binary":binary,"arguments":args,"exit_code":output.status.code(),
                "wall_ms":started.elapsed().as_secs_f64()*1000.0,
                "runtime":runtime,"project":project,"repository":root
            }))?,
        )?;
    }
    if !output.status.success() {
        return Err(format!(
            "CLI exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn exact_source_and_publication_reading_preserve_complete_results_under_retained_growth(
) -> Result<(), Box<dyn Error>> {
    let proof = std::env::var_os("VOLICORD_ACCUMULATED_PROOF_DIR").map(std::path::PathBuf::from);
    if let Some(proof) = &proof {
        assert!(proof.is_absolute());
        fs::create_dir(proof)?;
    }
    let temporary = if let Some(proof) = &proof {
        tempfile::tempdir_in(proof)?
    } else {
        tempfile::tempdir()?
    };
    let root = temporary.path().join("repository");
    let runtime = temporary.path().join("runtime");
    fs::create_dir(&root)?;
    // All seven structural languages remain in the fixed input, including the
    // three semantic ecosystems. This is a small control, not a capacity claim.
    for (name, body) in [
        ("main.py", "value = 4\n"),
        ("main.rs", "fn main() {}\n"),
        ("main.ts", "export function run(): number { return 4; }\n"),
        ("main.js", "export function run() { return 4; }\n"),
        (
            "Main.java",
            "class Main { static int run() { return 4; } }\n",
        ),
        ("main.c", "int run(void) { return 4; }\n"),
        ("main.cpp", "int run() { return 4; }\n"),
    ] {
        fs::write(root.join(name), body)?;
    }
    let operations = LocalOperations::new(RuntimeLayout::new(runtime.clone())?);
    let project = operations
        .initialize_project("Synthetic history control", Some(&root))?
        .project
        .id;
    let work = operations
        .record_current_host_user_context(
            project,
            "synthetic-control".into(),
            "synthetic-session".into(),
            "Inspect the retained value.".into(),
            ContextItemRole::Goal,
            "Inspect the retained value.".into(),
        )?
        .context_item_id;
    let analysis = operations
        .analyze(project, vec![])?
        .value
        .ok_or("analysis")?
        .analysis;
    let observed = operations.observe_work_file(volicord_operations::FileObservationRequest {
        project_id: project,
        work_item_id: work,
        analysis_snapshot_id: analysis.identity,
        path: "main.py".into(),
        byte_range: None,
        before_source_id: None,
        retain_seconds: 3600,
    })?;
    let subject = volicord_projections::ExplanationSubject::WorkName(work);
    let plan = operations.prepare_explanation(project, subject, "en")?;
    let realization = volicord_projections::ExplanationRealization {
        format_kind: volicord_projections::EXPLANATION_KIND.into(),
        format_version: volicord_projections::EXPLANATION_VERSION,
        language: "en".into(),
        plan_fingerprint: plan.fingerprint,
        generator: volicord_projections::ExplanationGenerator {
            host: "synthetic-control".into(),
            session: "synthetic-session".into(),
            agent: None,
            model: None,
        },
        paragraphs: vec![volicord_projections::ExplanationParagraph {
            question: volicord_projections::ExplanationQuestion::Heading,
            text: "Inspect the retained value".into(),
            evidence_keys: vec!["goal".into()],
            citations: vec![],
        }],
        diagrams: vec![],
    };
    operations.record_explanation(project, subject, "en", realization)?;
    let project_text = project.to_string();
    let work_text = work.to_string();
    let source_text = observed.source_id.to_string();
    let commands = [vec![
        "work",
        "source",
        "read",
        "--work",
        &work_text,
        "--source",
        &source_text,
    ]];
    let expected_publication = operations.read_explanation_page(project, subject, "en", 0, None)?;
    // Declaration precedes measurements: 0/24 regular control, or 0/32/256
    // proof records, each at the maintained maximum managed-content size.
    let stages: &[usize] = if proof.is_some() {
        &[0, 32, 256]
    } else {
        &[0, 24]
    };
    let mut expected = Vec::new();
    let mut retained = Vec::new();
    let mut store = PrivacyStore::open(operations.layout().privacy_store())?;
    for &count in stages {
        while retained.len() < count {
            retained.push(store.record_managed_derived(ManagedDerivedDraft {
                project_id: project,
                kind: ManagedDerivedKind::CachedSummary,
                provider: None,
                model: None,
                purpose: format!("synthetic-unselected-body:{}", retained.len()),
                analysis_snapshot: Some(analysis.identity),
                included_sources: vec![],
                canonical_links: vec![],
                content: "x".repeat(volicord_privacy::MANAGED_DERIVED_CONTENT_BYTE_LIMIT),
                uncertainty: None,
                retained_until: Some(TimestampMicros::from_unix_micros(i64::MAX)),
                retention_basis: "Synthetic accumulated-body control; not a publication".into(),
            })?);
        }
        let started = std::time::Instant::now();
        let actual = operations.read_explanation_page(project, subject, "en", 0, None)?;
        let wall_ms = started.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(actual, expected_publication);
        let restarted = LocalOperations::new(RuntimeLayout::new(runtime.clone())?);
        assert_eq!(
            restarted.read_explanation_page(project, subject, "en", 0, None)?,
            expected_publication
        );
        if let Some(proof) = &proof {
            fs::write(
                proof.join(format!("history-{count}-publication.json")),
                serde_json::to_vec_pretty(&json!({"wall_ms":wall_ms,"page":actual}))?,
            )?;
        }
        for (index, args) in commands.iter().enumerate() {
            let value = read_cli(
                &root,
                &runtime,
                &project_text,
                args,
                proof.as_deref(),
                &format!("history-{count}-read-{index}"),
            )?;
            if count == 0 {
                expected.push(value);
            } else {
                assert_eq!(value, expected[index]);
            }
        }
        // Each CLI request above uses a new process. A repeated
        // request has warm OS caches, not a claimed cold-disk boundary.
        if proof.is_some() {
            for (index, args) in commands.iter().enumerate() {
                assert_eq!(
                    read_cli(
                        &root,
                        &runtime,
                        &project_text,
                        args,
                        proof.as_deref(),
                        &format!("history-{count}-repeat-{index}")
                    )?,
                    expected[index]
                );
            }
        }
    }
    // Independent durable reads check every original record, including bodies
    // not selected by the public readers. Nothing is pruned or rewritten.
    for record in retained {
        assert_eq!(store.get_derived(project, record.id)?, record);
    }
    if let Some(proof) = proof {
        fs::write(
            proof.join("input.json"),
            serde_json::to_vec_pretty(&json!({
                "synthetic":true,"history_counts":stages,"bytes_per_body":volicord_privacy::MANAGED_DERIVED_CONTENT_BYTE_LIMIT,
                "runtime":runtime,"repository":root,"project":project_text,"work":work_text,
                "analysis":analysis.identity,"source":source_text,
                "structural_facts":analysis.structural_facts.len(),"semantic_results":analysis.semantic_results.len()
            }))?,
        )?;
        let _retained_root = temporary.keep();
    }
    Ok(())
}

#[test]
fn accumulated_work_and_checkpoint_history_remains_complete_after_restart(
) -> Result<(), Box<dyn Error>> {
    // Declare the independent canonical-growth axis before any observation.
    // Article/body and Analysis growth have their own separate controls.
    for work_count in [1, 8, 32] {
        let mut input: Value = serde_json::from_str(reading_fixture::SCENARIO)?;
        input["prior_checkpoint_count"] = json!(0);
        input["later_checkpoint_count"] = json!(0);
        input["unassociated_checkpoint"] = json!(false);
        input["decisions"] = json!([]);
        input["works"] = Value::Array((0..work_count).map(|work| json!({
            "key":format!("synthetic-{work}"),"title":format!("Synthetic Work {work}"),"paths":[],
            "checkpoints":(0..8).map(|checkpoint|json!({
                "key":format!("checkpoint-{checkpoint}"),"work":"Paused",
                "verification":"NotRun","review":"NotRequested","acceptance":"NotRequested",
                "state_change":format!("Synthetic observation {work}/{checkpoint}"),
                "next_step":"Inspect the remaining synthetic history"
            })).collect::<Vec<_>>()
        })).collect());
        let fixture = reading_fixture::fixture_scenario(input)?;
        let started = std::time::Instant::now();
        let canonical = fixture.operations.canonical_basis(fixture.project)?;
        let first_ms = started.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(canonical.checkpoint_history.len(), work_count * 8);
        assert_eq!(
            canonical
                .context_items
                .iter()
                .filter(|item| item.role == ContextItemRole::Goal)
                .count(),
            work_count + 1 // The shared fixture also declares one distractor Goal.
        );
        let restarted = LocalOperations::new(fixture.operations.layout().clone());
        assert_eq!(restarted.canonical_basis(fixture.project)?, canonical);
        for work in fixture.goals.values() {
            let checkpoints = canonical
                .checkpoint_history
                .iter()
                .filter(|cp| cp.work_item_id == Some(*work))
                .collect::<Vec<_>>();
            assert_eq!(checkpoints.len(), 8);
            assert_eq!(
                checkpoints.last().ok_or("last Checkpoint")?.next_step,
                "Inspect the remaining synthetic history"
            );
        }
        println!("synthetic canonical growth selected_works={work_count} distractor_goals=1 checkpoints={} sources={} first_read_ms={first_ms:.3}",canonical.checkpoint_history.len(),canonical.sources.len());
    }
    Ok(())
}
