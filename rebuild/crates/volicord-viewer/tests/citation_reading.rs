//! Independent multi-citation control through real retained Product and readers.
//! Authored prose demonstrates reading contracts, not semantic or human approval.
use serde_json::Value;
use std::{error::Error, fs, io::Cursor};
use volicord_context::ProjectId;
use volicord_operations::*;
use volicord_projections::*;
use volicord_repository_intelligence::AnalysisSnapshotId;
use volicord_viewer::*;
#[path = "support/code_text.rs"]
mod code_text;
use code_text::code_texts;

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
        Self::new_sources(parent, production, None)
    }
    fn new_sources(
        parent: Option<&std::path::Path>,
        production: bool,
        custom: Option<Vec<(String, String)>>,
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
        if let Some(custom) = custom {
            assert_eq!(custom.len(), 11);
            names = custom.iter().map(|(name, _)| name.clone()).collect();
            originals = custom.into_iter().map(|(_, source)| source).collect();
        }
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
                    "reading.js",
                    include_str!("../src/reading.js"),
                    "  async function readCitation(",
                    "  const fragmentNotice =",
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
            diagrams: Vec::new(),
            plan_fingerprint:plan.fingerprint, language:"en".into(), generator:ExplanationGenerator {host:"authored-control".into(),session:"bounded-reader".into(),agent:None,model:None},
            paragraphs:vec![
                RepositoryReaderUnit {question:ExplanationQuestion::Observation,text:"These inventory files are retained observations.".into(), evidence_keys:vec!["repository_basis".into()],sources:ranges(&(0..8).collect::<Vec<_>>())},
                RepositoryReaderUnit {question:ExplanationQuestion::Narrative,text:if production {"The exact citation reader checks the current retained realization and validates its Source dependencies before returning a bounded page."} else {"Admission calls verification; the retained result is conditional on acceptance."}.into(),evidence_keys:vec!["repository_basis".into()],sources:ranges(&[8,9,10])},
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
        Ok(self.ops.read_repository_explanation(
            self.project,
            "en",
            volicord_projections::FixedLocale::English,
        )?)
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
    assert!(answers.source_excerpts.len() >= 3 && answers.source_excerpts.len() < 11);
    assert_eq!(
        answers.embedded_source_scope[1].state,
        SourceWindowState::Complete
    );
    assert_eq!(
        answers.embedded_source_scope[1].included_citation_indices,
        vec![0, 1, 2]
    );
    assert_eq!(
        answers.embedded_source_scope[0].state,
        SourceWindowState::InitialOmission
    );
    assert_eq!(
        answers.source_reading_scope(true)[0].state,
        SourceWindowState::OfflineOmission
    );
    assert_eq!(
        answers.omitted_source_excerpt_count,
        11 - answers.source_excerpts.len()
    );
    let total: usize = answers.source_excerpts.iter().map(|e| e.text.len()).sum();
    assert!(total <= EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT);
    for e in &answers.source_excerpts {
        let index = f
            .realization
            .citations()
            .position(|c| c.source_id == e.citation.source_id)
            .ok_or("original source")?;
        let original = &f.originals[index];
        assert_eq!(&e.text, original);
        assert_eq!(e.omitted_utf8_bytes, original.len() - e.text.len());
        assert!(e.text.len() <= 16_384);
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
        assert!(code_texts(&f.get("/?view=repository")?)
            .iter()
            .any(|s| s.contains(needle)));
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
        assert!(code_texts(&offline).iter().any(|s| s.contains(needle)));
    }
    assert!(!offline.contains("<script"));
    assert!(offline.contains("data-source-window-state=\"offline_omission\""));
    assert!(offline.contains("All cited code included"));
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
        publication_id: f
            .answers()?
            .provenance
            .ok_or("provenance")?
            .publication_id
            .ok_or("publication identity")?,
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
        "/citation?subject=repository:{}&language=en&publication={}&realization={}&unit={}&citation={}&offset={}",
        f.analysis, s.publication_id, s.realization_fingerprint, s.unit_index, s.citation_index, s.offset
    )
}
fn body(response: &str) -> Result<Value, Box<dyn Error>> {
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    let raw = response.split_once("\r\n\r\n").ok_or("body")?.1;
    assert!(raw.len() <= 32768);
    Ok(serde_json::from_str(raw)?)
}

fn failure_body(response: &str) -> Result<Value, Box<dyn Error>> {
    assert!(response.starts_with("HTTP/1.1 409"));
    Ok(serde_json::from_str(
        response.split_once("\r\n\r\n").ok_or("HTTP body")?.1,
    )?)
}

fn highlighting_sources() -> Vec<(String, String)> {
    [
        ("pages.py", format!("# page boundary\r\n{}", "# 한국어 🦀 repeated context\r\n".repeat(240))),
        ("untrusted.js", "export function render(value) {\r\n\tconst s = \"</code><script>alert('x')</script>&amp; 🦀\";\r\n\t// <img src=x onerror=alert(1)>\r\n\treturn s + value;\r\n}\r\n".into()),
        ("raw.rs", "pub fn text<'a>(v: &'a str) -> &'a str {\n /* outer /* inner */ done */\n let raw = r##\"</span><script>🦀</script>\"##;\n let ch = '한'; // 0/1/many tokens\n v\n}\n".into()),
        ("strings.py", "def render(값):\r\n\ttext = r'''<script>\n한국어 🐍\n'''\n\treturn text + 값 # not a claim\n# incomplete\n\"unterminated".into()),
        ("script.sh", "#!/bin/sh\nif [ \"$x\" = 'it\\s' ]; then\n  export value=${name}\n  printf '%s\\n' \"$value\" # comment\nfi\n".into()),
        ("value.json", "{\"key\": \"<script>🦀&amp;\", \"ok\": true, \"n\": 42}\n".into()),
        ("stdout.log", "\u{1b}[31mERROR\u{1b}[0m <script>alert(1)</script>\r\nnot Rust: fn main() {}\n".into()),
        ("dense.ts", "let x=1; ".repeat(450)),
        ("admit.rs", "pub fn admit(value: Option<&str>) -> bool {\n    // 한국어 🦀\n    let quoted = r#\"<script> ' \\\" &amp;\"#;\n    value.is_some()\n}\n".into()),
        ("verify.js", "export function verify(value) {\n    // 🦀 literal Source\n    const text = `</code><script>alert('x')</script>`;\n    return value !== null;\n}\n".into()),
        ("output.unknown", "return <script>plain 🦀</script>\n\nlast line without newline".into()),
    ].into_iter().map(|(name, source)|(name.into(),source)).collect()
}

#[test]
fn highlighting_preserves_static_offline_and_live_admitted_text() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new_sources(None, false, Some(highlighting_sources()))?;
    let before = f.ops.privacy_status(f.project)?;
    let canonical = f.ops.canonical_basis(f.project)?;
    let answers = f.answers()?;
    let destination = f.temp.path().join("highlighting.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &destination)?;
    for output in [
        f.get("/?view=repository")?,
        fs::read_to_string(destination)?,
    ] {
        let codes = code_texts(&output);
        for excerpt in &answers.source_excerpts {
            assert!(
                codes.contains(&excerpt.text),
                "{}",
                excerpt.citation.locator
            );
        }
        for class in [
            "keyword", "comment", "string", "type", "function", "variable", "number",
        ] {
            assert!(
                output.contains(&format!("class=\"syn-{class}\"")),
                "{class}"
            );
        }
        assert!(output.contains("data-syntax=\"shell\""));
        assert!(output.contains("data-syntax=\"json\""));
        assert!(
            output.contains("&#13;"),
            "HTML must not normalize original CRLF"
        );
        assert!(!output.contains("<script>alert"));
        assert!(!output.contains("<img src=x"));
        assert!(output.contains("Observed cited range"));
    }
    for (unit, citation, index, language) in [
        (0, 0, 0, "python"),
        (0, 1, 1, "javascript"),
        (0, 2, 2, "rust"),
        (0, 3, 3, "python"),
        (0, 4, 4, "shell"),
        (0, 5, 5, "json"),
        (0, 6, 6, "plain"),
        (0, 7, 7, "typescript"),
        (1, 0, 8, "rust"),
        (1, 1, 9, "javascript"),
        (1, 2, 10, "plain"),
    ] {
        let s = selection(&f, unit, citation, 0)?;
        let response = body(&f.get(&citation_target(&f, &s))?)?;
        let text = response["chunk"]["text"].as_str().ok_or("text")?;
        assert_eq!(text, &f.originals[index][..text.len()]);
        let decoration = &response["highlight"];
        assert_eq!(decoration["language"].as_str().unwrap_or("plain"), language);
        let tokens = decoration["tokens"].as_array().ok_or("tokens")?;
        assert!(tokens.len() <= 256);
        if language == "plain" || index == 7 {
            assert!(
                tokens.is_empty(),
                "unknown/log and token overflow stay plain"
            );
        } else {
            assert!(!tokens.is_empty(), "{language}");
        }
        let units: Vec<_> = text.encode_utf16().collect();
        let mut previous = 0;
        for t in tokens {
            let start = t[0].as_u64().ok_or("start")? as usize;
            let end = t[1].as_u64().ok_or("end")? as usize;
            assert!(previous <= start && start < end && end <= units.len());
            String::from_utf16(&units[start..end])?;
            previous = end;
        }
    }
    assert_eq!(before, f.ops.privacy_status(f.project)?);
    assert_eq!(canonical, f.ops.canonical_basis(f.project)?);
    Ok(())
}

