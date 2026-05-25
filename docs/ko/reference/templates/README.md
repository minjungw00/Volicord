# Template 참조

## 사용 시점

Projection template과 표시 카드가 렌더링하는 Markdown 형태를 확인할 때 이 파일들을 사용합니다. Projection rule, 권한 경계, 최신성 동작은 [문서 Projection 참조](../document-projection.md)가 정의합니다.

## Template 계층

Projection template은 API `ProjectionKind` tier와 일치합니다.

| Tier | Templates | Rule |
|---|---|---|
| MVP-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | MVP projector는 이를 렌더링해야 합니다. |
| MVP-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Policy가 적용되거나, 기록이 있거나, user/operator가 projection을 켰을 때 렌더링합니다. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | 해당 선택 projection이 켜져 있을 때만 렌더링합니다. |

Template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Kernel field, MCP schema, SQLite DDL, gate behavior, artifact integrity rule을 재정의하면 안 됩니다.

렌더링 placeholder, label, table column, 예시 front matter key는 표시를 위한 template binding입니다. Binding은 기존 owner record field 또는 ref를 보여주거나, template이 나열한 source record에서 파생한 표시 전용 요약이어야 합니다. Source record 또는 ref가 없으면 상태를 만들어내지 말고 `none`, `unknown`, `not_required`, unavailable/blocking note로 렌더링합니다.

파생 표시 요약에는 `approval_covers`, `approval_does_not_cover`, `secret_exposure_boundary` 같은 Approval boundary line, close context, close blocker, waiver path, projection freshness, redaction availability, compact context, Journey Card, judgment-context 관련 summary가 포함됩니다. 이 이름들은 새로운 기준 기록, schema field, DDL column, `ProjectionKind` value, gate, 권한 입력, 권한 경로가 아닙니다. 요약 대상인 owner record, ref, gate, artifact, Decision Packet을 통하지 않고 validator input으로 사용하면 안 됩니다.

렌더링 예시는 이 경계를 독자가 바로 볼 수 있어야 합니다. `source_state_version`은 렌더링에 사용한 상태 clock을 가리키고, `projection_version` 또는 projection status는 렌더/template/job 보기를 가리키며, `updated_at`은 그 보기가 만들어진 시각을 가리킵니다. Freshness line은 이 보기가 source record와 아직 맞는지 표시할 뿐이며 Task result, gate value, Approval, acceptance, evidence가 아닙니다.

Managed block은 projector가 소유하는 표시 영역입니다. Managed block을 직접 편집한 내용은 상태 변경이 아니라 drift이며 reconcile candidate가 되어야 합니다. `User Notes and Proposals` 같은 사람이 편집할 수 있는 section은 제안 접점입니다. Reconcile 또는 다른 Core state-changing path가 관련 `state.sqlite.task_events` row를 추가한 뒤에야 상태가 됩니다.

Artifact ref를 렌더링하는 모든 template은 `redaction_state`를 보존해야 합니다. 크거나 민감한 artifact 본문은 기본적으로 포함하지 않습니다. `secret_omitted` entry는 안전한 note 또는 handle을 보여줄 수 있고, 보이는 nonsecret evidence만 뒷받침할 수 있습니다. `blocked` entry는 커밋된 metadata-only notice를 사용할 수 없는 입력으로 보여줍니다. Template은 생략된 secret/PII 값 또는 차단된 원본 payload를 inline 표시하거나 재구성하거나 요약하거나 export하면 안 됩니다.

`redaction_availability_summary`, 생략/차단 영향 line, `이후 영향` column 같은 표시 field는 표시 전용 요약일 뿐입니다. 이 값들은 `ArtifactRef.redaction_state`, owner 기록, 이후 gate, 근거, QA, 검증, projection, export, Release Handoff 상태에서 파생됩니다.

Decision Packet visibility는 standalone `DEC` Markdown projection에 의존하지 않습니다. MVP surface는 active Decision Packet을 `TASK`, status/next response, judgment-context resource, decision-packet resource를 통해 계속 보여줘야 합니다. Standalone `DEC`는 해당 projection이 켜져 있을 때만 쓰는 선택적 렌더링 보기입니다.

표시 카드는 서로 다른 세 문제를 구분해야 합니다. Stale projection은 읽기용 보기가 source record보다 뒤처졌을 수 있다는 뜻이고, stale state 또는 stale evidence는 기반 state, baseline, artifact input이 이동했거나 부족해졌다는 뜻이며, MCP unavailable은 surface가 필요한 Harness/Core capability에 닿지 못한다는 뜻입니다. 상태 변경은 owner record와 Core transition만 할 수 있습니다.

## MVP-required templates

- [TASK](task.md)
- [APR](approval.md)
- [RUN-SUMMARY](run-summary.md)
- [EVIDENCE-MANIFEST](evidence-manifest.md)
- [EVAL](eval.md)
- [DIRECT-RESULT](direct-result.md)

## MVP-optional templates

- [MANUAL-QA](manual-qa.md)
- [TDD-TRACE](tdd-trace.md)
- [DOMAIN-LANGUAGE](domain-language.md)
- [MODULE-MAP](module-map.md)
- [INTERFACE-CONTRACT](interface-contract.md)

## Extension templates

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

이 디렉터리는 projection template 본문과 표시 카드 형태의 기준 참조 위치입니다.
