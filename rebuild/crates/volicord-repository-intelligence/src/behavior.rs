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
    pub control: BodyControl,
    pub value: Option<BodyValue>,
}

/// References are indices into this callable's bounded observations, not graph edges.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum BodyControl {
    StraightLine,
    Conditional { condition: usize },
    AfterEarlyReturn { condition: usize, returned: usize },
    Unspecified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyValueKind {
    Call,
    Calculation,
    Value,
}

/// Byte offsets into the retained expression avoid duplicating source payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyValue {
    pub kind: BodyValueKind,
    pub start_byte: usize,
    pub end_byte: usize,
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
        result.push(
            BodyObservationKind::Inputs,
            parameters,
            source,
            BodyControl::StraightLine,
        );
    }
    if language == &Language::Python {
        if let Some(first) = body.named_child(0) {
            if first.kind() == "expression_statement"
                && first.named_child(0).is_some_and(|n| n.kind() == "string")
            {
                result.push(
                    BodyObservationKind::Documentation,
                    first,
                    source,
                    BodyControl::StraightLine,
                );
            }
        }
    }
    let tail_control = result.sequence(body, source);
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
                result.push(BodyObservationKind::Return, last, source, tail_control);
            }
        }
    }
    Some(result)
}

impl BodyObservations {
    fn push(
        &mut self,
        kind: BodyObservationKind,
        node: Node<'_>,
        source: &[u8],
        control: BodyControl,
    ) -> Option<usize> {
        if node.has_error() || node.is_missing() {
            return None;
        }
        let Ok(expression) = node.utf8_text(source) else {
            return None;
        };
        if self.observations.len() == BODY_OBSERVATIONS_LIMIT
            || expression.len() > BODY_EXPRESSION_BYTE_LIMIT
        {
            self.omitted_count += 1;
            return None;
        }
        // A binding/return containing a callable value must not expose its nested
        // body's operations as behavior of this function.
        if contains_callable(node) {
            self.omitted_count += 1;
            return None;
        }
        let position = |p: tree_sitter::Point| SourcePosition {
            line: p.row as u64,
            column: p.column as u64,
        };
        let value_node = match kind {
            BodyObservationKind::Return => {
                if matches!(node.kind(), "return_statement" | "return_expression") {
                    node.named_child(0)
                } else {
                    Some(node)
                }
            }
            BodyObservationKind::Assignment
                if matches!(
                    node.kind(),
                    "augmented_assignment"
                        | "augmented_assignment_expression"
                        | "compound_assignment_expr"
                ) =>
            {
                None
            }
            BodyObservationKind::Binding | BodyObservationKind::Assignment => node
                .child_by_field_name("value")
                .or_else(|| node.child_by_field_name("right")),
            _ => None,
        };
        let value = value_node.map(|value| BodyValue {
            kind: match value.kind() {
                "call" | "call_expression" => BodyValueKind::Call,
                "binary_expression" | "binary_operator" | "unary_expression" | "unary_operator" => {
                    BodyValueKind::Calculation
                }
                _ => BodyValueKind::Value,
            },
            start_byte: value.start_byte() - node.start_byte(),
            end_byte: value.end_byte() - node.start_byte(),
        });
        let index = self.observations.len();
        self.observations.push(BodyObservation {
            kind,
            expression: expression.to_owned(),
            start: position(node.start_position()),
            end: position(node.end_position()),
            control,
            value,
        });
        Some(index)
    }

