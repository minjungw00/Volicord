# 템플릿 참조

## 사용 시점

Projection 템플릿과 표시 카드가 렌더링하는 Markdown 형태를 확인할 때 이 파일들을 사용합니다. Projection 규칙, 권한 경계, 최신성 동작은 [문서 Projection 참조](../document-projection.md)가 정의합니다.

Owner 경계: 템플릿은 렌더링 결과일 뿐 기준 상태가 아닙니다. 문서 세트가 구현 계획에 사용할 수 있다고 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들 권한을 주지 않습니다. 첫 실행 목표는 코어 권한 조각(v0.1 Core Authority Slice)이며, 커널 스모크(Kernel Smoke)는 이 조각을 위한 좁은 conformance authoring profile입니다. 첫 제품 MVP 목표는 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)입니다. v0.3과 v0.4는 assurance, stewardship, operations, handoff behavior를 단단하게 만들고, v1+ Expansion은 owner 문서가 명시적으로 승격하고 증명하기 전까지 roadmap 범위에 남습니다.

## 템플릿 계층

Projection 템플릿은 API `ProjectionKind` staged/reference support tier와 일치합니다.

| Tier | Templates | Rule |
|---|---|---|
| Reference-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | Staged/reference projection support는 source record가 존재하거나 변경될 때 이를 enqueue/render해야 합니다. |
| Reference-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Policy가 적용되거나, 기록이 있거나, user/operator가 projection을 켰을 때 렌더링합니다. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | 해당 선택 projection이 켜져 있을 때만 렌더링합니다. |

`Reference-required`는 관련 owner record가 존재한 뒤 staged/reference projection support에서 필요하다는 뜻입니다. 모든 코어 권한 조각(v0.1 Core Authority Slice) run이 모든 kind를 렌더링해야 한다는 뜻이 아닙니다. v0.1에는 owner path가 이미 만든 freshness/read fact를 보존하는 것 외의 projection rendering exit requirement가 없습니다. 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)는 사용자가 scope, judgment, evidence, close readiness, acceptance, residual risk를 이해할 만큼의 파생 projection 또는 card output을 제공합니다. 강화된 로컬 기준 지원은 source record가 존재하거나 변경될 때 전체 Reference-required projection set을 지원합니다.

`TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT`, 그 밖의 report projection은 owner record와 ref에서 나온 readable view입니다. Kernel field, MCP schema, SQLite DDL, gate behavior, artifact integrity rule을 재정의하면 안 됩니다.

렌더링 placeholder, label, table column, 예시 front matter key는 표시를 위한 template binding입니다. Binding은 기존 owner record field 또는 ref를 보여주거나, template이 나열한 source record에서 파생한 표시 전용 요약이어야 합니다. Source record 또는 ref가 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, unavailable/blocking note로 렌더링합니다.

여러 source가 관련될 때 compact authority display는 짧은 refs 줄을 우선 사용합니다. 예: `write=`, `decision=`, `approval=`, `evidence=`, `eval=`, `manual_qa=`, `acceptance=`, `residual_risk=`, `artifacts=`, `redaction=`, `freshness=`. 이 label은 기존 ref, redaction state, projection freshness를 가리킬 뿐이며 schema field나 authority record가 아닙니다.

파생 표시 요약에는 `approval_covers`, `approval_does_not_cover`, `secret_exposure_boundary` 같은 Approval boundary line, close context, close blocker, waiver path, projection freshness, redaction availability, compact context, Journey Card, Review Stages, judgment-context 관련 summary가 포함됩니다. 이 이름들은 새로운 기준 기록, schema field, DDL column, `ProjectionKind` value, gate, 권한을 만드는 입력이나 권한 경로가 아닙니다. 요약 대상인 owner record, ref, gate, artifact, Decision Packet을 통하지 않고 validator input으로 사용하면 안 됩니다.

렌더링 예시는 이 경계를 독자가 바로 볼 수 있어야 합니다. `source_state_version`은 렌더링에 사용한 상태 clock을 가리키고, `projection_version` 또는 projection status는 렌더/template/job 보기를 가리키며, `updated_at`은 그 보기가 만들어진 시각을 가리킵니다. 최신성 줄(freshness line)은 이 보기가 source record와 아직 맞는지 표시할 뿐이며 Task result, gate value, Approval, acceptance, evidence, close readiness, Core state rollback이 아닙니다.

