# 템플릿 참조

## 사용 시점

Projection 템플릿과 표시 카드가 렌더링하는 Markdown 형태를 확인할 때 이 파일들을 사용합니다. Projection 규칙, 권한 경계, 최신성 동작은 [문서 Projection 참조](../document-projection.md)가 정의합니다.

권한 규칙:

- Projection은 Core가 소유한 상태 기록과 아티팩트 참조에서 파생됩니다.
- Projection은 Core 상태가 아닙니다.
- 사용자가 Projection을 편집해도 그 내용이 자동으로 받아들여진 상태가 되지는 않습니다.
- Chat과 Markdown은 Core 상태를 덮어쓸 수 없습니다.

Owner 경계: 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../../build/implementation-overview.md#문서-수락-상태)에 있습니다.

이 디렉터리는 shape catalog이지 stage checklist가 아닙니다. Repository에 template이 있다는 사실은 그 template이 현재 단계에서 필수라는 뜻이 아닙니다.

## Projection audience

Audience를 분리합니다.

| Audience | Shape | Rule |
|---|---|---|
| User-facing compact card | [Compact Status Card](compact-status-card.md) | v0.2 First User-Value Slice projection입니다. 하나의 작은 현재 상태 card입니다. |
| Agent compact context/reference payload | Structured refs, blocker label, source clock, freshness, next-action hint | 파생된 지원 payload입니다. 기본은 compact이며, 단계상 꼭 필요할 때만 full report body를 pull합니다. |
| Future/diagnostic reports | `TASK`, Journey Card/Spine, Run Summary, detailed Evidence Manifest, detailed Eval, full 수동 QA, TDD Trace, Domain Language, Module Map, Interface Contract, Design, Export, full Approval Card, 그 밖의 polished report | Later/profile 또는 diagnostic output입니다. 표시 전용이며 authority가 아닙니다. |

## v0.2 First User-Value Slice projection

v0.2 First User-Value Slice projection은 하나의 compact status card입니다. 반드시 다음을 보여줘야 합니다.

- 현재 Task 요약
- 작업 모양
- 현재 범위와 하지 않을 일
- 대기 중인 사용자 판단
- 활성 blocker
- 다음 안전한 행동
- 알려진 근거 또는 근거 gap
- close blocker
- 보이는 잔여 위험
- guarantee level
- source/freshness ref

Card는 사용자가 읽기 쉬우면서도 에이전트가 부담 없이 다룰 만큼 작아야 합니다. Schema field, DDL, event log, full artifact, full reference doc, full Evidence Manifest, full report body를 쏟아내면 안 됩니다.

## Template-to-stage matrix

