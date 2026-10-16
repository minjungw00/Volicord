//! Independent multi-citation control through real retained Product and readers.
//! Authored prose demonstrates reading contracts, not semantic or human approval.
use serde_json::Value;
use std::{error::Error, fs, io::Cursor};
use volicord_context::ProjectId;
use volicord_operations::*;
use volicord_projections::*;
use volicord_repository_intelligence::AnalysisSnapshotId;
use volicord_viewer::*;

struct Fixture {
    temp: tempfile::TempDir,
    ops: LocalOperations,
    project: ProjectId,
    analysis: AnalysisSnapshotId,
    realization: ExplanationRealization,
    originals: Vec<String>,
}
impl Fixture {
    fn new() -> Result<Self, Box<dyn Error>> {
        Self::new_in(None)
    }
    fn new_in(parent: Option<&std::path::Path>) -> Result<Self, Box<dyn Error>> {
        Self::new_kind(parent, false)
    }
    fn new_kind(
        parent: Option<&std::path::Path>,
        production: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let temp = match parent {
            Some(path) => tempfile::tempdir_in(path)?,
            None => tempfile::tempdir()?,
        };
        let repo = temp.path().join("repo");
        fs::create_dir(&repo)?;
        let mut originals: Vec<_> = (0..8)
            .map(|i| {
                format!(
                    "# Leading inventory {i}\n{}",
                    "# unrelated inventory\n".repeat(300)
                )
            })
            .collect();
        // Independently authored mechanism, in three languages and exact ranges.
        originals.extend([
            format!(
                "def admit(value):\n    return verify(value)\n{}",
                "# 검증 맥락\n".repeat(100)
            ),
            format!(
                "export function verify(value) {{ return value !== null; }}\n{}",
                "// retained context\n".repeat(100)
            ),
            format!(
                "int retain(int accepted) {{ return accepted ? 1 : 0; }}\n{}",
                "// original context\n".repeat(100)
            ),
        ]);
        let mut names: Vec<_> = (0..11)
            .map(|i| {
                if i < 8 {
                    format!("inventory{i}.py")
                } else {
                    ["admit.py", "verify.ts", "retain.c"][i - 8].into()
                }
            })
            .collect();
        let mut files = originals.clone();
        let mut intervals: Vec<_> = originals.iter().map(|s| (0, s.len())).collect();
        if production {
            // Exact current maintained source files, copied whole into a private
            // repository; only explicitly bounded function regions are observed.
            for (i, name, full, start, end) in [
                (
                    8,
                    "explanation.rs",
                    include_str!("../../volicord-operations/src/explanation.rs"),
                    "    pub fn read_explanation_citation(",
                    "    pub(crate) fn attach_explanations(",
                ),
                (
                    9,
                    "explanation_sources.rs",
                    include_str!("../../volicord-operations/src/explanation_sources.rs"),
                    "    pub(crate) fn validate_subject_sources_selected(",
                    "            match citation.use_kind {",
                ),
                (
                    10,
                    "repository_explanation.rs",
                    include_str!("../../volicord-operations/src/repository_explanation.rs"),
                    "    pub fn read_repository_explanation_source(",
                    "    pub fn delete_repository_source_observation(",
                ),
            ] {
                let start = full.find(start).ok_or("function start")?;
                let end = start + full[start..].find(end).ok_or("function end")?;
                intervals[i] = (start, end);
                files[i] = full.into();
                originals[i] = full[start..end].into();
                names[i] = name.into();
            }
        }
        for (name, text) in names.iter().zip(&files) {
            fs::write(repo.join(name), text)?;
        }
        fs::write(repo.join("alternate.py"), &originals[8])?;
        let ops = LocalOperations::new(RuntimeLayout::new(temp.path().join("runtime"))?);
        let project = ops
            .initialize_project("Verify a bounded mechanism", Some(&repo))?
            .project
            .id;
        let analysis = ops
            .analyze(project, vec![])?
            .value
            .ok_or("analysis")?
            .analysis
            .identity;
        let mut sources = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let receipt = ops.observe_repository_file(RepositoryFileObservationRequest {
                project_id: project,
                analysis_snapshot_id: analysis,
                path: name.into(),
                byte_range: (production && i >= 8).then_some(FileByteRange {
                    start_byte: intervals[i].0,
                    end_byte: intervals[i].1,
                }),
                retain_seconds: 3600,
            })?;
            let id = receipt["source_id"].as_str().ok_or("source")?;
            let mut bytes = [0; 16];
            for (n, byte) in bytes.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&id[n * 2..n * 2 + 2], 16)?;
            }
            sources.push(volicord_context::SourceId::from_bytes(bytes));
        }
        let plan =
            ops.prepare_explanation(project, ExplanationSubject::Repository(analysis), "en")?;
        let ranges = |indices: &[usize]| {
            indices
                .iter()
                .map(|&i| RepositoryClaimRange {
                    source_id: sources[i],
                    start_byte: intervals[i].0,
                    end_byte: intervals[i].1,
                    claim: None,
                })
                .collect()
        };
        let realization = ops.compose_repository_explanation(project, analysis, RepositoryExplanationDraft {
            plan_fingerprint:plan.fingerprint, language:"en".into(), generator:ExplanationGenerator {host:"authored-control".into(),session:"bounded-reader".into(),agent:None,model:None},
            paragraphs:vec![
                RepositoryReaderUnit {question:ExplanationQuestion::Observation,text:"These inventory files are retained observations.".into(), evidence_keys:vec!["repository_basis".into()],sources:ranges(&(0..8).collect::<Vec<_>>())},
                RepositoryReaderUnit {question:ExplanationQuestion::ReadingPoint,text:if production {"The exact citation reader checks the current retained realization and validates its Source dependencies before returning a bounded page."} else {"Admission calls verification; the retained result is conditional on acceptance."}.into(),evidence_keys:vec!["repository_basis".into()],sources:ranges(&[8,9,10])},
                RepositoryReaderUnit {question:ExplanationQuestion::Observation,text:if production {"The same reader rejects an old realization instead of rebinding its claim index."} else {"The same admission function is cited again with a different claim."}.into(),evidence_keys:vec!["repository_basis".into()],sources:ranges(&[8])},
            ]})?;
        ops.record_explanation(
            project,
            ExplanationSubject::Repository(analysis),
            "en",
            realization.clone(),
        )?;
        Ok(Self {
            temp,
            ops,
            project,
            analysis,
            realization,
            originals,
        })
    }
    fn answers(&self) -> Result<QuestionAnswers, Box<dyn Error>> {
        Ok(self.ops.read_repository_explanation(self.project, "en")?)
    }
    fn request(&self) -> ViewerRequest {
        ViewerRequest {
            project_id: self.project,
            locale: ViewerLocale::English,
            view: ViewerView::RepositoryExplanation,
            requested_language: "en".into(),
            guarded_request: None,
        }
    }
    fn server(&self) -> Result<ViewerServer, Box<dyn Error>> {
        Ok(ViewerServer::new(
            ViewerAdapter::new(LocalOperations::new(self.ops.layout().clone())),
            self.project,
            ViewerLocale::English,
            ViewerView::RepositoryExplanation,
            "en".into(),
            "127.0.0.1:43129".parse()?,
        )?)
    }
    fn get(&self, target: &str) -> Result<String, Box<dyn Error>> {
        let mut output = Vec::new();
        self.server()?.serve_connection(
            &mut Cursor::new(format!(
                "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:43129\r\n\r\n"
            )),
            &mut output,
        )?;
        Ok(String::from_utf8(output)?)
    }
}
#[test]
fn important_late_multi_citation_survives_initial_and_offline_bounds() -> Result<(), Box<dyn Error>>
{
    let f = Fixture::new()?;
    let canonical = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let answers = f.answers()?;
    assert_eq!(answers.explanation_state, ExplanationState::Current);
    assert_eq!(
        answers.source_excerpts.len(),
        11,
        "shared range counted once"
    );
    assert_eq!(answers.omitted_source_excerpt_count, 0);
    let total: usize = answers.source_excerpts.iter().map(|e| e.text.len()).sum();
    assert!((16_380..=16_384).contains(&total));
    for (e, original) in answers.source_excerpts.iter().zip(&f.originals) {
        assert_eq!(e.text, original[..e.text.len()]);
        assert_eq!(e.omitted_utf8_bytes, original.len() - e.text.len());
        assert!(e.text.len() <= 4096);
        assert_eq!(e.displayed_end_byte, e.text.len());
        assert_eq!(
            e.displayed_end_line,
            1 + e.text.as_bytes()[..e.text.len() - 1]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
        );
    }
    for needle in [
        "return verify(value)",
        "return value !== null",
        "accepted ? 1 : 0",
    ] {
        assert!(f.get("/?view=repository")?.contains(needle));
    }
    let destination = f.temp.path().join("offline.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &destination)?;
    let offline = fs::read_to_string(destination)?;
    assert_eq!(privacy, f.ops.privacy_status(f.project)?);
    for needle in [
        "return verify(value)",
        "return value !== null",
        "accepted ? 1 : 0",
    ] {
        assert!(offline.contains(needle));
    }
    assert!(!offline.contains("<script"));
    assert!(offline.contains("Omitted UTF-8 bytes in this cited range"));
    // A different traversal and duplicated early citations must preserve allocations.
    let mut reordered = f.realization.clone();
    reordered.paragraphs[0].citations.reverse();
    reordered.paragraphs[1].citations.reverse();
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        reordered,
    )?;
    let after = f.answers()?;
    for e in &answers.source_excerpts {
        let same = after
            .source_excerpts
            .iter()
            .find(|x| x.citation.source_id == e.citation.source_id)
            .ok_or("range")?;
        assert_eq!(same.text, e.text);
    }
    assert_eq!(canonical, f.ops.canonical_basis(f.project)?);
    // Only the explicit record above writes; ordinary GET/export did not.
    assert_eq!(
        privacy.managed_derived.len() + 1,
        f.ops.privacy_status(f.project)?.managed_derived.len()
    );
    Ok(())
}

