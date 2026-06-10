# 현재 MVP 범위 참조

이 참조 문서는 하네스 계획 문서에서 현재 MVP의 상세 범위를 맡는 기준 담당 문서입니다.

## 이 문서가 담당하는 것

이 문서는 현재 MVP 기능 경계, 제품 범위 수준의 포함/제외 기준, profile-gated 값과 이후 후보가 현재 범위에 미치는 경계, 다른 문서가 반복하지 말고 요약해 연결해야 하는 범위 수준 보장과 비보장 문구를 담당합니다.

어떤 기능이 현재 MVP에 속하는지 판단할 때는 이 문서를 기준으로 삼습니다. README, Build, Later, 참조 색인, 다른 경로 문서는 상세 목록을 다시 쓰지 말고 이 문서로 연결합니다.

## 이 문서가 담당하지 않는 것

이 문서는 구현 준비 상태, 서버 코딩 인계, 유지보수자 수락 상태, 빌드 순서를 담당하지 않습니다. 그런 결정은 [MVP 계획](../build/mvp-plan.md)이 담당합니다.

API 메서드 동작, 스키마 필드, 저장 효과, 보안 증명, 템플릿 본문, 커넥터 동작, 이후 후보 세부사항도 이 문서가 담당하지 않습니다. 그런 세부 계약은 아래 담당 문서로 갑니다.

## 현재 저장소 상태

이 저장소는 향후 하네스 서버를 위한 문서 전용 원천 자료입니다. `docs/*/build/mvp-plan.md`의 유지보수자 인계 상태가 명시적으로 다르게 말하지 않는 한 런타임/서버 구현은 시작되지 않았습니다.

이 저장소는 사용자의 제품 저장소도 아니고 Harness Runtime Home도 아닙니다. 이 문서들은 런타임 상태, 생성된 상태 보기, 아티팩트, 증거 기록, QA 기록, 수락 기록, 닫기 기록, 잔여 위험 기록, 실행 가능한 fixture, 적합성 결과, 구현 완료 동작을 만들지 않습니다.

## 현재 MVP에 포함되는 것

현재 MVP 범위는 평이한 언어 입력과 Task 생성, 범위 업데이트, 상태 및 닫기 준비 상태 확인, 쓰기 준비 승인, 로컬 접점 등록, 아티팩트 스테이징, 실행 및 증거 기록, 집중된 사용자 판단 기록, 닫기 시도로 제한됩니다.

포함되는 범위는 아래와 같습니다.

| 범위 항목 | 현재 MVP에서의 의미 | 주 담당 문서 |
|---|---|---|
| 평이한 언어 입력과 Task 생성 | 사용자의 평이한 의도에서 로컬 Task를 시작할 수 있습니다. | [MVP API](api/mvp-api.md), [Core 모델](core-model.md) |
| 범위 업데이트 | 활성 범위 업데이트 경로로 Task와 Change Unit 범위를 갱신할 수 있습니다. | [MVP API](api/mvp-api.md), [Core 모델](core-model.md) |
| 상태와 닫기 준비 상태 확인 | 생성된 상태 보기나 런타임 아티팩트를 만들지 않고 현재 상태, 증거 충분성, 알려진 차단 사유, 닫기 준비 상태를 읽을 수 있습니다. | [API 상태 스키마](api/schema-state.md), [오류](api/errors.md), [상태 보기 권한 참조](projection-and-templates.md) |
| 쓰기 준비 승인 | `harness.prepare_write`는 호환되는 제품 파일 쓰기 시도에 대해 담당 범위의 1회용 `Write Authorization`을 만들 수 있습니다. | [MVP API](api/mvp-api.md), [저장 효과](storage-effects.md), [보안](security.md) |
| 로컬 접점 등록 | 등록된 로컬 접점은 현재 범위 확인에 필요한 활성 접점과 지원 역량을 식별할 수 있습니다. | [에이전트 통합](agent-integration.md), [접점별 사용 레시피](../use/surface-recipes.md), [보안](security.md) |
| 아티팩트 스테이징 | 새 아티팩트 바이트는 활성 스테이징 경로로만 현재 범위에 들어오고, 기존 아티팩트는 호환되는 지속 아티팩트 참조를 통해서만 연결됩니다. | [API 아티팩트 스키마](api/schema-artifacts.md), [아티팩트 저장소](storage-artifacts.md), [저장 효과](storage-effects.md) |
| 실행 및 증거 기록 | 활성 작업의 Run과 간결한 증거 요약을 기록할 수 있으며, 아티팩트 담당 문서가 허용할 때 호환되는 아티팩트 승격이나 연결도 함께 다룰 수 있습니다. | [MVP API](api/mvp-api.md), [저장 효과](storage-effects.md), [Core 모델](core-model.md) |
| 집중된 사용자 판단 기록 | 활성 판단 경로로 사용자 소유 판단을 요청하고 기록할 수 있습니다. 판단 담당 문서가 허용하는 경우 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소도 여기에 포함됩니다. | [Core 모델](core-model.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md) |
| 닫기 시도 | `harness.close_task`는 증거, 최종 수락, 잔여 위험, 대체 불가 경계를 지키면서 닫기 준비 상태를 확인하고 지원되는 닫기 결과를 시도할 수 있습니다. | [MVP API](api/mvp-api.md), [Core 모델](core-model.md), [오류](api/errors.md) |

