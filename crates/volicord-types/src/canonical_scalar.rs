//! Reusable byte-level contracts for canonical scalar values.

use serde::Serialize;

use crate::{canonical::canonical_json_sha256, ids::RequestHash};

/// Input categories expanded into the differential invalid corpus for one
/// canonical scalar specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CanonicalScalarInvalidCorpus {
    /// ASCII space cases, including surrounding and embedded space.
    pub ascii_space_cases: &'static [&'static str],
    /// Representative non-ASCII whitespace code points.
    pub unicode_whitespace: &'static [&'static str],
    /// Whether every ASCII control byte, including DEL, is included.
    pub all_ascii_control_bytes: bool,
    /// Whether every printable ASCII byte outside the allowed alphabet is included.
    pub all_disallowed_printable_ascii: bool,
    /// Whether one value beyond the maximum byte length is included.
    pub overlength_value: bool,
}

/// One portable byte-level canonical scalar contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CanonicalScalarSpec {
    /// Stable semantic type name.
    pub semantic_name: &'static str,
    /// Minimum accepted byte length.
    pub minimum_length: usize,
    /// Maximum accepted byte length.
    pub maximum_length: usize,
    /// Complete accepted byte alphabet. Every byte must be ASCII.
    pub allowed_ascii_bytes: &'static [u8],
    /// Complete values rejected even when every byte otherwise matches.
    pub forbidden_complete_values: &'static [&'static str],
    /// Representative canonical values owned by this specification.
    pub examples: &'static [&'static str],
    /// Categories used to generate the shared invalid corpus.
    pub invalid_corpus: CanonicalScalarInvalidCorpus,
}

impl CanonicalScalarSpec {
    /// Returns whether the exact UTF-8 bytes satisfy this contract.
    pub fn accepts(self, value: &str) -> bool {
        let bytes = value.as_bytes();
        (self.minimum_length..=self.maximum_length).contains(&bytes.len())
            && bytes
                .iter()
                .all(|byte| self.allowed_ascii_bytes.contains(byte))
            && !self.forbidden_complete_values.contains(&value)
    }

    /// Generates the anchored JSON Schema pattern for the exact byte alphabet
    /// and length interval.
    pub fn json_schema_pattern(self) -> String {
        let mut class = String::new();
        for byte in self.allowed_ascii_bytes {
            match byte {
                b'\\' | b'[' | b']' | b'-' | b'^' => {
                    class.push('\\');
                    class.push(char::from(*byte));
                }
                _ => class.push(char::from(*byte)),
            }
        }
        format!(
            "^[{class}]{{{},{}}}$",
            self.minimum_length, self.maximum_length
        )
    }

    /// Renders the exact byte alphabet without maintaining a second textual
    /// description of the accepted characters.
    pub fn displayed_alphabet(self) -> String {
        self.allowed_ascii_bytes
            .iter()
            .map(|byte| char::from(*byte))
            .collect()
    }

