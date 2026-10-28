//! Borrowed presentation roles over admitted meaning. No authored copy or geometry.
use crate::*;

/// One selectable role, with its original claim binding and precise context.
/// Renderer-owned suffixes cannot collide with admitted element tokens.
#[derive(Clone, Copy, Debug)]
pub enum DiagramElement<'a> {
    Node(&'a DiagramNode),
    Identifier(&'a DiagramNode),
    Relation(&'a DiagramRelation),
    Condition(&'a DiagramRelation),
    StateOwner(&'a DiagramLabel),
    GroupCondition {
        group: &'a DiagramGroup,
        index: usize,
    },
    Message {
        message: &'a DiagramMessage,
        group: &'a DiagramGroup,
        group_index: usize,
        message_index: usize,
    },
}

impl GroundedDiagram {
    /// Stable semantic order, independent of layout width, locale or display text.
    pub fn presentation_elements(&self) -> Vec<DiagramElement<'_>> {
        let mut result = Vec::new();
        for node in self.nodes() {
            result.push(DiagramElement::Node(node));
            if node.identifier.is_some() {
                result.push(DiagramElement::Identifier(node));
            }
        }
        match &self.meaning {
            DiagramMeaning::Concept { relations, .. }
            | DiagramMeaning::Component { relations, .. }
            | DiagramMeaning::Flowchart { relations, .. }
            | DiagramMeaning::State { relations, .. } => {
                if let DiagramMeaning::State { owner, .. } = &self.meaning {
                    result.push(DiagramElement::StateOwner(owner));
                }
                for relation in relations {
                    result.push(DiagramElement::Relation(relation));
                    if relation.condition.is_some() {
                        result.push(DiagramElement::Condition(relation));
                    }
                }
            }
            DiagramMeaning::Sequence { groups, .. } => {
                for (index, group) in groups.iter().enumerate() {
                    if group.condition.is_some() {
                        result.push(DiagramElement::GroupCondition { group, index });
                    }
                    for (message_index, message) in group.messages.iter().enumerate() {
                        result.push(DiagramElement::Message {
                            message,
                            group,
                            group_index: index,
                            message_index,
                        });
                    }
                }
            }
        }
        result
    }
}

impl<'a> DiagramElement<'a> {
    pub fn id(self) -> String {
        match self {
            Self::Node(n) => n.id.clone(),
            Self::Identifier(n) => format!("{}:identifier", n.id),
            Self::Relation(r) => r.id.clone(),
            Self::Condition(r) => format!("{}:condition", r.id),
            Self::StateOwner(_) => "state:owner".into(),
            Self::GroupCondition { index, .. } => format!("guard:{}", index + 1),
            Self::Message { message, .. } => message.id.clone(),
        }
    }

    pub fn label(self) -> Option<&'a DiagramLabel> {
        match self {
            Self::Node(n) => Some(&n.label),
            Self::Identifier(n) => n.identifier.as_ref(),
            Self::Relation(r) => Some(&r.label),
            Self::Condition(r) => r.condition.as_ref(),
            Self::StateOwner(owner) => Some(owner),
            Self::GroupCondition { group, .. } => group.condition.as_ref(),
            Self::Message { message, .. } => Some(&message.label),
        }
    }

    /// Full nonvisual/detail context. Compact scene labels remain separate roles.
    /// Missing endpoints/bindings fail rather than inventing a partial caption.
    pub fn caption(
        self,
        diagram: &GroundedDiagram,
        texts: &[&str],
        locale: FixedLocale,
    ) -> Result<String, String> {
        let label = |l| diagram_label(l, texts);
        let node = |id: &str| {
            diagram
                .nodes()
                .iter()
                .find(|n| n.id == id)
                .ok_or_else(|| "diagram endpoint unavailable".to_owned())
                .and_then(|n| diagram_node_identity(n, texts))
        };
        let condition = |l: Option<&DiagramLabel>| -> Result<String, String> {
            l.map(|l| {
                diagram_label(l, texts)
                    .map(|s| format!(" · {} {s}", diagram_ui(locale, "condition:", "조건:")))
            })
            .transpose()
            .map(Option::unwrap_or_default)
        };
        let owner = match &diagram.meaning {
            DiagramMeaning::State { owner, .. } => format!(
                " · {} {}",
                diagram_ui(locale, "State owner:", "상태 소유자:"),
                label(owner)?
            ),
            _ => String::new(),
        };
        let order = |group: &DiagramGroup| match group.order {
            DiagramOrder::Sequential => diagram_ui(locale, "strict · sequential", "strict · 순차"),
            DiagramOrder::Parallel => {
                diagram_ui(locale, "par · no relative order", "par · 상대 순서 없음")
            }
        };
        Ok(match self {
            Self::Node(n) => format!(
                "{} · {}{owner}",
                node(&n.id)?,
                n.kind.localized_text(locale)
            ),
            Self::Identifier(n) => format!(
                "{} · {}{owner}",
                node(&n.id)?,
                diagram_ui(locale, "Identifier", "식별자")
            ),
            Self::StateOwner(l) => format!(
                "{} {}",
                diagram_ui(locale, "State owner:", "상태 소유자:"),
                label(l)?
            ),
            Self::Relation(r) | Self::Condition(r) => format!(
                "{} → {} · {}{owner}{}",
                node(&r.from)?,
                node(&r.to)?,
                r.kind.localized_text(locale),
                condition(r.condition.as_ref())?
            ),
            Self::GroupCondition { group, index } => format!(
                "{}. {} · opt{}",
                index + 1,
                order(group),
                condition(group.condition.as_ref())?
            ),
            Self::Message {
                message,
                group,
                group_index,
                message_index,
            } => format!(
                "{}.{} {} → {} · {} · {}{}",
                group_index + 1,
                message_index + 1,
                node(&message.from)?,
                node(&message.to)?,
                message.kind.localized_text(locale),
                order(group),
                condition(group.condition.as_ref())?
            ),
        })
    }
}
