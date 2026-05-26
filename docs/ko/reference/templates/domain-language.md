# DOMAIN-LANGUAGE Template

## 사용 시점

현재 domain term의 의미, code representation, 대기 중인 term decision, deprecated term, 사람이 제안한 변경 사항을 읽기 쉬운 projection으로 볼 때 `DOMAIN-LANGUAGE`를 사용합니다.

이 문서는 template 참조 문서입니다. 재설계 문서가 승인되기 전에는 runtime/server 구현, 생성된 운영 파일, 실행 가능한 fixture 파일, runtime data를 만들라는 뜻이 아닙니다. 첫 구현/증명 대상은 계속 Kernel Smoke입니다. Agency-Hardened MVP와 post-MVP automation은 owner 문서가 승격하고 증명하기 전까지 범위 밖입니다.

## 기준 기록

- `domain_terms`
- domain term 변경을 제안하는 reconcile item
- term을 도입하거나 reconcile로 조정한 Task 참조
- domain-language conflict에 사용자 소유 판단이 필요할 때 관련 Decision Packet
- `domain_language` 관련 design-quality validator 결과
- 표시되는 경우 domain-language ref에 영향을 주는 routed stewardship finding
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- Summary
- Terms
- Pending Term Decisions
- Deprecated Terms
- User Notes and Proposals

## 전체 템플릿

````md
---
doc_type: domain_language
project_id: PRJ-0001
status: active
projection_version: 1
source_state_version: 12
updated_at: 2026-05-06T09:30:15+09:00
---

# Domain Language

> Projection 보기: `domain_terms`와 관련 ref를 `source_state_version` / `updated_at` 기준으로 렌더링한 보기입니다. Managed section은 생성된 표시 영역이며, reconcile 입력은 `User Notes and Proposals`에 적습니다.

<!-- HARNESS:BEGIN managed -->
## Summary
- current status:
- latest reconciled task:
- stale conditions:

## Terms
| Term | Meaning | Code Representation | Not This | Related Terms | Source | Status |
|---|---|---|---|---|---|---|
| Account | login-capable user identity | `src/auth/account.ts` | Profile | User, Session | TASK-0001 | active |

## Pending Term Decisions
| Term | Question | Options | Recommendation | Owner |
|---|---|---|---|---|

## Deprecated Terms
| Term | Replaced By | Reason | Since |
|---|---|---|---|
<!-- HARNESS:END managed -->

## User Notes and Proposals
<!-- Human-editable: 여기의 term proposal은 reconcile/Core를 통해 accepted되기 전에는 기준 domain term이 아닙니다. -->
-
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 domain term 참조는 `StateRecordRef.record_kind=domain_term`을 사용합니다. Pending term decision, latest-review text, human proposal은 display 또는 reconcile input입니다. 그 자체로 gate를 충족하거나, write를 승인하거나, evidence를 만들거나, risk를 받아들이거나, work를 close하지 않습니다.

Term conflict가 제품 의미, public behavior, API/interface naming, documentation promise, 수용 기준, module responsibility를 바꾸면 해당 판단은 기존 design-quality 및 Decision Packet 경로로 라우팅합니다. Conflict를 여기에 렌더링하는 것만으로 `design_gate`, `decision_gate`, close impact가 해소되지는 않습니다.
