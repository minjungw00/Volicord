//! Exact-current, lossless explanation wire representation. Domain consumers use
//! complete citations; repeated provenance is shared only during serialization.
use crate::{
    ExplanationCitation, ExplanationGenerator, ExplanationParagraph, ExplanationQuestion,
    ExplanationRealization, ExplanationSourceUse, GroundedDiagram, EXPLANATION_BYTE_LIMIT,
    EXPLANATION_KIND, EXPLANATION_VERSION,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct CitationContext {
    analysis_snapshot_id: String,
    repository_snapshot_id: String,
    repository_source_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
struct CitationSource {
    context: usize,
    source_id: String,
    source_revision: u64,
    observation_fingerprint: String,
    locator: String,
    field: String,
    use_kind: ExplanationSourceUse,
}

// Source index, evidence bytes, evidence lines, range hash, claim bytes/hash.
// Fixed tuple length/types are checked by serde before reference expansion.
type Citation = (
    usize,
    usize,
    usize,
    usize,
    usize,
    String,
    usize,
    usize,
    String,
);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Paragraph {
    question: ExplanationQuestion,
    text: String,
    evidence_keys: Vec<String>,
    citations: Vec<Citation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    format_kind: String,
    format_version: u32,
    plan_fingerprint: String,
    language: String,
    generator: ExplanationGenerator,
    paragraphs: Vec<Paragraph>,
    diagrams: Vec<GroundedDiagram>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    citation_contexts: Vec<CitationContext>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    citation_sources: Vec<CitationSource>,
}

fn intern<T: Clone + Ord>(value: T, entries: &mut Vec<T>, seen: &mut BTreeMap<T, usize>) -> usize {
    if let Some(index) = seen.get(&value) {
        return *index;
    }
    let index = entries.len();
    entries.push(value.clone());
    seen.insert(value, index);
    index
}

impl From<&ExplanationRealization> for Wire {
    fn from(value: &ExplanationRealization) -> Self {
        let mut citation_contexts = Vec::new();
        let mut citation_sources = Vec::new();
        let mut contexts = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let paragraphs = value
            .paragraphs
            .iter()
            .map(|paragraph| {
                let citations = paragraph
                    .citations
                    .iter()
                    .map(|citation| {
                        let context = intern(
                            CitationContext {
                                analysis_snapshot_id: citation.analysis_snapshot_id.clone(),
                                repository_snapshot_id: citation.repository_snapshot_id.clone(),
                                repository_source_id: citation.repository_source_id.clone(),
                            },
                            &mut citation_contexts,
                            &mut contexts,
                        );
                        let source = intern(
                            CitationSource {
                                context,
                                source_id: citation.source_id.clone(),
                                source_revision: citation.source_revision,
                                observation_fingerprint: citation.observation_fingerprint.clone(),
                                locator: citation.locator.clone(),
                                field: citation.field.clone(),
                                use_kind: citation.use_kind,
                            },
                            &mut citation_sources,
                            &mut sources,
                        );
                        (
                            source,
                            citation.start_byte,
                            citation.end_byte,
                            citation.start_line,
                            citation.end_line,
                            citation.range_sha256.clone(),
                            citation.claim_start_byte,
                            citation.claim_end_byte,
                            citation.claim_sha256.clone(),
                        )
                    })
                    .collect();
                Paragraph {
                    question: paragraph.question,
                    text: paragraph.text.clone(),
                    evidence_keys: paragraph.evidence_keys.clone(),
                    citations,
                }
            })
            .collect();
        Self {
            format_kind: value.format_kind.clone(),
            format_version: value.format_version,
            plan_fingerprint: value.plan_fingerprint.clone(),
            language: value.language.clone(),
            generator: value.generator.clone(),
            paragraphs,
            diagrams: value.diagrams.clone(),
            citation_contexts,
            citation_sources,
        }
    }
}

impl Wire {
    fn expand(&self) -> Result<ExplanationRealization, String> {
        // Same producer bounds, before cloning shared identity strings. This
        // bounds logical alias expansion independently of compact input length.
        if self.paragraphs.len() > 32 || self.paragraphs.iter().any(|p| p.citations.len() > 32) {
            return Err("explanation exceeds unit/citation bound".into());
        }
        let count = self
            .paragraphs
            .iter()
            .map(|p| p.citations.len())
            .sum::<usize>();
        if self.citation_sources.len() > count
            || self.citation_contexts.len() > self.citation_sources.len()
        {
            return Err("explanation has unused citation provenance".into());
        }
        let mut paragraphs = Vec::new();
        for paragraph in &self.paragraphs {
            let mut citations = Vec::new();
            for (
                source,
                start_byte,
                end_byte,
                start_line,
                end_line,
                range_sha256,
                claim_start_byte,
                claim_end_byte,
                claim_sha256,
            ) in &paragraph.citations
            {
                let source = self
                    .citation_sources
                    .get(*source)
                    .ok_or("explanation citation Source reference is outside its table")?;
                let context = self
                    .citation_contexts
                    .get(source.context)
                    .ok_or("explanation citation context reference is outside its table")?;
                citations.push(ExplanationCitation {
                    source_id: source.source_id.clone(),
                    source_revision: source.source_revision,
                    observation_fingerprint: source.observation_fingerprint.clone(),
                    analysis_snapshot_id: context.analysis_snapshot_id.clone(),
                    repository_snapshot_id: context.repository_snapshot_id.clone(),
                    repository_source_id: context.repository_source_id.clone(),
                    locator: source.locator.clone(),
                    field: source.field.clone(),
                    use_kind: source.use_kind,
                    start_byte: *start_byte,
                    end_byte: *end_byte,
                    start_line: *start_line,
                    end_line: *end_line,
                    range_sha256: range_sha256.clone(),
                    claim_start_byte: *claim_start_byte,
                    claim_end_byte: *claim_end_byte,
                    claim_sha256: claim_sha256.clone(),
                });
            }
            paragraphs.push(ExplanationParagraph {
                question: paragraph.question,
                text: paragraph.text.clone(),
                evidence_keys: paragraph.evidence_keys.clone(),
                citations,
            });
        }
        let value = ExplanationRealization {
            format_kind: self.format_kind.clone(),
            format_version: self.format_version,
            plan_fingerprint: self.plan_fingerprint.clone(),
            language: self.language.clone(),
            generator: self.generator.clone(),
            paragraphs,
            diagrams: self.diagrams.clone(),
        };
        // Reject duplicate, unused, reordered or aliased provenance. The one
        // encoder defines a deterministic first-use representation.
        if Self::from(&value) != *self {
            return Err(
                "explanation citation tables are not canonical first-use provenance".into(),
            );
        }
        Ok(value)
    }
}

impl Serialize for ExplanationRealization {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Wire::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ExplanationRealization {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = serde_json::Value::deserialize(deserializer)?;
        // Check the version envelope before interpreting paragraph/provenance
        // meaning. No old-shape decoder, upgrade or write-back exists.
        if raw["format_kind"].as_str() != Some(EXPLANATION_KIND)
            || raw["format_version"].as_u64() != Some(u64::from(EXPLANATION_VERSION))
        {
            return Err(D::Error::custom(
                "unsupported explanation format; regenerate from current preparation",
            ));
        }
        let size = serde_json::to_vec(&raw).map_err(D::Error::custom)?.len();
        if size > EXPLANATION_BYTE_LIMIT {
            return Err(D::Error::custom(format!("explanation compact realization JSON is {size} bytes; limit {EXPLANATION_BYTE_LIMIT}; preserve the complete explanation and grounding; seek Product support for this measured capacity limitation")));
        }
        let wire: Wire = serde_json::from_value(raw).map_err(D::Error::custom)?;
        wire.expand().map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explanation_text_hash;
    use serde_json::json;

    fn fixture() -> ExplanationRealization {
        let mut paragraphs = (0..30)
            .map(|i| ExplanationParagraph {
                question: ExplanationQuestion::Narrative,
                text: format!("입력 {i}의 상태와 결과를 근거와 함께 설명합니다. ").repeat(9),
                evidence_keys: vec!["repository_basis".into()],
                citations: Vec::new(),
            })
            .collect::<Vec<_>>();
        for i in 0..77 {
            let source = i % 55;
            let context = source % 3;
            let text_len = paragraphs[i % 30].text.len();
            let text_hash = explanation_text_hash(&paragraphs[i % 30].text);
            paragraphs[i % 30].citations.push(ExplanationCitation {
                source_id: format!("{source:032x}"),
                source_revision: 1,
                observation_fingerprint: explanation_text_hash(&format!("observation {source}")),
                analysis_snapshot_id: format!("{context:064x}"),
                repository_snapshot_id: format!("{:064x}", context + 10),
                repository_source_id: format!("{:032x}", context + 20),
                locator: format!("src/implementation_{source:02}.py"),
                start_byte: source * 80,
                end_byte: source * 80 + 55,
                start_line: source + 1,
                end_line: source + 2,
                field: "body".into(),
                range_sha256: explanation_text_hash(&format!("range {source}")),
                claim_start_byte: 0,
                claim_end_byte: text_len,
                claim_sha256: text_hash,
                use_kind: if context == 0 {
                    ExplanationSourceUse::BeforeCode
                } else {
                    ExplanationSourceUse::CurrentCode
                },
            });
        }
        ExplanationRealization {
            format_kind: EXPLANATION_KIND.into(),
            format_version: EXPLANATION_VERSION,
            plan_fingerprint: explanation_text_hash("prepared"),
            language: "ko".into(),
            generator: ExplanationGenerator {
                host: "test-host".into(),
                session: "test-session".into(),
                agent: None,
                model: None,
            },
            paragraphs,
            diagrams: Vec::new(),
        }
    }

    #[test]
    fn shared_provenance_preserves_every_expanded_field_within_the_same_bound() {
        let original = fixture();
        // Independent expanded view: paragraph/citation structs keep their full
        // fields. This is measurement support, not another accepted wire format.
        let expanded = json!({"format_kind":original.format_kind,"format_version":original.format_version,
            "plan_fingerprint":original.plan_fingerprint,"language":original.language,
            "generator":original.generator,"paragraphs":original.paragraphs,"diagrams":original.diagrams});
        let encoded = serde_json::to_vec(&original).unwrap();
        assert!(serde_json::to_vec(&expanded).unwrap().len() > EXPLANATION_BYTE_LIMIT);
        assert!(encoded.len() <= EXPLANATION_BYTE_LIMIT, "{}", encoded.len());
        let decoded: ExplanationRealization = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, original);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), encoded);
        let value: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(value["citation_contexts"].as_array().unwrap().len(), 3);
        assert_eq!(value["citation_sources"].as_array().unwrap().len(), 55);
        assert_eq!(decoded.citations().count(), 77);
    }

    #[test]
    fn wrong_versions_and_malformed_or_noncanonical_tables_fail_closed() {
        let original = serde_json::to_value(fixture()).unwrap();
        for version in [EXPLANATION_VERSION - 1, EXPLANATION_VERSION + 1] {
            let mut bad = original.clone();
            bad["format_version"] = json!(version);
            bad["paragraphs"] = json!("not a domain structure");
            assert!(serde_json::from_value::<ExplanationRealization>(bad)
                .unwrap_err()
                .to_string()
                .contains("unsupported explanation format"));
        }
        let mut variants = Vec::new();
        let mut bad = original.clone();
        bad["paragraphs"][0]["citations"][0][0] = json!(9999);
        variants.push(bad);
        let mut bad = original.clone();
        bad["citation_sources"][0]["context"] = json!(9999);
        variants.push(bad);
        let mut bad = original.clone();
        bad["paragraphs"][0]["citations"][0][1] = json!("not a byte");
        variants.push(bad);
        let mut bad = original.clone();
        bad["paragraphs"][0]["citations"][0]
            .as_array_mut()
            .unwrap()
            .pop();
        variants.push(bad);
        let mut bad = original.clone();
        bad["citation_sources"][0]["unexpected"] = json!(true);
        variants.push(bad);
        let mut bad = original.clone();
        let unused = bad["citation_sources"][0].clone();
        bad["citation_sources"].as_array_mut().unwrap().push(unused);
        variants.push(bad);
        let mut bad = original.clone();
        let unused = bad["citation_contexts"][0].clone();
        bad["citation_contexts"]
            .as_array_mut()
            .unwrap()
            .push(unused);
        variants.push(bad);
        let mut bad = original.clone();
        bad["citation_sources"].as_array_mut().unwrap().swap(0, 1);
        for paragraph in bad["paragraphs"].as_array_mut().unwrap() {
            for citation in paragraph["citations"].as_array_mut().unwrap() {
                let index = citation[0].as_u64().unwrap();
                if index < 2 {
                    citation[0] = json!(1 - index);
                }
            }
        }
        variants.push(bad);
        for bad in variants {
            assert!(serde_json::from_value::<ExplanationRealization>(bad).is_err());
        }
    }

    #[test]
    fn empty_provenance_is_absent_and_alias_expansion_keeps_existing_unit_bounds() {
        let mut small = fixture();
        small.paragraphs.truncate(1);
        small.paragraphs[0].citations.clear();
        small.paragraphs[0].text = "Recorded scope only".into();
        let value = serde_json::to_value(&small).unwrap();
        assert!(value.get("citation_contexts").is_none());
        assert!(value.get("citation_sources").is_none());
        assert_eq!(
            serde_json::from_value::<ExplanationRealization>(value.clone()).unwrap(),
            small
        );
        let mut bad = value;
        bad["paragraphs"] = json!(vec![bad["paragraphs"][0].clone(); 33]);
        assert!(serde_json::from_value::<ExplanationRealization>(bad)
            .unwrap_err()
            .to_string()
            .contains("unit/citation bound"));
    }
}
