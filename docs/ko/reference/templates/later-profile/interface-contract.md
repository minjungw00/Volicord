# INTERFACE-CONTRACT 템플릿

## 사용 시점

모듈 인터페이스, 호출자 영향, 호환성 위험, 테스트 경계를 읽기 쉬운 projection으로 볼 때 `INTERFACE-CONTRACT`를 사용합니다.

경계: projection template일 뿐이며 runtime/server 구현이나 생성된 운영 산출물에 권한을 주지 않습니다. 공통 phase와 projection 규칙은 [템플릿 참조](README.md#사용-시점)를 따릅니다.

구현 계층: 향후/진단용 projections입니다. Interface Contract output은 owner profile이 명시적으로 승격하지 않는 한 later reference view입니다.

## 기준 기록

- `interface_contracts`
- impacted caller 참조
- 관련 module map item
- 관련 user judgment 또는 design 참조
- boundary, integration, contract test 참조
- `deep_module_interface` 관련 design-quality validator 결과
- 표시되는 경우 interface 또는 compatibility ref에 영향을 주는 routed stewardship finding
- 읽기용 보기 최신성(projection freshness) 입력

## 렌더링 섹션

- 식별 정보
- 계약
- 영향받는 호출자
- 테스트 경계
- 검토
- 참조
- 사용자 메모와 제안

## 전체 템플릿

````md
---
doc_type: interface_contract
interface_contract_id: IFACE-0001
task_id: TASK-0001
review_status: pending
projection_version: 1
source_state_version: 42
updated_at: 2026-05-06T09:30:15+09:00
---

# IFACE-0001 Interface 제목

> Projection 보기: `interface_contracts`와 관련 ref를 `source_state_version` / `updated_at` 기준으로 렌더링한 보기입니다. Managed section은 생성된 표시 영역이며, reconcile 입력은 `사용자 메모와 제안`에 적습니다.

<!-- HARNESS:BEGIN managed -->
## 식별 정보
- module:
- interface:
- change type: new | changed | deprecated | removed

## 계약
- inputs:
- outputs:
- errors:
- side effects:
- compatibility impact: none | minor | breaking

## 영향받는 호출자
- caller:

## 테스트 경계
- boundary tests:
- integration tests:
- contract tests:

## 검토
- review_status: pending | reviewed
- reviewed by:
- decision:
- waiver reason:

## 참조
- TASK:
- DESIGN:
- DEC:
- EVIDENCE-MANIFEST:
<!-- HARNESS:END managed -->

## 사용자 메모와 제안
<!-- Human-editable: 여기의 interface proposal은 reconcile/Core를 통해 accepted되기 전에는 기준 Interface Contract record가 아닙니다. -->
-
````

## 메모

이 template은 렌더링 결과일 뿐 기준 상태가 아닙니다. 기준 interface 참조는 `StateRecordRef.record_kind=interface_contract`를 사용합니다. `검토` section은 interface, validator, decision ref 위의 projection display이며 Approval, evidence, QA, verification, 작업 수락, 잔여 위험 수용, close, Write Authorization이 아닙니다.

Public interface change, compatibility risk, breaking change, caller-impact choice에 사용자 소유 제품 판단이나 기술 판단이 필요하면 기존 design-quality 및 user judgment 경로로 라우팅합니다. Contract를 여기에 렌더링하는 것만으로 `design_gate`, `decision_gate`, close impact가 해소되지는 않습니다.