fn selection(
    f: &Fixture,
    unit: usize,
    citation: usize,
    offset: usize,
) -> Result<ExplanationCitationSelection, Box<dyn Error>> {
    Ok(ExplanationCitationSelection {
        realization_fingerprint: f
            .answers()?
            .provenance
            .ok_or("provenance")?
            .realization_fingerprint,
        unit_index: unit,
        citation_index: citation,
        offset,
    })
}
fn citation_target(f: &Fixture, s: &ExplanationCitationSelection) -> String {
    format!(
        "/citation?subject=repository:{}&language=en&realization={}&unit={}&citation={}&offset={}",
        f.analysis, s.realization_fingerprint, s.unit_index, s.citation_index, s.offset
    )
}
fn body(response: &str) -> Result<Value, Box<dyn Error>> {
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let raw = response.split_once("\r\n\r\n").ok_or("body")?.1;
    assert!(raw.len() <= 32768);
    Ok(serde_json::from_str(raw)?)
}
#[test]
fn selected_citation_reads_original_bytes_in_bounded_pages_without_writes(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let canonical = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let first = selection(&f, 0, 0, 0)?;
    let data = body(&f.get(&citation_target(&f, &first))?)?;
    assert_eq!(data["chunk"]["text"], &f.originals[0][..4096]);
    assert_eq!(data["chunk"]["displayed_start_byte"], 0);
    assert_eq!(data["chunk"]["displayed_end_byte"], 4096);
    assert_eq!(data["chunk"]["next_offset"], 4096);
    let next = ExplanationCitationSelection {
        offset: 4096,
        ..first.clone()
    };
    let tail = body(&f.get(&citation_target(&f, &next))?)?;
    assert_eq!(tail["chunk"]["text"], &f.originals[0][4096..]);
    assert_eq!(tail["chunk"]["omitted_before_utf8_bytes"], 4096);
    assert_eq!(tail["chunk"]["omitted_after_utf8_bytes"], 0);
    assert!(tail["chunk"]["next_offset"].is_null());
    for (unit, citation, index) in [(1, 0, 8), (1, 1, 9), (1, 2, 10), (2, 0, 8)] {
        let s = selection(&f, unit, citation, 0)?;
        let result = body(&f.get(&citation_target(&f, &s))?)?;
        assert_eq!(result["claim"], f.realization.paragraphs[unit].text);
        assert_eq!(result["chunk"]["text"], f.originals[index]);
        assert_eq!(result["chunk"]["citation"]["source_revision"], 1);
        assert_eq!(
            result["chunk"]["citation"]["analysis_snapshot_id"],
            f.analysis.to_string()
        );
        use sha2::{Digest, Sha256};
        assert_eq!(
            result["chunk"]["citation"]["range_sha256"],
            format!("sha256:{:x}", Sha256::digest(f.originals[index].as_bytes()))
        );
    }
    let korean_split = f.originals[8].find('검').ok_or("Korean")? + 1;
    let bad = selection(&f, 1, 0, korean_split)?;
    assert!(f
        .get(&citation_target(&f, &bad))?
        .starts_with("HTTP/1.1 409"));
    for (unit, citation, offset) in [(99, 0, 0), (1, 99, 0), (1, 0, usize::MAX)] {
        let bad = selection(&f, unit, citation, offset)?;
        let result = f.get(&citation_target(&f, &bad))?;
        assert!(result.starts_with("HTTP/1.1 409"));
        assert!(!result.contains("return verify"));
    }
    assert_eq!(canonical, f.ops.canonical_basis(f.project)?);
    assert_eq!(privacy, f.ops.privacy_status(f.project)?);
    Ok(())
}
#[test]
fn exact_citation_rejects_changed_output_wrong_subject_origin_and_deleted_support(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let selected = selection(&f, 1, 0, 0)?;
    let target = citation_target(&f, &selected);
    for (old, new) in [
        (
            format!("repository:{}", f.analysis),
            format!("repository:{}", "ab".repeat(32)),
        ),
        (
            format!("repository:{}", f.analysis),
            format!("work:{}", "ab".repeat(16)),
        ),
        ("language=en".into(), "language=ko".into()),
    ] {
        let response = f.get(&target.replace(&old, &new))?;
        assert!(response.starts_with("HTTP/1.1 409"));
        assert!(!response.contains("return verify"));
    }
    assert!(f
        .get(&(target.clone() + "&source_id=any"))?
        .starts_with("HTTP/1.1 400"));
    for header in [
        "Origin: https://foreign.invalid",
        "Sec-Fetch-Site: cross-site",
        "Host: foreign.invalid",
    ] {
        let mut output = Vec::new();
        let host = if header.starts_with("Host:") {
            String::new()
        } else {
            "Host: 127.0.0.1:43129\r\n".into()
        };
        f.server()?.serve_connection(
            &mut Cursor::new(format!("GET {target} HTTP/1.1\r\n{host}{header}\r\n\r\n")),
            &mut output,
        )?;
        let response = String::from_utf8(output)?;
        assert!(response.starts_with("HTTP/1.1 403") || response.starts_with("HTTP/1.1 421"));
        assert!(!response.contains("return verify"));
    }
    let mut changed = f.realization.clone();
    changed.paragraphs[1].citations.reverse();
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        changed,
    )?;
    assert!(f.get(&target)?.starts_with("HTTP/1.1 409"));
    let current = selection(&f, 1, 2, 0)?;
    assert_eq!(
        body(&f.get(&citation_target(&f, &current))?)?["chunk"]["text"],
        f.originals[8]
    );
    let source = &f.realization.paragraphs[1].citations[0].source_id;
    let bytes: Vec<u8> = (0..32)
        .step_by(2)
        .map(|n| u8::from_str_radix(&source[n..n + 2], 16))
        .collect::<Result<_, _>>()?;
    f.ops.delete_repository_source_observation(
        f.project,
        f.analysis,
        volicord_context::SourceId::from_bytes(bytes.try_into().map_err(|_| "source")?),
    )?;
    let canonical = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let unavailable = f.get(&citation_target(&f, &current))?;
    assert!(unavailable.starts_with("HTTP/1.1 409"));
    assert!(!unavailable.contains("return verify"));
    assert!(!f
        .get("/?view=repository")?
        .contains("<code lang=\"\">def admit"));
    assert_eq!(canonical, f.ops.canonical_basis(f.project)?);
    assert_eq!(privacy, f.ops.privacy_status(f.project)?);
    Ok(())
}