#[test]
fn decoration_yields_to_the_existing_http_response_budget() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new_kind(None, true)?;
    let mut realization = f.realization.clone();
    let mut unit = realization.paragraphs[1].clone();
    // Leave at most a few bytes below the public response limit using measured
    // undecorated output. Function refactors must not move this boundary control
    // away from the boundary it is meant to exercise.
    unit.text = "한".repeat(8500);
    unit.citations.truncate(1);
    unit.citations[0].claim_end_byte = unit.text.len();
    unit.citations[0].claim_sha256 = explanation_text_hash(&unit.text);
    realization.paragraphs = vec![unit.clone()];
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization.clone(),
    )?;
    let s = selection(&f, 0, 0, 0)?;
    let baseline = f.ops.read_explanation_citation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        &s,
    )?;
    let additional = (32_768 - baseline.to_string().len() - 32) / 3;
    unit.text.push_str(&"한".repeat(additional));
    unit.citations[0].claim_end_byte = unit.text.len();
    unit.citations[0].claim_sha256 = explanation_text_hash(&unit.text);
    realization.paragraphs = vec![unit];
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization,
    )?;
    let s = selection(&f, 0, 0, 0)?;
    let mut original = f.ops.read_explanation_citation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        &s,
    )?;
    let mut response = body(&f.get(&citation_target(&f, &s))?)?;
    assert!(
        response["highlight"].is_null(),
        "decoration must yield instead of widening the bound"
    );
    original["applicability"]["checked_at_unix_micros"] = serde_json::json!(null);
    response["applicability"]["checked_at_unix_micros"] = serde_json::json!(null);
    assert_eq!(
        response, original,
        "Source, claim and all coordinates stay exact"
    );
    Ok(())
}

