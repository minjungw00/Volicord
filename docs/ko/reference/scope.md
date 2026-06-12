# 범위 참조

이 참조 문서는 현재 하네스 MVP의 기능 경계를 담당합니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 현재 MVP 기능 경계
- 제품 범위 수준의 포함 항목과 제외 항목
- 활성 범위에 영향을 주는 예약된 값과 프로필 조건부 값의 경계
- 다른 문서가 반복하지 말고 요약해 연결해야 하는 범위 수준 보장과 비주장 문구

이 문서가 담당하지 않습니다.

- 구현 순서. [구현 가이드](../build/implementation-guide.md)를 봅니다.
- API 메서드 동작
- 스키마 필드
- 저장 효과
- 보안 증명
- 템플릿 본문
- 커넥터 동작
- 범위 밖 기능의 상세 명세

어떤 기능이 현재 MVP에 속하는지 판단할 때는 이 문서를 기준으로 삼습니다. 경로 문서, 빌드 문서, README, 참조 문서는 상세 범위 목록을 반복하지 말고 이 문서로 연결합니다.

## 현재 MVP에 포함되는 것

현재 MVP 범위는 아래 항목으로 제한됩니다.

- 자연어 접수와 Task 생성
- 범위 업데이트
- 상태 및 닫기 준비 상태 확인
- 쓰기 준비 승인
- 로컬 접점 등록
- 아티팩트 스테이징
- 실행 및 증거 기록
- 집중된 사용자 판단 기록
- 닫기 시도

