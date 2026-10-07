use crate::MapEntity;
use volicord_repository_intelligence::{
    BodyObservationKind, BodyObservations, CodeEntity, CoordinateConvention, FreshnessState,
    Language, SourceRange, BODY_EXPRESSION_BYTE_LIMIT, BODY_OBSERVATIONS_KEY,
    BODY_OBSERVATIONS_LIMIT,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeExplanationState {
    Current,
    Partial,
    Stale,
    Unsupported,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeBehaviorClaim {
    pub kind: BodyObservationKind,
    pub expression: String,
    pub source_range: SourceRange,
}

/// Source syntax, separate from generated interpretations and actual execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeBehaviorReading {
    pub state: CodeExplanationState,
    pub claims: Vec<CodeBehaviorClaim>,
    pub omitted_count: usize,
    pub limitations: Vec<String>,
}

impl CodeBehaviorReading {
    pub fn unavailable() -> Self {
        Self { state: CodeExplanationState::Unavailable, claims: Vec::new(), omitted_count: 0,
            limitations: vec!["No bounded function-body observations were retained; structure alone cannot explain behavior.".into()] }
    }

    pub(crate) fn from_entity(entity: &CodeEntity) -> Self {
        let mut reading = Self::unavailable();
        if !matches!(
            entity.language,
            Language::Rust | Language::Python | Language::JavaScript | Language::TypeScript
        ) {
            reading.state = CodeExplanationState::Unsupported;
            reading.limitations = vec!["Function-body explanation observations are not supported for this language; existing inventory and analysis remain available.".into()];
            return reading;
        }
        let Some(range) = &entity.source_range else {
            return reading;
        };
        let extensions = entity
            .extensions
            .iter()
            .filter(|extension| extension.values.contains_key(BODY_OBSERVATIONS_KEY))
            .collect::<Vec<_>>();
        let [extension] = extensions.as_slice() else {
            return reading;
        };
        if extension.language != entity.language
            || extension.owning_adapter != range.adapter
            || extension.source_range.as_ref() != Some(range)
            || range.source != entity.source
            || range.repository_snapshot != entity.repository_snapshot
            || range.locator != entity.area.path
            || range.coordinate_convention != CoordinateConvention::ZeroBasedUtf8Byte
        {
            reading.limitations = vec![
                "Function-body observation Source, range or adapter does not match this entity."
                    .into(),
            ];
            return reading;
        }
        let value = &extension.values[BODY_OBSERVATIONS_KEY];
        if !value
            .get("observations")
            .and_then(|v| v.as_array())
            .is_some_and(|v| v.len() <= BODY_OBSERVATIONS_LIMIT)
        {
            return reading;
        }
        let Ok(body) = serde_json::from_value::<BodyObservations>(value.clone()) else {
            return reading;
        };
        let position = |p: volicord_repository_intelligence::SourcePosition| (p.line, p.column);
        if body.observations.iter().any(|claim| {
            claim.expression.is_empty()
                || claim.expression.len() > BODY_EXPRESSION_BYTE_LIMIT
                || position(claim.start) < position(range.start)
                || position(claim.end) > position(range.end)
                || position(claim.start) >= position(claim.end)
        }) {
            return reading;
        }
        reading.claims = body
            .observations
            .into_iter()
            .map(|claim| {
                let mut source_range = range.clone();
                source_range.start = claim.start;
                source_range.end = claim.end;
                source_range.meaning = volicord_repository_intelligence::RangeMeaning::Symbol;
                CodeBehaviorClaim {
                    kind: claim.kind,
                    expression: claim.expression,
                    source_range,
                }
            })
            .collect();
        reading.omitted_count = body.omitted_count;
        reading.state = match entity.freshness.state {
            FreshnessState::Stale => CodeExplanationState::Stale,
            FreshnessState::Unknown => CodeExplanationState::Unavailable,
            FreshnessState::Current
                if !entity.diagnostics.is_empty()
                    || !extension.diagnostics.is_empty()
                    || body.omitted_count > 0 =>
            {
                CodeExplanationState::Partial
            }
            FreshnessState::Current => CodeExplanationState::Current,
        };
        reading.limitations = vec!["Source syntax only: branch execution, call outcomes, external effects and runtime/data flow were not observed.".into()];
        if reading.state != CodeExplanationState::Current {
            reading.limitations.push(format!("Explanation evidence is {:?}; inspect source freshness, parser diagnostics and omissions.", reading.state));
        }
        reading
    }
}

pub(crate) fn behavior_sentences(entity: &MapEntity, korean: bool) -> String {
    let behavior = &entity.behavior;
    if !matches!(
        behavior.state,
        CodeExplanationState::Current | CodeExplanationState::Partial
    ) || behavior.claims.is_empty()
    {
        return String::new();
    }
    let mut sentences = Vec::new();
    for claim in &behavior.claims {
        let verb = match (claim.kind, korean) {
            (BodyObservationKind::Inputs, false) => "declares inputs",
            (BodyObservationKind::Condition, false) => "contains a condition",
            (BodyObservationKind::Call, false) => "contains a call expression",
            (BodyObservationKind::Binding, false) => "binds a value with",
            (BodyObservationKind::Assignment, false) => "contains an assignment",
            (BodyObservationKind::Return, false) => "has a return expression",
            (BodyObservationKind::Documentation, false) => "documents its responsibility as",
            (BodyObservationKind::Inputs, true) => "입력 선언",
            (BodyObservationKind::Condition, true) => "조건식",
            (BodyObservationKind::Call, true) => "호출식",
            (BodyObservationKind::Binding, true) => "값 바인딩",
            (BodyObservationKind::Assignment, true) => "대입식",
            (BodyObservationKind::Return, true) => "반환식",
            (BodyObservationKind::Documentation, true) => "소스에 기록된 책임",
        };
        sentences.push(format!(
            "{} {verb}: `{}`.",
            entity.display_name, claim.expression
        ));
    }
    sentences.push(if korean { "소스 구문을 설명하며 분기 실행, 호출 결과와 외부 효과는 관찰되지 않았습니다." } else { "These source expressions do not establish branch execution, call outcomes or external effects." }.into());
    sentences.join(" ")
}