#[test]
fn nul_transport_remains_exact_and_offline_discloses_html_parser_limit(
) -> Result<(), Box<dyn Error>> {
    let mut sources = highlighting_sources();
    sources[10].1.push('\0');
    let f = Fixture::new_sources(None, false, Some(sources))?;
    let chosen = selection(&f, 1, 2, 0)?;
    let result = body(&f.get(&citation_target(&f, &chosen))?)?;
    assert_eq!(result["chunk"]["text"], f.originals[10]);
    let destination = f.temp.path().join("nul.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &destination)?;
    let output = fs::read_to_string(destination)?;
    assert!(output.contains("HTML parsing displays replacement characters for NUL"));
    assert!(
        code_texts(&output).contains(&f.originals[10]),
        "raw HTML retains original bytes; parser limitation is disclosed"
    );
    Ok(())
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
fn selected_source_read_uses_committed_canonical_state_during_a_wal_writer(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let writer = rusqlite::Connection::open(f.ops.layout().canonical_store())?;
    writer.execute_batch("BEGIN IMMEDIATE")?;
    let selected = selection(&f, 1, 0, 0)?;
    let result = body(&f.get(&citation_target(&f, &selected))?)?;
    assert_eq!(result["chunk"]["text"], f.originals[8]);
    writer.execute_batch("ROLLBACK")?;
    Ok(())
}

#[test]
fn concurrent_selected_refresh_and_document_reads_keep_original_bytes_and_state(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let canonical = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let targets = [(1, 0, 8), (1, 1, 9), (1, 2, 10), (2, 0, 8)]
        .into_iter()
        .map(|(unit, citation, original)| {
            Ok((
                citation_target(&f, &selection(&f, unit, citation, 0)?),
                original,
            ))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    for _ in 0..3 {
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(6));
        let mut jobs = Vec::new();
        for index in 0..6 {
            let layout = f.ops.layout().clone();
            let project = f.project;
            let barrier = barrier.clone();
            let selected = if index < 4 {
                Some(selection(
                    &f,
                    if index == 3 { 2 } else { 1 },
                    if index == 3 { 0 } else { index },
                    0,
                )?)
            } else {
                None
            };
            let analysis = f.analysis;
            let target = targets
                .get(index)
                .map_or_else(|| "/?view=repository".into(), |(target, _)| target.clone());
            jobs.push(std::thread::spawn(move || -> Result<String, String> {
                barrier.wait();
                let server = ViewerServer::new(
                    ViewerAdapter::new(LocalOperations::new(layout)),
                    project,
                    ViewerLocale::English,
                    ViewerView::RepositoryExplanation,
                    "en".into(),
                    "127.0.0.1:43129"
                        .parse()
                        .map_err(|e: std::net::AddrParseError| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                server
                    .adapter()
                    .operations()
                    .canonical_basis(project)
                    .map_err(|e| format!("concurrent canonical read: {e:?}"))?;
                if let Some(selected) = selected {
                    server
                        .adapter()
                        .operations()
                        .read_explanation_citation(
                            project,
                            ExplanationSubject::Repository(analysis),
                            "en",
                            &selected,
                        )
                        .map_err(|e| format!("concurrent scoped operation: {e:?}"))?;
                }
                let mut output = Vec::new();
                server
                    .serve_connection(
                        &mut Cursor::new(format!(
                            "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:43129\r\n\r\n"
                        )),
                        &mut output,
                    )
                    .map_err(|e| e.to_string())?;
                String::from_utf8(output).map_err(|e| e.to_string())
            }));
        }
        // Join every reader before inspecting failures, keeping the fixture alive.
        let responses: Vec<_> = jobs.into_iter().map(|job| job.join()).collect();
        for (index, response) in responses.into_iter().enumerate() {
            let response = response.map_err(|_| "reader thread failed")??;
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
            if index < 4 {
                let result = body(&response)?;
                assert_eq!(result["chunk"]["text"], f.originals[targets[index].1]);
                assert_eq!(result["project_id"], f.project.to_string());
            }
        }
    }
    assert_eq!(f.ops.canonical_basis(f.project)?, canonical);
    assert_eq!(f.ops.privacy_status(f.project)?, privacy);
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
    assert_eq!(failure_body(&unavailable)?["reason"], "deleted");
    assert_eq!(
        f.answers()?.applicability.limitation,
        Some(ExplanationFailureReason::Deleted)
    );
    assert!(!unavailable.contains("return verify"));
    assert!(code_texts(&f.get("/?view=repository")?).is_empty());
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
        "missing_repository",
        "redirected_file",
        "forgotten_source",
    ] {
        let f = Fixture::new()?;
        let chosen = selection(&f, 1, 0, 0)?;
        if variant == "missing_repository" {
            fs::rename(
                f.temp.path().join("repo"),
                f.temp.path().join("repository-unavailable"),
            )?;
        } else if variant == "changed_file" {
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
        let expected = match variant {
            "expired" => Some(("expired", ExplanationFailureReason::Expired)),
            "changed_file" => Some(("changed", ExplanationFailureReason::Changed)),
            "missing_repository" => Some((
                "comparison_unavailable",
                ExplanationFailureReason::ComparisonUnavailable,
            )),
            "unsupported_copy" | "unsupported_output" => {
                Some(("unsupported", ExplanationFailureReason::Unsupported))
            }
            "corrupt_output" => Some(("corrupt", ExplanationFailureReason::Corrupt)),
            _ => None,
        };
        if let Some((reason, typed)) = expected {
            assert_eq!(failure_body(&response)?["reason"], reason, "{variant}");
            assert_eq!(
                f.answers()?.applicability.limitation,
                Some(typed),
                "{variant}"
            );
        }
        assert_eq!(before, f.ops.privacy_status(f.project)?);
    }
    Ok(())
}
#[test]
fn seed_citation_browser_runtime() -> Result<(), Box<dyn Error>> {
    let custom = std::env::var_os("VOLICORD_HIGHLIGHT_REVIEW_ROOT");
    let Some(output) = custom
        .clone()
        .or_else(|| std::env::var_os("VOLICORD_CITATION_REVIEW_ROOT"))
    else {
        return Ok(());
    };
    let output = std::path::Path::new(&output);
    // Every browser attempt, including a retry, owns a fresh retained fixture.
    fs::create_dir(output)?;
    let f = Fixture::new_sources(
        Some(output),
        custom.is_none(),
        custom.map(|_| highlighting_sources()),
    )?;
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

#[test]
fn offline_keeps_a_complete_material_function_without_starving_other_ranges(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new_kind(None, true)?;
    let answers = f.answers()?;
    let reader = answers
        .source_excerpts
        .iter()
        .find(|e| e.citation.locator == "explanation.rs")
        .ok_or("reader")?;
    assert_eq!(
        reader.text, f.originals[8],
        "the representative function must reach its actual validation and bounded-return logic"
    );
    assert!(reader
        .text
        .contains("self.validate_subject_sources_in_read("));
    assert!(reader.text.contains("bytes.len() > 32_768"));
    assert!(answers.source_excerpts.len() >= 3);
    assert_eq!(
        answers.omitted_source_excerpt_count,
        11 - answers.source_excerpts.len()
    );
    assert!(answers
        .source_excerpts
        .iter()
        .all(|e| !e.text.is_empty() && e.omitted_utf8_bytes == 0));
    assert!(
        answers
            .source_excerpts
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            <= EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT
    );
    let mut changed = f.realization.clone();
    changed.paragraphs.reverse();
    for unit in &mut changed.paragraphs {
        unit.citations.reverse();
    }
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        changed,
    )?;
    for e in f.answers()?.source_excerpts {
        let same = answers
            .source_excerpts
            .iter()
            .find(|b| b.citation.source_id == e.citation.source_id)
            .ok_or("same")?;
        assert_eq!(e.text, same.text);
    }
    let destination = f.temp.path().join("mechanism.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &destination)?;
    let html = fs::read_to_string(destination)?;
    assert!(code_texts(&html)
        .iter()
        .any(|s| s.contains("self.validate_subject_sources_in_read(")));
    assert!(code_texts(&html)
        .iter()
        .any(|s| s.contains("bytes.len() > 32_768")));
    assert!(!html.contains("<script"));
    Ok(())
}

#[test]
fn complete_reader_mechanisms_fit_together_without_dropping_source_dependencies(
) -> Result<(), Box<dyn Error>> {
    let full = include_str!("../../volicord-operations/src/operations.rs");
    let start = full
        .find("    pub fn publish_viewer_snapshot(")
        .ok_or("publication start")?;
    let end = start
        + full[start..]
            .find("    pub fn inquiry_frontier(")
            .ok_or("publication end")?;
    let mut sources: Vec<_> = (0..11)
        .map(|i| {
            (
                format!("inventory{i}.py"),
                "# retained inventory\n".repeat(300),
            )
        })
        .collect();
    sources[7] = ("publication.rs".into(), full[start..end].into());
    let mut f = Fixture::new_sources(None, true, Some(sources))?;
    let caller = f.realization.paragraphs[0].citations[7].clone();
    let unit = &mut f.realization.paragraphs[1];
    unit.text = "The reader validates Source dependencies before returning bytes; local publication independently revalidates canonical, explanation and verification bases under its mutation lock.".into();
    unit.citations.push(caller);
    for citation in &mut unit.citations {
        citation.claim_start_byte = 0;
        citation.claim_end_byte = unit.text.len();
        citation.claim_sha256 = explanation_text_hash(&unit.text);
    }
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        f.realization.clone(),
    )?;
    let privacy = f.ops.privacy_status(f.project)?;
    let answers = f.answers()?;
    let mechanism_bytes: usize = f.originals[7..].iter().map(String::len).sum();
    assert!(mechanism_bytes > 16_384 && mechanism_bytes <= 32_768);
    assert_eq!(
        answers.embedded_source_scope[1].state,
        SourceWindowState::Complete
    );
    assert_eq!(
        answers.embedded_source_scope[1].included_citation_indices,
        vec![0, 1, 2, 3]
    );
    let destination = f.temp.path().join("complete-mechanisms.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &destination)?;
    let offline = fs::read_to_string(destination)?;
    for html in [f.get("/?view=repository")?, offline] {
        let included: usize = answers.source_excerpts.iter().map(|e| e.text.len()).sum();
        assert!(
            html.contains(&format!(
                "Included unique cited text: {included} / {} UTF-8 bytes",
                EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT
            )),
            "Source context must identify the actual aggregate budget"
        );
        let shown = code_texts(&html);
        for original in &f.originals[7..] {
            assert!(
                shown.iter().any(|text| text == original),
                "complete function window missing"
            );
        }
    }
    assert_eq!(privacy, f.ops.privacy_status(f.project)?);
    // More embedded bytes grant no fallback authority after Source deletion.
    let id = &f.realization.paragraphs[1].citations[2].source_id;
    let mut bytes = [0; 16];
    for (n, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&id[n * 2..n * 2 + 2], 16)?;
    }
    f.ops.delete_repository_source_observation(
        f.project,
        f.analysis,
        volicord_context::SourceId::from_bytes(bytes),
    )?;
    let denied = f.answers()?;
    assert_ne!(denied.explanation_state, ExplanationState::Current);
    assert!(denied.source_excerpts.is_empty());
    println!("complete production mechanism windows: {mechanism_bytes} bytes");
    Ok(())
}

#[test]
fn quiet_heading_provenance_preserves_exact_claims_and_duplicate_bindings(
) -> Result<(), Box<dyn Error>> {
    let (f, realization) = priority_fixture()?;
    let before = f.answers()?;
    let canonical = f.ops.canonical_basis(f.project)?;
    let privacy = f.ops.privacy_status(f.project)?;
    let snapshot = ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .render_snapshot(
            &f.request(),
            volicord_context::TimestampMicros::from_unix_micros(123),
        )?;
    let live = f.get("/?view=repository")?;
    for html in [&live, &snapshot.html] {
        let article = html
            .split("<article class=\"doc\"")
            .nth(1)
            .ok_or("article")?
            .split("</article>")
            .next()
            .ok_or("article end")?;
        let (primary, supporting) = article
            .split_once("class=\"reading-section evidence-index\"")
            .ok_or("supporting index")?;
        assert_eq!(primary.matches("class=\"evidence-link\"").count(), 4,
            "three independent ranges plus the separate same-file assertion; no heading/lead actions");
        assert_eq!(
            supporting.matches("class=\"evidence-link\"").count(),
            8,
            "every admitted citation, including identical duplicates, stays inspectable"
        );
        assert!(!html.contains("<script>hostile</script>"));
        assert!(html.contains("Observed &lt;script&gt;hostile&lt;/script&gt; return boundary"));
        let ids = html
            .split(" id=\"")
            .skip(1)
            .filter_map(|s| s.split('"').next())
            .collect::<Vec<_>>();
        assert_eq!(
            ids.len(),
            ids.iter().collect::<std::collections::BTreeSet<_>>().len()
        );
    }
    // The quiet originals, both duplicate indices and the distinct same-file
    // assertion retain the same authority as a prominent technical action.
    for (index, p) in realization.paragraphs.iter().enumerate() {
        for (citation_index, expected) in p.citations.iter().enumerate() {
            let selected = selection(&f, index, citation_index, 0)?;
            let response = body(&f.get(&citation_target(&f, &selected))?)?;
            assert_eq!(
                response["chunk"]["citation"],
                serde_json::to_value(expected)?
            );
            assert_eq!(response["claim"], p.text);
            let chunk = response["chunk"]["text"].as_str().ok_or("chunk")?;
            assert!(f.originals.iter().any(|s| s.starts_with(chunk)));
        }
    }
    let mut after = f.answers()?;
    assert!(after.applicability.checked_at_unix_micros.is_some());
    after.applicability.checked_at_unix_micros = before.applicability.checked_at_unix_micros;
    assert_eq!(
        before, after,
        "presentation must not modify admitted units or proof"
    );
    assert_eq!(canonical, f.ops.canonical_basis(f.project)?);
    assert_eq!(privacy, f.ops.privacy_status(f.project)?);
    // A formerly quiet citation is still a real dependency: deletion withholds
    // the entire article rather than salvaging its title or uncited-looking text.
    let hex = &realization.paragraphs[0].citations[0].source_id;
    let bytes = (0..32)
        .step_by(2)
        .map(|n| u8::from_str_radix(&hex[n..n + 2], 16))
        .collect::<Result<Vec<_>, _>>()?;
    let source = volicord_context::SourceId::from_slice(&bytes)?;
    f.ops
        .delete_repository_source_observation(f.project, f.analysis, source)?;
    assert_ne!(f.answers()?.explanation_state, ExplanationState::Current);
    assert!(!f
        .get(&citation_target(
            &f,
            &ExplanationCitationSelection {
                publication_id: before
                    .provenance
                    .as_ref()
                    .and_then(|p| p.publication_id.clone())
                    .ok_or("publication identity")?,
                realization_fingerprint: before
                    .provenance
                    .ok_or("provenance")?
                    .realization_fingerprint,
                unit_index: 0,
                citation_index: 0,
                offset: 0,
            }
        ))?
        .starts_with("HTTP/1.1 200"));
    Ok(())
}

fn priority_fixture() -> Result<(Fixture, ExplanationRealization), Box<dyn Error>> {
    let f = Fixture::new()?;
    let mut realization = f.realization.clone();
    let unit = |question, text: &str, base: usize| {
        let mut p = f.realization.paragraphs[base].clone();
        p.question = question;
        p.text = text.into();
        for c in &mut p.citations {
            c.claim_start_byte = 0;
            c.claim_end_byte = text.len();
            c.claim_sha256 = explanation_text_hash(text);
        }
        p
    };
    let mut body_unit = f.realization.paragraphs[1].clone();
    body_unit.citations.push(body_unit.citations[0].clone());
    realization.paragraphs = vec![
        unit(
            ExplanationQuestion::Heading,
            "Observed <script>hostile</script> return boundary",
            2,
        ),
        unit(
            ExplanationQuestion::Synopsis,
            "A bounded investigation of the retained functions.",
            2,
        ),
        unit(
            ExplanationQuestion::SectionHeading,
            "검사한 함수 · retained functions",
            2,
        ),
        body_unit,
        f.realization.paragraphs[2].clone(),
    ];
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization.clone(),
    )?;
    Ok((f, realization))
}

#[test]
fn seed_evidence_priority_browser_control() -> Result<(), Box<dyn Error>> {
    if std::env::var_os("VOLICORD_EVIDENCE_PRIORITY_CAPTURE").is_none() {
        return Ok(());
    }
    let (f, realization) = priority_fixture()?;
    if let Some(root) = std::env::var_os("VOLICORD_EVIDENCE_PRIORITY_CAPTURE") {
        fs::create_dir_all(&root)?;
        fs::write(
            std::path::Path::new(&root).join("cases.json"),
            serde_json::to_vec_pretty(&serde_json::json!([{
                "key":"authored-evidence-priority", "runtime":f.ops.layout().root(), "project":f.project.to_string(),
                "language":"en", "locale":"en", "evidence_class":"authored admitted control, not host generation",
                "expected_units":realization.paragraphs, "evidence_priority":true
            }]))?,
        )?;
        let _retained = f.temp.keep();
    }
    Ok(())
}

#[test]
fn important_complete_window_survives_smaller_supporting_windows() -> Result<(), Box<dyn Error>> {
    fn window(full: &str, first: &str, next: &str) -> Result<String, Box<dyn Error>> {
        let start = full.find(first).ok_or("function start")?;
        let end = start + full[start..].find(next).ok_or("function end")?;
        Ok(full[start..end].into())
    }
    let explanation = include_str!("../../volicord-operations/src/explanation.rs");
    let repository = include_str!("../../volicord-operations/src/repository_explanation.rs");
    let mut sources = Vec::new();
    for (name, full, first, next) in [
        (
            "page.rs",
            explanation,
            "    pub fn read_explanation_page(",
            "    fn permitted_read_publication(",
        ),
        (
            "authority.rs",
            explanation,
            "    fn permitted_read_publication(",
            "    pub(crate) fn validate_read_publication(",
        ),
        (
            "publication.rs",
            explanation,
            "    pub(crate) fn validate_read_publication(",
            "    pub fn prepare_explanation(",
        ),
        (
            "record.rs",
            explanation,
            "    pub fn record_explanation(",
            "    pub fn delete_explanations(",
        ),
        (
            "compose.rs",
            repository,
            "    pub fn compose_repository_explanation(",
            "    pub fn repository_explanation_evidence(",
        ),
        (
            "plan.rs",
            repository,
            "    pub(crate) fn repository_explanation_plan(",
            "    pub(crate) fn current_explanation_repository(",
        ),
        (
            "observe.rs",
            repository,
            "    pub fn observe_repository_file(",
            "    pub(crate) fn repository_source_metadata(",
        ),
        (
            "citation.rs",
            explanation,
            "    pub fn read_explanation_citation(",
            "    pub(crate) fn attach_explanations(",
        ),
        (
            "current.rs",
            repository,
            "    pub(crate) fn current_explanation_repository(",
            "    pub fn observe_repository_file(",
        ),
        (
            "metadata.rs",
            repository,
            "    pub(crate) fn repository_source_metadata(",
            "    pub fn read_repository_explanation_source(",
        ),
        (
            "repository_source.rs",
            repository,
            "    pub fn read_repository_explanation_source(",
            "    pub fn delete_repository_source_observation(",
        ),
    ] {
        sources.push((name.into(), window(full, first, next)?));
    }
    // Supporting fixture context grows with the aggregate policy. Keep the
    // original implementation bytes intact, followed by explicitly authored
    // comments, so this still exercises competition rather than an old ceiling.
    let target = (EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT / 10 + 512).min(sources[10].1.len() - 1);
    for (_, source) in &mut sources[..10] {
        while source.len() + 32 <= target {
            source.push_str("// Authored supporting context.\n");
        }
    }
    let important = &sources[10].1;
    let support_size: usize = sources[..10].iter().map(|(_, text)| text.len()).sum();
    assert!(important.len() <= EXPLANATION_SOURCE_WINDOW_BYTE_LIMIT);
    assert!(sources[..10]
        .iter()
        .all(|(_, text)| text.len() < important.len()));
    assert!(support_size > EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT - important.len());
    let capture =
        std::env::var_os("VOLICORD_EXCERPT_COMPETITION_CAPTURE").map(std::path::PathBuf::from);
    if let Some(path) = &capture {
        fs::create_dir(path)?;
    }
    let mut f = Fixture::new_sources(capture.as_deref(), false, Some(sources))?;
    if capture.is_some() {
        f.temp.disable_cleanup(true);
    }
    let important_citation = f.realization.paragraphs[1].citations[2].clone();
    let supports: Vec<_> = f.realization.paragraphs[0]
        .citations
        .iter()
        .cloned()
        .chain(f.realization.paragraphs[1].citations[..2].iter().cloned())
        .collect();
    let unit = |question, text: String, mut citation: ExplanationCitation| {
        citation.claim_start_byte = 0;
        citation.claim_end_byte = text.len();
        citation.claim_sha256 = explanation_text_hash(&text);
        ExplanationParagraph {
            question,
            text,
            evidence_keys: vec!["repository_basis".into()],
            citations: vec![citation],
        }
    };
    for role in [
        ExplanationQuestion::Narrative,
        ExplanationQuestion::ReadingPoint,
        ExplanationQuestion::Heading,
    ] {
        let mut realization = f.realization.clone();
        realization.paragraphs = vec![unit(role,
            "Read the repository Source loader from its lifetime checks through the final observation and fingerprint return.".into(),important_citation.clone())];
        // A second claim shares the exact complete window and consumes no new text budget.
        realization.paragraphs.push(unit(ExplanationQuestion::Observation,
            "The same Source loader exposes the identity and content checks for this reading question.".into(),important_citation.clone()));
        realization
            .paragraphs
            .extend(supports.iter().enumerate().map(|(i, citation)| {
                unit(
                    ExplanationQuestion::Narrative,
                    format!("Supporting implementation {}.", i + 1),
                    citation.clone(),
                )
            }));
        let receipt = f.ops.record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            realization.clone(),
        )?;
        f.realization = realization;
        let answers = f.answers()?;
        let selected = selection(&f, 0, 0, 0)?;
        let mut live = Vec::new();
        let mut offset = 0;
        loop {
            let selected = ExplanationCitationSelection {
                offset,
                ..selected.clone()
            };
            let page = body(&f.get(&citation_target(&f, &selected))?)?;
            live.extend_from_slice(
                page["chunk"]["text"]
                    .as_str()
                    .ok_or("Source text")?
                    .as_bytes(),
            );
            let Some(next) = page["chunk"]["next_offset"].as_u64() else {
                break;
            };
            offset = next as usize;
        }
        assert_eq!(live, f.originals[10].as_bytes());
        let destination = f.temp.path().join(format!("competition-{role:?}.html"));
        ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
            .export_snapshot(&f.request(), &destination)?;
        let offline = fs::read_to_string(&destination)?;
        let live_html = f.get("/?view=repository")?;
        if let Some(path) = &capture {
            fs::write(
                path.join(format!("{role:?}-receipt.json")),
                serde_json::to_vec_pretty(&serde_json::json!({"explanation":receipt}))?,
            )?;
            fs::write(
                path.join(format!("{role:?}-realization.json")),
                serde_json::to_vec_pretty(&f.realization)?,
            )?;
            fs::write(
                path.join(format!("{role:?}-answers.json")),
                serde_json::to_vec_pretty(&answers)?,
            )?;
            fs::write(path.join(format!("{role:?}-live.http")), &live_html)?;
            fs::write(
                path.join(format!("fixture-{role:?}.json")),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "runtime":f.ops.layout().root(),"project":f.project.to_string(),"analysis":f.analysis.to_string(),
                    "repository":f.temp.path().join("repo"),"critical_source":f.originals[10],
                    "critical_citation":f.realization.paragraphs[0].citations[0],"offline":destination,"role":format!("{role:?}"),
                    "original_realization":path.join(format!("{role:?}-realization.json")),
                    "recording_receipt":path.join(format!("{role:?}-receipt.json")),
                    "public":path.join(format!("{role:?}-answers.json")),
                    "reader_question":"Where does the loader compare the retained copy with its canonical File binding and compute the fingerprint returned with the observation?",
                    "provenance":"authored contention control with exact current implementation windows; no human or model verdict"
                }))?,
            )?;
        }
        let included: usize = answers.source_excerpts.iter().map(|e| e.text.len()).sum();
        assert!(included <= EXPLANATION_SOURCE_EXCERPT_BYTE_LIMIT);
        assert!(answers
            .source_excerpts
            .iter()
            .all(|e| e.omitted_utf8_bytes == 0));
        for html in [live_html, offline] {
            assert!(code_texts(&html).iter().any(|s|s==&f.originals[10]),
                "the reader cannot inspect the complete Source loader while smaller supporting windows compete ({role:?})");
        }
        assert_eq!(
            answers
                .source_excerpts
                .iter()
                .filter(|e| e.citation.source_id == important_citation.source_id)
                .count(),
            1
        );
        assert_eq!(
            answers.embedded_source_scope[0].state,
            SourceWindowState::Complete
        );
        assert_eq!(
            answers.embedded_source_scope[1].state,
            SourceWindowState::Complete
        );
        for scope in &answers.embedded_source_scope[2..] {
            let citation = &f.realization.paragraphs[scope.unit_index].citations[0];
            let included = answers
                .source_excerpts
                .iter()
                .any(|e| e.citation.source_id == citation.source_id);
            assert_eq!(
                scope.state,
                if included {
                    SourceWindowState::Complete
                } else {
                    SourceWindowState::InitialOmission
                }
            );
            assert_eq!(
                scope.included_citation_indices,
                if included { vec![0] } else { vec![] }
            );
            assert_eq!(
                scope.omitted_citation_indices,
                if included { vec![] } else { vec![0] }
            );
        }
        for scope in answers.source_reading_scope(true) {
            assert_eq!(
                scope.state,
                if scope.omitted_citation_indices.is_empty() {
                    SourceWindowState::Complete
                } else {
                    SourceWindowState::OfflineOmission
                }
            );
        }
        assert_eq!(
            answers.omitted_source_excerpt_count,
            11 - answers.source_excerpts.len()
        );
        assert!(answers.omitted_source_excerpt_count > 0);
        println!("competition {role:?}: important={} support={support_size} included={included} omitted={}",f.originals[10].len(),answers.omitted_source_excerpt_count);
    }
    Ok(())
}