#[test]
fn selected_citation_withholds_expired_corrupt_unsupported_and_changed_file_evidence(
) -> Result<(), Box<dyn Error>> {
    for variant in [
        "expired",
        "corrupt_copy",
        "unsupported_copy",
        "unsupported_output",
        "corrupt_output",
        "corrupt_range",
        "changed_file",
        "redirected_file",
        "forgotten_source",
    ] {
        let f = Fixture::new()?;
        let chosen = selection(&f, 1, 0, 0)?;
        if variant == "changed_file" {
            fs::write(
                f.temp.path().join("repo/admit.py"),
                "def admit(value): return True\n",
            )?;
        } else if variant == "redirected_file" {
            #[cfg(unix)]
            {
                fs::remove_file(f.temp.path().join("repo/admit.py"))?;
                std::os::unix::fs::symlink("alternate.py", f.temp.path().join("repo/admit.py"))?;
            }
        } else if variant == "forgotten_source" {
            let c = &f.realization.paragraphs[1].citations[0];
            let bytes: Vec<u8> = (0..32)
                .step_by(2)
                .map(|n| u8::from_str_radix(&c.source_id[n..n + 2], 16))
                .collect::<Result<_, _>>()?;
            let source =
                volicord_context::SourceId::from_bytes(bytes.try_into().map_err(|_| "source")?);
            let auth = f
                .ops
                .record_user_source(
                    f.project,
                    "authored-control".into(),
                    "forget-fixture".into(),
                    "Forget this observed Source".into(),
                )?
                .identity;
            let bytes: Vec<u8> = (0..32)
                .step_by(2)
                .map(|n| u8::from_str_radix(&auth[n..n + 2], 16))
                .collect::<Result<_, _>>()?;
            let auth = volicord_context::SourceId::from_bytes(
                bytes.try_into().map_err(|_| "authorization")?,
            );
            f.ops.forget_record(
                f.project,
                volicord_context::CanonicalRecordId::Source(source),
                auth,
            )?;
        } else {
            let connection = rusqlite::Connection::open(f.ops.layout().privacy_store())?;
            let records: Vec<(Vec<u8>, String)> = connection
                .prepare("SELECT id,record_json FROM managed_derived")?
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<_, _>>()?;
            let (id, raw) = records
                .iter()
                .find(|(_, raw)| {
                    let kind = if variant.ends_with("output") {
                        "cached_summary"
                    } else {
                        "source_observation"
                    };
                    raw.contains("admit.py")
                        && serde_json::from_str::<Value>(raw).is_ok_and(|r| r["kind"] == kind)
                })
                .ok_or("copy")?;
            let mut record: Value = serde_json::from_str(raw)?;
            if variant == "expired" {
                record["retained_until"] = serde_json::json!(1);
            } else if variant == "corrupt_output" {
                record["content"] = serde_json::json!("invalid retained explanation JSON");
            } else {
                let mut copy: Value =
                    serde_json::from_str(record["content"].as_str().ok_or("copy body")?)?;
                match variant {
                    "unsupported_output" => {
                        copy["realization"]["format_version"] = serde_json::json!(999)
                    }
                    "corrupt_copy" => copy["detail"]["body"] = serde_json::json!("invented bytes"),
                    "unsupported_copy" => copy["format_version"] = serde_json::json!(999),
                    _ => {
                        copy["detail"]["region"] =
                            serde_json::json!({"start_byte":999999,"end_byte":1})
                    }
                }
                record["content"] = serde_json::json!(serde_json::to_string(&copy)?);
            }
            connection.execute(
                "UPDATE managed_derived SET record_json=?1 WHERE id=?2",
                rusqlite::params![serde_json::to_string(&record)?, id],
            )?;
        }
        let before = f.ops.privacy_status(f.project)?;
        let response = f.get(&citation_target(&f, &chosen))?;
        assert!(
            response.starts_with("HTTP/1.1 409"),
            "{variant}: {response}"
        );
        assert!(!response.contains("return verify"), "{variant}");
        assert_eq!(before, f.ops.privacy_status(f.project)?);
    }
    Ok(())
}
#[test]
fn seed_citation_browser_runtime() -> Result<(), Box<dyn Error>> {
    let Some(output) = std::env::var_os("VOLICORD_CITATION_REVIEW_ROOT") else {
        return Ok(());
    };
    let output = std::path::Path::new(&output);
    fs::create_dir_all(output)?;
    let f = Fixture::new_kind(Some(output), true)?;
    let answers = f.answers()?;
    let path = output.join("offline.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &path)?;
    fs::write(
        output.join("fixture.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
        "project":f.project.to_string(),"analysis":f.analysis.to_string(),"runtime":f.ops.layout().root(),
        "originals":f.originals,"answers":answers,"input_provenance":"independently-authored-control; no human verdict"}))?,
    )?;
    let _ = f.temp.keep();
    Ok(())
}
