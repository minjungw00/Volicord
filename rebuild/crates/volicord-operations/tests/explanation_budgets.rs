//! Authored size controls through the real CLI; these are not model-generation proof.
#[path = "support/reading_fixture.rs"]
#[allow(dead_code)]
mod reading_fixture;
use serde_json::{json, Value};
use std::process::{Command, Output};
use volicord_projections::*;

fn fixture() -> Result<reading_fixture::Fixture, Box<dyn std::error::Error>> {
    reading_fixture::fixture_scenario(reading_fixture::explanation_size_scenario(
        "metadata_heavy",
    )?)
}
fn invoke(
    f: &reading_fixture::Fixture,
    args: &[&str],
) -> Result<Output, Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicUsize, Ordering};
    static INVOCATION: AtomicUsize = AtomicUsize::new(0);
    // Pin actual executable bytes against a concurrent Cargo support build.
    let binary = f._temporary.path().join("volicord-size-control");
    if !binary.exists() {
        std::fs::copy(env!("CARGO_BIN_EXE_volicord"), &binary)?;
    }
    let argv = vec![
        "--runtime".to_string(),
        f.operations.layout().root().to_string_lossy().into_owned(),
        "--project".into(),
        f.project.to_string(),
        "--json".into(),
    ];
    let started = std::time::Instant::now();
    let output = Command::new(&binary)
        .args(&argv)
        .args(args)
        .current_dir(&f.repository)
        .output()?;
    let duration_ms = started.elapsed().as_secs_f64() * 1000.0;
    if let Some(root) = std::env::var_os("VOLICORD_EXPLANATION_SIZE_PROOF_DIR") {
        let root = std::path::PathBuf::from(root);
        if !root.is_absolute() {
            return Err("size proof directory must be absolute and ignored".into());
        }
        let directory = root.join(format!("{:04}", INVOCATION.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&root)?;
        std::fs::create_dir(&directory)?; // evidence is create-only in a fresh proof root
        std::fs::write(directory.join("stdout"), &output.stdout)?;
        std::fs::write(directory.join("stderr"), &output.stderr)?;
        if let Some(index) = args.iter().position(|arg| *arg == "--input") {
            std::fs::copy(
                args.get(index + 1).ok_or("input path")?,
                directory.join("input.json"),
            )?;
        }
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            output.status.signal()
        };
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        let digest_path = binary.with_extension("sha256");
        let hash = if digest_path.exists() {
            std::fs::read_to_string(&digest_path)?
        } else {
            // This fixture-owned copy never changes during its lifetime.
            let hash = format!("{:x}", Sha256::digest(std::fs::read(&binary)?));
            std::fs::write(&digest_path, &hash)?;
            hash
        };
        let retained_binary = root.join(format!("volicord-{hash}"));
        if !retained_binary.exists() {
            std::fs::copy(&binary, &retained_binary)?;
        }
        let mut complete_argv = vec![binary.to_string_lossy().into_owned()];
        complete_argv.extend(argv);
        complete_argv.extend(args.iter().map(|s| (*s).to_owned()));
        std::fs::write(
            directory.join("result.json"),
            serde_json::to_vec_pretty(&json!({
                "argv":complete_argv,"cwd":f.repository,"executable_sha256":hash,
                "exit_code":output.status.code(),"termination":if signal.is_some(){"signal"}else{"exited"},
                "signal":signal,"duration_ms":duration_ms,
                "stdout_bytes":output.stdout.len(),"stderr_bytes":output.stderr.len(),
                "stdout_sha256":format!("{:x}",Sha256::digest(&output.stdout)),
                "stderr_sha256":format!("{:x}",Sha256::digest(&output.stderr))
            }))?,
        )?;
    }
    Ok(output)
}
fn value(output: Output) -> Result<Value, Box<dyn std::error::Error>> {
    assert_eq!(
        output.status.code(),
        Some(0),
        "exit={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(serde_json::from_slice(&output.stdout)?)
}
fn response(
    plan: &ExplanationPlan,
    target: usize,
) -> Result<ExplanationRealization, Box<dyn std::error::Error>> {
    let questions: &[(ExplanationQuestion, &str)] = match plan.subject {
        ExplanationSubject::Work(_) => &[
            (ExplanationQuestion::Purpose, "goal"),
            (ExplanationQuestion::ReportedChange, "result"),
            (ExplanationQuestion::ExpectedEffect, "result"),
            (ExplanationQuestion::Verification, "verification"),
            (ExplanationQuestion::NextStep, "next_step"),
        ],
        ExplanationSubject::Decision(_) => &[
            (ExplanationQuestion::UserRationale, "user_rationale"),
            (ExplanationQuestion::Recommendation, "recommendation"),
            (ExplanationQuestion::Consequences, "consequences"),
            (ExplanationQuestion::Applicability, "applicability"),
        ],
    };
    let mut r = ExplanationRealization {
        format_kind: EXPLANATION_KIND.into(),
        format_version: EXPLANATION_VERSION,
        plan_fingerprint: plan.fingerprint.clone(),
        language: plan.requested_language.clone(),
        generator: ExplanationGenerator {
            host: "authored-size-control".into(),
            session: "disposable-cli".into(),
            agent: None,
            model: None,
        },
        paragraphs: questions
            .iter()
            .map(|(question, key)| ExplanationParagraph {
                question: *question,
                text: if plan.requested_language == "ko" {
                    "구조 검증용 설명: 근거의 한계를 보존합니다. \"인용\"\\".into()
                } else {
                    "Authored structural control: preserve evidence limitations. \"quote\"\\".into()
                },
                evidence_keys: vec![(*key).into()],
            })
            .collect(),
    };
    let size = serde_json::to_vec(&r)?.len();
    assert!(size <= target);
    r.paragraphs[0].text.push_str(&"x".repeat(target - size));
    assert_eq!(serde_json::to_vec(&r)?.len(), target);
    Ok(r)
}
#[test]
fn metadata_heavy_work_and_decision_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture()?;
    let canonical = f.operations.canonical_basis(f.project)?;
    for (kind, selector, id, subject, read) in [
        (
            "work",
            "--work",
            f.goals["relay"].to_string(),
            ExplanationSubject::Work(f.goals["relay"]),
            "status",
        ),
        (
            "decision",
            "--decision",
            f.decisions["project"].to_string(),
            ExplanationSubject::Decision(f.decisions["project"]),
            "decisions",
        ),
    ] {
        for language in ["en", "ko"] {
            let prepared = value(invoke(
                &f,
                &[
                    kind,
                    "explain",
                    "prepare",
                    selector,
                    &id,
                    "--language",
                    language,
                ],
            )?)?;
            let plan: ExplanationPlan = serde_json::from_value(prepared["plan"].clone())?;
            assert_eq!(plan.subject, subject);
            assert!(plan.source_status.len() >= 2);
            let realization = response(&plan, 3181)?;
            validate_explanation(&plan, &realization)?;
            let path = f._temporary.path().join("response.json");
            std::fs::write(&path, serde_json::to_vec(&realization)?)?;
            let recorded = value(invoke(
                &f,
                &[
                    kind,
                    "explain",
                    "record",
                    selector,
                    &id,
                    "--language",
                    language,
                    "--input",
                    path.to_str().ok_or("path")?,
                ],
            )?)?;
            let retained: RetainedExplanation =
                serde_json::from_value(recorded["explanation"].clone())?;
            let bytes = serde_json::to_vec(&retained)?.len();
            println!("{kind}/{language}: response=3181 retained={bytes} Source_count={} evidence_count={}",retained.source_status.len(),retained.evidence.len());
            assert!(
                bytes > 16384,
                "fixture must cross the original storage boundary"
            );
            assert_eq!(retained.realization, realization);
            let expected_status: Vec<_> = plan
                .source_status
                .iter()
                .cloned()
                .map(|mut s| {
                    if let Some(object) = s.as_object_mut() {
                        object.remove("observation");
                    }
                    s
                })
                .collect();
            assert_eq!(retained.source_status, expected_status);
            assert_eq!(retained.conflicts, plan.conflicts);
            assert_eq!(retained.question, plan.question);
            assert_eq!(retained.project_id, plan.project_id);
            let inspection = f.operations.privacy_status(f.project)?;
            let stored = inspection
                .managed_derived
                .iter()
                .max_by_key(|r| (r.created_at, r.id))
                .ok_or("stored")?;
            let links = serde_json::to_value(&stored.canonical_links)?;
            for evidence in &plan.evidence {
                assert!(!evidence.identity.is_empty());
                // Link serialization carries exact canonical identity, with no
                // inferred replacement or source pruning.
                let bytes: Vec<u8> = (0..16)
                    .map(|i| u8::from_str_radix(&evidence.identity[2 * i..2 * i + 2], 16))
                    .collect::<Result<_, _>>()?;
                assert!(
                    links
                        .as_array()
                        .ok_or("links")?
                        .iter()
                        .any(|link| link["kind"] == evidence.record_kind
                            && link["identity"] == json!(bytes)),
                    "{links}"
                );
            }
            let source_ids: std::collections::BTreeSet<_> =
                plan.evidence.iter().flat_map(|e| &e.sources).collect();
            assert_eq!(
                stored
                    .included_sources
                    .iter()
                    .map(|s| s.identity().to_string())
                    .collect::<std::collections::BTreeSet<_>>(),
                source_ids.into_iter().cloned().collect()
            );
            for (a, b) in retained.evidence.iter().zip(&plan.evidence) {
                assert_eq!(
                    (&a.key, &a.identity, a.revision, &a.sources),
                    (&b.key, &b.identity, b.revision, &b.sources)
                );
                assert!(a.content.is_null());
            }
            let readback = value(invoke(&f, &[read, "--language", language])?)?;
            assert!(readback.to_string().contains(&plan.fingerprint));
            assert!(readback
                .to_string()
                .contains("self_reported_not_independently_verified"));
        }
    }
    assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    Ok(())
}

