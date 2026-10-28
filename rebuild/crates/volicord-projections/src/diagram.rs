//! Admitted diagram meaning. Coordinates and renderer state never enter this model.
//! Authored display text is bound to an exact Source-grounded explanatory claim.
use crate::{explanation_text_hash, ExplanationRealization, ExplanationSubject};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagramLabel {
    /// Display wording, independent of the wording of its supporting claim.
    pub text: String,
    /// Exact explanatory claim, not an implicit label dictionary or geometry.
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
    /// Exact authored identifier; the label is always the primary reader name.
    pub identifier: Option<DiagramLabel>,
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
/// Fixed reader vocabulary follows UI locale; authored spans and wire enums do not.
pub fn diagram_ui(
    locale: crate::FixedLocale,
    english: &'static str,
    korean: &'static str,
) -> &'static str {
    match locale {
        crate::FixedLocale::English => english,
        crate::FixedLocale::Korean => korean,
    }
}
impl DiagramNodeKind {
    pub fn localized_text(self, locale: crate::FixedLocale) -> &'static str {
        let korean = match self {
            Self::Record => "기록",
            Self::ContextRole => "맥락 역할",
            Self::ReadingGroup => "읽기 묶음",
            Self::Component => "구성요소",
            Self::Participant => "참여자",
            Self::Process => "처리",
            Self::Branch => "분기",
            Self::Initial => "시작",
            Self::Final => "종료",
            Self::State => "상태",
        };
        diagram_ui(locale, self.text(), korean)
    }
}
impl DiagramRelationKind {
    pub fn localized_text(self, locale: crate::FixedLocale) -> &'static str {
        let korean = match self {
            Self::References => "참조",
            Self::Groups => "묶음",
            Self::RoleOf => "역할",
            Self::DependsOn => "의존",
            Self::Produces => "생성",
            Self::DataFlow => "데이터 흐름",
            Self::ControlFlow => "제어 흐름",
            Self::Transition => "전이",
        };
        diagram_ui(locale, self.text(), korean)
    }
}
impl DiagramMessageKind {
    pub fn localized_text(self, locale: crate::FixedLocale) -> &'static str {
        let korean = match self {
            Self::Call => "호출",
            Self::Return => "반환",
            Self::Await => "대기",
            Self::StateChange => "상태 변경",
        };
        diagram_ui(locale, self.text(), korean)
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
        std::iter::once(&self.title)
            .chain(
                self.presentation_elements()
                    .into_iter()
                    .filter_map(|e| e.label()),
            )
            .collect()
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
pub fn diagram_label<'a>(label: &'a DiagramLabel, texts: &[&str]) -> Result<&'a str, String> {
    texts
        .get(label.unit)
        .and_then(|s| s.get(label.start_byte..label.end_byte))
        .map(|_| label.text.as_str())
        .ok_or_else(|| "diagram supporting claim unavailable".into())
}