| 범위 항목 | 주 담당 문서 |
|---|---|
| 자연어 접수와 Task 생성 | [접수 메서드](api/method-intake.md), [Core 모델](core-model.md) |
| 범위 업데이트 | [범위 갱신 메서드](api/method-update-scope.md), [Core 모델](core-model.md) |
| 상태 확인 | [상태 메서드](api/method-status.md), [API 상태 스키마](api/schema-state.md), [상태 보기 권한 참조](projection-and-templates.md) |
| 닫기 준비 상태 확인 | [Task 닫기 메서드](api/method-close-task.md), [API 상태 스키마](api/schema-state.md), [오류](api/errors.md) |
| 쓰기 준비 승인 | [쓰기 준비 메서드](api/method-prepare-write.md), [저장 효과](storage-effects.md), [보안](security.md) |
| 로컬 접점 등록 | [에이전트 통합](agent-integration.md), [접점별 사용 레시피](../use/surface-recipes.md), [보안](security.md) |
| 아티팩트 스테이징 | [아티팩트 스테이징 담당 문서](#artifact-staging-owners) 참고 |
| 실행 및 증거 기록 | [실행 기록 메서드](api/method-record-run.md), [저장 효과](storage-effects.md), [Core 모델](core-model.md) |
| 집중된 사용자 판단 기록 | [사용자 판단 담당 문서](#user-judgment-owners) 참고 |
| 닫기 시도 | [Task 닫기 메서드](api/method-close-task.md), [Core 모델](core-model.md), [오류](api/errors.md) |

자연어 접수와 Task 생성:
- 활성 의미: 활성 접수 경로를 통해 사용자의 자연어 의도에서 로컬 Task를 시작할 수 있습니다.

범위 업데이트:
- 활성 의미: 활성 범위 업데이트 경로로 Task와 Change Unit 범위를 갱신할 수 있습니다.

상태와 닫기 준비 상태 확인:
- 활성 의미: 현재 상태, 증거 충분성, 알려진 차단 사유, 닫기 준비 상태를 읽을 수 있습니다.
- 포함되지 않는 것: 이 읽기는 생성된 상태 보기나 런타임 아티팩트를 만들지 않습니다.

쓰기 준비 승인:
- 활성 의미: `harness.prepare_write`는 담당 범위의 1회용 `Write Authorization`을 만들 수 있습니다.
- 조건: 이 승인은 호환되는 제품 파일 쓰기 시도 하나에만 쓰입니다.

로컬 접점 등록:
- 활성 의미: 등록된 로컬 접점은 활성 접점과 지원 역량을 식별할 수 있습니다.
- 조건: 이 사실은 현재 범위 확인에만 사용됩니다.

<a id="artifact-staging-owners"></a>
아티팩트 스테이징 담당 문서:
- 메서드 동작: [아티팩트 스테이징 메서드](api/method-stage-artifact.md).
- API 형태: [API 아티팩트 스키마](api/schema-artifacts.md).
- 생명주기와 저장 효과: [아티팩트 저장소](storage-artifacts.md), [저장 효과](storage-effects.md).

아티팩트 스테이징:
- 활성 의미: 새 아티팩트 바이트는 활성 스테이징 경로로 현재 범위에 들어올 수 있습니다.
- 조건: 기존 아티팩트는 호환되는 지속 아티팩트 참조를 통해서만 연결됩니다.

실행 및 증거 기록:
- 활성 의미: 활성 작업의 실행 기록과 간결한 증거 요약을 남길 수 있습니다.
- 조건: 호환되는 아티팩트 승격이나 연결은 아티팩트 담당 문서가 허용할 때만 포함됩니다.

<a id="user-judgment-owners"></a>
사용자 판단 담당 문서:
- 메서드 동작: [사용자 판단 메서드](api/method-user-judgment.md).
- 제품 의미: [Core 모델](core-model.md).
- API 형태와 값: [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md).

집중된 사용자 판단 기록:
- 활성 의미: 활성 판단 경로로 사용자 소유 판단을 요청하고 기록할 수 있습니다.
- 포함되는 판단 경로: 판단 담당 문서가 허용하는 민감 동작 승인, 최종 수락, 잔여 위험 수락, 취소입니다.

닫기 시도:
- 활성 의미: `harness.close_task`는 닫기 준비 상태를 확인하고 지원되는 닫기 결과를 시도할 수 있습니다.
- 필요한 경계: 증거, 최종 수락, 잔여 위험, 대체 불가 경계는 그대로 유지됩니다.

상태 표시 경계:
- 현재 범위: 읽는 시점의 상태나 파생 표시는 상태 및 닫기 준비 상태 확인의 일부일 때만 현재 MVP 범위에 들어옵니다.
- 포함되지 않는 것: 지속 저장되는 상태 보기 작업, 생성된 상태 보기 파일, 관리되는 상태 보기 복구입니다.

## 현재 MVP에 포함되지 않는 것

현재 MVP는 의도적으로 좁습니다.

포함되지 않는 것:

- 접점 자체 아티팩트 캡처와 `captured_artifact`
- 상태 보기 조정, 지속 저장되는 상태 보기 작업, 관리 블록 불일치 복구
- 전체 `Evidence Manifest`
- 수동 QA 작업 흐름, `qa_gate`, `verification_gate`
- 명령, 네트워크, 비밀값 접근 관찰
- 명령, 네트워크, 비밀값의 도구 실행 전 차단
- 예방형 보장과 `isolated` 보장 의미
- 호스팅 대시보드
- 커넥터 마켓플레이스
- 내보내기 또는 인계 형식
- 실행 가능한 픽스처 실행기
- 생성된 적합성 산출물
- 운영 프로필

의미하지 않는 것:
- 제외된 기능은 활성 요구사항이 아닙니다.
- 민감 동작 승인은 담당 문서가 해당 역량을 승격하지 않는 한 현재 관찰이나 차단을 만들지 않습니다.

담당 문서 링크:
- 보안 비주장, 보장 수준, 관찰 경계: [보안](security.md).
- 값 이름과 예약된 값: [API 값 집합](api/schema-value-sets.md).

## 예약된 값과 프로필 조건부 값

일부 값 이름은 예약된 값이거나 프로필 조건부 값일 수 있지만, 그것만으로 사용자에게 보이는 활성 기능이 되지는 않습니다.

의미하지 않는 것:
- 예약된 값이나 프로필 조건부 값인 보장 라벨은 현재 MVP 범위를 확장하지 않습니다.
- 예시나 스키마에 나온다고 해서 동작이 활성화되지 않습니다.
- 값 집합에 나온다고 해서 보장이 제공되지 않습니다.
- 값 집합에 나온다고 해서 현재 MVP 기본값이 되지 않습니다.

담당 문서 링크:
- 정확한 보장 라벨 값 항목: [API 값 집합](api/schema-value-sets.md).
- `isolated`의 비주장을 포함한 보장 의미: [보안](security.md).

## 범위 밖 기능의 활성화

범위 밖 기능은 이 범위 참조와 영향받는 담당 문서가 좁은 활성 계약, 대체 동작, 증명 기대, 한영 문서 동시 유지를 정의하기 전까지 비활성입니다.

의미하지 않는 것:
- 제외되었거나 예약된 기능이 예시, 경로 문구, 스키마 메모, 이 참조 문서에 언급되어도 승격이 아니며 현재 MVP 요구사항이 되지 않습니다.

## 현재 보장 경계

현재 범위:
- 현재 MVP의 보장 경계는 기본적으로 `cooperative`입니다.
- `harness.prepare_write`와 `Write Authorization`은 제품 파일 쓰기 호환성 메커니즘입니다.

포함되지 않는 것:
- 현재 MVP는 `isolated` 보장 의미를 제공하지 않습니다.

의미하지 않는 것:
- 예약된 값이나 프로필 조건부 값인 보장 라벨은 현재 MVP 범위를 확장하지 않습니다.

담당 문서 링크:
- 보장 의미, 탐지형 표현, `preventive`와 `isolated` 승격 규칙, 보안 비주장: [보안](security.md).
- 보장 라벨 값 항목: [API 값 집합](api/schema-value-sets.md).
- 메서드 동작: [API 메서드](api/methods.md)가 안내하는 [쓰기 준비 메서드](api/method-prepare-write.md).
- Core 의미: [Core 모델](core-model.md).

## 문서 트리 경계

문서 트리는 유지되는 제품 문서와 시스템 문서를 저장합니다.

저장하지 않는 것:
- 런타임 상태
- 생성된 상태 보기
- 생성된 아티팩트
- 증거 기록
- QA 기록
- 수락 기록
- 닫기 기록
- 잔여 위험 기록
- 실행 가능한 픽스처
- 적합성 결과
- 제품 구현 산출물

담당 문서 링크:
- 구현 경로: [구현 가이드](../build/implementation-guide.md).
- 런타임, 저장소, 서버 경계: [런타임 경계](runtime-boundaries.md).

## 담당 문서 링크

| 필요 | 담당 문서 |
|---|---|
| 구현 경로 | [구현 가이드](../build/implementation-guide.md) |
| Core 권한, Task 상태, 사용자 소유 판단 경계 | [Core 모델](core-model.md) |
| API 메서드 동작 | [API 메서드](api/methods.md)와 메서드 담당 문서 |
| API 스키마와 값 집합 | [참조 색인의 API와 스키마 담당 문서](README.md#api와-스키마-담당-문서) |
| 공개 오류와 닫기 차단 사유 경로 | [오류](api/errors.md) |
| 저장소 기록, 효과, 아티팩트 생명주기, 버전 관리, 잠금 | [참조 색인의 저장소 담당 문서](README.md#저장소-담당-문서) |
| 런타임, 제품 저장소, 서버 경계 | [런타임 경계](runtime-boundaries.md) |
| 보안 주장과 비주장 | [보안](security.md) |
| 접점과 커넥터 동작 | [에이전트 통합](agent-integration.md), [접점별 사용 레시피](../use/surface-recipes.md) |
| 상태 보기 권한과 원천 상태/최신성 경계 | [상태 보기 권한 참조](projection-and-templates.md) |
| 읽기용 표시의 템플릿 본문 | [템플릿 본문](template-bodies.md) |
| 범위 밖 기능과 예약된 기능의 경계 | [범위](scope.md) |
| 제품 용어 | [용어집](glossary.md), [번역 가이드](../maintain/translation-guide.md), [docs/terminology-map.yaml](../../terminology-map.yaml) |