fn rejected(output: Output, dimension: &str, measured: &str, allowed: &str) {
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "no success receipt on rejection");
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains(dimension) && error.contains(measured) && error.contains(allowed),
        "{error}"
    );
    assert!(
        error.contains("retry") || error.contains("support"),
        "{error}"
    );
    assert!(
        !error.contains("--help") && !error.contains("grounding\\\""),
        "{error}"
    );
    println!("controlled rejection: exit=1 stderr={error}");
}
#[test]
fn compact_response_and_raw_input_boundaries_are_atomic() -> Result<(), Box<dyn std::error::Error>>
{
    assert_eq!(
        EXPLANATION_RETAINED_BYTE_LIMIT,
        volicord_privacy::MANAGED_DERIVED_CONTENT_BYTE_LIMIT
    );
    for heavy in [false, true] {
        let f = if heavy {
            fixture()?
        } else {
            reading_fixture::fixture_scenario(reading_fixture::rich_scenario()?)?
        };
        let canonical = f.operations.canonical_basis(f.project)?;
        for (kind, selector, id, subject) in [
            (
                "work",
                "--work",
                f.goals["relay"].to_string(),
                ExplanationSubject::Work(f.goals["relay"]),
            ),
            (
                "decision",
                "--decision",
                f.decisions["project"].to_string(),
                ExplanationSubject::Decision(f.decisions["project"]),
            ),
        ] {
            for language in ["en", "ko"] {
                let plan = f
                    .operations
                    .prepare_explanation(f.project, subject, language)?;
                assert_eq!(plan.retention_budget.response_byte_capacity, 16384);
                let path = f._temporary.path().join("response.json");
                let record = |bytes: &[u8]| -> Result<Output, Box<dyn std::error::Error>> {
                    std::fs::write(&path, bytes)?;
                    invoke(
                        &f,
                        &[
                            kind,
                            "explain",
                            "record",
                            selector,
                            &id,
                            "--language",
                            language,
                            "--input",
                            path.to_str().ok_or("path")?,
                        ],
                    )
                };
                for size in [16383, 16384] {
                    let mut r = response(&plan, size)?;
                    // Move capacity into varying self-reported generator metadata.
                    let text = &mut r.paragraphs[0].text;
                    text.truncate(text.len() - 1996);
                    r.generator.model = Some("m".repeat(1998)); // null -> quotes + string
                    let compact = serde_json::to_vec(&r)?;
                    assert_eq!(compact.len(), size);
                    let pretty = serde_json::to_vec_pretty(&r)?;
                    assert!(pretty.len() > 16384);
                    let retained = value(record(&pretty)?)?["explanation"].clone();
                    assert_eq!(retained["realization"], serde_json::to_value(&r)?);
                    assert!(serde_json::to_vec(&retained)?.len() <= 147456);
                    let privacy = f.operations.privacy_status(f.project)?;
                    rejected(
                        record(&serde_json::to_vec(&response(&plan, 16385)?)?)?,
                        "compact realization JSON",
                        "16385",
                        "16384",
                    );
                    assert_eq!(privacy, f.operations.privacy_status(f.project)?);
                    // A structurally valid retry succeeds after the rejected request.
                    value(record(&compact)?)?;
                }
                let r = response(&plan, 3181)?;
                for size in [65535, 65536, 65537] {
                    let mut bytes = serde_json::to_vec(&r)?;
                    bytes.resize(size, b' '); // formatting does not change realization size
                    let before = f.operations.privacy_status(f.project)?;
                    let out = record(&bytes)?;
                    if size <= 65536 {
                        value(out)?;
                    } else {
                        rejected(out, "input file", "65537", "65536");
                        assert_eq!(before, f.operations.privacy_status(f.project)?);
                    }
                }
                // Byte fit never licenses a stale preparation.
                let mut stale = r.clone();
                stale.plan_fingerprint = "sha256:".to_owned() + &"0".repeat(64);
                let before = f.operations.privacy_status(f.project)?;
                let out = record(&serde_json::to_vec(&stale)?)?;
                assert_eq!(out.status.code(), Some(1));
                assert!(String::from_utf8_lossy(&out.stderr).contains("stale preparation"));
                assert_eq!(before, f.operations.privacy_status(f.project)?);
            }
        }
        assert_eq!(canonical, f.operations.canonical_basis(f.project)?);
    }
    Ok(())
}