fn diagram_control(f: &Fixture, sequence: bool) -> Result<ExplanationRealization, Box<dyn Error>> {
    // Independent fixture source is admit(value) -> verify(value), plus a separate
    // accepted-state consumer. This exercises structure, not tab-close semantics.
    let label = |needle: &str| -> Result<DiagramLabel, Box<dyn Error>> {
        let unit = 1;
        let start_byte = f.realization.paragraphs[unit]
            .text
            .find(needle)
            .ok_or("label")?;
        Ok(DiagramLabel {
            unit,
            start_byte,
            end_byte: start_byte + needle.len(),
        })
    };
    let mut d = GroundedDiagram {
        id: "mechanism".into(),
        title: label("Admission calls verification")?,
        caption_unit: 1,
        semantic_fingerprint: String::new(),
        meaning: DiagramMeaning::Concept {
            nodes: vec![],
            relations: vec![],
        },
    };
    let nodes = vec![
        DiagramNode {
            id: "admit".into(),
            kind: if sequence {
                DiagramNodeKind::Participant
            } else {
                DiagramNodeKind::Component
            },
            label: label("Admission")?,
        },
        DiagramNode {
            id: "verify".into(),
            kind: if sequence {
                DiagramNodeKind::Participant
            } else {
                DiagramNodeKind::Component
            },
            label: label("verification")?,
        },
    ];
    d.meaning = if sequence {
        DiagramMeaning::Sequence {
            participants: nodes,
            groups: vec![
                DiagramGroup {
                    order: DiagramOrder::Sequential,
                    condition: None,
                    messages: vec![DiagramMessage {
                        id: "invoke".into(),
                        from: "admit".into(),
                        to: "verify".into(),
                        kind: DiagramMessageKind::Call,
                        label: label("calls verification")?,
                    }],
                },
                DiagramGroup {
                    order: DiagramOrder::Parallel,
                    condition: Some(label("acceptance")?),
                    messages: vec![DiagramMessage {
                        id: "answer".into(),
                        from: "verify".into(),
                        to: "admit".into(),
                        kind: DiagramMessageKind::Return,
                        label: label("retained result is conditional on acceptance")?,
                    }],
                },
            ],
        }
    } else {
        DiagramMeaning::Concept {
            nodes,
            relations: vec![DiagramRelation {
                id: "dependency".into(),
                from: "admit".into(),
                to: "verify".into(),
                kind: DiagramRelationKind::DependsOn,
                label: label("calls verification")?,
                condition: Some(label("acceptance")?),
            }],
        }
    };
    let plan =
        f.ops
            .prepare_explanation(f.project, ExplanationSubject::Repository(f.analysis), "en")?;
    let paragraphs = f
        .realization
        .paragraphs
        .iter()
        .map(|p| -> Result<_, Box<dyn Error>> {
            Ok(RepositoryReaderUnit {
                question: p.question,
                text: p.text.clone(),
                evidence_keys: p.evidence_keys.clone(),
                sources: p
                    .citations
                    .iter()
                    .map(|c| -> Result<_, Box<dyn Error>> {
                        let bytes = (0..16)
                            .map(|i| u8::from_str_radix(&c.source_id[i * 2..i * 2 + 2], 16))
                            .collect::<Result<Vec<_>, _>>()?;
                        Ok(RepositoryClaimRange {
                            source_id: volicord_context::SourceId::from_slice(&bytes)?,
                            start_byte: c.start_byte,
                            end_byte: c.end_byte,
                            claim: Some(FileByteRange {
                                start_byte: c.claim_start_byte,
                                end_byte: c.claim_end_byte,
                            }),
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(f.ops.compose_repository_explanation(
        f.project,
        f.analysis,
        RepositoryExplanationDraft {
            diagrams: vec![d],
            plan_fingerprint: plan.fingerprint,
            language: "en".into(),
            generator: f.realization.generator.clone(),
            paragraphs,
        },
    )?)
}

#[test]
fn admitted_diagrams_roundtrip_public_read_svg_text_exact_source_and_loss(
) -> Result<(), Box<dyn Error>> {
    for sequence in [false, true] {
        let f = Fixture::new()?;
        let r = diagram_control(&f, sequence)?;
        let subject = ExplanationSubject::Repository(f.analysis);
        let receipt = f
            .ops
            .record_explanation(f.project, subject, "en", r.clone())?;
        let a = f.answers()?;
        assert_eq!(a.diagrams, r.diagrams);
        let texts = a.prose.iter().map(|u| u.text.as_str()).collect::<Vec<_>>();
        let alternative = diagram_text(&r.diagrams[0], &texts)?;
        assert!(alternative.contains("Admission → verification"));
        if sequence {
            assert!(alternative.contains("parallel (no relative order asserted) [if acceptance]"));
        } else {
            assert!(alternative.contains("depends on: calls verification [if acceptance]"));
        }
        assert!(a.text().contains(&alternative));
        let public = compact_publication(&receipt, "fixture-publication");
        assert_eq!(public["diagrams"], serde_json::to_value(&r.diagrams)?);
        let live = f.get("/?view=repository")?;
        let destination = f.temp.path().join("diagrams.html");
        ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
            .export_snapshot(&f.request(), &destination)?;
        let offline = fs::read_to_string(&destination)?;
        let publication = a
            .provenance
            .as_ref()
            .and_then(|p| p.publication_id.as_deref())
            .ok_or("managed publication")?;
        for html in [&live, &offline] {
            assert!(html.contains("data-renderer=\"volicord-static-svg/8\""));
            assert!(html.contains("data-semantic-fingerprint"));
            assert!(html.contains("diagram-alternative"));
            assert!(html.contains("calls verification"));
            assert!(!html.contains("<foreignObject"));
            // One failed citation invalidates every dependent region, including
            // closed evidence and other Source windows. The browser must have
            // exact managed identity for all these independently rendered copies.
            for marker in [
                "<figure class=\"admitted-diagram\"",
                "<section class=\"source-point\"",
                "<details class=\"reading-section evidence-index\"",
                "<details class=\"document-nav\"",
            ] {
                let regions = html.split(marker).skip(1).collect::<Vec<_>>();
                assert!(!regions.is_empty(), "rendered consumer region {marker}");
                for region in regions {
                    assert!(region
                        .split('>')
                        .next()
                        .ok_or("opening tag")?
                        .contains(&format!("data-reader-publication=\"{publication}\"")));
                }
            }
        }
        assert!(!offline.contains("<script"));
        for href in offline
            .split("href=\"#")
            .skip(1)
            .map(|s| s.split('"').next().unwrap_or(""))
        {
            assert_eq!(
                offline.matches(&format!(" id=\"{href}\"")).count(),
                1,
                "exact script-free return target {href}"
            );
        }
        assert!(offline.contains("class=\"diagram-claim\""));
        assert!(offline.contains("diagram-return-index"));
        assert!(offline.contains("class=\"diagram-controls\" hidden"));
        assert!(offline.contains("Inspect calls verification"));
        assert!(offline.contains(if sequence {
            "guard:2-claim"
        } else {
            "dependency:condition-claim"
        }));
        let p = a.provenance.ok_or("provenance")?;
        let selected = f.ops.read_explanation_citation(
            f.project,
            subject,
            "en",
            &ExplanationCitationSelection {
                publication_id: p.publication_id.ok_or("publication identity")?,
                realization_fingerprint: p.realization_fingerprint,
                unit_index: 1,
                citation_index: 0,
                offset: 0,
            },
        )?;
        assert_eq!(selected["chunk"]["text"], f.originals[8]);
        let id = &r.paragraphs[1].citations[0].source_id;
        let bytes = (0..16)
            .map(|i| u8::from_str_radix(&id[i * 2..i * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()?;
        f.ops.delete_repository_source_observation(
            f.project,
            f.analysis,
            volicord_context::SourceId::from_slice(&bytes)?,
        )?;
        assert!(f.answers()?.diagrams.is_empty());
        assert!(!f
            .get("/?view=repository")?
            .contains("data-renderer=\"volicord-static-svg/8\""));
    }
    Ok(())
}

#[test]
fn diagram_semantics_and_nested_bindings_cannot_bypass_admission() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let subject = ExplanationSubject::Repository(f.analysis);
    let plan = f.ops.prepare_explanation(f.project, subject, "en")?;
    let r = diagram_control(&f, false)?;
    let sequence = diagram_control(&f, true)?;
    f.ops
        .record_explanation(f.project, subject, "en", r.clone())?;
    let before = f.ops.privacy_status(f.project)?;
    for case in 0..14 {
        let mut altered = if case >= 10 {
            sequence.clone()
        } else {
            r.clone()
        };
        match case {
            0 => {
                if let DiagramMeaning::Concept { relations, .. } = &mut altered.diagrams[0].meaning
                {
                    relations[0].from = "verify".into();
                    relations[0].to = "admit".into();
                }
            }
            1 => {
                if let DiagramMeaning::Concept { relations, .. } = &mut altered.diagrams[0].meaning
                {
                    relations[0].to = "missing".into();
                }
            }
            2 => {
                if let DiagramMeaning::Concept { nodes, .. } = &mut altered.diagrams[0].meaning {
                    nodes.pop();
                }
            }
            3 => {
                if let DiagramMeaning::Concept { relations, .. } = &mut altered.diagrams[0].meaning
                {
                    relations[0].condition = None;
                }
            }
            4 => altered.diagrams[0].title.unit = 31,
            5 => altered.paragraphs[1].citations.clear(),
            6 => altered.language = "ko".into(),
            7 => altered.plan_fingerprint = "foreign".into(),
            8 => altered.format_version = EXPLANATION_VERSION - 1,
            9 => altered.format_version = EXPLANATION_VERSION + 1,
            10 => {
                if let DiagramMeaning::Sequence { groups, .. } = &mut altered.diagrams[0].meaning {
                    groups.swap(0, 1);
                }
            }
            11 => {
                if let DiagramMeaning::Sequence { groups, .. } = &mut altered.diagrams[0].meaning {
                    groups[1].order = DiagramOrder::Sequential;
                }
            }
            12 => {
                if let DiagramMeaning::Sequence { groups, .. } = &mut altered.diagrams[0].meaning {
                    groups[1].condition.as_mut().ok_or("condition")?.end_byte = usize::MAX;
                }
            }
            _ => {
                if let DiagramMeaning::Sequence { groups, .. } = &mut altered.diagrams[0].meaning {
                    groups[0].messages[0].to = "missing".into();
                }
            }
        }
        assert!(
            validate_explanation(&plan, &altered).is_err(),
            "case {case}"
        );
        assert!(
            f.ops
                .record_explanation(f.project, subject, "en", altered.clone())
                .is_err(),
            "case {case}"
        );
        assert_eq!(
            f.ops.privacy_status(f.project)?,
            before,
            "failed publication mutated state {case}"
        );
        if ![4, 5, 8, 9, 12].contains(&case) {
            assert_ne!(
                altered.diagrams[0].fingerprint(&altered)?,
                altered.diagrams[0].semantic_fingerprint,
                "semantic fingerprint {case}"
            );
        }
    }
    let mut json = serde_json::to_value(&r)?;
    json["diagrams"][0]["meaning"]["relations"][0]["unguarded_alt_text"] =
        serde_json::json!("<script>unbound</script>");
    assert!(serde_json::from_value::<ExplanationRealization>(json).is_err());
    assert!(validate_diagrams(
        ExplanationSubject::WorkName(volicord_context::ContextItemId::from_bytes([1; 16])),
        &r
    )
    .is_err());
    let mut ungrounded = r.clone();
    ungrounded.paragraphs[1].citations[0].claim_end_byte = 9;
    ungrounded.paragraphs[1].citations.truncate(1);
    seal_diagrams(&mut ungrounded)?;
    assert!(validate_diagrams(subject, &ungrounded).is_err());
    Ok(())
}

#[test]
fn diagram_component_flow_and_owned_state_survive_public_consumers() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let subject = ExplanationSubject::Repository(f.analysis);
    for shape in ["component", "flowchart", "state"] {
        let mut r = diagram_control(&f, false)?;
        let DiagramMeaning::Concept {
            mut nodes,
            mut relations,
        } = r.diagrams[0].meaning.clone()
        else {
            return Err("fixture concept".into());
        };
        let owner = nodes[0].label.clone();
        for n in &mut nodes {
            n.kind = match shape {
                "component" => DiagramNodeKind::Component,
                "flowchart" => DiagramNodeKind::Process,
                _ => DiagramNodeKind::State,
            };
        }
        relations[0].kind = match shape {
            "component" => DiagramRelationKind::ControlFlow,
            "flowchart" => DiagramRelationKind::ControlFlow,
            _ => DiagramRelationKind::Transition,
        };
        r.diagrams[0].meaning = match shape {
            "component" => DiagramMeaning::Component { nodes, relations },
            "flowchart" => DiagramMeaning::Flowchart { nodes, relations },
            _ => DiagramMeaning::State {
                owner,
                nodes,
                relations,
            },
        };
        seal_diagrams(&mut r)?;
        f.ops
            .record_explanation(f.project, subject, "en", r.clone())?;
        let a = f.answers()?;
        assert_eq!(a.diagrams, r.diagrams);
        let html = f.get("/?view=repository")?;
        let destination = f.temp.path().join(format!("{shape}.html"));
        ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
            .export_snapshot(&f.request(), &destination)?;
        let offline = fs::read_to_string(destination)?;
        for text in [&html, &offline] {
            assert!(text.contains("volicord-static-svg/8"));
            assert!(text.contains(if shape == "state" {
                "State owner: Admission"
            } else {
                "control flow"
            }));
            assert!(text.contains("calls verification"));
        }
        let mut bad = r.clone();
        match &mut bad.diagrams[0].meaning {
            DiagramMeaning::Component { relations, .. }
            | DiagramMeaning::Flowchart { relations, .. }
            | DiagramMeaning::State { relations, .. } => {
                relations[0].kind = DiagramRelationKind::RoleOf
            }
            _ => return Err("shape".into()),
        }
        seal_diagrams(&mut bad)?; // A valid seal cannot authorize the wrong meaning.
        assert!(f
            .ops
            .record_explanation(f.project, subject, "en", bad)
            .is_err());
        if shape == "state" {
            let mut bad = r.clone();
            if let DiagramMeaning::State { owner, .. } = &mut bad.diagrams[0].meaning {
                owner.unit = 31;
            }
            assert!(seal_diagrams(&mut bad).is_err());
        }
        for invalid_node in [DiagramNodeKind::Initial, DiagramNodeKind::Branch] {
            let mut bad = r.clone();
            match &mut bad.diagrams[0].meaning {
                DiagramMeaning::Component { nodes, .. }
                | DiagramMeaning::Flowchart { nodes, .. }
                | DiagramMeaning::State { nodes, .. } => nodes[1].kind = invalid_node,
                _ => return Err("shape".into()),
            }
            seal_diagrams(&mut bad)?;
            assert!(f
                .ops
                .record_explanation(f.project, subject, "en", bad)
                .is_err());
        }
        let mut changed = r.clone();
        match &mut changed.diagrams[0].meaning {
            DiagramMeaning::Component { relations, .. }
            | DiagramMeaning::Flowchart { relations, .. }
            | DiagramMeaning::State { relations, .. } => relations[0].condition = None,
            _ => return Err("shape".into()),
        }
        assert!(f
            .ops
            .record_explanation(f.project, subject, "en", changed)
            .is_err());
    }
    Ok(())
}

#[test]
fn shared_diagram_caption_preserves_separate_elements_and_exact_origins(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let mut realization = diagram_control(&f, false)?;
    let mut second = realization.diagrams[0].clone();
    second.id = "independent-detail".into();
    realization.diagrams.push(second);
    seal_diagrams(&mut realization)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization,
    )?;
    let html = f.get("/?view=repository")?;
    assert_eq!(html.matches("class=\"unit diagram-caption\"").count(), 1);
    assert_eq!(html.matches("class=\"shared-diagram-basis\"").count(), 1);
    assert_eq!(html.matches("class=\"admitted-diagram\"").count(), 2);
    for href in html
        .split("href=\"#")
        .skip(1)
        .map(|s| s.split('"').next().unwrap_or(""))
    {
        assert_eq!(
            html.matches(&format!(" id=\"{href}\"")).count(),
            1,
            "exact shared-caption origin {href}"
        );
    }
    assert!(html.contains("-diagram-mechanism-dependency-claim"));
    assert!(html.contains("-diagram-independent-detail-dependency-claim"));
    Ok(())
}

#[test]
fn complete_permission_windows_fit_without_changing_source_or_transport_bounds(
) -> Result<(), Box<dyn Error>> {
    // Eleven independent 4 KiB windows: too large for the prior 32 KiB total.
    // Literal fixture inputs are the oracle for every byte in the real export.
    let sources = (0..11).map(|i| {
        let mut body = format!("def permission_{i}(allowed):\n    if not allowed:\n        return None\n    return allowed\n");
        while body.len() < 4_000 { body.push_str("# retained permission context\n"); }
        (format!("permission_{i}.py"), body)
    }).collect();
    let f = Fixture::new_sources(None, false, Some(sources))?;
    let answers = f.answers()?;
    let bytes: usize = f.originals.iter().map(String::len).sum();
    assert!(bytes > 32_768 && bytes < 49_152);
    assert_eq!(answers.omitted_source_excerpt_count, 0);
    assert!(answers
        .source_reading_scope(true)
        .iter()
        .all(|s| s.state == SourceWindowState::Complete));
    let output = f.temp.path().join("permission.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &output)?;
    let html = fs::read_to_string(output)?;
    let code = code_texts(&html);
    assert!(!html.contains("<script"));
    for original in &f.originals {
        assert!(code.contains(original), "complete implementing window");
    }
    Ok(())
}

#[test]
fn same_named_nodes_retain_distinct_nonvisual_endpoints_in_actual_publication(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let mut realization = diagram_control(&f, true)?;
    if let DiagramMeaning::Sequence { participants, .. } = &mut realization.diagrams[0].meaning {
        participants[1].label = participants[0].label.clone();
    }
    seal_diagrams(&mut realization)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization,
    )?;
    let text = f.answers()?.text();
    assert!(text.contains("Admission [participant; node=admit]"));
    assert!(text.contains("Admission [participant; node=verify]"));
    assert!(text.contains("[message=invoke; from=admit; to=verify]"));
    assert!(text.contains("[message=answer; from=verify; to=admit]"));
    assert!(text.contains("parallel (no relative order asserted) [if acceptance]"));
    Ok(())
}

#[test]
fn selected_connections_keep_endpoints_guards_and_sequence_context_in_live_and_offline_reads(
) -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    for sequence in [false, true] {
        let realization = diagram_control(&f, sequence)?;
        f.ops.record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            realization,
        )?;
        let before = f.answers()?;
        let live = f.get("/?view=repository")?;
        let destination = f.temp.path().join(format!("connection-{sequence}.html"));
        ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
            .export_snapshot(&f.request(), &destination)?;
        let offline = fs::read_to_string(destination)?;
        for html in [&live, &offline] {
            assert!(!html.contains("data-initial-connection"));
            assert!(html.contains("<div class=\"diagram-full-view\">"));
            assert!(!html.contains("<details class=\"diagram-full-view\""));
            let caption = html
                .find("class=\"unit diagram-caption\"")
                .ok_or("caption")?;
            let map = html.find("class=\"diagram-full-view\"").ok_or("map")?;
            let controls = html.find("class=\"diagram-controls\"").ok_or("controls")?;
            let detail = html.find("class=\"diagram-claims\"").ok_or("detail")?;
            assert!(
                caption < map && map < controls && controls < detail,
                "native reading order precedes detail"
            );
            assert!(html.contains("connection-navigation"));
            let id = if sequence { "answer" } else { "dependency" };
            let detail = html
                .split(&format!("data-connection=\"{id}\""))
                .nth(1)
                .ok_or("missing selected connection")?
                .split("</div><div class=\"diagram-claim-context\">")
                .next()
                .ok_or("detail")?;
            assert!(detail.contains("connection-endpoint diagram-text-link"));
            assert!(detail.contains("-admit-claim"));
            assert!(detail.contains("-verify-claim"));
            assert!(detail.contains("acceptance"));
            if sequence {
                assert!(detail.contains("2.1 · parallel · no relative order"));
                assert!(detail.find("-verify-claim") < detail.find("-admit-claim"));
            } else {
                assert!(detail.find("-admit-claim") < detail.find("-verify-claim"));
            }
        }
        assert_eq!(before.diagrams, f.answers()?.diagrams);
        assert_eq!(before.provenance, f.answers()?.provenance);
    }
    Ok(())
}

#[test]
fn supporting_omissions_stay_at_the_affected_diagram_claim() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let mut realization = diagram_control(&f, false)?;
    // The first unit has eight large, genuinely retained inventory Sources.
    // Only some fit. Selecting its grounded wording must disclose that locally.
    let label = DiagramLabel {
        unit: 0,
        start_byte: 0,
        end_byte: 14,
    };
    let d = &mut realization.diagrams[0];
    d.caption_unit = 0;
    d.title = label.clone();
    if let DiagramMeaning::Concept { nodes, relations } = &mut d.meaning {
        for n in nodes {
            n.label = label.clone();
        }
        relations[0].label = label;
        relations[0].condition = None;
    }
    seal_diagrams(&mut realization)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization,
    )?;
    assert!(f.answers()?.omitted_source_excerpt_count > 0);
    let html = f.get("/?view=repository")?;
    let claim = html
        .split("-diagram-mechanism-dependency-claim\" tabindex")
        .nth(1)
        .ok_or("exact claim")?
        .split("</section>")
        .next()
        .ok_or("claim close")?;
    assert!(claim.contains("class=\"source-window-omission\""));
    assert!(claim.contains("unavailable offline"));
    Ok(())
}

#[test]
fn adversarial_labels_reach_the_packaged_renderer_only_as_inert_text() -> Result<(), Box<dyn Error>>
{
    let f = Fixture::new()?;
    let mut realization = diagram_control(&f, false)?;
    let hostile = "</text><image href='https://invalid.example/x' onload='alert(1)'/>한&";
    let unit = &mut realization.paragraphs[1];
    unit.text = hostile.into();
    for citation in &mut unit.citations {
        citation.claim_start_byte = 0;
        citation.claim_end_byte = hostile.len();
        citation.claim_sha256 = explanation_text_hash(hostile);
    }
    let label = DiagramLabel {
        unit: 1,
        start_byte: 0,
        end_byte: hostile.len(),
    };
    let d = &mut realization.diagrams[0];
    d.title = label.clone();
    if let DiagramMeaning::Concept { nodes, relations } = &mut d.meaning {
        for n in nodes {
            n.label = label.clone();
        }
        for r in relations {
            r.label = label.clone();
            r.condition = Some(label.clone());
        }
    }
    seal_diagrams(&mut realization)?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        realization.clone(),
    )?;
    let output = f.temp.path().join("hostile.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &output)?;
    let html = fs::read_to_string(output)?;
    assert!(html.contains("data-renderer=\"volicord-static-svg/8\""));
    assert!(html.contains("&lt;image"));
    assert!(!html.contains("<image"));
    assert!(!html.contains("<script"));
    assert!(!html.contains("<foreignObject"));
    let mut malformed = realization;
    malformed.diagrams[0].title.end_byte += 1;
    assert!(seal_diagrams(&mut malformed).is_ok());
    assert!(f
        .ops
        .record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            malformed
        )
        .is_err());
    assert_eq!(
        f.answers()?.diagrams.len(),
        1,
        "failed input preserves the original publication"
    );
    if let Some(root) = std::env::var_os("VOLICORD_RENDERER_SAFETY_CAPTURE") {
        fs::create_dir(&root)?;
        fs::write(std::path::Path::new(&root).join("hostile.html"), html)?;
    }
    Ok(())
}

#[test]
fn identical_wording_republication_cannot_rebind_a_selected_citation() -> Result<(), Box<dyn Error>>
{
    let f = Fixture::new()?;
    let selected = selection(&f, 1, 0, 0)?;
    let previous = f.answers()?.provenance.ok_or("original provenance")?;
    f.ops.record_explanation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        f.realization.clone(),
    )?;
    let current = f.answers()?.provenance.ok_or("current provenance")?;
    assert_eq!(
        previous.realization_fingerprint, current.realization_fingerprint,
        "wording and bindings are deliberately identical"
    );
    assert!(
        f.ops
            .read_explanation_citation(
                f.project,
                ExplanationSubject::Repository(f.analysis),
                "en",
                &selected
            )
            .is_err(),
        "a selected older publication must not silently bind to its replacement"
    );
    Ok(())
}

#[test]
fn repeated_code_payload_keeps_distinct_claims_and_native_returns() -> Result<(), Box<dyn Error>> {
    let f = Fixture::new()?;
    let output = f.temp.path().join("shared-code.html");
    ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()))
        .export_snapshot(&f.request(), &output)?;
    let html = fs::read_to_string(output)?;
    let codes = code_texts(&html);
    assert_eq!(
        codes.iter().filter(|c| *c == &f.originals[8]).count(),
        1,
        "one physical complete Source payload"
    );
    assert!(html.contains("class=\"source-code-reference\""));
    assert!(html.contains("class=\"shared-source-return\""));
    for section in html
        .split("<section class=\"shared-source-target\"")
        .skip(1)
    {
        let id = section
            .split_once("id=\"")
            .ok_or("alias ID")?
            .1
            .split('"')
            .next()
            .ok_or("alias")?;
        let point = id.strip_suffix("-shared-code").ok_or("native alias")?;
        assert!(section
            .split("</section>")
            .next()
            .ok_or("alias close")?
            .contains(&format!("href=\"#{point}\"")));
        assert_eq!(
            html.matches(&format!(" id=\"{point}\"")).count(),
            1,
            "original claim identity survives sharing"
        );
    }
    assert!(html.contains(&f.realization.paragraphs[2].text));
    assert!(!html.contains("<script"));
    Ok(())
}

