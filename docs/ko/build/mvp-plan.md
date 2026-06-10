# 빌드: MVP 계획

이 문서는 첫 하네스 서버 구현 묶음을 준비하기 위한 빌드 인계 문서입니다. 준비 상태, 계획 전제, 구현 순서, 첫 내부 smoke target의 목적, 완료 기준을 기록합니다. 현재 MVP 범위, API 동작, 스키마, 저장 효과, 보안 보장은 이 문서가 정의하지 않습니다.

<a id="문서-수락-상태"></a>
## 저장소 상태

유지보수자 인계 상태: **서버 구현 수락 전**.

이 저장소는 아직 향후 하네스 서버를 위한 문서 전용 원천 자료입니다. 하네스 서버 구현, Harness Runtime Home, Product Repository, 런타임 기록 저장소, 생성된 상태 보기 저장소, 증거 저장소, QA 기록, 수락 기록, 닫기 기록이 아닙니다.

현재 범위의 canonical 설명은 [현재 MVP 범위 참조](../reference/active-mvp-scope.md)를 확인하세요. 런타임 위치 경계는 [런타임 경계](../reference/runtime-boundaries.md)가 담당합니다.

현재 문서 묶음에는 시작, 사용, 빌드, 참조, 이후, 유지보수 경로의 영어/한국어 문서가 함께 있습니다. 기준 계약은 참조 담당 문서에 두고, 이 빌드 계획은 유지보수자가 서버 구현을 시작할 준비가 되었을 때 어떤 순서로 계획할지만 설명합니다.