#[test]
fn preparation_checks_final_json_and_impossible_metadata_before_generation(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = fixture()?;
    let subject = ExplanationSubject::Work(f.goals["relay"]);
    let canonical = f.operations.canonical_basis(f.project)?;
    let base = prepare_explanation(&canonical, subject, "ko")?;
    let original_size = serde_json::to_vec(&base)?.len();
    for target in [131071, 131072, 131073] {
        let mut c = canonical.clone();
        let goal = c
            .context_items
            .iter_mut()
            .find(|c| c.id == f.goals["relay"])
            .ok_or("goal")?;
        goal.statement.push_str(&"x".repeat(target - original_size));
        let out = prepare_explanation(&c, subject, "ko");
        if target <= 131072 {
            assert_eq!(serde_json::to_vec(&out?)?.len(), target);
        } else {
            let error = out.err().ok_or("oversize accepted")?;
            assert!(
                error.contains("preparation JSON")
                    && error.contains("131073")
                    && error.contains("131072"),
                "{error}"
            );
        }
    }
    // This supported canonical shape has metadata alone beyond any supported
    // retained envelope. Actual CLI preparation must fail before requesting prose.
    let mut input = reading_fixture::rich_scenario()?;
    input["source_actor_identity"] = json!("기록 \"actor\"\\".repeat(10000));
    let impossible = reading_fixture::fixture_scenario(input)?;
    for (kind, selector, id) in [
        ("work", "--work", impossible.goals["relay"].to_string()),
        (
            "decision",
            "--decision",
            impossible.decisions["project"].to_string(),
        ),
    ] {
        let out = invoke(
            &impossible,
            &[
                kind,
                "explain",
                "prepare",
                selector,
                &id,
                "--language",
                "ko",
            ],
        )?;
        rejected(out, "retained envelope", "metadata", "147456");
        assert!(impossible
            .operations
            .privacy_status(impossible.project)?
            .managed_derived
            .is_empty());
    }
    Ok(())
}

