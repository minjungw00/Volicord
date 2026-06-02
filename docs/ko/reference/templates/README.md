# 템플릿 참조

## 사용 시점

Projection 템플릿과 표시 카드가 렌더링하는 Markdown 형태를 확인할 때 이 파일들을 사용합니다. Projection 규칙, 권한 경계, 최신성 동작은 [문서 Projection 참조](../document-projection.md)가 정의합니다.

Owner 경계: 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. 현재 저장소 단계와 구현 인계 상태는 [구현 개요](../../build/implementation-overview.md#문서-수락-상태)에 있습니다.

이 디렉터리를 초기 구현에 모두 필요한 목록처럼 읽으면 안 됩니다. 아래 계층 표는 Core status output, 최소 사용자 대상 읽기용 요약, agency assurance report, operations/export report, future/diagnostic projection을 구분합니다.

## 산출물 계층

Projection과 card shape는 다섯 단계의 산출물 계층을 지원합니다.

| 산출물 계층 | 포함되는 것 | Rule |
|---|---|---|
| Core status output | Core state에서 온 최소 현재 상태, 막힘, 다음 허용 행동, ref, freshness fact. | v0.1은 plain structured output을 반환해도 됩니다. Compact card는 선택 사항이며 full projection support를 뜻하지 않습니다. |
| User-facing MVP summaries | 현재 작업 상태, 사용자 판단 요청, 근거 요약, 닫기 준비 상태 / blocker 요약. | v0.2 사용자 가치를 지원하는 데 필요하지만, status/next text, compact card, 최소 `TASK` section으로 렌더링할 수 있습니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만 필수 projection kind를 늘리지는 않습니다. |
| Agency assurance reports | 해당 profile이 켜졌을 때의 approval, 수동 QA, verification, waiver, assurance card/report view. | v0.3 profile 범위입니다. 이 view가 v0.1이나 최소 v0.2 요구사항이 되지는 않습니다. |
| Operations/export reports | Export, release-handoff, projection freshness, artifact-integrity, operator report view. | Operations support가 켜진 v0.4 profile 범위입니다. Report는 읽기용 view이며 운영 권한이 아닙니다. |
| Future/diagnostic projections | Detailed Evidence Manifest, detailed Eval, Run Summary, TDD Trace, Module Map, Interface Contract, Journey Card 또는 Journey Spine-style view, standalone Decision Packet Markdown, design/domain-language map. | 필요할 때만 가져오거나 owner가 승격한 later-profile output으로 둡니다. 첫 runnable slice나 최소 사용자 대상 MVP의 필수 항목이 아닙니다. |

## 템플릿 구현 계층

템플릿 구현 계층은 렌더링 shape의 단계를 나눌 뿐 authority를 바꾸지 않습니다.

| 계층 | Templates | Rule |
|---|---|---|
| Core status output | [Compact Status Card](compact-status-card.md) 또는 동등한 status/blocker response shape | 구현자가 card shape를 선택할 때 current Core state에서 오는 최소 read-only status입니다. Plain structured response만으로 충분하며 persisted Markdown projection job이나 template renderer는 필요하지 않습니다. |
| User-facing MVP summaries | [TASK](task.md) 최소 continuity summary; standalone `DEC` `ProjectionKind`가 아닌 [Decision Packet 사용자 판단 요청 display/card shape](decision-packet.md); active direct-work profile을 위한 optional [DIRECT-RESULT](direct-result.md) | 현재 상태, 사용자 판단 요청, 근거 요약, 닫기 준비 상태/blocker를 보여줄 만큼이면 됩니다. 작업 수락과 잔여 위험 사실은 관련 있을 때 distinct하게 남지만 필수 projection kind를 늘리지는 않습니다. Standalone persisted `DEC` Markdown은 standalone Decision Packet projection 기능이 켜진 경우에만 사용합니다. |
| Agency assurance reports | [APR](approval.md), [Approval Card](approval-card.md), [MANUAL-QA](manual-qa.md), [Manual QA Card](manual-qa-card.md), [Verification Result Card](verification-result-card.md) | 해당 approval, 수동 QA, waiver, verification, assurance profile이 active일 때만 사용합니다. |
| Operations/export reports | [EXPORT](export.md) | Export, release-handoff, operations report support가 켜졌을 때만 사용합니다. Standalone Markdown report는 Core state나 artifact ref를 대체하지 않습니다. |
| Future/diagnostic projections | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [TDD-TRACE](tdd-trace.md), [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [DESIGN](design.md), [JOURNEY-CARD](journey-card.md), enabled 상태의 standalone `DEC` Markdown | Detailed reference, diagnostic, stewardship, map, trace, persisted Journey Card, Journey Spine-style, standalone Decision Packet, detailed Evaluation view입니다. Owner가 승격한 later profile에서 사용할 수 있게 유지하되 초기 필수 범위로 만들지 않습니다. |

코어 권한 조각(v0.1 Core Authority Slice)은 넓은 template rendering이나 full projection renderer를 요구하지 않습니다. 필요한 산출물은 Build가 이름 붙인 구조화된 상태/막힘 응답이며, 그 compact card가 가장 단순한 구현 선택일 때만 이 템플릿으로 렌더링할 수 있습니다. 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)는 사용자가 현재 작업 상태, 사용자 판단, 근거, 닫기 막힘을 이해할 만큼의 파생 산출물이 필요합니다. 작업 수락과 잔여 위험은 관련 있을 때 별도 Core meaning으로 남지만, 그 최소 요약 안에서 다루고 필수 projection kind를 늘리지 않습니다. 이것은 지원 표시 범위이지 단계의 주된 정체성이 아니며, Run Summary, Evidence Manifest, detailed Eval, TDD Trace, Journey Card, Module Map, Interface Contract, Export projection polish를 요구하지는 않습니다.

`Future/diagnostic projections`는 later-profile 또는 diagnostic 범위라는 뜻이며, 자동으로 v1+ 전용이라는 뜻은 아닙니다.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection은 owner record와 ref에서 나온 readable view입니다. Kernel field, MCP schema, SQLite DDL, gate behavior, artifact integrity rule을 재정의하면 안 됩니다.

렌더링 placeholder, label, table column, 예시 front matter key는 표시를 위한 template binding입니다. Binding은 기존 owner record field 또는 ref를 보여주거나, template이 나열한 source record에서 파생한 표시 전용 요약이어야 합니다. Source record 또는 ref가 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, unavailable/blocking note로 렌더링합니다.

여러 source가 관련될 때 compact authority display는 짧은 refs 줄을 우선 사용합니다. 예: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, `freshness=`. 이 label은 기존 ref, redaction state, projection freshness를 가리킬 뿐이며 schema field나 authority record가 아닙니다.

파생 표시 요약에는 `approval_covers`, `approval_does_not_cover`, `secret_exposure_boundary` 같은 Approval boundary line, close context, close blocker, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, judgment-context 관련 summary가 포함됩니다. 이 이름들은 새로운 기준 기록, schema field, DDL column, `ProjectionKind` value, gate, 권한을 만드는 입력이나 권한 경로가 아닙니다. Label 자체는 검증 입력으로 사용하면 안 됩니다. 검증기는 그 label이 요약하는 owner record, ref, gate, artifact, Decision Packet을 읽어야 합니다.

렌더링 예시는 이 경계를 독자가 바로 볼 수 있어야 합니다. `source_state_version`은 렌더링에 사용한 상태 clock을 가리키고, `projection_version` 또는 projection status는 렌더/template/job 보기를 가리키며, `updated_at`은 그 보기가 만들어진 시각을 가리킵니다. 최신성 줄(freshness line)은 이 보기가 source record와 아직 맞는지 표시할 뿐이며 Task result, gate value, 민감 동작 승인, 작업 수락, evidence, close readiness, Core state rollback이 아닙니다.

관리 영역(managed block)은 projector가 소유하는 표시 영역입니다. 관리 영역을 직접 편집한 내용은 상태 변경이 아니라 drift이며 reconcile candidate가 되어야 합니다. `User Notes and Proposals` 같은 사람이 편집할 수 있는 section은 제안 접점입니다. Proposal -> reconcile item -> 관련 `state.sqlite.task_events` row가 있는 accepted Core state-changing action을 거쳐야 상태가 되며, 그렇지 않으면 rejected, deferred, note-only content로 남습니다.

Artifact ref를 렌더링하는 모든 템플릿은 `redaction_state`를 보존해야 합니다. 큰 log, diff, trace, screenshot, bundle, recording, 민감한 artifact body는 기본적으로 embed하지 않고 `ArtifactRef`로 참조합니다. `secret_omitted` entry는 안전한 note 또는 handle을 보여줄 수 있고, 보이는 nonsecret evidence만 뒷받침할 수 있습니다. `blocked` entry는 커밋된 metadata-only notice를 사용할 수 없는 입력으로 보여줍니다. 템플릿은 생략된 secret/PII 값 또는 차단된 원본 payload를 inline 표시하거나 재구성하거나 요약하거나 export하면 안 됩니다.

`redaction_availability_summary`, 생략/차단 영향 line, `이후 영향` column 같은 표시 field는 표시 전용 요약일 뿐입니다. 이 값들은 `ArtifactRef.redaction_state`, owner 기록, 이후 gate, 근거, QA, 검증, projection, export, Release Handoff 상태에서 파생됩니다.

Decision Packet visibility는 standalone `DEC` Markdown projection에 의존하지 않습니다. Required surfaces는 active Decision Packet을 `TASK`, status/next response, judgment-context resource, decision-packet resource를 통해 계속 보여줘야 합니다. Standalone `DEC`는 해당 projection이 켜져 있을 때만 쓰는 선택적 렌더링 보기입니다.

Decision Packet 표시는 decision title, `judgment_category`, `judgment_route`, `display_depth`, 왜 지금 필요한지, 사용자가 정확히 판단하는 것, 간결한 options 또는 상세 trade-offs, recommendation, uncertainty, deferral consequence, 해당하는 경우 residual risk 같은 기준 schema field와 읽기용 shape field를 포함할 수 있습니다. `judgment_route`는 owner path와 recorded-answer route입니다. `display_depth`는 schema가 소유하는 prompt depth입니다. Validator는 `judgment_route`와 함께 matching `judgment_payload`와 route-specific required field를 validate합니다. 값은 `simple`, `tradeoff`, `high-risk`, `close-affecting`입니다. `judgment_category`는 schema가 소유하는 enum이며 값은 `product_ux`, `technical_architecture`, `security_privacy`, `scope_autonomy`, `qa_verification`, `work_acceptance`, `residual_risk`, `mixed`입니다. Template은 이 값을 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, 범위/자율성 판단, QA/verification, 작업 수락, 잔여 위험, 복합 같은 label로 렌더링할 수 있습니다. Judgment가 여러 영역에 걸쳐 있으면 category를 배타적으로 다루지 말고 부차적인 고려사항을 trade-offs, affected gates, risk, evidence, follow-up에 렌더링해야 합니다. `display_depth` 또는 `judgment_category`에서 파생한 표시용 label은 독자를 돕지만 schema field, `ProjectionKind` value, gate, owner record, validator input, close aggregation rule, authority path, `judgment_route` replacement가 아닙니다.

표시 카드는 서로 다른 세 문제를 구분해야 합니다. 오래된 projection(stale projection)은 읽기용 보기가 source record보다 뒤처졌을 수 있다는 뜻이고, stale state 또는 stale evidence는 기반 state, baseline, artifact input이 이동했거나 부족해졌다는 뜻이며, MCP에 닿지 못하는 상태(MCP unavailable)는 접점이 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. 상태 변경은 owner record와 Core transition만 할 수 있습니다.

닫기와 assurance 표시는 self-checked 작업, `detached_verified` assurance, waived verification, QA waiver, residual-risk accepted `completed_with_risk_accepted` close를 서로 다른 label로 유지해야 합니다. 같은 compact card에 함께 나타날 수는 있지만, 각 상태를 뒷받침하는 owner ref 없이 "done", "verified", "accepted"로 뭉개면 안 됩니다.

## Future/diagnostic Projection 템플릿

- [DESIGN](design.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [INTERFACE-CONTRACT](interface-contract.md)
- [JOURNEY-CARD](journey-card.md)
- [MODULE-MAP](module-map.md)
- [RUN-SUMMARY](run-summary.md)
- [TDD-TRACE](tdd-trace.md)

## Core Status Output

- [Compact Status Card](compact-status-card.md)

## User-Facing MVP Summary Shapes

- [TASK](task.md) 최소 continuity summary
- [Decision Packet 사용자 판단 요청 display shape](decision-packet.md)
- [DIRECT-RESULT](direct-result.md), direct-work compact result display가 active일 때만

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
