//! Presentation of admitted independent facts. Never parse a quotation into a
//! check identity, assertion, environment or a causal retest relationship.
use super::*;
use volicord_projections::*;

fn detail(value: &EvidenceDetail<String>, locale: ViewerLocale) -> &str {
    match value {
        EvidenceDetail::Observed(value) => value,
        EvidenceDetail::NotObserved => text(locale, "Not observed", "관측되지 않음"),
        EvidenceDetail::Unavailable => text(locale, "Unavailable", "이용 불가"),
    }
}
fn body_label(state: CommandBodyState, locale: ViewerLocale) -> &'static str {
    match state {
        CommandBodyState::Available => text(
            locale,
            "Available retained observation",
            "보존된 관측 이용 가능",
        ),
        CommandBodyState::NotRetained => text(locale, "Body not retained", "본문 미보존"),
        CommandBodyState::Deleted => text(locale, "Body deleted", "본문 삭제됨"),
        CommandBodyState::Expired => text(locale, "Body expired", "본문 보존 기간 만료"),
        CommandBodyState::Changed => text(locale, "Body changed", "본문 변경됨"),
        CommandBodyState::Invalidated => text(locale, "Body invalidated", "본문 무효화됨"),
        CommandBodyState::Unavailable => text(locale, "Body unavailable", "본문 이용 불가"),
    }
}
fn source_state(state: SourceEvidenceState, locale: ViewerLocale) -> &'static str {
    match state {
        SourceEvidenceState::Available => text(locale, "Available", "이용 가능"),
        SourceEvidenceState::Current => text(locale, "Current", "현재 유효"),
        SourceEvidenceState::Stale => text(locale, "Stale", "오래됨"),
        SourceEvidenceState::Unavailable => text(locale, "Unavailable", "이용 불가"),
        SourceEvidenceState::Unknown => text(locale, "Unknown", "알 수 없음"),
    }
}
fn termination(state: ExecutionTermination, locale: ViewerLocale) -> &'static str {
    match state {
        ExecutionTermination::Exited => text(locale, "Exited", "종료됨"),
        ExecutionTermination::SpawnFailed => text(locale, "Could not start", "시작 실패"),
        ExecutionTermination::Signaled => text(locale, "Terminated by signal", "시그널로 종료됨"),
        ExecutionTermination::Indeterminate => {
            text(locale, "Termination unknown", "종료 상태 알 수 없음")
        }
    }
}
fn instant(value: i64, locale: ViewerLocale) -> String {
    timestamp_label(TimestampMicros::from_unix_micros(value), locale)
}
fn stream(html: &mut String, label: &str, stream: &StreamCapture, locale: ViewerLocale) {
    let representation = match stream.representation {
        StreamRepresentation::WholeStream => {
            text(locale, "Whole captured stream", "캡처된 전체 스트림")
        }
        StreamRepresentation::BoundedExcerpt => text(locale, "Bounded excerpt", "제한된 발췌"),
        StreamRepresentation::Unavailable => text(locale, "Unavailable", "이용 불가"),
    };
    let completeness = match stream.capture_completeness {
        CaptureCompleteness::Complete => text(locale, "Complete capture", "완전한 캡처"),
        CaptureCompleteness::Incomplete => text(locale, "Incomplete capture", "불완전한 캡처"),
        CaptureCompleteness::Unobserved => text(
            locale,
            "Capture completeness unobserved",
            "캡처 완전성 미관측",
        ),
    };
    definition(html, label, &format!("{representation} · {completeness}"));
    definition(
        html,
        text(locale, "Observed bytes", "관측된 바이트"),
        &stream.observed_bytes.to_string(),
    );
    definition(
        html,
        text(locale, "Omitted bytes", "생략된 바이트"),
        &stream
            .omitted_bytes
            .map(|n| n.to_string())
            .unwrap_or_else(|| text(locale, "Not observed", "관측되지 않음").into()),
    );
}
fn check(
    html: &mut String,
    r: &ViewerRequest,
    answers: &QuestionAnswers,
    check: &VerificationReading,
) {
    html.push_str(&format!("<section class=\"verification-check\" data-checkpoint=\"{}\" data-check-index=\"{}\"><h4>{}: {}</h4><dl>",
        escape(&check.original.checkpoint_id),check.original.verification_index,
        text(r.locale,"Checkpoint report", "Checkpoint 보고"),check.reported_state.label(r.locale.fixed())));
    definition(
        html,
        text(r.locale, "Check purpose / name", "검증 목적 / 이름"),
        detail(&check.intended_check, r.locale),
    );
    definition(
        html,
        text(r.locale, "Reported at", "보고 시각"),
        &instant(check.reported_at_unix_micros, r.locale),
    );
    definition(
        html,
        text(r.locale, "Assertion result", "Assertion 결과"),
        detail(&check.assertion_result, r.locale),
    );
    definition(
        html,
        text(r.locale, "Applicability", "적용성"),
        match check.applicability {
            VerificationApplicability::HistoricalAfterRecordedChange => text(
                r.locale,
                "Historical: later changes were recorded",
                "과거 결과: 이후 변경이 기록됨",
            ),
            VerificationApplicability::Unknown => text(
                r.locale,
                "Current applicability unestablished",
                "현재 적용 가능성 미확인",
            ),
        },
    );
    html.push_str("</dl>");
    if let Some(quote) = &check.original_outcome {
        html.push_str(&format!(
            "<p class=\"source-note\">{}</p><blockquote lang=\"\">{}</blockquote>",
            text(
                r.locale,
                "Original verification report quotation",
                "원래 검증 보고 인용"
            ),
            escape(quote)
        ));
    }
    html.push_str(&format!("<p class=\"source-note\">{}</p><details><summary>{}</summary><p><a href=\"#{}\">Checkpoint {}@{} · verification[{}]</a></p><p>{}: {}</p><p>{}: {}</p></details></section>",
        text(r.locale,"Comparable retest: not established. A different run's pass does not resolve this report.", "비교 가능한 재검증: 미확인. 다른 실행의 통과로 이 보고가 해결되지는 않습니다."),
        text(r.locale,"Inspect original report identity", "원래 보고 식별자 검사"),document_anchor(answers,"audit"),
        escape(&check.original.checkpoint_id),check.original.checkpoint_revision,check.original.verification_index,
        text(r.locale,"Command Source", "명령 Source"),escape(check.source_id.as_deref().unwrap_or(text(r.locale,"None recorded", "기록 없음"))),
        text(r.locale,"Later changed Checkpoints", "이후 변경 Checkpoint"),escape(&check.later_changed_checkpoint_ids.join(", "))));
}