#[test]
fn envelopes_above_the_old_read_bound_round_trip_through_actual_cli(
) -> Result<(), Box<dyn std::error::Error>> {
    let f = reading_fixture::fixture_scenario(reading_fixture::explanation_size_scenario(
        "near_read_limit",
    )?)?;
    for (kind, selector, id, subject, read) in [
        (
            "work",
            "--work",
            f.goals["relay"].to_string(),
            ExplanationSubject::Work(f.goals["relay"]),
            "status",
        ),
        (
            "decision",
            "--decision",
            f.decisions["project"].to_string(),
            ExplanationSubject::Decision(f.decisions["project"]),
            "decisions",
        ),
    ] {
        for language in ["en", "ko"] {
            let plan = f
                .operations
                .prepare_explanation(f.project, subject, language)?;
            let r = response(&plan, 16384)?;
            let path = f._temporary.path().join("near-limit.json");
            std::fs::write(&path, serde_json::to_vec_pretty(&r)?)?;
            let result = value(invoke(
                &f,
                &[
                    kind,
                    "explain",
                    "record",
                    selector,
                    &id,
                    "--language",
                    language,
                    "--input",
                    path.to_str().ok_or("path")?,
                ],
            )?)?;
            let retained: RetainedExplanation =
                serde_json::from_value(result["explanation"].clone())?;
            let size = serde_json::to_vec(&retained)?.len();
            println!("near-read-bound {kind}/{language}: retained={size}");
            assert!(size > 131072 && size <= 147456);
            let budget = &plan.retention_budget;
            assert!(size <= budget.metadata_byte_reserve + 16384);
            assert_eq!(retained.realization, r);
            let readback = value(invoke(&f, &[read, "--language", language])?)?;
            assert!(readback.to_string().contains(&plan.fingerprint));
        }
    }
    Ok(())
}
