//! Optional real-data regression. Inputs are immutable local publications, not
//! generated fixtures. Every mutation and output belongs to a disposable copy.
use serde_json::Value;
use std::{error::Error, fs, path::Path};
use volicord_context::{FixedClock, ProjectId, SystemIdGenerator, TimestampMicros};
use volicord_operations::{LocalOperations, RuntimeLayout};
use volicord_privacy::{ManagedDerivedKind, PrivacyStore};
use volicord_projections::*;
use volicord_viewer::*;

fn copy_directory(source: &Path, target: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err("original Runtime contains a symlink".into());
        }
        let destination = target.join(entry.file_name());
        if kind.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires VOLICORD_ORIGINAL_PUBLICATIONS frozen local input manifest"]
fn original_admitted_meaning_survives_real_cleanup_and_restart() -> Result<(), Box<dyn Error>> {
    let manifest = std::env::var("VOLICORD_ORIGINAL_PUBLICATIONS")?;
    let output = std::path::PathBuf::from(std::env::var("VOLICORD_ORIGINAL_OUTPUT")?);
    fs::create_dir(&output)?;
    let cli = std::env::var("VOLICORD_TEST_CLI")?;
    let cases: Vec<Value> = serde_json::from_slice(&fs::read(manifest)?)?;
    assert!(!cases.is_empty());
    for (index, case) in cases.into_iter().enumerate() {
        let temp = output.join(format!("case-{index}"));
        fs::create_dir(&temp)?;
        let runtime = temp.join("runtime");
        copy_directory(
            Path::new(case["runtime"].as_str().ok_or("runtime")?),
            &runtime,
        )?;
        let hex = case["project_id"].as_str().ok_or("project")?;
        let bytes = (0..16)
            .map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()?;
        let project = ProjectId::from_slice(&bytes)?;
        let subject: ExplanationSubject = serde_json::from_value(case["subject"].clone())?;
        let language = case["language"].as_str().ok_or("language")?;
        let original: ExplanationRealization = serde_json::from_value(case["realization"].clone())?;
        let layout = RuntimeLayout::new(runtime)?;
        let before = LocalOperations::new(layout.clone())
            .read_explanation_page(project, subject, language, 0, None)?;
        assert_eq!(before["publication_id"], case["publication_id"]);
        let inspection = LocalOperations::new(layout.clone()).privacy_status(project)?;
        let publication = inspection
            .managed_derived
            .iter()
            .find(|r| r.id.to_string() == case["publication_id"].as_str().unwrap_or(""))
            .ok_or("publication")?;
        assert!(publication.source_computed_explanation_deadline(&inspection.managed_derived));
        let frozen_content = publication.content.as_deref().ok_or("original content")?;
        assert_eq!(
            explanation_text_hash(frozen_content),
            format!(
                "sha256:{}",
                case["retained_sha256"].as_str().ok_or("frozen hash")?
            )
        );
        let deadline = inspection
            .managed_derived
            .iter()
            .filter(|r| r.kind == ManagedDerivedKind::SourceObservation)
            .filter_map(|r| r.retained_until)
            .max()
            .ok_or("deadline")?;
        let mut privacy = PrivacyStore::open_with(
            layout.privacy_store(),
            SystemIdGenerator,
            FixedClock::new(TimestampMicros::from_unix_micros(
                deadline.as_unix_micros() + 1,
            )),
        )?;
        let erased = privacy.cleanup_expired(project)?;
        assert!(!erased.is_empty());
        drop(privacy);
        let ops = LocalOperations::new(layout.clone());
        let after = ops.read_explanation_page(
            project,
            subject,
            language,
            0,
            Some(before["fingerprint"].as_str().ok_or("fingerprint")?),
        )?;
        assert_eq!(after["fingerprint"], before["fingerprint"]);
        assert_eq!(after["publication_id"], case["publication_id"]);
        assert!(after["applicability"]["source_ranges"]
            .as_array()
            .ok_or("ranges")?
            .iter()
            .all(|s| s["available"] == false));
        let stored_after = ops
            .privacy_status(project)?
            .managed_derived
            .into_iter()
            .find(|r| Some(r.id.to_string()) == case["publication_id"].as_str().map(str::to_owned))
            .ok_or("retained publication")?;
        assert_eq!(stored_after.content.as_deref(), Some(frozen_content));
        // A separate executable process opens the cleaned Runtime; this is not
        // only reconstruction of an in-process Operations handle.
        let mut command = std::process::Command::new(&cli);
        command.args([
            "--runtime",
            layout.root().to_str().ok_or("runtime")?,
            "--project",
            hex,
            "--json",
        ]);
        if matches!(subject, ExplanationSubject::Repository(_)) {
            command.args(["repository", "explain", "read", "--language", language]);
        } else {
            command.args(["status", "--language", language]);
        }
        let started = std::time::Instant::now();
        let child = command.output()?;
        fs::write(temp.join("cli.stdout"), &child.stdout)?;
        fs::write(temp.join("cli.stderr"), &child.stderr)?;
        fs::write(
            temp.join("cli-process.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"exit":child.status.code(),"duration_ms":started.elapsed().as_secs_f64()*1000.0}),
            )?,
        )?;
        assert!(
            child.status.success(),
            "CLI failed: {}",
            String::from_utf8_lossy(&child.stderr)
        );
        let data: Value = serde_json::from_slice(&child.stdout)?;
        let answers = if matches!(subject, ExplanationSubject::Repository(_)) {
            &data["answers"]
        } else {
            data["work_history"]
                .as_array()
                .ok_or("Work history")?
                .iter()
                .chain(std::iter::once(&data["selected_work"]))
                .find(|w| w["answers"]["provenance"]["publication_id"] == case["publication_id"])
                .map(|w| &w["answers"])
                .ok_or("exact Work publication")?
        };
        assert_eq!(
            answers["provenance"]["publication_id"],
            case["publication_id"]
        );
        assert_eq!(answers["source_excerpts"], serde_json::json!([]));
        let request = ViewerRequest {
            project_id: project,
            locale: ViewerLocale::English,
            view: ViewerView::RepositoryExplanation,
            requested_language: language.into(),
            guarded_request: None,
        };
        let adapter = ViewerAdapter::new(ops);
        let path = temp.join("retained.html");
        adapter.export_snapshot(&request, &path)?;
        let html = fs::read_to_string(path)?;
        // Snapshot includes all authorized reader subjects. Every original unit
        // and diagram remains published without any Source code region.
        for paragraph in &original.paragraphs {
            let escaped = paragraph
                .text
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;");
            assert!(
                html.contains(&escaped),
                "missing original unit {:?}",
                paragraph.question
            );
        }
        assert!(!html.contains("<pre class=\"code-region\""));
        assert!(html.contains("data-range-state=\"expired\""));
        println!("original={} language={language} units={} diagrams={} erased_sources={} retained_bytes={} export_bytes={} cli_exit={:?} identity_preserved=true",case["publication_id"],original.paragraphs.len(),original.diagrams.len(),erased.len(),frozen_content.len(),html.len(),child.status.code());
    }
    Ok(())
}