| Template | Audience | First active stage | Authority status | Notes |
|---|---|---|---|---|
| [Compact Status Card](compact-status-card.md) | User-facing compact card; agent compact context source | v0.2 First User-Value Slice projection; v0.1 status rendering에서는 optional | 파생 표시 전용 | 유일한 v0.2 First User-Value Slice projection shape입니다. v0.1은 plain structured output만으로도 충분합니다. |
| [Decision Packet display](decision-packet.md) | 사용자 판단 prompt/display | 사용자 판단 flow가 active인 v0.2 | `state.sqlite.decision_packets`에서 파생된 표시입니다. Standalone authority가 아닙니다. | 필요한 판단은 status/next 또는 resource를 통해 나타날 수 있습니다. Standalone `DEC` Markdown은 later optional입니다. |
| [TASK](task.md) | Continuity/reference report | Later/profile 또는 diagnostic | 파생 표시 전용 | v0.2 First User-Value Slice projection이 아닙니다. Expanded continuity section은 later polish입니다. |
| [DIRECT-RESULT](direct-result.md) | Compact direct-work result | Direct-work display가 active인 later/profile | 파생 표시 전용 | Optional convenience shape입니다. Compact status card MVP에는 필요하지 않습니다. |
| [APR](approval.md) | Sensitive-action approval report | v0.3 agency assurance profile | Approval과 Decision Packet ref를 표시합니다. Approval을 부여하지 않습니다. | Approval support/profile이 active인 뒤에만 사용합니다. |
| [Approval Card](approval-card.md) | Sensitive-action approval prompt/card | v0.3 agency assurance profile | Approval boundary를 표시합니다. Write를 허가하지 않습니다. | Full Approval Card는 v0.2 First User-Value Slice가 아닙니다. |
| [MANUAL-QA](manual-qa.md) | 수동 QA report | v0.3 agency assurance profile | `manual_qa_records`/`qa_gate`를 표시합니다. QA를 수행하지 않습니다. | Full 수동 QA projection은 later/profile scope입니다. |
| [Manual QA Card](manual-qa-card.md) | 수동 QA prompt/card | v0.3 agency assurance profile | QA requirement/waiver ref를 표시합니다. QA를 기록하지 않습니다. | Full Manual QA Card는 later/profile scope입니다. |
| [Verification Result Card](verification-result-card.md) | Verification/Eval display | v0.3 agency assurance profile | Eval/gate ref를 표시합니다. 그 자체로 verification을 만들지 않습니다. | Verification profile이 active일 때의 compact assurance display입니다. |
| [RUN-SUMMARY](run-summary.md) | Diagnostic run report | Future/diagnostic 또는 owner-promoted profile | Run/artifact ref에서 파생된 표시입니다. | v0.2에 필요하지 않습니다. |
| [EVIDENCE-MANIFEST](evidence-manifest.md) | Detailed evidence report | Future/diagnostic 또는 owner-promoted profile | Evidence record와 artifact ref를 표시합니다. Evidence 자체가 아닙니다. | v0.2 card는 evidence summary/gap만 보여줍니다. |
| [EVAL](eval.md) | Detailed verification report | Future/diagnostic 또는 owner-promoted profile | Eval ref를 표시합니다. Assurance를 만들지 않습니다. | Detailed Eval은 v0.2가 아닙니다. |
| [TDD-TRACE](tdd-trace.md) | TDD diagnostic/reference | Future/diagnostic 또는 owner-promoted profile | TDD ref를 표시합니다. 그 자체로 gate가 아닙니다. | Later policy/profile output입니다. |
| [DOMAIN-LANGUAGE](domain-language.md) | Stewardship/reference report | Future/diagnostic 또는 owner-promoted profile | `domain_terms`를 표시합니다. Term authority가 아닙니다. | Later reference view입니다. |
| [MODULE-MAP](module-map.md) | Stewardship/reference report | Future/diagnostic 또는 owner-promoted profile | `module_map_items`를 표시합니다. Module authority가 아닙니다. | Later reference view입니다. |
| [INTERFACE-CONTRACT](interface-contract.md) | Stewardship/reference report | Future/diagnostic 또는 owner-promoted profile | `interface_contracts`를 표시합니다. Contract authority가 아닙니다. | Later reference view입니다. |
| [DESIGN](design.md) | Design/reference report | Future/diagnostic 또는 owner-promoted profile | Design record/proposal을 표시합니다. Design authority가 아닙니다. | Later standalone projection입니다. |
| [JOURNEY-CARD](journey-card.md) | Journey/resume diagnostic card | Future/diagnostic 또는 owner-promoted profile | 파생 current-position display일 뿐입니다. | v0.2는 compact status card를 사용합니다. |
| [EXPORT](export.md) | Operations/export report | v0.4 operations/export profile | Snapshot과 artifact ref를 나열합니다. Core state나 artifact authority가 아닙니다. | Optional handoff/report output입니다. |

`Future/diagnostic projections`는 later-profile 또는 diagnostic 범위라는 뜻이며, 자동으로 v1+ 전용이라는 뜻은 아닙니다.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection은 owner record와 ref에서 나온 readable view입니다. Kernel field, MCP schema, SQLite DDL, gate behavior, artifact integrity rule을 재정의하면 안 됩니다.

렌더링 placeholder, label, table column, 예시 front matter key는 표시를 위한 template binding입니다. Binding은 기존 owner record field 또는 ref를 보여주거나, template이 나열한 source record에서 파생한 표시 전용 요약이어야 합니다. Source record 또는 ref가 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, unavailable/blocking note로 렌더링합니다.

여러 source가 관련될 때 compact authority display는 짧은 refs 줄을 우선 사용합니다. 예: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, `freshness=`. 이 label은 기존 ref, redaction state, projection freshness를 가리킬 뿐이며 schema field나 authority record가 아닙니다.

파생 표시 요약에는 `approval_covers`, `approval_does_not_cover`, `secret_exposure_boundary` 같은 Approval boundary line, close context, close blocker, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, judgment-context 관련 summary가 포함됩니다. 이 이름들은 새로운 기준 기록, schema field, DDL column, `ProjectionKind` value, gate, 권한을 만드는 입력이나 권한 경로가 아닙니다. Label 자체는 검증 입력으로 사용하면 안 됩니다. 검증기는 그 label이 요약하는 owner record, ref, gate, artifact, Decision Packet을 읽어야 합니다.

렌더링 예시는 이 경계를 독자가 바로 볼 수 있어야 합니다. `source_state_version`은 렌더링에 사용한 상태 clock을 가리키고, `projection_version` 또는 projection status는 렌더/template/job 보기를 가리키며, `updated_at`은 그 보기가 만들어진 시각을 가리킵니다. 최신성 줄(freshness line)은 이 보기가 source record와 아직 맞는지 표시할 뿐이며 Task result, gate value, 민감 동작 승인, 작업 수락, evidence, close readiness, Core state rollback이 아닙니다.

관리 영역(managed block)은 projector가 소유하는 표시 영역입니다. 관리 영역을 직접 편집한 내용은 상태 변경이 아니라 drift이며 reconcile candidate가 되어야 합니다. `User Notes and Proposals` 같은 사람이 편집할 수 있는 section은 제안 접점입니다. Proposal -> reconcile item -> 관련 `state.sqlite.task_events` row가 있는 accepted Core state-changing action을 거쳐야 상태가 되며, 그렇지 않으면 rejected, deferred, note-only content로 남습니다.

