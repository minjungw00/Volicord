# 템플릿 참조

## 사용 시점

Projection 템플릿과 표시 카드가 렌더링하는 Markdown 형태를 확인할 때 이 파일들을 사용합니다. Projection 규칙, 권한 경계, 최신성 동작은 [문서 Projection 참조](../document-projection.md)가 정의합니다.

Owner 경계: 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. 문서 수락과 별도의 구현 계획 준비 결정 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들 권한을 주지 않습니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 좁은 future smoke-check 작성 label입니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack)과 운영과 인계 팩(v0.4 Operations & Handoff Pack)은 agency assurance, operations, handoff behavior를 단단하게 만들고, v1+ Expansion은 owner 문서가 명시적으로 승격하고 증명하기 전까지 roadmap 범위에 남습니다.

이 디렉터리를 초기 구현에 모두 필요한 목록처럼 읽으면 안 됩니다. 아래 계층 표는 초기 필수 표시 형태, 초기 선택 표시 형태, 미래/진단 템플릿을 구분합니다.

## 산출물 계층

Projection과 card shape는 세 산출물 계층을 지원합니다.

| 산출물 계층 | 포함되는 것 | Rule |
|---|---|---|
| 사용자 읽기용 산출물 | 현재 작업 상태, 사용자 결정 요청, 근거 요약, 닫기 준비 상태 / blocker 요약, 필요한 경우 작업 수락 필요 여부/상태와 잔여 위험 표시. | 사용자 대상 MVP를 지원하는 데 필요하지만, status/next text, compact card, 최소 `TASK` section으로 렌더링할 수 있습니다. |
| 에이전트용 간결한 현재 맥락 | 다음 안전한 단계에 필요한 최소 current state: active Task, scope, 필요한 경우 active Change Unit, pending user decision, evidence/close blocker, next action, refs, freshness. | Compact하게 유지하고 긴 history나 detailed artifact를 embed하지 않습니다. |
| 참조/진단용 산출물 | 자세한 manifest, trace, map, Journey Card 또는 Journey Spine view, Run Summary, detailed Eval report, export bundle, operator report. | 필요할 때만 가져오거나 later profile 산출물로 둡니다. 첫 runnable slice나 최소 사용자 대상 MVP의 필수 항목이 아닙니다. |

## 템플릿 구현 계층

템플릿 구현 계층은 렌더링 shape의 단계를 나눌 뿐 authority를 바꾸지 않습니다.

| 계층 | Templates | Rule |
|---|---|---|
| 코어 권한 조각(v0.1 Core Authority Slice)에서 허용 | [Compact Status Card](compact-status-card.md) 또는 동등한 status/blocker response shape | 구현자가 card shape를 선택할 때 current Core state에서 오는 최소 read-only status입니다. Plain structured response만으로 충분하며 persisted Markdown projection job이나 template renderer는 필요하지 않습니다. |
| 사용자 대상 MVP에 필요 | [TASK](task.md) 최소 continuity summary; standalone `DEC` `ProjectionKind`가 아닌 [Decision Packet 사용자 결정 요청 display/card shape](decision-packet.md) | 현재 상태, 사용자 결정 요청, 근거 요약, 닫기 준비 상태/blocker, 작업 수락 필요 여부/상태, 잔여 위험 표시를 보여줄 만큼이면 됩니다. Standalone persisted `DEC` Markdown은 standalone Decision Packet projection 기능이 켜진 경우에만 사용합니다. |
| 초기 선택 사항 | [APR](approval.md), [Approval Card](approval-card.md), [DIRECT-RESULT](direct-result.md), [MANUAL-QA](manual-qa.md), [Manual QA Card](manual-qa-card.md), [Verification Result Card](verification-result-card.md) | 해당 approval, direct-work, 수동 QA, verification profile이 active일 때만 사용합니다. |
| 미래 / 진단 | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [TDD-TRACE](tdd-trace.md), [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [DESIGN](design.md), [EXPORT](export.md), [JOURNEY-CARD](journey-card.md) | Detailed reference, diagnostic, handoff, stewardship, map, trace, export view입니다. 에이전시 보증 팩(v0.3 Agency Assurance Pack), 운영과 인계 팩(v0.4 Operations & Handoff Pack) 또는 owner가 승격한 다른 later profile에서 사용할 수 있게 유지하되 초기 필수 범위로 만들지 않습니다. |

코어 권한 조각(v0.1 Core Authority Slice)은 넓은 template rendering이나 full projection renderer를 요구하지 않습니다. 필요한 산출물은 Build가 이름 붙인 구조화된 상태/막힘 응답이며, 그 compact card가 가장 단순한 구현 선택일 때만 이 템플릿으로 렌더링할 수 있습니다. 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)는 사용자가 범위, 사용자 결정, 근거, 닫기 준비 상태, 작업 수락, 잔여 위험을 이해할 만큼의 파생 산출물이 필요합니다. 이것은 지원 표시 범위이지 단계의 주된 정체성이 아니며, Run Summary, Evidence Manifest, detailed Eval, TDD Trace, Journey Card, Module Map, Interface Contract, Export projection polish를 요구하지는 않습니다.