유지보수자가 [서버 구현 전 결정 사항](#서버-구현-전-결정-사항)을 수락하거나, 해결하거나, 이름 붙은 범위 영향과 함께 명시적으로 미루기 전까지 이 저장소에서 서버 구현을 시작하면 안 됩니다.

## 계획 전제

- 현재 작업은 구현 계획이지 런타임 구현이 아닙니다.
- 현재 MVP 범위는 [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md)가 담당합니다. 이 문서는 범위 목록을 반복하지 않습니다.
- API 메서드 동작은 [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md)가 담당합니다. 이 문서는 요청, 응답, 분기, 오류 동작을 반복하지 않습니다.
- 공통 API 요청 래퍼와 응답 분기는 [`../reference/api/schema-core.md`](../reference/api/schema-core.md)가 담당합니다. 상태, 아티팩트, 판단, 값 집합 스키마는 분리된 API 스키마 참조가 담당합니다.
- 저장 효과는 [`../reference/storage-effects.md`](../reference/storage-effects.md)가 담당합니다. 이 문서는 테이블, 마이그레이션, 아티팩트 생명주기, 상태 효과를 정의하지 않습니다.
- 보안 주장은 [`../reference/security.md`](../reference/security.md)가 담당하고, 런타임 홈과 접근 경계는 [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md)가 담당합니다.
- 이후 후보는 담당 문서를 통해 승격되기 전까지 현재 MVP 요구사항이 아닙니다.

## 구현 순서

유지보수자 인계 뒤 첫 구현 계획은 아래 순서로 잡습니다.

1. [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md)에서 현재 MVP 경계를 확인합니다.
2. 이후 후보에 기대지 않고 평범한 사용자 작업 흐름 하나를 다룰 수 있는 가장 작은 서버 조각을 고릅니다.
3. 코드 구조를 설계하기 전에 각 서버 접점이 어느 참조 담당 문서를 따르는지 연결합니다.
4. API, 스키마, 저장소, 보안, 런타임 경계 담당 문서가 해당 조각을 수락한 뒤에만 계약을 해치지 않는 뼈대를 만듭니다.
5. 오래 남는 저장 동작은 [`../reference/storage-effects.md`](../reference/storage-effects.md)에 있는 내용만 사용합니다.
6. API와 도구 동작은 [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md)를 따르고, 스키마는 관련된 분리 API 스키마 담당 문서를 따릅니다.
7. 상태와 표시 동작은 담당 문서가 정한 상태에서 파생해 읽는 것으로 다루며, 독립 권한처럼 만들지 않습니다.
8. 수락, 잔여 위험 수락, 증거, 검증, 닫기 준비 상태를 구현 작업에서 서로 구분합니다.

이 순서는 일부러 작게 둡니다. 어떤 단계에 계약 세부사항이 필요하다면 이 빌드 계획을 늘리지 말고 담당 참조 문서를 고치거나 수락합니다.

<a id="첫-내부-스모크-목표"></a>
## 첫 내부 smoke target

첫 내부 smoke target은 첫 서버 조각이 수락된 현재 MVP 계약만 사용해 평범한 작업 하나를 접수부터 닫기 준비 상태 평가까지 다룰 수 있는지 확인하는 목표입니다.

이 목표는 아래 계획 지점을 확인해야 합니다.

- 사용자의 평소 말에서 작업을 접수하거나 이어가기
- 범위 담당 문서를 통한 현재 MVP 범위 판정
- 사용자 소유 판단과 Core 소유 상태의 분리
- 기록을 지어내지 않는 증거와 검증 기대치 참조
- 스키마/API 담당 문서에서 온 닫기 준비 상태 보고
- 수락된 저장 효과 안으로 제한된 저장 쓰기
- 파생되고 최신성 상태가 드러나는 상태 출력
- 사용할 수 없거나 뒷받침되지 않는 권한을 분명하게 보고

이 목표는 적합성 테스트 모음도, fixture 명세도, 하네스 구현 증명도 아닙니다. 정확한 예시, 메서드 호출, 스키마, 저장 동작, 오류 동작은 참조 담당 문서에 둡니다.

<a id="서버-코딩-전-결정"></a>
## 서버 구현 전 결정 사항

유지보수자는 구현을 시작하기 전에 각 항목에 대해 첫 서버 조각 수락, 이름 붙은 영향이 있는 차단, 이름 붙은 영향이 있는 보류 중 하나를 기록해야 합니다.

| 결정 항목 | 구현 전 필요한 결과 |
|---|---|
| 빌드 인계 | 유지보수자가 이 문서를 구현 계획의 활성 빌드 진입점으로 확인합니다. |
| 현재 MVP 범위 | 유지보수자가 [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md)의 경계를 수락하거나, 해결되지 않은 범위 영향을 이름 붙입니다. |
| API와 스키마 | 유지보수자가 [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md)와 필요한 API 스키마 담당 문서의 해당 조각을 수락합니다. |
| 저장 효과 | 런타임 저장 파일, DDL, 아티팩트 저장소를 만들기 전에 유지보수자가 [`../reference/storage-effects.md`](../reference/storage-effects.md)의 해당 조각을 수락합니다. |
| 보안과 런타임 경계 | 유지보수자가 [`../reference/security.md`](../reference/security.md)와 [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md)의 관련 주장과 비보장을 수락합니다. |
| smoke target | 유지보수자가 첫 내부 smoke target을 적합성 주장이 아니라 구현 계획 목표로 수락합니다. |
| 미룬 자료 | 담당 문서에서 승격하지 않은 이후 후보가 첫 서버 조각에 필요하지 않음을 유지보수자가 확인합니다. |

## 문서 전용 경계

이 저장소의 편집은 런타임 동작을 만들지 않습니다. 서버 코드, 런타임 상태, 생성된 운영 파일, 생성된 상태 보기, 증거 기록, QA 기록, 수락 기록, 닫기 기록, 잔여 위험 기록, 실행 가능한 fixture, 적합성 실행기 출력을 추가하지 않습니다.

경로 허용 목록, 작업 묶음 경계, 담당 문서 링크, 계획 순서는 문서 유지보수 통제입니다. 하네스 런타임 권한, 쓰기 권한 부여, 샌드박스 보장, 집행 증명이 아닙니다.

이 계획의 완료 기준을 통과해도 문서가 향후 구현 묶음을 안내할 준비가 되었다는 뜻일 뿐입니다. 하네스가 구현되거나, 런타임 적합성이 증명되거나, Product Repository 쓰기가 승인되는 것은 아닙니다.

## 참조 담당 문서

이 빌드 계획에서 계약을 반복하지 말고 아래 담당 문서를 사용합니다.

| 주제 | 담당 문서 |
|---|---|
| 현재 MVP 범위 | [`../reference/active-mvp-scope.md`](../reference/active-mvp-scope.md) |
| API 메서드 동작 | [`../reference/api/mvp-api.md`](../reference/api/mvp-api.md) |
| 공통 API 요청 래퍼와 응답 분기 | [`../reference/api/schema-core.md`](../reference/api/schema-core.md) |
| 상태 스키마와 닫기 준비 상태 구조 | [`../reference/api/schema-state.md`](../reference/api/schema-state.md) |
| 아티팩트 스키마 | [`../reference/api/schema-artifacts.md`](../reference/api/schema-artifacts.md) |
| 사용자 소유 판단 스키마 | [`../reference/api/schema-judgment.md`](../reference/api/schema-judgment.md) |
| API 값 집합 | [`../reference/api/schema-value-sets.md`](../reference/api/schema-value-sets.md) |
| 저장 효과 | [`../reference/storage-effects.md`](../reference/storage-effects.md) |
| 보안 보장과 비보장 | [`../reference/security.md`](../reference/security.md) |
| 런타임 홈과 접근 경계 | [`../reference/runtime-boundaries.md`](../reference/runtime-boundaries.md) |

주변 참조 문서와 경로 안내는 [`../reference/README.md`](../reference/README.md)를 사용합니다.

## 완료 기준

구현 계획은 아래 조건을 만족할 때만 완료할 수 있습니다.

- 유지보수자가 이 빌드 계획을 구현 계획 진입점으로 수락합니다.
- [서버 구현 전 결정 사항](#서버-구현-전-결정-사항)의 모든 항목에 이름 붙은 영향이 있는 수락, 차단, 보류 결과가 있습니다.
- 첫 서버 조각을 중복 계약 문구가 아니라 담당 문서 링크로 설명할 수 있습니다.
- 영어와 한국어 빌드 문서가 같은 독자 목적, 담당 문서 경로, 인계 상태를 유지합니다.
- 이후 후보가 현재 MVP 요구사항처럼 제시되지 않습니다.
- 이 저장소에 임시 계획 파일, 생성된 런타임 기록, 실행 가능한 fixture, 적합성 결과, 제품 구현 출력물이 남아 있지 않습니다.

이 기준을 만족한 뒤의 다음 단계는 유지보수자가 승인한 구현 묶음입니다. 그 전까지 이 저장소는 계획 자료로 남습니다.
