//! Lossless public reading projection. Retained authoring data remains unchanged.
use crate::*;
use serde_json::{json, Value};

/// Units retain authored order and exact claim coordinates. Only equal Source
/// bindings are interned; two claims citing the same file remain distinct units.
pub fn compact_publication(retained: &RetainedExplanation, publication_id: &str) -> Value {
    let mut bindings = Vec::<Value>::new();
    let units: Vec<_> = retained.realization.generated_units().map(|unit| {
        let citations: Vec<_> = unit.citations.iter().map(|citation| {
            let mut binding = json!(citation);
            for field in ["claim_start_byte", "claim_end_byte", "claim_sha256"] {
                binding.as_object_mut().map(|b| b.remove(field));
            }
            let index = bindings.iter().position(|b| b == &binding).unwrap_or_else(|| {
                bindings.push(binding);
                bindings.len() - 1
            });
            json!({"binding":index,"claim_start_byte":citation.claim_start_byte,
                "claim_end_byte":citation.claim_end_byte,"claim_sha256":citation.claim_sha256})
        }).collect();
        json!({"question":unit.question,"text":unit.text,"evidence_keys":unit.evidence_keys,"citations":citations})
    }).collect();
    let mut provenance = json!(ExplanationProvenance::from(retained));
    // This list is exactly the ordered concatenation of unit citations. It is
    // not a second authority or another copy of every binding in transport.
    provenance.as_object_mut().map(|p| p.remove("citations"));
    json!({"publication_id":publication_id,"provenance":provenance,"units":units,
        "bindings":bindings,"sections":article_sections(retained.realization.generated_units().map(|p|p.question)).ok(),
        "read_only":true,"semantic_approval":"not_established"})
}
