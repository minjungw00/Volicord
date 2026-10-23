//! Admitted diagram meaning. Coordinates and renderer state never enter this model.
//! Every authored label is a UTF-8 selection of an ordinary grounded reader unit.
use crate::{explanation_text_hash, ExplanationRealization, ExplanationSubject};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramLabel {
    pub unit: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramNodeKind {
    Record,
    ContextRole,
    ReadingGroup,
    Component,
    Participant,
    Process,
    Branch,
    Initial,
    Final,
    State,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramNode {
    pub id: String,
    pub kind: DiagramNodeKind,
    pub label: DiagramLabel,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramRelationKind {
    References,
    Groups,
    RoleOf,
    DependsOn,
    Produces,
    DataFlow,
    ControlFlow,
    Transition,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramRelation {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: DiagramRelationKind,
    pub label: DiagramLabel,
    pub condition: Option<DiagramLabel>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramMessageKind {
    Call,
    Return,
    Await,
    StateChange,
}
impl DiagramMessageKind {
    pub fn text(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Return => "return",
            Self::Await => "await",
            Self::StateChange => "state change",
        }
    }
}
impl DiagramRelationKind {
    pub fn text(self) -> &'static str {
        match self {
            Self::References => "references",
            Self::Groups => "groups",
            Self::RoleOf => "role of",
            Self::DependsOn => "depends on",
            Self::Produces => "produces",
            Self::DataFlow => "data flow",
            Self::ControlFlow => "control flow",
            Self::Transition => "transition",
        }
    }
}
impl DiagramNodeKind {
    pub fn text(self) -> &'static str {
        match self {
            Self::Record => "record",
            Self::ContextRole => "context role",
            Self::ReadingGroup => "read grouping",
            Self::Component => "component",
            Self::Participant => "participant",
            Self::Process => "process",
            Self::Branch => "branch",
            Self::Initial => "initial",
            Self::Final => "final",
            Self::State => "state",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: DiagramMessageKind,
    pub label: DiagramLabel,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagramOrder {
    Sequential,
    Parallel,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramGroup {
    pub order: DiagramOrder,
    pub condition: Option<DiagramLabel>,
    pub messages: Vec<DiagramMessage>,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiagramMeaning {
    Concept {
        nodes: Vec<DiagramNode>,
        relations: Vec<DiagramRelation>,
    },
    Component {
        nodes: Vec<DiagramNode>,
        relations: Vec<DiagramRelation>,
    },
    Flowchart {
        nodes: Vec<DiagramNode>,
        relations: Vec<DiagramRelation>,
    },
    State {
        owner: DiagramLabel,
        nodes: Vec<DiagramNode>,
        relations: Vec<DiagramRelation>,
    },
    Sequence {
        participants: Vec<DiagramNode>,
        groups: Vec<DiagramGroup>,
    },
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundedDiagram {
    pub id: String,
    pub title: DiagramLabel,
    /// Complete grounded unit carrying scope, uncertainty and interpretation class.
    pub caption_unit: usize,
    /// Product compose seals meaning, language, preparation and referenced claims.
    #[serde(default)]
    pub semantic_fingerprint: String,
    pub meaning: DiagramMeaning,
}
impl GroundedDiagram {
    pub fn nodes(&self) -> &[DiagramNode] {
        match &self.meaning {
            DiagramMeaning::Concept { nodes, .. }
            | DiagramMeaning::Component { nodes, .. }
            | DiagramMeaning::Flowchart { nodes, .. }
            | DiagramMeaning::State { nodes, .. } => nodes,
            DiagramMeaning::Sequence { participants, .. } => participants,
        }
    }
    /// Exhaustive semantic selections; no captions/guards/alternative labels escape.
    pub fn labels(&self) -> Vec<&DiagramLabel> {
        let mut labels = vec![&self.title];
        labels.extend(self.nodes().iter().map(|n| &n.label));
        match &self.meaning {
            DiagramMeaning::Concept { relations, .. }
            | DiagramMeaning::Component { relations, .. }
            | DiagramMeaning::Flowchart { relations, .. }
            | DiagramMeaning::State { relations, .. } => {
                if let DiagramMeaning::State { owner, .. } = &self.meaning {
                    labels.push(owner);
                }
                for r in relations {
                    labels.push(&r.label);
                    labels.extend(r.condition.as_ref());
                }
            }
            DiagramMeaning::Sequence { groups, .. } => {
                for g in groups {
                    labels.extend(g.condition.as_ref());
                    labels.extend(g.messages.iter().map(|m| &m.label));
                }
            }
        }
        labels
    }
    pub fn fingerprint(&self, response: &ExplanationRealization) -> Result<String, String> {
        let mut meaning = self.clone();
        meaning.semantic_fingerprint.clear();
        let mut units = self
            .labels()
            .iter()
            .map(|l| l.unit)
            .collect::<BTreeSet<_>>();
        units.insert(self.caption_unit);
        let claims = units
            .into_iter()
            .map(|u| {
                response
                    .paragraphs
                    .get(u)
                    .map(|claim| (u, claim))
                    .ok_or_else(|| "diagram unit missing".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let bytes = serde_json::to_string(&(
            &response.plan_fingerprint,
            &response.language,
            meaning,
            claims,
        ))
        .map_err(|e| e.to_string())?;
        Ok(explanation_text_hash(&bytes))
    }
}
pub fn seal_diagrams(response: &mut ExplanationRealization) -> Result<(), String> {
    let seals = response
        .diagrams
        .iter()
        .map(|d| d.fingerprint(response))
        .collect::<Result<Vec<_>, _>>()?;
    for (diagram, seal) in response.diagrams.iter_mut().zip(seals) {
        diagram.semantic_fingerprint = seal;
    }
    Ok(())
}
pub fn diagram_label<'a>(label: &DiagramLabel, texts: &[&'a str]) -> Result<&'a str, String> {
    texts
        .get(label.unit)
        .and_then(|s| s.get(label.start_byte..label.end_byte))
        .ok_or_else(|| "diagram label unavailable".into())
}
/// Faithful linear alternative, including endpoints, kind, guard and concurrency.
/// CLI text and Viewer derive it from exactly the same admitted meaning as SVG.
pub fn diagram_text(d: &GroundedDiagram, texts: &[&str]) -> Result<String, String> {
    let label = |l: &DiagramLabel| diagram_label(l, texts);
    let node = |id: &str| {
        d.nodes()
            .iter()
            .find(|n| n.id == id)
            .ok_or_else(|| "diagram endpoint unavailable".to_owned())
            .and_then(|n| label(&n.label))
    };
    let mut lines = vec![
        label(&d.title)?.to_owned(),
        texts
            .get(d.caption_unit)
            .ok_or("caption unavailable")?
            .to_string(),
    ];
    for n in d.nodes() {
        lines.push(format!("{} [{}]", label(&n.label)?, n.kind.text()));
    }
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            if let DiagramMeaning::State { owner, .. } = &d.meaning {
                lines.push(format!("State owner: {}", label(owner)?));
            }
            for r in relations {
                let guard = r
                    .condition
                    .as_ref()
                    .map(label)
                    .transpose()?
                    .map(|s| format!(" [if {s}]"))
                    .unwrap_or_default();
                lines.push(format!(
                    "{} → {} · {}: {}{guard}",
                    node(&r.from)?,
                    node(&r.to)?,
                    r.kind.text(),
                    label(&r.label)?
                ));
            }
        }
        DiagramMeaning::Sequence { groups, .. } => {
            for (i, g) in groups.iter().enumerate() {
                let guard = g
                    .condition
                    .as_ref()
                    .map(label)
                    .transpose()?
                    .map(|s| format!(" [if {s}]"))
                    .unwrap_or_default();
                lines.push(format!(
                    "{}. {}{guard}",
                    i + 1,
                    match g.order {
                        DiagramOrder::Sequential => "sequential",
                        DiagramOrder::Parallel => "parallel (no relative order asserted)",
                    }
                ));
                for (j, m) in g.messages.iter().enumerate() {
                    lines.push(format!(
                        "  {}.{} {} → {} · {}: {}",
                        i + 1,
                        j + 1,
                        node(&m.from)?,
                        node(&m.to)?,
                        m.kind.text(),
                        label(&m.label)?
                    ));
                }
            }
        }
    }
    Ok(lines.join("\n"))
}
fn identity(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 48
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
pub fn validate_diagrams(
    subject: ExplanationSubject,
    response: &ExplanationRealization,
) -> Result<(), String> {
    if response.diagrams.len() > 4 {
        return Err("at most four diagrams per publication".into());
    }
    if !response.diagrams.is_empty()
        && !matches!(
            subject,
            ExplanationSubject::Work(_) | ExplanationSubject::Repository(_)
        )
    {
        return Err("diagrams require a Work article or Repository publication".into());
    }
    let mut ids = BTreeSet::new();
    for d in &response.diagrams {
        if !identity(&d.id) || !ids.insert(&d.id) {
            return Err("invalid/duplicate diagram identity".into());
        }
        if d.nodes().is_empty() || d.nodes().len() > 12 {
            return Err("diagram needs 1..12 nodes/participants".into());
        }
        let mut elements = BTreeSet::new();
        for n in d.nodes() {
            if !identity(&n.id) || !elements.insert(n.id.clone()) {
                return Err("invalid/duplicate element identity".into());
            }
        }
        let nodes = elements.clone();
        let mut edge = |id: &String, from: &String, to: &String| -> Result<(), String> {
            if !identity(id)
                || !elements.insert(id.clone())
                || !nodes.contains(from)
                || !nodes.contains(to)
            {
                return Err("diagram edge identity/endpoints missing or duplicated".into());
            }
            Ok(())
        };
        match &d.meaning {
            DiagramMeaning::Concept { relations, .. }
            | DiagramMeaning::Component { relations, .. }
            | DiagramMeaning::Flowchart { relations, .. }
            | DiagramMeaning::State { relations, .. } => {
                if relations.is_empty() || relations.len() > 20 {
                    return Err("relationship figure needs 1..20 relations".into());
                }
                let allowed = |n: &DiagramNode| match &d.meaning {
                    DiagramMeaning::Concept { .. } => matches!(
                        n.kind,
                        DiagramNodeKind::Record
                            | DiagramNodeKind::ContextRole
                            | DiagramNodeKind::ReadingGroup
                            | DiagramNodeKind::Component
                    ),
                    DiagramMeaning::Component { .. } => n.kind == DiagramNodeKind::Component,
                    DiagramMeaning::Flowchart { .. } => matches!(
                        n.kind,
                        DiagramNodeKind::Process
                            | DiagramNodeKind::Branch
                            | DiagramNodeKind::Initial
                            | DiagramNodeKind::Final
                    ),
                    DiagramMeaning::State { .. } => matches!(
                        n.kind,
                        DiagramNodeKind::State | DiagramNodeKind::Initial | DiagramNodeKind::Final
                    ),
                    _ => false,
                };
                if d.nodes().iter().any(|n| !allowed(n)) {
                    return Err("node kind does not belong to this figure".into());
                }
                for r in relations {
                    edge(&r.id, &r.from, &r.to)?;
                    let allowed = match &d.meaning {
                        DiagramMeaning::Concept { .. } => {
                            matches!(
                                r.kind,
                                DiagramRelationKind::References
                                    | DiagramRelationKind::Groups
                                    | DiagramRelationKind::RoleOf
                                    | DiagramRelationKind::DependsOn
                                    | DiagramRelationKind::Produces
                            ) && r.from != r.to
                        }
                        DiagramMeaning::Component { .. } => {
                            matches!(
                                r.kind,
                                DiagramRelationKind::DependsOn
                                    | DiagramRelationKind::Produces
                                    | DiagramRelationKind::DataFlow
                                    | DiagramRelationKind::ControlFlow
                            ) && r.from != r.to
                        }
                        DiagramMeaning::Flowchart { .. } => {
                            r.kind == DiagramRelationKind::ControlFlow
                        }
                        DiagramMeaning::State { .. } => r.kind == DiagramRelationKind::Transition,
                        _ => false,
                    };
                    if !allowed {
                        return Err(
                            "unsupported relationship kind/endpoints for this figure".into()
                        );
                    }
                    if d.nodes().iter().any(|n| {
                        (n.id == r.to && n.kind == DiagramNodeKind::Initial)
                            || (n.id == r.from && n.kind == DiagramNodeKind::Final)
                    }) {
                        return Err(
                            "initial nodes cannot receive and final nodes cannot emit transitions"
                                .into(),
                        );
                    }
                    if r.kind == DiagramRelationKind::RoleOf
                        && !d
                            .nodes()
                            .iter()
                            .any(|n| n.id == r.from && n.kind == DiagramNodeKind::ContextRole)
                    {
                        return Err("role_of must originate at a context role".into());
                    }
                }
                for n in d.nodes() {
                    if !relations.iter().any(|r| r.from == n.id || r.to == n.id) {
                        return Err("unconnected diagram node".into());
                    }
                    if n.kind == DiagramNodeKind::Branch {
                        let outgoing = relations
                            .iter()
                            .filter(|r| r.from == n.id)
                            .collect::<Vec<_>>();
                        if outgoing.len() < 2 || outgoing.iter().any(|r| r.condition.is_none()) {
                            return Err(
                                "branch needs at least two explicitly guarded outcomes".into()
                            );
                        }
                    }
                }
            }
            DiagramMeaning::Sequence { groups, .. } => {
                if d.nodes()
                    .iter()
                    .any(|n| n.kind != DiagramNodeKind::Participant)
                {
                    return Err("sequence needs participants".into());
                }
                if groups.is_empty()
                    || groups.len() > 12
                    || groups.iter().map(|g| g.messages.len()).sum::<usize>() > 20
                {
                    return Err("sequence group/message budget exceeded".into());
                }
                for g in groups {
                    if g.messages.is_empty() {
                        return Err("empty sequence group".into());
                    }
                    for m in &g.messages {
                        edge(&m.id, &m.from, &m.to)?;
                    }
                }
            }
        }
        for label in d.labels() {
            let p = response
                .paragraphs
                .get(label.unit)
                .ok_or("diagram unit missing")?;
            let text = p
                .text
                .get(label.start_byte..label.end_byte)
                .filter(|s| !s.trim().is_empty())
                .ok_or("invalid diagram label UTF-8 span")?;
            if text.chars().count() > 96 || text.chars().any(char::is_control) {
                return Err("diagram label must be at most 96 printable characters".into());
            }
            if !p.citations.iter().any(|c| {
                c.claim_start_byte <= label.start_byte && c.claim_end_byte >= label.end_byte
            }) {
                return Err("diagram label requires its own covering Source claim binding".into());
            }
        }
        let caption = response
            .paragraphs
            .get(d.caption_unit)
            .ok_or("diagram caption missing")?;
        if matches!(
            caption.question,
            crate::ExplanationQuestion::Heading
                | crate::ExplanationQuestion::Synopsis
                | crate::ExplanationQuestion::SectionHeading
        ) {
            return Err("diagram caption must be a substantive reader unit".into());
        }
        if !caption
            .citations
            .iter()
            .any(|c| c.claim_start_byte == 0 && c.claim_end_byte == caption.text.len())
        {
            return Err("diagram scope/caption requires Source grounding".into());
        }
        if d.semantic_fingerprint != d.fingerprint(response)? {
            return Err(
                "diagram meaning/binding changed; compose again from current evidence".into(),
            );
        }
    }
    Ok(())
}