#[test]
fn diagram_ui_locale_preserves_authored_english_claims_and_publication_identity(
) -> Result<(), Box<dyn Error>> {
    for sequence in [false, true] {
        let f = Fixture::new()?;
        let realized = diagram_control(&f, sequence)?;
        f.ops.record_explanation(
            f.project,
            ExplanationSubject::Repository(f.analysis),
            "en",
            realized.clone(),
        )?;
        let before = f.answers()?;
        let adapter = ViewerAdapter::new(LocalOperations::new(f.ops.layout().clone()));
        for locale in [ViewerLocale::English, ViewerLocale::Korean] {
            let mut request = f.request();
            request.locale = locale;
            for page in [
                adapter.render(&request, "fixture")?,
                adapter.render_snapshot(
                    &request,
                    volicord_context::TimestampMicros::from_unix_micros(123),
                )?,
            ] {
                let figure = page
                    .html
                    .split("<svg ")
                    .nth(1)
                    .ok_or("actual SVG")?
                    .split("</svg>")
                    .next()
                    .ok_or("SVG close")?;
                assert!(
                    figure.contains("calls verification"),
                    "authored English relation is visibly retained"
                );
                assert!(figure.contains("Admission"));
                if !sequence {
                    let common = if locale == ViewerLocale::Korean {
                        "모든 노드: 구성요소"
                    } else {
                        "All nodes: component"
                    };
                    assert_eq!(
                        figure.matches(common).count(),
                        2,
                        "one type key per responsive geometry"
                    );
                    assert!(figure.contains("Admission → verification"));
                    assert!(figure.contains("acceptance"));
                }
                if locale == ViewerLocale::Korean {
                    assert!(figure.contains(if sequence { "호출" } else { "의존" }));
                    assert!(!figure.contains(if sequence { ">call<" } else { ">depends on<" }));
                    assert!(figure.contains("조건:"));
                }
            }
        }
        let after = f.answers()?;
        assert_eq!(after.provenance, before.provenance);
        assert_eq!(after.diagrams, realized.diagrams);
        assert_eq!(after.prose, before.prose);
    }
    Ok(())
}