    /// Renders the complete forbidden-value set for generated diagnostics and
    /// schema descriptions.
    pub fn displayed_forbidden_values(self) -> String {
        self.forbidden_complete_values
            .iter()
            .map(|value| format!("`{value}`"))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Generates a human-readable description from the authoritative fields.
    pub fn description(self) -> String {
        format!(
            "Canonical {} containing {} to {} ASCII bytes from the exact alphabet `{}`; forbidden complete values: {}.",
            self.semantic_name,
            self.minimum_length,
            self.maximum_length,
            self.displayed_alphabet(),
            self.displayed_forbidden_values()
        )
    }

    /// Generates a parse diagnostic from the authoritative fields.
    pub fn parse_error_message(self) -> String {
        format!(
            "{} must contain {} to {} bytes from the exact alphabet `{}`; forbidden complete values: {}",
            self.semantic_name,
            self.minimum_length,
            self.maximum_length,
            self.displayed_alphabet(),
            self.displayed_forbidden_values()
        )
    }

    /// Generates the exact SQLite predicate for one non-null `TEXT` value.
    pub fn sqlite_non_null_predicate(self, value_expression: &str) -> String {
        let byte_length = format!("length(CAST({value_expression} AS BLOB))");
        let glob_class = self.sqlite_glob_class();
        let mut clauses = vec![
            format!(
                "{byte_length} BETWEEN {} AND {}",
                self.minimum_length, self.maximum_length
            ),
            format!("{byte_length} = length({value_expression})"),
            format!("{value_expression} NOT GLOB '*[^{glob_class}]*'"),
        ];
        clauses.extend(
            self.forbidden_complete_values
                .iter()
                .map(|value| format!("{value_expression} <> '{}'", value.replace('\'', "''"))),
        );
        clauses.join("\nAND ")
    }

    /// Generates the exact SQLite predicate for one nullable `TEXT` value.
    pub fn sqlite_nullable_predicate(self, value_expression: &str) -> String {
        format!(
            "{value_expression} IS NULL\nOR (\n{}\n)",
            indent(&self.sqlite_non_null_predicate(value_expression), 2)
        )
    }

    /// Generates the exact SQLite predicate for a position that requires a
    /// present value satisfying this scalar contract.
    pub fn sqlite_required_predicate(self, value_expression: &str) -> String {
        format!(
            "{value_expression} IS NOT NULL\nAND {}",
            self.sqlite_non_null_predicate(value_expression)
        )
    }

    /// Generates one deterministic invalid corpus from this specification.
    pub fn generated_invalid_corpus(self) -> Vec<String> {
        let mut corpus = vec![String::new()];
        corpus.extend(
            self.forbidden_complete_values
                .iter()
                .map(|value| (*value).to_owned()),
        );
        corpus.extend(
            self.invalid_corpus
                .ascii_space_cases
                .iter()
                .map(|value| (*value).to_owned()),
        );

        if self.invalid_corpus.all_ascii_control_bytes {
            corpus.extend(
                (0_u8..=31)
                    .chain([127])
                    .map(|byte| char::from(byte).to_string()),
            );
        }
        corpus.extend(
            self.invalid_corpus
                .unicode_whitespace
                .iter()
                .map(|value| (*value).to_owned()),
        );
        if self.invalid_corpus.all_disallowed_printable_ascii {
            corpus.extend(
                (33_u8..=126)
                    .filter(|byte| !self.allowed_ascii_bytes.contains(byte))
                    .map(|byte| char::from(byte).to_string()),
            );
        }
        if self.invalid_corpus.overlength_value {
            let fill = self
                .allowed_ascii_bytes
                .iter()
                .copied()
                .find(u8::is_ascii_lowercase)
                .unwrap_or(b'a');
            corpus.push(char::from(fill).to_string().repeat(self.maximum_length + 1));
        }

        corpus.sort();
        corpus.dedup();
        corpus
    }

    fn sqlite_glob_class(self) -> String {
        let mut class = String::new();
        if self.allowed_ascii_bytes.contains(&b']') {
            class.push(']');
        }
        if self.allowed_ascii_bytes.contains(&b'-') {
            class.push('-');
        }
        for byte in self.allowed_ascii_bytes {
            if !matches!(byte, b']' | b'-') {
                class.push(char::from(*byte));
            }
        }
        class
    }
}

fn indent(value: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    value
        .lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

const BASELINE_REF_ALLOWED_ASCII_BYTES: &[u8] = b"-0123456789_abcdefghijklmnopqrstuvwxyz";
const BASELINE_REF_FORBIDDEN_COMPLETE_VALUES: &[&str] = &["null"];
const BASELINE_REF_EXAMPLES: &[&str] = &[
    "baseline_example_001",
    "baseline-example-001",
    "0123456789abcdef0123456789abcdef01234567",
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
];
const BASELINE_REF_ASCII_SPACE_CASES: &[&str] = &[" ", " baseline", "baseline ", "base line"];
const BASELINE_REF_UNICODE_WHITESPACE: &[&str] = &[
    "\u{00a0}", "\u{1680}", "\u{2003}", "\u{202f}", "\u{205f}", "\u{3000}",
];

/// The one canonical byte-level contract for `BaselineRef`.
pub const BASELINE_REF_SPEC: CanonicalScalarSpec = CanonicalScalarSpec {
    semantic_name: "BaselineRef",
    minimum_length: 1,
    maximum_length: 64,
    allowed_ascii_bytes: BASELINE_REF_ALLOWED_ASCII_BYTES,
    forbidden_complete_values: BASELINE_REF_FORBIDDEN_COMPLETE_VALUES,
    examples: BASELINE_REF_EXAMPLES,
    invalid_corpus: CanonicalScalarInvalidCorpus {
        ascii_space_cases: BASELINE_REF_ASCII_SPACE_CASES,
        unicode_whitespace: BASELINE_REF_UNICODE_WHITESPACE,
        all_ascii_control_bytes: true,
        all_disallowed_printable_ascii: true,
        overlength_value: true,
    },
};

#[derive(Serialize)]
struct ScalarContractDigestBasis<'a> {
    domain: &'static str,
    semantic_name: &'a str,
    minimum_length: usize,
    maximum_length: usize,
    allowed_ascii_bytes: &'a [u8],
    forbidden_complete_values: &'a [&'a str],
}

