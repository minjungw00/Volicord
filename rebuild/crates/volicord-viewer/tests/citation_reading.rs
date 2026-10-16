//! Independent multi-citation control through real retained Product and readers.
//! Authored prose demonstrates reading contracts, not semantic or human approval.
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
        let temp = tempfile::tempdir()?;
        let repo = temp.path().join("repo");
        fs::create_dir(&repo)?;
        let mut originals: Vec<_> = (0..8)
            .map(|i| {
                format!(
                    "# Leading inventory {i}\n{}",
                    "# unrelated inventory\n".repeat(190)
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
        let names: Vec<_> = (0..11)
            .map(|i| {
                if i < 8 {
                    format!("inventory{i}.py")
                } else {
                    ["admit.py", "verify.ts", "retain.c"][i - 8].into()
                }
            })
            .collect();
        for (name, text) in names.iter().zip(&originals) {
            fs::write(repo.join(name), text)?;
        }
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
        for name in &names {
            let receipt = ops.observe_repository_file(RepositoryFileObservationRequest {
                project_id: project,
                analysis_snapshot_id: analysis,
                path: name.into(),
                byte_range: None,
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
                    start_byte: 0,
                    end_byte: originals[i].len(),
                    claim: None,
                })
                .collect()
        };
        let realization = ops.compose_repository_explanation(project, analysis, RepositoryExplanationDraft {
            plan_fingerprint:plan.fingerprint, language:"en".into(), generator:ExplanationGenerator {host:"authored-control".into(),session:"bounded-reader".into(),agent:None,model:None},
            paragraphs:vec![
                RepositoryReaderUnit {question:ExplanationQuestion::Observation,text:"These inventory files are retained observations.".into(), evidence_keys:vec!["repository_basis".into()],sources:ranges(&(0..8).collect::<Vec<_>>())},
                RepositoryReaderUnit {question:ExplanationQuestion::ReadingPoint,text:"Admission calls verification; the retained result is conditional on acceptance.".into(),evidence_keys:vec!["repository_basis".into()],sources:ranges(&[8,9,10])},
                RepositoryReaderUnit {question:ExplanationQuestion::Observation,text:"The same admission function is cited again with a different claim.".into(),evidence_keys:vec!["repository_basis".into()],sources:ranges(&[8])},
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
