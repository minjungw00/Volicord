//! Bounded syntax observations from the existing structural parser. These are
//! source expressions, never an execution trace or callee-effect analysis.
use crate::{Language, SourcePosition};
use serde::{Deserialize, Serialize};
use tree_sitter::Node;

pub const BODY_OBSERVATIONS_KEY: &str = "body_observations";
pub const BODY_OBSERVATIONS_LIMIT: usize = 16;
pub const BODY_EXPRESSION_BYTE_LIMIT: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyObservationKind {
    Inputs,
    Condition,
    Call,
    Binding,
    Assignment,
    Return,
    Documentation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyObservation {
    pub kind: BodyObservationKind,
    pub expression: String,
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyObservations {
    pub observations: Vec<BodyObservation>,
    pub omitted_count: usize,
}

pub(crate) fn observe_body(
    node: Node<'_>,
    language: &Language,
    source: &[u8],
) -> Option<BodyObservations> {
    // Keep other languages' existing structural/semantic capabilities intact.
    if !matches!(
        language,
        Language::Rust | Language::Python | Language::JavaScript | Language::TypeScript
    ) {
        return None;
    }
    let body = node.child_by_field_name("body")?;
    let mut result = BodyObservations {
        observations: Vec::new(),
        omitted_count: 0,
    };
    if let Some(parameters) = node.child_by_field_name("parameters") {
        result.push(BodyObservationKind::Inputs, parameters, source);
    }
    if language == &Language::Python {
        if let Some(first) = body.named_child(0) {
            if first.kind() == "expression_statement"
                && first.named_child(0).is_some_and(|n| n.kind() == "string")
            {
                result.push(BodyObservationKind::Documentation, first, source);
            }
        }
    }
    result.visit(body, source);
    if language == &Language::Rust {
        if let Some(last) =
            body.named_child(u32::try_from(body.named_child_count().saturating_sub(1)).ok()?)
        {
            // An expression without a semicolon is Rust's implicit return.
            // Control constructs and declarations are outside this bounded rule.
            if matches!(
                last.kind(),
                "identifier"
                    | "call_expression"
                    | "binary_expression"
                    | "field_expression"
                    | "integer_literal"
                    | "string_literal"
                    | "boolean_literal"
                    | "tuple_expression"
                    | "struct_expression"
                    | "try_expression"
            ) {
                result.push(BodyObservationKind::Return, last, source);
            }
        }
    }
    Some(result)
}

impl BodyObservations {
    fn push(&mut self, kind: BodyObservationKind, node: Node<'_>, source: &[u8]) {
        if node.has_error() || node.is_missing() {
            return;
        }
        let Ok(expression) = node.utf8_text(source) else {
            return;
        };
        if self.observations.len() == BODY_OBSERVATIONS_LIMIT
            || expression.len() > BODY_EXPRESSION_BYTE_LIMIT
        {
            self.omitted_count += 1;
            return;
        }
        let position = |p: tree_sitter::Point| SourcePosition {
            line: p.row as u64,
            column: p.column as u64,
        };
        self.observations.push(BodyObservation {
            kind,
            expression: expression.to_owned(),
            start: position(node.start_position()),
            end: position(node.end_position()),
        });
    }

    fn visit(&mut self, node: Node<'_>, source: &[u8]) {
        // Nested callables have their own responsibility. Their body must not
        // be attributed to the enclosing function, even when invoked later.
        if matches!(
            node.kind(),
            "function_item"
                | "function_definition"
                | "function_declaration"
                | "function_expression"
                | "arrow_function"
                | "lambda"
                | "closure_expression"
                | "class_definition"
                | "class_declaration"
                | "method_definition"
        ) {
            return;
        }
        match node.kind() {
            "if_expression" | "if_statement" | "elif_clause" | "while_expression"
            | "while_statement" => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    self.push(BodyObservationKind::Condition, condition, source);
                }
            }
            "call" | "call_expression" => self.push(BodyObservationKind::Call, node, source),
            "let_declaration" | "variable_declarator" => {
                self.push(BodyObservationKind::Binding, node, source)
            }
            "assignment"
            | "assignment_expression"
            | "augmented_assignment"
            | "compound_assignment_expr" => {
                self.push(BodyObservationKind::Assignment, node, source)
            }
            "return_statement" | "return_expression" => {
                self.push(BodyObservationKind::Return, node, source)
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, source);
        }
    }
}