/// Returns the canonical semantic digest of the accepted `BaselineRef` value set.
pub fn baseline_ref_scalar_contract_digest() -> RequestHash {
    canonical_json_sha256(&ScalarContractDigestBasis {
        domain: "volicord.scalar-contract",
        semantic_name: BASELINE_REF_SPEC.semantic_name,
        minimum_length: BASELINE_REF_SPEC.minimum_length,
        maximum_length: BASELINE_REF_SPEC.maximum_length,
        allowed_ascii_bytes: BASELINE_REF_SPEC.allowed_ascii_bytes,
        forbidden_complete_values: BASELINE_REF_SPEC.forbidden_complete_values,
    })
    .expect("static BaselineRef scalar contract always serializes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_ref_spec_is_ascii_bounded_and_deterministic() {
        assert!(BASELINE_REF_SPEC
            .allowed_ascii_bytes
            .iter()
            .all(u8::is_ascii));
        assert!(BASELINE_REF_SPEC
            .examples
            .iter()
            .all(|value| BASELINE_REF_SPEC.accepts(value)));
        assert!(BASELINE_REF_SPEC
            .generated_invalid_corpus()
            .iter()
            .all(|value| !BASELINE_REF_SPEC.accepts(value)));
        assert_eq!(
            BASELINE_REF_SPEC.sqlite_non_null_predicate("baseline_ref"),
            BASELINE_REF_SPEC.sqlite_non_null_predicate("baseline_ref")
        );
        assert_eq!(
            BASELINE_REF_SPEC.json_schema_pattern(),
            BASELINE_REF_SPEC.json_schema_pattern()
        );
    }

    #[test]
    fn baseline_ref_invalid_corpus_covers_required_byte_classes() {
        let invalid = BASELINE_REF_SPEC.generated_invalid_corpus();
        for byte in (0_u8..=31).chain([127]) {
            assert!(invalid.contains(&char::from(byte).to_string()));
        }
        for required in ["", "null", " ", "\t", "\n", "\u{00a0}", "\u{2003}"] {
            assert!(invalid.iter().any(|value| value == required));
        }
        assert!(invalid
            .iter()
            .any(|value| value.len() > BASELINE_REF_SPEC.maximum_length));
    }

    #[test]
    fn baseline_ref_scalar_contract_digest_is_canonical_and_value_set_bound() {
        let digest = baseline_ref_scalar_contract_digest();
        assert!(crate::canonical::is_canonical_sha256_digest(
            digest.as_str()
        ));
        let changed = canonical_json_sha256(&ScalarContractDigestBasis {
            domain: "volicord.scalar-contract",
            semantic_name: BASELINE_REF_SPEC.semantic_name,
            minimum_length: BASELINE_REF_SPEC.minimum_length,
            maximum_length: BASELINE_REF_SPEC.maximum_length - 1,
            allowed_ascii_bytes: BASELINE_REF_SPEC.allowed_ascii_bytes,
            forbidden_complete_values: BASELINE_REF_SPEC.forbidden_complete_values,
        })
        .expect("test scalar contract serializes");
        assert_ne!(digest, changed);
    }
}