읽는 시점의 상태나 파생 표시는 상태 및 닫기 준비 상태 확인의 일부일 때만 현재 범위에 들어옵니다. 지속 저장되는 상태 보기 작업, 생성된 상태 보기 파일, 관리되는 상태 보기 복구는 현재 범위가 아닙니다.

## 현재 MVP에서 제외되는 것

현재 MVP는 의도적으로 좁습니다. 보안 비주장과 보장 수준의 canonical 설명은 [보안](security.md)을 확인하세요. 이 저장소가 문서 전용 상태인 동안 현재 MVP는 런타임 구현도 아닙니다.

현재 MVP에는 접점 자체 아티팩트 캡처, `captured_artifact`, 상태 보기 조정, 지속 저장되는 상태 보기 작업, 관리 블록 불일치 복구, 전체 `Evidence Manifest`, 수동 QA 작업 흐름, `qa_gate`, `verification_gate`, 명령 관찰, 네트워크 관찰, 비밀값 접근 관찰, 명령/네트워크/비밀값의 도구 실행 전 차단, 예방형 보장, 격리형 보장, 호스팅 대시보드, 커넥터 마켓플레이스, 내보내기 또는 인계 형식, 실행 가능한 fixture 실행기, 생성된 적합성 산출물, 운영 프로필이 포함되지 않습니다.

민감 동작 승인은 담당 문서가 해당 역량을 승격하지 않는 한 현재 관찰이나 차단을 만들지 않습니다. 보안과 관찰 경계는 [보안](security.md)과 관련 이후 후보 담당 문서가 담당합니다.

## Profile-gated 값

일부 값 이름은 예약되었거나 profile-gated 값으로 분류될 수 있지만, 그것만으로 활성 사용자 표시 기능이 되지는 않습니다. 예약 값이나 profile-gated 값이 예시, 스키마, 이후 후보 표에 나온다고 해서 현재 MVP가 넓어지지 않습니다.

정확한 값 집합 위치는 [API 값 집합](api/schema-value-sets.md)이 담당합니다. 보안과 보장 의미는 [보안](security.md)이 담당합니다.

## 이후 후보

[이후 후보 색인](../later/index.md)은 미뤄 둔 후보 이름과 승격 경계를 담당합니다. 이후 후보는 담당 문서가 좁은 기능 범위, 대체 동작, 증명 기대, 한영 문서 동시 유지를 갖춰 승격하기 전까지 동작하지 않습니다.

이후 후보가 예시, 경로 문구, 스키마 메모, 이 참조 문서에 언급되어도 승격이 아니며 현재 MVP 요구사항이 되지 않습니다.

## 현재 보장 경계

현재 MVP의 보장 경계는 기본적으로 협력형입니다. 보장 수준, 탐지형 표현, 예방형/격리형 승격 규칙, 보안 비주장의 canonical 설명은 [보안](security.md)을 확인하세요.

`harness.prepare_write`와 `Write Authorization`은 제품 파일 쓰기 호환성 메커니즘입니다. 메서드 동작은 [MVP API](api/mvp-api.md)가 담당하고, Core 의미는 [Core 모델](core-model.md)이 담당합니다.

## 문서 전용 경계

이 문서나 연결된 참조 문서를 편집해도 하네스 서버 구현, 런타임 상태 생성, 적합성 실행, 상태 보기 생성, 아티팩트 스테이징, 증거 기록, QA 수락, 잔여 위험 수락, Task 닫기, 서버 코딩 승인이 일어나지 않습니다.

구현 준비 상태와 유지보수자 인계 상태는 [MVP 계획](../build/mvp-plan.md)에 남습니다. 그 계획이 런타임 작업을 명시적으로 승인하지 않는 한 이 저장소는 문서 전용입니다.

## 담당 문서 링크

| 필요 | 담당 문서 |
|---|---|
| 구현 준비와 유지보수자 인계 상태 | [MVP 계획](../build/mvp-plan.md) |
| Core 권한, Task 상태, 사용자 소유 판단 경계 | [Core 모델](core-model.md) |
| API 메서드 동작 | [MVP API](api/mvp-api.md) |
| API 스키마와 값 집합 | [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md) |
| 공개 오류와 닫기 차단 사유 경로 | [오류](api/errors.md) |
| 저장소 기록, 효과, 아티팩트 생명주기, 버전 관리, 잠금 | [저장소 기록](storage-records.md), [저장 효과](storage-effects.md), [아티팩트 저장소](storage-artifacts.md), [저장소 버전 관리](storage-versioning.md) |
| 런타임, 저장소, 서버 경계 | [런타임 경계](runtime-boundaries.md) |
| 보안 주장과 비보장 | [보안](security.md) |
| 접점과 커넥터 동작 | [에이전트 통합](agent-integration.md), [접점별 사용 레시피](../use/surface-recipes.md) |
| 상태 보기 권한과 원천 상태/최신성 경계 | [상태 보기 권한 참조](projection-and-templates.md) |
| 읽기용 표시의 템플릿 본문 | [템플릿 본문](template-bodies.md) |
| 이후 후보와 승격 경계 | [이후 후보 색인](../later/index.md) |
| 제품 용어 | [용어집](glossary.md), [번역 가이드](../maintain/translation-guide.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
