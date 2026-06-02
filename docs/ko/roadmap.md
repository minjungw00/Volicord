# 로드맵

## 이 문서가 도와주는 일

이 문서는 아직 staged delivery에 들어오지 않은 하네스의 향후 후보 항목을 모아 둡니다. 독자가 나중에 다룰 수 있는 방향을 볼 수 있게 하되, 그것을 현재 요구사항, 권한 경로, 작업 수락 경로, QA 경로, 검증 경로, 실행 보장으로 오해하지 않게 하는 것이 목적입니다.

이 문서는 로드맵 문서입니다. 문서 수락과 별도의 구현 계획 준비 결정 전에는 런타임/서버 구현, 생성된 운영 파일, 실행 가능한 fixture, fixture 파일, 읽기용 요약, 데이터베이스, 런타임 데이터를 만들라는 뜻이 아닙니다.

## 이런 때 읽기

- 어떤 아이디어가 staged delivery 밖에 있는지 확인할 때.
- 향후 능력이 단계 계획으로 승격될 준비가 되었는지 확인할 때.
- 유용한 미래 아이디어가 담당 문서가 명시적으로 범위를 정하고 증명하기 전까지 권한 없는 후보로 남아야 함을 확인할 때.

## 읽기 전에

단계별 전달은 [MVP 계획](build/mvp-plan.md)이 담당합니다. 현재 인계와 구현 계획은 [구현 개요의 문서 인계 요약](build/implementation-overview.md#문서-인계-요약)에서 시작한 뒤 [서버 코딩 전 필요한 구현 결정](build/mvp-plan.md#서버-코딩-전-필요한-구현-결정), [첫 실행 가능한 조각](build/first-runnable-slice.md), [MVP 계획](build/mvp-plan.md)을 확인합니다. 정확한 계약은 Reference 문서를 사용합니다.

현재 단계 이름은 다음과 같습니다.

- 코어 권한 조각(v0.1 Core Authority Slice)
- 사용자 대상 하네스 MVP(v0.2 User-Facing Harness MVP)
- 에이전시 보증 팩(v0.3 Agency Assurance Pack)
- 운영과 인계 팩(v0.4 Operations & Handoff Pack)
- v1+ Expansion

## 핵심 생각

로드맵 항목은 후보 항목이지 staged delivery 약속이 아닙니다. 이곳에 이름이 있다고 해서 권한, 적합성, 구현 준비 상태, 사용자 작업 수락, QA 완료, 검증 충족, 잔여 위험 수용, 보안 보장, 런타임 동작이 생기지 않습니다.

후보 항목은 향후 담당 문서가 명시적으로 승격하기 전까지 v0.1부터 v0.4까지의 단계 밖에 남습니다. 승격되더라도 사용자 소유 판단을 보존하고, 지속 상태와 아티팩트는 Core 소유 권한 경로로 보내며, 근거/검증/QA/작업 수락/잔여 위험을 분리하고, 실제로 증명된 능력에 맞는 정직한 보안 표현을 사용해야 합니다.

## 로드맵 경계

이 문서는 kernel invariant, public MCP schema, storage profile, fixture profile exit, 단계 필수 API surface, operator surface, 구현 점검 목록을 소유하지 않습니다. 그런 세부 내용은 Build 문서와 Reference 담당 문서에 둡니다.

로드맵 후보 항목은 관련 담당 문서가 제한적으로 허용할 때만 읽기 전용 표시, 메타데이터, 아티팩트 후보, fixture 후보, prototype, 계획 메모로 쓸 수 있습니다. Core 소유 상태, `task_events`, 아티팩트 참조, Decision Packet, 수동 QA, Eval record, 작업 수락, 잔여 위험 수용, 읽기용 요약 최신성, 닫기 준비 상태, 구현 준비 상태를 우회하는 지름길이 되면 안 됩니다.

지속 아티팩트 등록, 근거 연결, 상태 변경, gate 결과, QA 기록, 검증 결과, 작업 수락 기록, 잔여 위험 기록은 기존 Core/MCP 담당 경로 또는 향후 승격된 담당 계약을 거쳐야 합니다. 여기에 이름이 있다는 이유만으로 권한 경로가 되지는 않습니다.

## 단계 승격 조건

후보 항목은 향후 담당 결정이 다음을 모두 정의하고 증명하기 전까지 staged delivery에 들어갈 수 없습니다.

- 명시적인 향후 버전 또는 향후 단계 담당 결정과 좁은 범위
- 관련되는 경우 작업 수락, 잔여 위험 수용, 제품 판단, 중요한 기술 판단, QA 면제 판단을 포함한 사용자 소유 판단 보존
- Core 권한, Core 소유 상태, 아티팩트 참조, gate 의미, 닫기 의미, 담당 기록 생명주기 우회 없음
- 보안 위협 모델과 맞는 단계별 보안 보장 표현. 예방적 차단이나 격리 주장은 증명된 대상 메커니즘과 fallback이 있을 때만 가능
- 근거, 검증, QA, 작업 수락, 잔여 위험에 미치는 영향. 무엇을 도울 수 있고 무엇을 충족하면 안 되는지 명확히 적어야 함
- 새 API, 저장소, 아티팩트, 읽기용 요약, fixture, 운영자 동작, connector, UI 동작의 정확한 계약과 담당 문서 위치
- 런타임 접점을 캡처하거나 저장하는 경우 redaction, secret/PII 처리, test environment, 아티팩트 보존 규칙
- 승격된 동작을 위한 fixture 또는 적합성 목표
- 지원하지 않는 접점, 빠진 능력, 사용할 수 없는 도구, 오래된 데이터, 부분 캡처에 대한 fallback 동작
- 읽기용 요약, dashboard, index, connector output, 생성 문서를 기준 상태로 취급하는 의존성 없음
- 후보 항목이 v0.1부터 v0.4까지의 요구사항을 부풀리거나 지원하지 않는 접점을 초기 단계 실패로 만들지 않는다는 초기 단계 범위 부풀림 점검

어느 조건이라도 빠지면 그 항목은 로드맵 후보로 남습니다.

## 후보 항목 목록

아래 예시는 후보 영역만 설명합니다. 단계 요구사항을 추가하지 않으며 위의 단계 승격 조건을 완화하지도 않습니다.

| 후보 영역 | 승격 전 경계 |
|---|---|
| 대시보드, 호스팅된 작업 UI, 아티팩트 대시보드, 더 풍부한 카드, 더 풍부한 시각화 | Core에서 파생된 상태나 읽기용 요약을 표시할 수 있습니다. 권한, 구현 준비 상태, 닫기 준비 상태, 작업 수락, 잔여 위험 수용, QA 완료, 검증 충족, 읽기용 요약 최신성, 작업 흐름 라우팅, 지표 해석이 되면 안 됩니다. |
| 브라우저 캡처 자동화 | Screenshot, console log, network trace, accessibility snapshot, workflow recording을 아티팩트 후보로 모을 수 있습니다. 사람의 수동 QA 판단, 작업 수락, 분리 검증, redaction policy, 기존 수동 QA/아티팩트 경로를 대체하면 안 됩니다. |
| 여러 접점 검증 | 승격 뒤 verification bundle을 다른 agent 접점이나 evaluator environment로 보낼 수 있습니다. Core 소유 반환 기록과 필요한 독립성 의미 없이 Eval을 기록하거나, 검증을 충족하거나, assurance를 올리거나, 결과를 수락하거나, Task를 닫으면 안 됩니다. |
| 넓은 커넥터 생태계, 커넥터 시장, 호스팅 UI, 호스팅/원격 런타임 | 나중에 접점을 확장할 수 있습니다. MCP 노출을 넓히거나, 권한을 만들거나, Core를 우회하거나, 로컬 기준 증명을 대체하거나, 원격/런타임 보장을 암시하거나, 지원하지 않는 접점을 초기 단계 실패로 만들면 안 됩니다. |
| 네이티브 후크, 예방적 가드 확장, 고급 사이드카 워처 | 접점이 메커니즘을 증명한 곳에서 guard 표시, 아티팩트 캡처, command 관찰, file write 관찰을 강화할 수 있습니다. Label만으로 pre-execution blocking, OS 격리, tamper-proof storage, arbitrary-tool control을 주장하면 안 됩니다. 관찰 결과가 상태에 영향을 주려면 Core 기록, validator, 아티팩트 등록, reconcile 중 맞는 경로를 거쳐야 합니다. |
| 맥락 색인, 로컬 파생 지표, 장기 지표 | 읽기 전용 검색이나 진단을 제공할 수 있습니다. Write Authorization 생성, 쓰기 허가, Decision Packet 해소, Approval 부여, gate 충족, 근거 생성, 검증 또는 QA 기록, 읽기용 요약 refresh, readiness 선언, 위험 수용, 결과 수락, assurance 상승, Task close를 하면 안 됩니다. |
| 팀 작업 흐름, 권한, 공유 프로필, 내보내기/가져오기, 오케스트레이션, 병렬 lane | 향후 작업 조율을 도울 수 있습니다. Staged delivery, single-project local authority, 작업 수락, QA, 검증, 잔여 위험 수용, close의 필수 요소가 되면 안 됩니다. |
| 고급 export, release/deployment/canary/rollback/merge/production-monitoring automation | 향후 통합 작업이 될 수 있습니다. 담당 문서가 더 많은 권한을 승격하기 전까지 release handoff는 report/export 경계로 남고, deployment, merge, rollback, production authority는 외부에 둡니다. |
| 고급 validator, language 또는 interface check | 향후 stewardship 또는 진단 범위가 될 수 있습니다. 담당 문서가 정확한 policy, severity, waiver, fixture 동작을 정의하기 전까지 초기 단계 fixture failure, 작업 수락, QA, close 기준이 되면 안 됩니다. |

단계별 전달 경계는 [MVP 계획의 Roadmap 범위의 v1+ Expansion 후보](build/mvp-plan.md#roadmap-범위의-v1-expansion-후보)를 사용합니다. 이 문서는 승격 전까지 staged delivery 밖에 남는 후보 항목을 추적하는 데만 사용합니다.
