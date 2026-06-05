# 나중 프로필(Later-Profile) 템플릿 카탈로그

## 사용 시점

나중의 담당 프로필, 진단 경로, 보증 프로필, 운영 프로필, 내보내기/릴리스 인계 경로, 또는 명시적인 세부 보기가 상세 렌더링 본문을 요구할 때만 이 템플릿을 사용합니다. 이 폴더의 템플릿은 MVP-1 요구사항이 아닙니다.

MVP-1 작은 출력 세트는 네 가지 사용자용 출력인 [상태 카드](../status-card.md), [판단 요청](../judgment-request.md), [실행/근거 요약](../run-evidence-summary.md), [닫기 결과](../close-result.md)와 에이전트용 패킷 하나인 [에이전트 맥락 패킷](../agent-context-packet.md)으로 제한됩니다.

권한 규칙: 이 폴더의 모든 템플릿은 렌더링된 보기일 뿐입니다. 상태, 근거, 민감 동작 승인, 최종 수락, 잔여 위험 수락, QA, 검증, 쓰기 허가 기록(Write Authorization), 닫기 준비 상태, 닫기를 만들 수 없습니다.

## 산출물 계층

| 계층 | 템플릿 | 규칙 |
|---|---|---|
| 전체 판단과 민감 동작 표시 | [DEC / Decision Packet](decision-packet.md), [APR](approval.md), [민감 동작 승인 카드(Approval Card)](approval-card.md) | 복잡한 판단, 나중 프로필 판단, 또는 기록된 민감 동작 승인(Approval) 표시가 필요할 때 사용합니다. MVP-1은 [판단 요청](../judgment-request.md)을 사용합니다. |
| 상세 근거, 실행, 검증 보고서 | [RUN-SUMMARY](run-summary.md), [EVIDENCE-MANIFEST](evidence-manifest.md), [EVAL](eval.md), [검증 결과 카드(Verification Result Card)](verification-result-card.md) | 상세 근거, Eval(분리 검증 결과), 보증 프로필 표시가 활성화된 경우 사용합니다. MVP-1은 [실행/근거 요약](../run-evidence-summary.md)을 사용합니다. |
| 수동 QA와 보증 표시 | [MANUAL-QA](manual-qa.md), [수동 QA 카드](manual-qa-card.md) | 수동 QA 프로필이 활성화된 경우 사용합니다. |
| 이어가기와 진단 보고서 | [TASK](task.md), [DIRECT-RESULT](direct-result.md), [JOURNEY-CARD](journey-card.md), [DESIGN](design.md) | 나중의 이어가기, 진단, 전체 보고서 보기에 사용합니다. MVP-1은 루트의 네 가지 사용자용 출력과 에이전트용 패킷 하나만 사용합니다. [상태 카드](../status-card.md), [판단 요청](../judgment-request.md), [실행/근거 요약](../run-evidence-summary.md), [닫기 결과](../close-result.md), [에이전트 맥락 패킷](../agent-context-packet.md)입니다. |
| 스튜어드십/참조 보고서 | [DOMAIN-LANGUAGE](domain-language.md), [MODULE-MAP](module-map.md), [INTERFACE-CONTRACT](interface-contract.md), [TDD-TRACE](tdd-trace.md) | 담당 프로필이 스튜어드십, TDD, 참조 상태 보기를 승격했을 때 사용합니다. |
| 운영/내보내기 보고서 | [EXPORT](export.md) | 운영 프로필 내보내기 또는 릴리스 인계 담당 경로가 활성화된 경우에만 사용합니다. |

## 템플릿 구현 계층

나중/전체 프로필 템플릿은 필요할 때만 불러옵니다. 기본 에이전트 맥락에 넣으면 안 되며, 단계 점검표처럼 다루면 안 됩니다.

나중 프로필이 활성화되지 않았으면 루트 MVP-1 보기에서 관련 간결 요약, 참조, 부재 표시, 막힘, 사용 불가 메모를 보여주고 상세 템플릿 본문을 끌어오지 않습니다.

## 한국어 렌더링 라벨

이 폴더의 한국어 템플릿은 나중 프로필(later-profile), 선택 사항, 파생 보기입니다. 사용자에게 보이는 렌더링 제목, 섹션 목록, 카드 라벨, 설명 라벨은 자연스러운 한국어로 씁니다. `DEC`, `APR`, `TASK`, `RUN-SUMMARY`, `EVIDENCE-MANIFEST`, `EVAL`, `EXPORT`, `DOMAIN-LANGUAGE`, `MODULE-MAP`, `INTERFACE-CONTRACT`, `TDD-TRACE` 같은 템플릿 ID, YAML 키, schema/API 필드, enum 값, 파일 경로, 오류 코드, validator ID, `{placeholder}` 변수는 정확히 유지합니다. `Decision Packet`, `Approval`, `Residual Risk`, `Change Unit`, `Run`, `Evidence Manifest`처럼 안정적인 기록/프로필 라벨을 조회용(lookup)으로 남겨야 할 때는 판단 패킷(Decision Packet), 민감 동작 승인(Approval), 잔여 위험(Residual Risk), 작업 조각(Change Unit), 실행(Run), 근거 목록(Evidence Manifest)처럼 한국어 별칭이나 설명을 함께 둡니다. `blocked_by`처럼 렌더링 표에 노출되는 필드형 라벨은 `차단 원인(blocked_by)`처럼 한국어 별칭을 앞세울 수 있고, `manifest hash`처럼 필드명이 아닌 표시 라벨은 한국어로 옮깁니다.

## 메모

이 폴더의 파일은 향후 담당 프로필을 위한 상세 본문을 보존합니다. 설계 자료를 보존하는 것이며, 내부 엔지니어링 점검이나 MVP-1 범위를 넓히지 않습니다.