`미래 / 진단`은 later-profile 또는 diagnostic 범위라는 뜻이며, 자동으로 v1+ 전용이라는 뜻은 아닙니다.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection은 owner record와 ref에서 나온 readable view입니다. Kernel field, MCP schema, SQLite DDL, gate behavior, artifact integrity rule을 재정의하면 안 됩니다.

렌더링 placeholder, label, table column, 예시 front matter key는 표시를 위한 template binding입니다. Binding은 기존 owner record field 또는 ref를 보여주거나, template이 나열한 source record에서 파생한 표시 전용 요약이어야 합니다. Source record 또는 ref가 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, unavailable/blocking note로 렌더링합니다.

여러 source가 관련될 때 compact authority display는 짧은 refs 줄을 우선 사용합니다. 예: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, `freshness=`. 이 label은 기존 ref, redaction state, projection freshness를 가리킬 뿐이며 schema field나 authority record가 아닙니다.

파생 표시 요약에는 `approval_covers`, `approval_does_not_cover`, `secret_exposure_boundary` 같은 Approval boundary line, close context, close blocker, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, judgment-context 관련 summary가 포함됩니다. 이 이름들은 새로운 기준 기록, schema field, DDL column, `ProjectionKind` value, gate, 권한을 만드는 입력이나 권한 경로가 아닙니다. 요약 대상인 owner record, ref, gate, artifact, Decision Packet을 통하지 않고 validator input으로 사용하면 안 됩니다.

렌더링 예시는 이 경계를 독자가 바로 볼 수 있어야 합니다. `source_state_version`은 렌더링에 사용한 상태 clock을 가리키고, `projection_version` 또는 projection status는 렌더/template/job 보기를 가리키며, `updated_at`은 그 보기가 만들어진 시각을 가리킵니다. 최신성 줄(freshness line)은 이 보기가 source record와 아직 맞는지 표시할 뿐이며 Task result, gate value, Approval, acceptance, evidence, close readiness, Core state rollback이 아닙니다.

관리 영역(managed block)은 projector가 소유하는 표시 영역입니다. 관리 영역을 직접 편집한 내용은 상태 변경이 아니라 drift이며 reconcile candidate가 되어야 합니다. `User Notes and Proposals` 같은 사람이 편집할 수 있는 section은 제안 접점입니다. Proposal -> reconcile item -> 관련 `state.sqlite.task_events` row가 있는 accepted Core state-changing action을 거쳐야 상태가 되며, 그렇지 않으면 rejected, deferred, note-only content로 남습니다.

Artifact ref를 렌더링하는 모든 템플릿은 `redaction_state`를 보존해야 합니다. 큰 log, diff, trace, screenshot, bundle, recording, 민감한 artifact body는 기본적으로 embed하지 않고 `ArtifactRef`로 참조합니다. `secret_omitted` entry는 안전한 note 또는 handle을 보여줄 수 있고, 보이는 nonsecret evidence만 뒷받침할 수 있습니다. `blocked` entry는 커밋된 metadata-only notice를 사용할 수 없는 입력으로 보여줍니다. 템플릿은 생략된 secret/PII 값 또는 차단된 원본 payload를 inline 표시하거나 재구성하거나 요약하거나 export하면 안 됩니다.