/// Full identity for text, accessibility and deliberate detail. Compact scene
/// endpoints may use the primary name, with the identifier for equal names.
pub fn diagram_node_identity(node: &DiagramNode, texts: &[&str]) -> Result<String, String> {
    let name = diagram_label(&node.label, texts)?;
    Ok(match &node.identifier {
        Some(identifier) => format!("{name} ({})", diagram_label(identifier, texts)?),
        None => name.to_owned(),
    })
}
/// Faithful linear alternative, including endpoints, kind, guard and concurrency.
/// CLI text and Viewer derive it from exactly the same admitted meaning as SVG.
pub fn diagram_text(d: &GroundedDiagram, texts: &[&str]) -> Result<String, String> {
    diagram_text_localized(d, texts, crate::FixedLocale::English)
}
/// Same admitted meaning with localized explanatory vocabulary for Reader UI.
pub fn diagram_text_localized<'a>(
    d: &'a GroundedDiagram,
    texts: &[&str],
    locale: crate::FixedLocale,
) -> Result<String, String> {
    let label = |l: &'a DiagramLabel| diagram_label(l, texts);
    let node = |id: &str| {
        d.nodes()
            .iter()
            .find(|n| n.id == id)
            .ok_or_else(|| "diagram endpoint unavailable".to_owned())
            .and_then(|n| diagram_node_identity(n, texts))
    };
    let mut lines = vec![
        format!("{} [diagram={}]", label(&d.title)?, d.id),
        texts
            .get(d.caption_unit)
            .ok_or("caption unavailable")?
            .to_string(),
    ];
    for n in d.nodes() {
        lines.push(format!(
            "{} [{}; node={}]",
            diagram_node_identity(n, texts)?,
            n.kind.localized_text(locale),
            n.id
        ));
    }
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            if let DiagramMeaning::State { owner, .. } = &d.meaning {
                lines.push(format!(
                    "{} {}",
                    diagram_ui(locale, "State owner:", "상태 소유자:"),
                    label(owner)?
                ));
            }
            for r in relations {
                let guard = r
                    .condition
                    .as_ref()
                    .map(label)
                    .transpose()?
                    .map(|s| format!(" [{} {s}]", diagram_ui(locale, "if", "조건:")))
                    .unwrap_or_default();
                lines.push(format!(
                    "{} → {} · {}: {}{guard} [edge={}; from={}; to={}]",
                    node(&r.from)?,
                    node(&r.to)?,
                    r.kind.localized_text(locale),
                    label(&r.label)?,
                    r.id,
                    r.from,
                    r.to
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
                    .map(|s| format!(" [{} {s}]", diagram_ui(locale, "if", "조건:")))
                    .unwrap_or_default();
                lines.push(format!(
                    "{}. {}{guard}",
                    i + 1,
                    match g.order {
                        DiagramOrder::Sequential => diagram_ui(locale, "sequential", "순차"),
                        DiagramOrder::Parallel => diagram_ui(
                            locale,
                            "parallel (no relative order asserted)",
                            "병렬 (상대 순서 없음)"
                        ),
                    }
                ));
                for (j, m) in g.messages.iter().enumerate() {
                    lines.push(format!(
                        "  {}.{} {} → {} · {}: {} [message={}; from={}; to={}]",
                        i + 1,
                        j + 1,
                        node(&m.from)?,
                        node(&m.to)?,
                        m.kind.localized_text(locale),
                        label(&m.label)?,
                        m.id,
                        m.from,
                        m.to
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
        for node in d.nodes() {
            let repeated = d
                .nodes()
                .iter()
                .any(|other| other.id != node.id && other.label.text == node.label.text);
            if repeated
                && (node.identifier.is_none()
                    || d.nodes().iter().any(|other| {
                        other.id != node.id
                            && other.label.text == node.label.text
                            && other.identifier.as_ref().map(|l| &l.text)
                                == node.identifier.as_ref().map(|l| &l.text)
                    }))
            {
                return Err("equal primary names require distinct grounded identifiers".into());
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
            p.text
                .get(label.start_byte..label.end_byte)
                .filter(|s| !s.trim().is_empty())
                .ok_or("invalid diagram claim UTF-8 span")?;
            let text = &label.text;
            if text.trim().is_empty()
                || text.chars().count() > 96
                || text
                    .chars()
                    .any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}')
            {
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

#[cfg(test)]
mod display_tests {
    use super::*;
    use crate::*;

    // Mechanical grounding control, not an assertion of Source entailment.
    fn fixture() -> ExplanationRealization {
        let prose = "Submitted requests settle before the conditional notification attempt.";
        let citation = ExplanationCitation {
            source_id: "01".repeat(16),
            source_revision: 1,
            observation_fingerprint: "observation".into(),
            analysis_snapshot_id: "snapshot".into(),
            repository_snapshot_id: "repository".into(),
            repository_source_id: "02".repeat(16),
            locator: "worker.js".into(),
            start_line: 1,
            end_line: 3,
            field: "body".into(),
            start_byte: 0,
            end_byte: 100,
            range_sha256: explanation_text_hash("source control"),
            claim_start_byte: 0,
            claim_end_byte: prose.len(),
            claim_sha256: explanation_text_hash(prose),
            use_kind: ExplanationSourceUse::CurrentCode,
        };
        let label = |text: &str| DiagramLabel {
            text: text.into(),
            unit: 0,
            start_byte: 0,
            end_byte: prose.len(),
        };
        ExplanationRealization {
            format_kind: EXPLANATION_KIND.into(),
            format_version: EXPLANATION_VERSION,
            plan_fingerprint: "preparation".into(),
            language: "en".into(),
            generator: ExplanationGenerator {
                host: "control".into(),
                session: "control".into(),
                agent: None,
                model: None,
            },
            paragraphs: vec![ExplanationParagraph {
                question: ExplanationQuestion::Narrative,
                text: prose.into(),
                evidence_keys: vec!["source".into()],
                citations: vec![citation],
            }],
            diagrams: vec![GroundedDiagram {
                id: "requests".into(),
                title: label("Submission and notification"),
                caption_unit: 0,
                semantic_fingerprint: String::new(),
                meaning: DiagramMeaning::Sequence {
                    participants: vec![DiagramNode {
                        id: "worker".into(),
                        kind: DiagramNodeKind::Participant,
                        label: label("Sync service"),
                        identifier: Some(label("createPlayerSyncService")),
                    }],
                    groups: vec![DiagramGroup {
                        order: DiagramOrder::Sequential,
                        condition: Some(label("paired token exists")),
                        messages: vec![DiagramMessage {
                            id: "notify".into(),
                            from: "worker".into(),
                            to: "worker".into(),
                            kind: DiagramMessageKind::Call,
                            label: label("attempt notification"),
                        }],
                    }],
                },
            }],
        }
    }
    fn subject() -> ExplanationSubject {
        ExplanationSubject::Work(volicord_context::ContextItemId::from_bytes([1; 16]))
    }
    #[test]
    fn roles_are_grounded_without_being_substrings_and_change_the_semantic_seal(
    ) -> Result<(), String> {
        let mut r = fixture();
        assert!(!r.paragraphs[0].text.contains("Sync service"));
        seal_diagrams(&mut r)?;
        validate_diagrams(subject(), &r)?;
        let seal = r.diagrams[0].semantic_fingerprint.clone();
        if let DiagramMeaning::Sequence { participants, .. } = &mut r.diagrams[0].meaning {
            participants[0]
                .identifier
                .as_mut()
                .ok_or("identifier")?
                .text = "Worker.create".into();
        }
        assert!(validate_diagrams(subject(), &r).is_err());
        seal_diagrams(&mut r)?;
        assert_ne!(seal, r.diagrams[0].semantic_fingerprint);
        validate_diagrams(subject(), &r)?;
        let texts = r
            .paragraphs
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>();
        let alternative = diagram_text(&r.diagrams[0], &texts)?;
        for expected in [
            "Sync service",
            "Worker.create",
            "attempt notification",
            "paired token exists",
        ] {
            assert!(alternative.contains(expected));
            assert!(r.generated_texts().any(|text| text == expected));
        }
        Ok(())
    }
    #[test]
    fn every_authored_role_requires_a_valid_covering_claim_and_printable_bounded_text(
    ) -> Result<(), String> {
        for variant in 0..7 {
            let mut r = fixture();
            if let DiagramMeaning::Sequence { participants, .. } = &mut r.diagrams[0].meaning {
                let l = participants[0].identifier.as_mut().ok_or("identifier")?;
                match variant {
                    0 => l.unit = 99,
                    1 => l.end_byte = usize::MAX,
                    2 => l.text.clear(),
                    3 => l.text = "x".repeat(97),
                    4 => l.text = "name\u{2028}identifier".into(),
                    5 => l.text = "hidden\0text".into(),
                    _ => r.paragraphs[0].citations[0].claim_end_byte = 1,
                }
            }
            // Invalid references can fail sealing before admission; neither can
            // turn an ungrounded identifier into admitted content.
            if seal_diagrams(&mut r).is_ok() {
                assert!(
                    validate_diagrams(subject(), &r).is_err(),
                    "variant {variant}"
                );
            }
        }
        let mut value = serde_json::to_value(fixture()).map_err(|e| e.to_string())?;
        value["diagrams"][0]["meaning"]["participants"][0]["hidden_label"] =
            serde_json::json!("untraversed");
        assert!(serde_json::from_value::<ExplanationRealization>(value).is_err());
        Ok(())
    }
}