    fn sequence(&mut self, body: Node<'_>, source: &[u8]) -> BodyControl {
        let mut control = BodyControl::StraightLine;
        let mut cursor = body.walk();
        for statement in body.named_children(&mut cursor) {
            let node = if statement.kind() == "expression_statement" {
                statement.named_child(0).unwrap_or(statement)
            } else {
                statement
            };
            if matches!(node.kind(), "if_expression" | "if_statement")
                && control == BodyControl::StraightLine
                && !node.has_error()
            {
                let condition = node
                    .child_by_field_name("condition")
                    .and_then(|n| self.push(BodyObservationKind::Condition, n, source, control));
                if let Some(condition) = condition {
                    let consequence = node.child_by_field_name("consequence");
                    if let Some(branch) = consequence {
                        let branch_control = BodyControl::Conditional { condition };
                        self.visit(branch, source, branch_control);
                        let only =
                            if matches!(branch.kind(), "return_statement" | "return_expression") {
                                Some(branch)
                            } else if branch.named_child_count() == 1 {
                                branch.named_child(0)
                            } else {
                                None
                            };
                        if node.child_by_field_name("alternative").is_none() {
                            let only = only.map(|n| {
                                if n.kind() == "expression_statement" {
                                    n.named_child(0).unwrap_or(n)
                                } else {
                                    n
                                }
                            });
                            if let Some(return_node) = only.filter(|n| {
                                matches!(n.kind(), "return_statement" | "return_expression")
                            }) {
                                if let Some(returned) = self.observations.iter().position(|o| {
                                    o.kind == BodyObservationKind::Return
                                        && o.start.line == return_node.start_position().row as u64
                                        && o.start.column
                                            == return_node.start_position().column as u64
                                }) {
                                    control = BodyControl::AfterEarlyReturn {
                                        condition,
                                        returned,
                                    };
                                }
                            }
                        }
                    }
                    if let Some(alternative) = node.child_by_field_name("alternative") {
                        self.visit(alternative, source, BodyControl::Unspecified);
                    }
                    if let Some(condition_node) = node.child_by_field_name("condition") {
                        self.visit(condition_node, source, BodyControl::Unspecified);
                    }
                    continue;
                }
                // The condition was omitted. Visit its children and branches once,
                // retaining honest omission counts and no inferred relationship.
                let mut branch_cursor = node.walk();
                for child in node.named_children(&mut branch_cursor) {
                    self.visit(child, source, BodyControl::Unspecified);
                }
                control = BodyControl::Unspecified;
                continue;
            }
            self.visit(statement, source, control);
            // Unsupported branching can change subsequent reachability. Never infer it.
            if matches!(
                node.kind(),
                "if_expression"
                    | "if_statement"
                    | "for_statement"
                    | "for_expression"
                    | "while_statement"
                    | "while_expression"
                    | "try_statement"
                    | "match_expression"
                    | "switch_statement"
            ) {
                control = BodyControl::Unspecified;
            }
        }
        control
    }

    fn visit(&mut self, node: Node<'_>, source: &[u8], control: BodyControl) {
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
        let control = if matches!(
            node.kind(),
            "if_expression"
                | "if_statement"
                | "elif_clause"
                | "while_expression"
                | "while_statement"
                | "for_statement"
                | "for_expression"
                | "try_statement"
                | "match_expression"
                | "switch_statement"
                | "conditional_expression"
        ) {
            BodyControl::Unspecified
        } else {
            control
        };
        match node.kind() {
            "if_expression" | "if_statement" | "elif_clause" | "while_expression"
            | "while_statement" => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    self.push(BodyObservationKind::Condition, condition, source, control);
                }
            }
            "call" | "call_expression" => {
                self.push(BodyObservationKind::Call, node, source, control);
            }
            "let_declaration" | "variable_declarator" => {
                self.push(BodyObservationKind::Binding, node, source, control);
            }
            "assignment"
            | "assignment_expression"
            | "augmented_assignment"
            | "augmented_assignment_expression"
            | "compound_assignment_expr" => {
                self.push(BodyObservationKind::Assignment, node, source, control);
            }
            "return_statement" | "return_expression" => {
                self.push(BodyObservationKind::Return, node, source, control);
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(child, source, control);
        }
    }
}

fn contains_callable(node: Node<'_>) -> bool {
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
        return true;
    }
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).any(contains_callable);
    found
}