관리 영역(managed block)은 projector가 소유하는 표시 영역입니다. 관리 영역을 직접 편집한 내용은 상태 변경이 아니라 drift이며 reconcile candidate가 되어야 합니다. `User Notes and Proposals` 같은 사람이 편집할 수 있는 section은 제안 접점입니다. Proposal -> reconcile item -> 관련 `state.sqlite.task_events` row가 있는 accepted Core state-changing action을 거쳐야 상태가 되며, 그렇지 않으면 rejected, deferred, note-only content로 남습니다.

Artifact ref를 렌더링하는 모든 템플릿은 `redaction_state`를 보존해야 합니다. 크거나 민감한 artifact 본문은 기본적으로 포함하지 않습니다. `secret_omitted` entry는 안전한 note 또는 handle을 보여줄 수 있고, 보이는 nonsecret evidence만 뒷받침할 수 있습니다. `blocked` entry는 커밋된 metadata-only notice를 사용할 수 없는 입력으로 보여줍니다. 템플릿은 생략된 secret/PII 값 또는 차단된 원본 payload를 inline 표시하거나 재구성하거나 요약하거나 export하면 안 됩니다.

`redaction_availability_summary`, 생략/차단 영향 line, `이후 영향` column 같은 표시 field는 표시 전용 요약일 뿐입니다. 이 값들은 `ArtifactRef.redaction_state`, owner 기록, 이후 gate, 근거, QA, 검증, projection, export, Release Handoff 상태에서 파생됩니다.

Decision Packet visibility는 standalone `DEC` Markdown projection에 의존하지 않습니다. Required surfaces는 active Decision Packet을 `TASK`, status/next response, judgment-context resource, decision-packet resource를 통해 계속 보여줘야 합니다. Standalone `DEC`는 해당 projection이 켜져 있을 때만 쓰는 선택적 렌더링 보기입니다.

Decision Packet 표시는 decision title, `judgment_domain`, 왜 지금 필요한지, 사용자가 정확히 결정하는 것, options, trade-offs, recommendation, uncertainty, deferral consequence, 해당하는 경우 residual risk 같은 읽기용 shape field를 포함할 수 있습니다. `judgment_domain`은 schema-owned 판단 영역이며 값은 `product_ux`, `technical_architecture`, `security_privacy`, `qa_acceptance`, `residual_risk`, `scope_autonomy`, `mixed`입니다. Template은 이 값을 Product / UX, Technical architecture, Security / privacy, QA / acceptance, Residual risk, Scope / autonomy, Mixed 같은 label로 렌더링할 수 있습니다. 결정이 여러 영역에 걸쳐 있으면 domain을 배타적으로 다루지 말고 부차적인 고려사항을 trade-offs, affected gates, risk, evidence, follow-up에 렌더링해야 합니다. 이 label은 독자를 돕지만 `ProjectionKind` value, gate, owner record, validator input, close aggregation rule, authority path가 아닙니다.

표시 카드는 서로 다른 세 문제를 구분해야 합니다. 오래된 projection(stale projection)은 읽기용 보기가 source record보다 뒤처졌을 수 있다는 뜻이고, stale state 또는 stale evidence는 기반 state, baseline, artifact input이 이동했거나 부족해졌다는 뜻이며, MCP에 닿지 못하는 상태(MCP unavailable)는 접점이 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. 상태 변경은 owner record와 Core transition만 할 수 있습니다.

닫기와 assurance 표시는 self-checked 작업, `detached_verified` assurance, waived verification, QA waiver, residual-risk accepted `completed_with_risk_accepted` close를 서로 다른 label로 유지해야 합니다. 같은 compact card에 함께 나타날 수는 있지만, 각 상태를 뒷받침하는 owner ref 없이 "done", "verified", "accepted"로 뭉개면 안 됩니다.

## Reference-required 템플릿

- [TASK](task.md)
- [APR](approval.md)
- [RUN-SUMMARY](run-summary.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [DIRECT-RESULT](direct-result.md)

## Reference-optional 템플릿

- [MANUAL-QA](manual-qa.md)
- [TDD-TRACE](tdd-trace.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [MODULE-MAP](module-map.md)
- [INTERFACE-CONTRACT](interface-contract.md)

## Extension 템플릿

- [DEC](decision-packet.md)
- [DESIGN](design.md)
- [EXPORT](export.md)
- [JOURNEY-CARD](journey-card.md)

## Display cards

- [Compact Status Card](compact-status-card.md)
- [Approval Card](approval-card.md)
- [Verification Result Card](verification-result-card.md)
- [Manual QA Card](manual-qa-card.md)

## 메모

이 디렉터리는 Projection 템플릿 본문과 표시 카드 형태의 기준 참조 위치입니다.
