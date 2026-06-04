# Later-Profile 템플릿 Catalog

## 사용 시점

나중의 owner profile, diagnostic path, 보증 프로필, 운영 프로필, export/release-handoff path, 또는 명시적인 drill-down이 상세 렌더링 본문을 요구할 때만 이 템플릿을 사용합니다. 이 폴더의 템플릿은 MVP-1 요구사항이 아닙니다.

MVP-1 템플릿 세트는 [상태 카드](../status-card.md), [에이전트 맥락 패킷](../agent-context-packet.md), [판단 요청](../judgment-request.md), [실행/근거 요약](../run-evidence-summary.md), [닫기 결과](../close-result.md)로 제한됩니다.

권한 규칙: 이 폴더의 모든 템플릿은 렌더링된 보기일 뿐입니다. 상태, 근거, 민감 동작 승인, 작업 수락, 잔여 위험 수용, QA, 검증, Write Authorization, 닫기 준비 상태, close를 만들 수 없습니다.

## 산출물 계층

| 계층 | 템플릿 | 규칙 |
|---|---|---|
| 전체 판단과 민감 동작 표시 | [DEC / Decision Packet](decision-packet.md), [APR](approval.md), [Approval Card](approval-card.md) | 복잡하거나 later-profile 판단 또는 committed Approval 표시가 필요할 때 사용합니다. MVP-1은 [판단 요청](../judgment-request.md)을 사용합니다. |
| 상세 근거, 실행, 검증 보고서 | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [Verification Result Card](verification-result-card.md) | 해당 evidence, Eval, assurance profile이 active일 때 사용합니다. MVP-1은 [실행/근거 요약](../run-evidence-summary.md)을 사용합니다. |
| 수동 QA와 보증 표시 | [MANUAL-QA](manual-qa.md), [수동 QA Card](manual-qa-card.md) | 수동 QA profile이 active일 때 사용합니다. |
| Continuity와 진단 보고서 | [TASK](task.md), [DIRECT-RESULT](direct-result.md), [JOURNEY-CARD](journey-card.md), [DESIGN](design.md) | later continuity, diagnostic, full-report view에 사용합니다. MVP-1은 루트의 다섯 가지 보기만 사용합니다. [상태 카드](../status-card.md), [에이전트 맥락 패킷](../agent-context-packet.md), [판단 요청](../judgment-request.md), [실행/근거 요약](../run-evidence-summary.md), [닫기 결과](../close-result.md)입니다. |
| Stewardship/reference 보고서 | [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [TDD-TRACE](tdd-trace.md) | Owner profile이 stewardship, TDD, reference projection을 승격했을 때 사용합니다. |
| 운영/export 보고서 | [EXPORT](export.md) | 운영 프로필 export 또는 release-handoff owner path가 active일 때만 사용합니다. |

## 템플릿 구현 계층

Later/full-profile 템플릿은 필요할 때만 불러옵니다. 기본 에이전트 맥락에 넣으면 안 되며, stage checklist처럼 다루면 안 됩니다.

Later profile이 active가 아니면 root MVP-1 보기에서 관련 compact summary, ref, absence, blocker, unavailable note를 보여주고 상세 템플릿 본문을 끌어오지 않습니다.

## 한국어 렌더링 라벨

이 폴더의 한국어 템플릿은 later-profile, optional, derived view입니다. 사용자에게 보이는 rendered heading, section list, card label, prose label은 자연스러운 한국어로 씁니다. `DEC`, `APR`, `TASK`, `RUN-SUMMARY`, `EVAL`, `EXPORT` 같은 template ID, YAML key, schema/API field, enum value, file path, error code, validator ID, `{placeholder}` 변수는 정확히 유지합니다. 안정적인 영어 label을 lookup용으로 남겨야 할 때는 한국어 alias나 설명을 함께 둡니다.

## 메모

이 폴더의 파일은 향후 owner profile을 위한 상세 본문을 보존합니다. 설계 자료를 보존하는 것이며, 내부 엔지니어링 점검이나 MVP-1 범위를 넓히지 않습니다.
