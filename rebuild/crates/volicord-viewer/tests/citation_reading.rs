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
    assert!(total <= 16_384);
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
    unit.text = "한".repeat(8500);
    unit.citations.truncate(1);
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
    let original = f.ops.read_explanation_citation(
        f.project,
        ExplanationSubject::Repository(f.analysis),
        "en",
        &s,
    )?;
    let response = body(&f.get(&citation_target(&f, &s))?)?;
    assert!(
        response["highlight"].is_null(),
        "decoration must yield instead of widening the bound"
    );
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
    let custom = std::env::var_os("VOLICORD_HIGHLIGHT_REVIEW_ROOT");
    let Some(output) = custom
        .clone()
        .or_else(|| std::env::var_os("VOLICORD_CITATION_REVIEW_ROOT"))
    else {
        return Ok(());
    };
    let output = std::path::Path::new(&output);
    fs::create_dir_all(output)?;
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
        .contains("self.validate_subject_sources_selected("));
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
            <= 16384
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
        .any(|s| s.contains("self.validate_subject_sources_selected(")));
    assert!(code_texts(&html)
        .iter()
        .any(|s| s.contains("bytes.len() > 32_768")));
    assert!(!html.contains("<script"));
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
    assert_eq!(
        before,
        f.answers()?,
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
