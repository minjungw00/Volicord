# Template 참조

## 사용 시점

Projection template과 표시 카드가 렌더링하는 Markdown 형태를 확인할 때 이 파일들을 사용합니다. Projection rule, 권한 경계, 최신성 동작은 [문서 Projection 참조](../document-projection.md)가 정의합니다.

## Template tiering

Projection template은 API `ProjectionKind` tier와 일치합니다.

| Tier | Templates | Rule |
|---|---|---|
| MVP-required | `TASK`, `APR`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `DIRECT-RESULT` | MVP projector는 이를 렌더링해야 합니다. |
| MVP-optional | `MANUAL-QA`, `TDD-TRACE`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT` | Policy가 적용되거나, 기록이 있거나, user/operator가 projection을 켰을 때 렌더링합니다. |
| Extension / optional | `DEC`, `DESIGN`, `EXPORT`, `JOURNEY-CARD` | 해당 선택 projection이 켜져 있을 때만 렌더링합니다. |

Template은 렌더링 결과일 뿐 기준 상태가 아닙니다. Kernel field, MCP schema, SQLite DDL, gate behavior, artifact integrity rule을 재정의하면 안 됩니다.

Artifact ref를 렌더링하는 모든 template은 `redaction_state`를 보존해야 합니다. Large 또는 sensitive artifact body는 기본적으로 embed하지 않습니다. `secret_omitted` entry는 safe note 또는 handle을 보여줄 수 있고 visible nonsecret evidence만 뒷받침할 수 있습니다. `blocked` entry는 committed metadata-only notice를 unavailable input으로 보여줍니다. Template은 생략된 secret/PII value 또는 blocked raw payload bytes를 inline, reconstruct, summarize, export하면 안 됩니다.

`redaction_availability_summary`, omitted 또는 blocked impact line, `Downstream Effect` column 같은 display field는 렌더링된 summary일 뿐입니다. 이 값들은 `ArtifactRef.redaction_state`, owner 기록, downstream gate, 근거, QA, 검증, projection, export, Release Handoff 상태에서 파생되며, 새 기준 기록, schema field, DDL value, 권한 입력이 아닙니다.

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