`redaction_availability_summary`, 생략/차단 영향 line, `이후 영향` column 같은 표시 field는 표시 전용 요약일 뿐입니다. 이 값들은 `ArtifactRef.redaction_state`, owner 기록, 이후 gate, 근거, QA, 검증, projection, export, Release Handoff 상태에서 파생됩니다.

Decision Packet visibility는 standalone `DEC` Markdown projection에 의존하지 않습니다. Required surfaces는 active Decision Packet을 `TASK`, status/next response, judgment-context resource, decision-packet resource를 통해 계속 보여줘야 합니다. Standalone `DEC`는 해당 projection이 켜져 있을 때만 쓰는 선택적 렌더링 보기입니다.

Decision Packet 표시는 decision title, `decision_profile`, `judgment_domain`, 왜 지금 필요한지, 사용자가 정확히 결정하는 것, 간결한 options 또는 상세 trade-offs, recommendation, uncertainty, deferral consequence, 해당하는 경우 residual risk 같은 읽기용 shape field를 포함할 수 있습니다. `decision_profile`은 schema가 소유하며 display가 간결한 `minimal_decision`인지, `product_ux_tradeoff`, `architecture_tradeoff`, `approval_shaped`, `waiver`, `acceptance`, `residual_risk_acceptance`, `reconcile`, `mixed` 같은 더 상세한 profile인지 제어합니다. `judgment_domain`은 schema가 소유하는 판단 영역이며 값은 `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, `mixed`입니다. Template은 이 값을 제품/UX 판단, 기술 구조 판단, 보안/개인정보 판단, QA/작업 수락, 잔여 위험, 범위/자율성 판단, 복합 같은 label로 렌더링할 수 있습니다. 결정이 여러 영역에 걸쳐 있으면 domain을 배타적으로 다루지 말고 부차적인 고려사항을 trade-offs, affected gates, risk, evidence, follow-up에 렌더링해야 합니다. 이 label은 독자를 돕지만 `ProjectionKind` value, gate, owner record, validator input, close aggregation rule, authority path가 아닙니다.

표시 카드는 서로 다른 세 문제를 구분해야 합니다. 오래된 projection(stale projection)은 읽기용 보기가 source record보다 뒤처졌을 수 있다는 뜻이고, stale state 또는 stale evidence는 기반 state, baseline, artifact input이 이동했거나 부족해졌다는 뜻이며, MCP에 닿지 못하는 상태(MCP unavailable)는 접점이 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. 상태 변경은 owner record와 Core transition만 할 수 있습니다.

닫기와 assurance 표시는 self-checked 작업, `detached_verified` assurance, waived verification, QA waiver, residual-risk accepted `completed_with_risk_accepted` close를 서로 다른 label로 유지해야 합니다. 같은 compact card에 함께 나타날 수는 있지만, 각 상태를 뒷받침하는 owner ref 없이 "done", "verified", "accepted"로 뭉개면 안 됩니다.

## 미래 / 진단 템플릿

- [DESIGN](design.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [EXPORT](export.md)
- [INTERFACE-CONTRACT](interface-contract.md)
- [JOURNEY-CARD](journey-card.md)
- [MODULE-MAP](module-map.md)
- [RUN-SUMMARY](run-summary.md)
- [TDD-TRACE](tdd-trace.md)

## 초기 필수 표시 형태

- [Compact Status Card](compact-status-card.md)
- [TASK](task.md) 최소 continuity summary
- [Decision Packet 사용자 결정 요청 display shape](decision-packet.md)

## 초기 선택 표시 형태

- [APR](approval.md)
- [Approval Card](approval-card.md)
- [DIRECT-RESULT](direct-result.md)
- [MANUAL-QA](manual-qa.md)
- [수동 QA Card](manual-qa-card.md)
- [Verification Result Card](verification-result-card.md)

## 메모

이 디렉터리는 Projection 템플릿 본문과 표시 카드 형태의 기준 참조 위치입니다.