Artifact ref를 렌더링하는 모든 템플릿은 `redaction_state`를 보존해야 합니다. 큰 log, diff, trace, screenshot, bundle, recording, 민감한 artifact body는 기본적으로 embed하지 않고 `ArtifactRef`로 참조합니다. `secret_omitted` entry는 안전한 note 또는 handle을 보여줄 수 있고, 보이는 nonsecret evidence만 뒷받침할 수 있습니다. `blocked` entry는 커밋된 metadata-only notice를 사용할 수 없는 입력으로 보여줍니다. 템플릿은 생략된 secret/PII 값 또는 차단된 원본 payload를 inline 표시하거나 재구성하거나 요약하거나 export하면 안 됩니다.

`redaction_availability_summary`, 생략/차단 영향 line, `이후 영향` column 같은 표시 field는 표시 전용 요약일 뿐입니다. 이 값들은 `ArtifactRef.redaction_state`, owner 기록, 이후 gate, 근거, QA, 검증, projection, export, Release Handoff 상태에서 파생됩니다.

Decision Packet visibility는 standalone `DEC` Markdown projection에 의존하지 않습니다. Required surfaces는 active Decision Packet을 compact status card, status/next response, judgment-context resource, decision-packet resource, 또는 dedicated prompt를 통해 보여줄 수 있습니다. Later continuity profile이 active이면 `TASK`도 이를 보여줄 수 있습니다. Standalone `DEC`는 해당 projection이 켜져 있을 때만 쓰는 선택적 렌더링 보기입니다.

Decision Packet 표시는 decision title, `judgment_category`, `judgment_route`, `display_depth`, 왜 지금 필요한지, 사용자가 정확히 판단하는 것, 간결한 options 또는 상세 trade-offs, recommendation, uncertainty, deferral consequence, 해당하는 경우 residual risk 같은 기준 schema field와 읽기용 shape field를 포함할 수 있습니다. `judgment_route`는 owner path와 recorded-answer route입니다. `display_depth`는 schema가 소유하는 prompt depth입니다. Validator는 `judgment_route`와 함께 matching `judgment_payload`와 route-specific required field를 validate합니다. 값은 `simple`, `tradeoff`, `high-risk`, `close-affecting`입니다. `judgment_category`는 schema가 소유하는 enum이며 값은 `product_ux`, `technical_architecture`, `security_privacy`, `scope_autonomy`, `qa_verification`, `work_acceptance`, `residual_risk`, `mixed`입니다. Template은 이 값을 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, 범위/자율성 판단, QA/verification, 작업 수락, 잔여 위험, 복합 같은 label로 렌더링할 수 있습니다. Judgment가 여러 영역에 걸쳐 있으면 category를 배타적으로 다루지 말고 부차적인 고려사항을 trade-offs, affected gates, risk, evidence, follow-up에 렌더링해야 합니다. `display_depth` 또는 `judgment_category`에서 파생한 표시용 label은 독자를 돕지만 schema field, `ProjectionKind` value, gate, owner record, validator input, close aggregation rule, authority path, `judgment_route` replacement가 아닙니다.

표시 카드는 서로 다른 세 문제를 구분해야 합니다. 오래된 projection(stale projection)은 읽기용 보기가 source record보다 뒤처졌을 수 있다는 뜻이고, stale state 또는 stale evidence는 기반 state, baseline, artifact input이 이동했거나 부족해졌다는 뜻이며, MCP에 닿지 못하는 상태(MCP unavailable)는 접점이 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. 상태 변경은 owner record와 Core transition만 할 수 있습니다.

닫기와 assurance 표시는 self-checked 작업, `detached_verified` assurance, waived verification, QA waiver, residual-risk accepted `completed_with_risk_accepted` close를 서로 다른 label로 유지해야 합니다. 같은 compact card에 함께 나타날 수는 있지만, 각 상태를 뒷받침하는 owner ref 없이 "done", "verified", "accepted"로 뭉개면 안 됩니다.

## Future/diagnostic Projection 템플릿

- [DESIGN](design.md)
- [DIRECT-RESULT](direct-result.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [INTERFACE-CONTRACT](interface-contract.md)
- [JOURNEY-CARD](journey-card.md)
- [MODULE-MAP](module-map.md)
- [RUN-SUMMARY](run-summary.md)
- [TASK](task.md)
- [TDD-TRACE](tdd-trace.md)

## Core Status Output

- [Compact Status Card](compact-status-card.md)

## User Judgment Prompt Shapes

- [Decision Packet 사용자 판단 요청 display shape](decision-packet.md), standalone `DEC` Markdown 아님

## Agency Assurance Report Shapes

- [APR](approval.md)
- [Approval Card](approval-card.md)
- [MANUAL-QA](manual-qa.md)
- [수동 QA Card](manual-qa-card.md)
- [Verification Result Card](verification-result-card.md)

## Operations/Export Report Shapes

- [EXPORT](export.md)

## 메모

이 디렉터리는 Projection 템플릿 본문과 표시 카드 형태의 기준 참조 위치입니다.