pub(super) fn render_verification(
    html: &mut String,
    r: &ViewerRequest,
    answers: &QuestionAnswers,
    report: &WorkVerificationReport,
) {
    html.push_str(&format!("<section class=\"reading-section verification-report\" id=\"{}\" tabindex=\"-1\"><h2>{}</h2><p>{}</p><p class=\"source-note\">{}</p>",
        document_anchor(answers,"verification"),text(r.locale,"Verification report", "검증 보고"),
        text(r.locale,"Observed executions and Checkpoint reports have separate provenance. Exit status proves no assertions.", "실행 관측과 Checkpoint 보고는 출처가 다릅니다. 종료 코드만으로 assertion을 확정하지 않습니다."),
        text(r.locale,"Comparison unresolved: no explicit comparable check/run relationship is retained. Environment, assertion detail and coverage are shown only when observed.","비교 미해결: 비교 가능한 검증·실행 관계가 명시적으로 보존되지 않았습니다. 환경·assertion 상세·coverage는 관측되었을 때만 표시합니다.")));
    if report.checks.is_empty() {
        html.push_str(&format!(
            "<p>{}</p>",
            text(
                r.locale,
                "No verification recorded in a Checkpoint",
                "Checkpoint에 기록된 검증 없음"
            )
        ));
    }
    // Identity joins only. Equal labels, statuses and adjacent times never merge runs.
    for execution in report.executions.iter().take(32) {
        html.push_str(&format!("<article class=\"verification-execution\" data-execution-source=\"{}\"><header><h3>{}</h3><p class=\"source-note\">{}</p></header><dl>",escape(&execution.source_id),escape(&execution.command_label),text(r.locale,"Recorded command label · one exact Source execution", "기록된 명령 라벨 · 하나의 정확한 Source 실행")));
        definition(
            html,
            text(r.locale, "Process termination", "프로세스 종료"),
            termination(execution.termination, r.locale),
        );
        definition(
            html,
            text(r.locale, "Numeric exit code", "숫자 종료 코드"),
            &execution
                .exit_code
                .map(|n| n.to_string())
                .unwrap_or_else(|| text(r.locale, "Not observed", "관측되지 않음").into()),
        );
        definition(
            html,
            text(r.locale, "Recorded at", "기록 시각"),
            &instant(execution.recorded_at_unix_micros, r.locale),
        );
        definition(
            html,
            text(r.locale, "Observation body", "관측 본문"),
            body_label(execution.body_state, r.locale),
        );
        definition(
            html,
            text(r.locale, "Source availability", "Source 가용성"),
            source_state(execution.source_status.availability, r.locale),
        );
        definition(
            html,
            text(r.locale, "Source freshness", "Source 최신성"),
            source_state(execution.source_status.freshness, r.locale),
        );
        definition(
            html,
            text(r.locale, "Environment", "환경"),
            detail(&execution.environment, r.locale),
        );
        definition(
            html,
            text(r.locale, "Assertion result", "Assertion 결과"),
            detail(&execution.assertion_result, r.locale),
        );
        html.push_str("</dl>");
        match &execution.observation {
            EvidenceDetail::Observed(o) => {
                html.push_str("<dl>");
                definition(html,text(r.locale,"Started", "시작 시각"),&instant(o.started_at_unix_micros,r.locale));
                definition(html,text(r.locale,"Ended", "종료 시각"),&instant(o.ended_at_unix_micros,r.locale));
                definition(html,text(r.locale,"Target limitation", "대상 한계"),text(r.locale,"Selected analysis only; execution-time tree was not rechecked", "선택된 분석이며 실행 시점의 트리는 재확인하지 않았습니다"));
                definition(html,text(r.locale,"Timeout detected", "시간 초과 감지"),if o.timeout_detected {text(r.locale,"Yes", "예")} else {text(r.locale,"No", "아니요")});
                definition(html,text(r.locale,"Cancellation requested", "취소 요청됨"),if o.cancellation_requested {text(r.locale,"Yes", "예")} else {text(r.locale,"No", "아니요")});
                definition(html,text(r.locale,"Signal number", "시그널 번호"),&o.signal_number.map(|n|n.to_string()).unwrap_or_else(||text(r.locale,"None observed", "관측 없음").into()));
                html.push_str("</dl><details><summary>");
                html.push_str(text(r.locale,"Target, capture and retention details", "대상·캡처·보존 상세"));
                html.push_str("</summary><dl>");
                definition(html,"Operation",&o.operation_id);
                definition(html,"Work revision",&o.work_revision.to_string());
                definition(html,"Analysis Snapshot",&o.analysis_snapshot_id);
                definition(html,"Repository Snapshot",&o.repository_snapshot_id);
                definition(html,"Repository Source",&o.repository_source_id);
                definition(html,text(r.locale,"Retained until", "보존 기한"),&instant(o.retained_until_unix_micros,r.locale));
                stream(html,"stdout",&o.stdout,r.locale);stream(html,"stderr",&o.stderr,r.locale);
                html.push_str("</dl></details>");
            },
            EvidenceDetail::NotObserved | EvidenceDetail::Unavailable => html.push_str(&format!("<p class=\"source-note\">{}</p>",text(r.locale,"Execution window, selected target and output capture are unavailable; only surviving canonical metadata is shown.","실행 시각·선택 대상·출력 캡처를 이용할 수 없습니다. 남아 있는 canonical 메타데이터만 표시합니다."))),
        }
        for c in report
            .checks
            .iter()
            .take(64)
            .filter(|c| c.source_id.as_deref() == Some(&execution.source_id))
        {
            check(html, r, answers, c);
        }
        if !report
            .checks
            .iter()
            .any(|c| c.source_id.as_deref() == Some(&execution.source_id))
        {
            html.push_str(&format!(
                "<p class=\"source-note\">{}</p>",
                text(
                    r.locale,
                    "No Checkpoint assertion report is linked to this execution.",
                    "이 실행에 연결된 Checkpoint assertion 보고가 없습니다."
                )
            ));
        }
        html.push_str(&format!(
            "<details><summary>{}</summary><dl>",
            text(r.locale, "Execution identity", "실행 식별자")
        ));
        definition(
            html,
            "Source",
            &format!("{}@{}", execution.source_id, execution.source_revision),
        );
        definition(
            html,
            "Invocation fingerprint",
            &execution.invocation_fingerprint,
        );
        html.push_str("</dl></details></article>");
    }
    for c in report.checks.iter().take(64).filter(|c| {
        !report
            .executions
            .iter()
            .take(32)
            .any(|e| c.source_id.as_deref() == Some(&e.source_id))
    }) {
        html.push_str(&format!(
            "<article class=\"verification-record-only\"><h3>{}</h3>",
            text(
                r.locale,
                "Recorded verification · execution details unavailable",
                "기록된 검증 · 실행 상세 이용 불가"
            )
        ));
        check(html, r, answers, c);
        html.push_str("</article>");
    }
    html.push_str(&format!(
        "<p class=\"source-note\">{}: {} / {} · {}: {} / {}. <a href=\"#{}\">{}</a></p></section>",
        text(r.locale, "Displayed reports / total", "표시된 보고 / 전체"),
        report.checks.len().min(64),
        report.checks.len(),
        text(
            r.locale,
            "Displayed executions / total",
            "표시된 실행 / 전체"
        ),
        report.executions.len().min(32),
        report.executions.len(),
        document_anchor(answers, "audit"),
        text(r.locale, "Inspect complete originals", "전체 원본 검사")
    ));
}
