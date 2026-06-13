# 용어집

이 문서는 하네스 문서의 공식 용어를 담당합니다. 제품 용어의 산문 의미, 한국어 기준 표현, 사용자용 표현, 식별자 보존, 피할 표현, 담당 문서 경로를 정리합니다.

정확한 스키마, 값 집합, DDL, 저장 효과, 보안 메커니즘, API 동작, 런타임 동작, 기준 범위 구현 읽기 경로는 이 문서가 정의하지 않습니다.

## 이 용어집을 사용하는 방법

요약 표는 빠른 경로 확인용으로 사용합니다. 각 용어의 실제 통제 내용은 용어 카드에서 관리합니다.

이 용어집은 [docs/terminology-map.yaml](../../terminology-map.yaml)과 함께 사용합니다. 용어집은 독자가 읽는 의미와 담당 문서 경로를 설명합니다.

용어 지도는 한영 용어 선택, 식별자 보존, 피해야 할 한국어 혼합 표현을 기계 판독 가능한 형태로 관리합니다.

정확한 식별자는 영어와 한국어 모두에서 백틱으로 보존합니다.

카드가 스키마, API, 저장소, 보안, 상태 보기, 런타임 계약을 가리킬 때는 계약 세부사항을 용어집에 복사하지 말고 담당 문서를 따릅니다.

## 요약 표

| 영어 용어 | 한국어 기준 용어 | 주 담당 문서 |
|---|---|---|
| Harness | 하네스 | [기준 범위](scope.md) |
| Product Repository | Product Repository | [런타임 경계](runtime-boundaries.md) |
| Harness Runtime Home | Harness Runtime Home | [런타임 경계](runtime-boundaries.md) |
| documentation | 문서 | [작성 가이드](../maintain/authoring-guide.md) |
| baseline scope | 기준 범위 | [기준 범위](scope.md) |
| supported scope | 지원 범위 | [기준 범위](scope.md) |
| supported behavior | 지원 동작 | [기준 범위](scope.md) |
| supported API method | 지원되는 API 메서드 | [API 메서드](api/methods.md) |
| supported API value | 지원되는 API 값 | [API 값 집합](api/schema-value-sets.md) |
| out-of-scope capability | 지원 범위 밖 기능 | [범위 참조](scope.md) |
| owner document | 담당 문서 | [작성 가이드](../maintain/authoring-guide.md) |
| applicable owner path | 적용되는 담당 경로 | [작성 가이드](../maintain/authoring-guide.md) |
| existing owner | 기존 담당 문서 | [작성 가이드](../maintain/authoring-guide.md) |
| promotion-time owner update | 승격 시점의 담당 문서 갱신 | [범위 참조](scope.md) |
| owner placeholder | 담당 문서 자리표시자 | [작성 가이드](../maintain/authoring-guide.md) |
| `Task` | `Task` | [Core 모델](core-model.md) |
| scope | 범위 | [Core 모델](core-model.md) |
| active scope | 현재 적용 범위 | [Core 모델](core-model.md) |
| active Change Unit | 현재 적용 Change Unit | [Core 모델](core-model.md) |
| user-owned judgment | 사용자 소유 판단 | [Core 모델](core-model.md) |
| close readiness | 닫기 준비 상태 | [Core 모델](core-model.md) |
| close readiness evaluation | 닫기 준비 상태 평가 | [Task 닫기 메서드](api/method-close-task.md) |
| close-readiness blocker | 닫기 차단 사유 | [Core 모델](core-model.md) |
| `CloseReadinessBlocker` | `CloseReadinessBlocker` | [API 상태 스키마](api/schema-state.md) |
| complete intent | `complete` | [API 값 집합](api/schema-value-sets.md) |
| full evaluation order | 전체 평가 순서 | [번역 가이드](../maintain/translation-guide.md) |
| artifact | 아티팩트 | [API 아티팩트 스키마](api/schema-artifacts.md) |
| evidence | 증거 | [Core 모델](core-model.md) |
| `ArtifactRef` | `ArtifactRef` | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `ArtifactInput` | `ArtifactInput` | [API 아티팩트 스키마](api/schema-artifacts.md) |
| `StagedArtifactHandle` | `StagedArtifactHandle` | [API 아티팩트 스키마](api/schema-artifacts.md) |
| projection | 상태 보기 | [상태 보기 권한 참조](projection-and-templates.md) |
| surface | 접점 | [에이전트 통합](agent-integration.md) |
| active surface context | 현재 적용 접점 맥락 | [에이전트 통합](agent-integration.md) |
| runtime | 런타임 | [런타임 경계](runtime-boundaries.md) |
| `Write Authorization` | 쓰기 권한 부여 | [Core 모델](core-model.md) |
| sensitive approval | 민감 동작 승인 | [Core 모델](core-model.md) |
| access class | 접근 등급 | [API 값 집합](api/schema-value-sets.md) |
| baseline guarantee | 기준 범위 보장 | [보안](security.md) |
| cooperative guarantee | 협력형 보장 | [보안](security.md) |
| detective guarantee | 탐지형 보장 | [보안](security.md) |
| reserved value | 예약된 값 | [기준 범위](scope.md) |
| profile-gated value | 프로필 조건부 값 | [기준 범위](scope.md) |
| error routing | 오류 처리 경로 | [API 오류 처리 경로](api/error-routing.md) |
| blocker routing | blocker 처리 경로 | [API blocker 처리 경로](api/blocker-routing.md) |
| `ToolError.details` | `ToolError.details` | [API 오류 세부사항](api/error-details.md) |
| dry-run | dry-run 미리보기 | [API 코어 스키마](api/schema-core.md) |
| blocked result | 차단 결과 | [API 오류 처리 경로](api/error-routing.md) |
| rejected response | 거부 응답 | [API 코어 스키마](api/schema-core.md) |
| migration | 마이그레이션 | [저장소 버전 관리](storage-versioning.md) |
| lifecycle | 생명주기 | [Core 모델](core-model.md) |

## 용어

### Harness

영어:
- Harness

한국어:
- 참조 문서: 하네스
- 사용자 문서: 하네스

보존할 식별자:
- 제품 이름을 가리킬 때 Harness

피할 표현:
- 이 문서 저장소를 작동 중인 서버로 보는 표현

담당 문서:
- [기준 범위](scope.md)
- [런타임 경계](runtime-boundaries.md)

설명:
- 하네스는 AI 지원 제품 작업을 위한 로컬 작업 권한 서버입니다.

### Product Repository

영어:
- Product Repository

한국어:
- 참조 문서: Product Repository
- 사용자 문서: 제품 저장소

보존할 식별자:
- 경계를 이름 붙일 때 `Product Repository`

피할 표현:
- 제품 파일을 하네스 기록으로 보는 표현

담당 문서:
- [런타임 경계](runtime-boundaries.md)

설명:
- `Product Repository`는 사용자의 프로젝트 작업 공간이며 하네스 런타임 상태가 아닙니다.

### Harness Runtime Home

영어:
- Harness Runtime Home

한국어:
- 참조 문서: Harness Runtime Home
- 사용자 문서: 런타임 홈

보존할 식별자:
- 경계를 이름 붙일 때 `Harness Runtime Home`

피할 표현:
- 이 문서 저장소나 `Product Repository`를 `Harness Runtime Home`으로 보는 표현

담당 문서:
- [런타임 경계](runtime-boundaries.md)

설명:
- `Harness Runtime Home`은 하네스 기록과 아티팩트를 담는 운영 데이터 공간입니다.

### documentation

영어:
- documentation

한국어:
- 참조 문서: 문서
- 사용자 문서: 문서

보존할 식별자:
- 파일 경로와 담당 문서 라벨

피할 표현:
- 문서를 구현 권한으로 보는 표현
- 런타임 준비 완료
- 생성된 운영 기록

담당 문서:
- [작성 가이드](../maintain/authoring-guide.md)
- [런타임 경계](runtime-boundaries.md)
- [구현 가이드](../build/implementation-guide.md)

설명:
- 문서 작업은 런타임 구현이나 생성된 런타임 기록을 승인하지 않습니다.

### baseline scope

영어:
- baseline scope

한국어:
- 참조 문서: 기준 범위
- 사용자 문서: 기준 범위

보존할 식별자:
- 담당 문서 제목과 정확한 값 문자열

피할 표현:
- 지원 범위 밖 기능이나 프로필 조건부 값을 기준 범위 요구사항처럼 쓰는 표현

담당 문서:
- [기준 범위](scope.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 기준 범위는 안정적인 범위 계약이며, `Task`나 Change Unit 안에서 현재 적용되는 범위를 뜻하지 않습니다.

### supported scope

영어:
- supported scope

한국어:
- 참조 문서: 지원 범위
- 사용자 문서: 지원되는 범위

보존할 식별자:
- 담당 문서 제목과 정확한 값 문자열

피할 표현:
- `Task`나 Change Unit 안에서 현재 적용되는 범위를 이 용어로 부르는 표현

담당 문서:
- [기준 범위](scope.md)

설명:
- 지원 범위는 지원된다고 문서화된 동작이나 역량을 뜻합니다.

### supported behavior

영어:
- supported behavior

한국어:
- 참조 문서: 지원 동작
- 사용자 문서: 지원 동작

보존할 식별자:
- 담당 문서 제목과 정확한 값 문자열

피할 표현:
- 값 집합에 있다는 사실, 예시, 경로 요약, 담당 문서 경로 안내 용어만 보고 지원 동작으로 추론하는 표현
- 런타임에서 현재 적용되는 세션 상태를 이 용어로 부르는 표현

담당 문서:
- [기준 범위](scope.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 지원 동작은 기준 범위와 영향받는 의미 담당 문서가 지원된다고 문서화한 동작입니다.

### supported API method

영어:
- supported API method

한국어:
- 참조 문서: 지원되는 API 메서드
- 사용자 문서: 지원되는 API 메서드

보존할 식별자:
- 정확한 공개 API 메서드 식별자

피할 표현:
- 언급되었거나 예약된 메서드 이름을 지원되는 메서드처럼 보는 표현

담당 문서:
- [API 메서드](api/methods.md)

설명:
- 지원되는 API 메서드는 지원된다고 문서화된 공개 메서드입니다.

### supported API value

영어:
- supported API value

한국어:
- 참조 문서: 지원되는 API 값
- 사용자 문서: 지원되는 API 값

보존할 식별자:
- 정확한 enum 값, 상태 값, 스키마 값 문자열

피할 표현:
- 값 집합에 있다는 사실만으로 지원된다고 보는 표현

담당 문서:
- [기준 범위](scope.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 지원되는 API 값은 지원된다고 문서화된 값입니다.
- 정확한 값 이름 질문은 API 값 집합으로 보내고, 동작이나 지원 여부 질문은 의미 담당 문서로 보냅니다.

### out-of-scope capability

영어:
- out-of-scope capability

한국어:
- 참조 문서: 지원 범위 밖 기능
- 사용자 문서: 지원 범위 밖 기능

보존할 식별자:
- 승격 요구사항을 안내할 때 정확한 담당 문서 경로

피할 표현:
- 미뤄 둔 자료를 기준 범위 요구사항처럼 부르는 표현

담당 문서:
- [범위 참조](scope.md)
- [기준 범위](scope.md)

설명:
- 지원 범위 밖 기능은 [범위 참조](scope.md)와 영향받는 담당 문서가 지원된다고 정의하기 전까지 기준 범위 동작이 아닙니다.

### owner document

영어:
- owner document

한국어:
- 참조 문서: 담당 문서
- 사용자 문서: 담당 문서

보존할 식별자:
- 파일 경로
- 앵커
- `doc_id` 값

피할 표현:
- 두 번째 기준 문서
- 복사된 계약 담당 문서

담당 문서:
- [작성 가이드](../maintain/authoring-guide.md)
- [참조 색인](README.md)

설명:
- 담당 문서는 제품 개념, 계약, 스키마 묶음, 경로, 용어 규칙의 기준 의미를 정의할 수 있는 기준 문서입니다.

### applicable owner path

영어:
- applicable owner path

한국어:
- 참조 문서: 적용되는 담당 경로
- 사용자 문서: 적용되는 담당 경로

보존할 식별자:
- 파일 경로
- 앵커
- `doc_id` 값

피할 표현:
- 담당 경로 안내에 `active`를 쓰는 표현
- 문서 경로 안내만 뜻하는 자리에 제품 지원 계약 표현을 쓰는 표현
- 담당 문서 경로 안내 용어를 제품 동작이나 저장 지속 조건처럼 쓰는 표현

담당 문서:
- [작성 가이드](../maintain/authoring-guide.md)
- [참조 색인](README.md)
- [doc-index.yaml](../../doc-index.yaml)

설명:
- 적용되는 담당 경로는 어떤 주제에 적용되는 담당 문서 경로입니다.
- 문서 경로 안내 개념일 뿐, 제품 동작, 런타임 상태, 저장 지속 조건이 아닙니다.

### existing owner

영어:
- existing owner
- existing canonical owner
- existing owner document

한국어:
- 참조 문서: 기존 담당 문서
- 사용자 문서: 기존 담당 문서

보존할 식별자:
- 파일 경로
- 앵커
- `doc_id` 값

피할 표현:
- 담당 문서 자리표시자를 기존 담당 문서처럼 이름 붙이는 표현

담당 문서:
- [작성 가이드](../maintain/authoring-guide.md)
- [참조 색인](README.md)
- [doc-index.yaml](../../doc-index.yaml)

설명:
- 기존 담당 문서는 이미 존재하며 규범 의미의 기준으로 연결할 수 있을 때만 그렇게 부릅니다.

### promotion-time owner update

영어:
- promotion-time owner update

한국어:
- 참조 문서: 승격 시점의 담당 문서 갱신
- 사용자 문서: 승격 시점의 담당 문서 갱신

보존할 식별자:
- 파일 경로
- 앵커

피할 표현:
- 아직 없는 담당 문서가 이미 기존 담당 문서인 것처럼 이름 붙이는 표현

담당 문서:
- [작성 가이드](../maintain/authoring-guide.md)
- [범위 참조](scope.md)
- [기준 범위](scope.md)

설명:
- 승격 시점의 담당 문서 갱신에는 담당 문서를 만들거나 지정한 뒤 기준 범위, 스키마, API 동작, 저장소, 템플릿, 점검, 한영 문서를 함께 맞추는 일이 포함될 수 있습니다.

### owner placeholder

영어:
- owner placeholder

한국어:
- 참조 문서: 담당 문서 자리표시자
- 사용자 문서: 담당 문서 자리표시자

보존할 식별자:
- 범위 밖 기능의 담당 문서 공백을 안내하는 정확한 표현

피할 표현:
- 자리표시자를 기존 담당 문서처럼 독자에게 안내하는 표현

담당 문서:
- [작성 가이드](../maintain/authoring-guide.md)
- [범위 참조](scope.md)

설명:
- 이 표현은 지원 범위 밖 기능을 기준 범위로 승격하기 전에 담당 문서를 만들거나 지정해야 할 수 있음을 나타낼 때만 씁니다.
- 담당 문서 자리표시자는 기존 담당 문서가 아닙니다.

### `Task`

영어:
- `Task`

한국어:
- 참조 문서: `Task`
- 사용자 문서: 정확한 엔티티가 필요 없을 때는 작업

보존할 식별자:
- `Task`
- `task_id`
- `active_task_id`

피할 표현:
- 식별자를 번역하는 표현
- 하네스 엔티티가 필요한 자리에서 일반 할 일처럼 쓰는 표현

담당 문서:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- `Task`는 구체화, 실행, 차단, 닫기의 대상이 되는 사용자 가치 단위입니다.

### scope

영어:
- scope

한국어:
- 참조 문서: 범위
- 사용자 문서: 범위

보존할 식별자:
- `scope`
- `scope_decision`
- `AuthorizedAttemptScope`
- `SensitiveActionScope`

피할 표현:
- 스코프
- 조용한 범위 확장
- 광범위한 승인

담당 문서:
- [Core 모델](core-model.md)
- [범위 갱신 메서드](api/method-update-scope.md)
- [API 판단 스키마](api/schema-judgment.md)

설명:
- 범위는 현재 `Task`나 Change Unit이 포함하고 제외하는 합의된 경계입니다.

### active scope

영어:
- active scope
- currently applied scope

한국어:
- 참조 문서: 현재 적용 범위
- 사용자 문서: 현재 적용 범위

보존할 식별자:
- `scope`
- `scope_decision`
- `AuthorizedAttemptScope`
- `SensitiveActionScope`

피할 표현:
- 현재 적용 범위를 기준 범위, 지원 범위, 문서 계약처럼 쓰는 표현

담당 문서:
- [Core 모델](core-model.md)
- [범위 갱신 메서드](api/method-update-scope.md)

설명:
- active scope는 `Task`나 Change Unit 맥락 안에서 현재 적용되는 범위입니다.
- 한국어에서는 뜻이 더 분명해지는 자리에서 active를 자동으로 활성으로 옮기지 말고 현재 적용 범위를 우선합니다.

### active Change Unit

영어:
- active Change Unit

한국어:
- 참조 문서: 현재 적용 Change Unit
- 사용자 문서: 현재 적용 Change Unit

보존할 식별자:
- Change Unit
- `change_unit_id`

피할 표현:
- active Change Unit을 프로젝트 단계나 문서 작업 단계처럼 보는 표현

담당 문서:
- [Core 모델](core-model.md)
- [범위 갱신 메서드](api/method-update-scope.md)

설명:
- active Change Unit은 권한 모델에서 현재 적용되는 Change Unit입니다.
- 한국어 산문에서도 Change Unit은 제품 용어로 보존합니다.

### user-owned judgment

영어:
- user-owned judgment

한국어:
- 참조 문서: 사용자 소유 판단
- 사용자 문서: 사용자 판단

보존할 식별자:
- `user_judgment`
- `UserJudgment`
- `judgment_kind`

피할 표현:
- 광범위한 승인을 수락, 잔여 위험 수락, 범위 변경, 민감 동작 승인, `Write Authorization`으로 보는 표현

담당 문서:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)

설명:
- 사용자 소유 판단은 하네스가 추론하지 않고 사용자에게 묻거나 사용자 선택으로 보존해야 하는 결정입니다.

### close readiness

영어:
- close readiness

한국어:
- 참조 문서: 닫기 준비 상태
- 사용자 문서: 닫기 가능 여부

보존할 식별자:
- `CloseReadinessBlocker`

피할 표현:
- close 가능성 평가
- 닫기 가능성 평가

담당 문서:
- [Core 모델](core-model.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API blocker 처리 경로](api/blocker-routing.md)

설명:
- 평가 개념이며 차단 사유 스키마 자체가 아닙니다.

### close readiness evaluation

영어:
- close readiness evaluation

한국어:
- 참조 문서: 닫기 준비 상태 평가
- 사용자 문서: 닫기 준비 상태 평가

보존할 식별자:
- `harness.close_task`
- `CloseTaskResult`
- `CloseReadinessBlocker`

피할 표현:
- close 가능성 평가
- 닫기 가능성 평가

담당 문서:
- [Core 모델](core-model.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [API blocker 처리 경로](api/blocker-routing.md)

설명:
- 담당 문서 경계에 따라 닫기 준비 상태와 남은 닫기 차단 사유를 도출하는 확인입니다.

### close-readiness blocker

영어:
- close-readiness blocker
- close blocker

한국어:
- 참조 문서: 닫기 차단 사유
- 사용자 문서: 닫기 차단 사유

보존할 식별자:
- `close_blockers`
- `CloseReadinessBlocker`

피할 표현:
- 닫기 차단 사유를 영어로 남기는 표현
- 차단 사유에 영어 단어를 섞는 표현

담당 문서:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [API blocker 처리 경로](api/blocker-routing.md)

설명:
- 닫기 차단 사유는 담당 문서가 정의한 조건이 해결되기 전까지 정직한 닫기 준비 상태를 막는 이유입니다.

### `CloseReadinessBlocker`

영어:
- `CloseReadinessBlocker`

한국어:
- 참조 문서: `CloseReadinessBlocker`
- 사용자 문서: 스키마를 말하지 않을 때는 닫기 차단 사유

보존할 식별자:
- `CloseReadinessBlocker`
- `CloseReadinessBlocker.code`

피할 표현:
- 식별자를 번역하는 표현
- prepare-write 판단 사유처럼 쓰는 표현
- 닫기 준비 상태 전체 개념처럼 쓰는 표현

담당 문서:
- [API 상태 스키마](api/schema-state.md)
- [API 값 집합](api/schema-value-sets.md)
- [API blocker 처리 경로](api/blocker-routing.md)

설명:
- `CloseReadinessBlocker`는 닫기 준비 상태의 차단 데이터를 나타내는 API 스키마 식별자입니다.

### complete intent

영어:
- complete intent
- 의도 값 이름으로서 `complete`

한국어:
- 참조 문서: `complete`
- 사용자 문서: `complete`

보존할 식별자:
- `complete`
- `intent=complete`

피할 표현:
- 전체, 전체 평가, 전체 평가 순서를 뜻하는 산문에서 `complete`를 보존하는 표현
- 전체 평가 뜻으로 `complete`를 붙이는 표현
- 닫기 준비 상태 맥락의 전체 순서에 `complete`를 붙이는 표현

담당 문서:
- [용어 지도](../../terminology-map.yaml)
- [Task 닫기 메서드](api/method-close-task.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 이 항목에서 `complete`는 값 문자열일 때만 씁니다.
- `complete`는 enum 값이나 명시적 식별자일 때만 보존합니다.
- `complete`가 enum 값인지 전체 뜻 산문인지 묻는 경우 용어 지도와 이 용어집을 먼저 봅니다. 정확한 값 이름 계약이 필요할 때만 API 값 집합을 엽니다.

### full evaluation order

영어:
- full evaluation order
- entire evaluation order

한국어:
- 참조 문서: 전체 평가 순서, 닫기 준비 상태 맥락에서는 전체 닫기 준비 상태 평가 순서
- 사용자 문서: 전체 평가 순서, 닫기 준비 상태 맥락에서는 전체 닫기 준비 상태 평가 순서

보존할 식별자:
- 해당 없음

피할 표현:
- `complete` 평가 순서
- complete 평가 순서
- `complete` 닫기 준비 상태 순서
- complete 닫기 준비 상태 순서

담당 문서:
- [번역 가이드](../maintain/translation-guide.md)
- [용어 지도](../../terminology-map.yaml)

설명:
- 영어 산문에서는 enum 값 `complete`와 헷갈릴 수 있는 자리에서 `full`이나 `entire`를 씁니다.
- 닫기 준비 상태 맥락에서는 전체 닫기 준비 상태 평가 순서라고 씁니다.

### artifact

영어:
- artifact

한국어:
- 참조 문서: 아티팩트
- 사용자 문서: 아티팩트

보존할 식별자:
- `ArtifactRef`
- `ArtifactInput`
- `StagedArtifactHandle`
- `artifact_id`

피할 표현:
- 아티팩트를 영어로 남긴 저장 표현
- 아티팩트 본문 바이트를 영어로 남긴 표현
- 원시 경로를 권한 근거로 쓰는 표현

담당 문서:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

설명:
- 아티팩트의 정확한 저장 동작은 아티팩트 계약이 담당합니다.
- 아티팩트가 있다는 사실만으로 증거 충분성이 성립하지는 않습니다.

### evidence

영어:
- evidence

한국어:
- 참조 문서: 증거
- 사용자 문서: 증거

보존할 식별자:
- `EvidenceSummary`
- `EvidenceCoverageItem`
- `evidence_summary`
- `evidence_updates`
- `evidence_summaries`

피할 표현:
- 증거를 영어로 남긴 기록 표현
- 증거 요약을 영어로 남긴 표현
- 아티팩트가 있다는 사실을 증거 충분성으로 보는 표현

담당 문서:
- [Core 모델](core-model.md)
- [API 상태 스키마](api/schema-state.md)
- [실행 기록 메서드](api/method-record-run.md)
- [저장소 기록](storage-records.md)

설명:
- 증거는 기록된 범위 안에서 기록된 주장을 뒷받침합니다. 최종 수락, 잔여 위험 수락, 포괄적 검증, 아티팩트 존재를 대신하지 않습니다.

### `ArtifactRef`

영어:
- `ArtifactRef`

한국어:
- 참조 문서: `ArtifactRef`
- 사용자 문서: 스키마를 말하지 않을 때는 아티팩트 참조

보존할 식별자:
- `ArtifactRef`
- `existing_artifact_ref`

피할 표현:
- 식별자를 번역하는 표현
- 표시된 참조를 본문 읽기 권한이나 증거 충분성으로 보는 표현

담당 문서:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

설명:
- `ArtifactRef`는 등록된 지속 아티팩트를 가리키는 공개 포인터입니다.

### `ArtifactInput`

영어:
- `ArtifactInput`

한국어:
- 참조 문서: `ArtifactInput`
- 사용자 문서: 스키마를 말할 때는 `ArtifactInput`, 일반 설명에서는 제공할 아티팩트

보존할 식별자:
- `ArtifactInput`

피할 표현:
- 식별자를 번역하는 표현
- 아티팩트 입력만으로 증거 충분성이나 지속 아티팩트 권한이 생긴다고 보는 표현

담당 문서:
- [API 아티팩트 스키마](api/schema-artifacts.md)

설명:
- `ArtifactInput`은 아티팩트를 다루는 메서드에 제공되는 아티팩트 데이터의 스키마 식별자입니다.

### `StagedArtifactHandle`

영어:
- `StagedArtifactHandle`

한국어:
- 참조 문서: `StagedArtifactHandle`
- 사용자 문서: 스테이징된 아티팩트 핸들

보존할 식별자:
- `StagedArtifactHandle`
- `staged_artifact_handle`

피할 표현:
- 스테이징 핸들에서 영어 표현만 남기는 표현
- 베어러 토큰
- 지속 아티팩트

담당 문서:
- [API 아티팩트 스키마](api/schema-artifacts.md)
- [아티팩트 저장소](storage-artifacts.md)

설명:
- `StagedArtifactHandle`은 임시 핸들이며 그 자체로 지속 아티팩트 권한이 아닙니다.

### projection

영어:
- projection

한국어:
- 참조 문서: 상태 보기
- 사용자 문서: 상태 보기

보존할 식별자:
- `Projection`
- `ProjectionKind`

피할 표현:
- 렌더링된 표시를 Core 상태, 증거, 수락, 권한으로 보는 표현

담당 문서:
- [상태 보기 권한 참조](projection-and-templates.md)
- [템플릿 본문](template-bodies.md)

설명:
- 상태 보기는 담당 기록에서 만든 읽기 전용 파생 표시 또는 지원 맥락입니다.

### surface

영어:
- surface

한국어:
- 참조 문서: 접점
- 사용자 문서: 접점

보존할 식별자:
- `surface_id`
- `surface_instance_id`
- `VerifiedSurfaceContext`

피할 표현:
- 접점 정보를 영어로 남긴 표현
- 접점을 권한처럼 보이게 하는 표현
- `surface_id`를 권한 증거로 보는 표현

담당 문서:
- [에이전트 통합](agent-integration.md)
- [보안](security.md)

설명:
- 접점은 하네스가 쓰이거나 관찰되는 사용자, 에이전트, 도구, 커넥터, 로컬 맥락입니다.

### active surface context

영어:
- active surface context

한국어:
- 참조 문서: 현재 적용 접점 맥락
- 사용자 문서: 현재 적용 접점 맥락

보존할 식별자:
- `VerifiedSurfaceContext`
- `VerifiedSurfaceContext.access_class`
- `surface_id`
- `surface_instance_id`

피할 표현:
- 현재 적용 접점 맥락만으로 권한, 접근, 바인딩, 역량이 증명된다고 보는 표현

담당 문서:
- [에이전트 통합](agent-integration.md)
- [보안](security.md)

설명:
- active surface context는 요청이나 상호작용에 현재 적용되는 접점 맥락입니다.
- 한국어에서는 뜻이 더 분명해지는 자리에서 현재 적용 접점 맥락을 우선하고, 정확한 접점 식별자는 보존합니다.

### runtime

영어:
- runtime

한국어:
- 참조 문서: 런타임
- 사용자 문서: 런타임

보존할 식별자:
- `Harness Runtime Home`

피할 표현:
- Markdown 원천 문서를 런타임 상태로 보는 표현
- Markdown 원천 문서를 생성된 런타임 출력으로 보는 표현

담당 문서:
- [런타임 경계](runtime-boundaries.md)
- [보안](security.md)

설명:
- 런타임은 실행되는 하네스 서버/런타임 동작과 런타임 데이터 공간을 뜻합니다.

### `Write Authorization`

영어:
- `Write Authorization`

한국어:
- 참조 문서: 쓰기 권한 부여
- 사용자 문서: 쓰기 권한 부여

보존할 식별자:
- `Write Authorization`
- `AuthorizedAttemptScope`
- `WriteAuthorization.basis_state_version`

피할 표현:
- write permission
- command approval
- 민감 동작 승인 대체 표현

담당 문서:
- [Core 모델](core-model.md)
- [보안](security.md)
- [쓰기 준비 메서드](api/method-prepare-write.md)

설명:
- `Write Authorization`은 호환되는 제품 파일 쓰기 시도 하나를 위한 Core 권한 부여입니다.
- OS 권한이나 민감 동작 승인이 아닙니다.

### sensitive approval

영어:
- sensitive approval
- sensitive-action approval

한국어:
- 참조 문서: 민감 동작 승인
- 사용자 문서: 민감 동작 승인

보존할 식별자:
- `sensitive_approval`
- `SensitiveActionScope`

피할 표현:
- `Write Authorization`으로 보는 표현
- 최종 수락으로 보는 표현
- 광범위한 승인으로 보는 표현

담당 문서:
- [Core 모델](core-model.md)
- [API 판단 스키마](api/schema-judgment.md)
- [보안](security.md)

설명:
- 영어 산문에서는 `sensitive-action approval`을 기본 표현으로 씁니다.

### access class

영어:
- access class

한국어:
- 참조 문서: 접근 등급
- 사용자 문서: 접근 등급

보존할 식별자:
- `access_class`
- `VerifiedSurfaceContext.access_class`

피할 표현:
- 접근 등급을 OS 권한으로 보는 표현
- 접근 등급을 광범위한 권한으로 보는 표현

담당 문서:
- [API 값 집합](api/schema-value-sets.md)
- [에이전트 통합](agent-integration.md)
- [보안](security.md)

설명:
- 접근 등급은 API와 보안 담당 문서가 보호된 접근 기대를 설명할 때 쓰는 분류입니다.

### baseline guarantee

영어:
- baseline guarantee

한국어:
- 참조 문서: 기준 범위 보장
- 사용자 문서: 기준 범위 보장

보존할 식별자:
- 정확한 보장 라벨 값

피할 표현:
- 예약된 값이나 프로필 조건부 값을 기준 범위 보장처럼 쓰는 표현

담당 문서:
- [보안](security.md)
- [기준 범위](scope.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 기준 범위와 보안 담당 문서가 모두 지원된다고 문서화한 보장만 기준 범위 보장입니다.

### cooperative guarantee

영어:
- cooperative guarantee

한국어:
- 참조 문서: 협력형 보장
- 사용자 문서: 협력형 보장

보존할 식별자:
- `cooperative`

피할 표현:
- 협력형 표현을 탐지형, 샌드박스, 강제 차단, 더 강한 격리 표현으로 강화하는 표현

담당 문서:
- [보안](security.md)

설명:
- 협력형 보장은 접점이 문서화된 절차를 따른다는 전제에 놓입니다.

### detective guarantee

영어:
- detective guarantee

한국어:
- 참조 문서: 탐지형 보장
- 사용자 문서: 탐지형 보장

보존할 식별자:
- `detective`

피할 표현:
- 전체 모니터링을 주장하는 표현
- 예방을 주장하는 표현

담당 문서:
- [보안](security.md)
- [에이전트 통합](agent-integration.md)

설명:
- 탐지형 보장은 문서화된 관찰 범위와 역량 확인이 뒷받침할 때만 씁니다.

### reserved value

영어:
- reserved value

한국어:
- 참조 문서: 예약된 값
- 사용자 문서: 예약된 값

보존할 식별자:
- 정확한 값 문자열

피할 표현:
- 기본값
- 필수값
- 지원됨
- 강제됨
- 수락됨
- 검증됨
- 닫기 준비 상태
- 기준 범위 보장

담당 문서:
- [기준 범위](scope.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 예약된 값은 어휘나 예약된 접점으로 존재할 수 있지만, 이름만으로 지원 동작이 되지는 않습니다.
- 값 집합에 있다는 사실만으로 동작이 지원되지는 않습니다.

### profile-gated value

영어:
- profile-gated value

한국어:
- 참조 문서: 프로필 조건부 값
- 사용자 문서: 프로필 조건부 값

보존할 식별자:
- 정확한 값 문자열

피할 표현:
- 값 집합에 있다는 이유만으로 프로필 조건부 값을 기준 범위 동작처럼 쓰는 표현

담당 문서:
- [기준 범위](scope.md)
- [API 값 집합](api/schema-value-sets.md)

설명:
- 프로필 조건부 값은 관련 프로필과 담당 동작이 지원된다고 정의되어 있을 때만 사용할 수 있습니다.
- 값 집합에 있다는 사실만으로 동작이 지원되지는 않습니다.

### migration

영어:
- migration

한국어:
- 참조 문서: 마이그레이션
- 사용자 문서: 마이그레이션

보존할 식별자:
- 정확한 마이그레이션 식별자와 파일 경로

피할 표현:
- 기술 마이그레이션을 이전 선택이나 이전 결정으로 옮기는 표현
- 일회성 계획 파일로 남는 마이그레이션 설명 파일

담당 문서:
- [저장소 버전 관리](storage-versioning.md)
- [저장소 개요](storage.md)

설명:
- 마이그레이션은 스키마, 저장소, 데이터, 문서 구조를 옮기거나 갱신하는 기술 개념에 씁니다.

### error routing

영어:
- error routing
- API response branch routing
- 담당 문서 제목을 말할 때 API error routing

한국어:
- 참조 문서: 오류 처리 경로
- 사용자 문서: 오류 처리 경로

보존할 식별자:
- `ToolRejectedResponse`
- `ToolDryRunResponse`
- `CloseTaskResult(close_state=blocked)`

피할 표현:
- 오류 처리 경로를 공개 `ErrorCode` 의미, 오류 우선순위, `ToolError.details`, 닫기 준비 상태 blocker 처리 경로처럼 쓰는 표현

담당 문서:
- [API 오류 처리 경로](api/error-routing.md)

설명:
- 오류 처리 경로는 거부 응답, 차단 결과, `dry_run` 미리보기의 API 응답 분기 경로를 다룹니다.

### blocker routing

영어:
- blocker routing
- close-readiness blocker routing
- 담당 문서 제목을 말할 때 API blocker routing

한국어:
- 참조 문서: blocker 처리 경로
- 사용자 문서: blocker 처리 경로

보존할 식별자:
- `CloseReadinessBlocker`
- `CloseReadinessBlocker.category`
- `CloseReadinessBlocker.code`

피할 표현:
- blocker 처리 경로를 공개 `ErrorCode` 의미, 응답 분기 경로, `ToolError.details`처럼 쓰는 표현

담당 문서:
- [API blocker 처리 경로](api/blocker-routing.md)

설명:
- blocker 처리 경로는 닫기 준비 상태 blocker 처리 경로, `close_task` blocker 매핑, 공개 코드와 blocker 경계를 다룹니다.

### `ToolError.details`

영어:
- `ToolError.details`

한국어:
- 참조 문서: `ToolError.details`
- 사용자 문서: 정확한 API 식별자를 말하지 않을 때는 오류 세부사항

보존할 식별자:
- `ToolError.details`

피할 표현:
- 식별자를 번역하는 표현
- 세부사항 보조 값을 최상위 공개 `ErrorCode` 값처럼 쓰는 표현

담당 문서:
- [API 오류 세부사항](api/error-details.md)

설명:
- `ToolError.details`는 기계 판독용 오류 세부사항을 나타내는 정확한 API 세부 식별자입니다.

### dry-run

영어:
- dry-run

한국어:
- 참조 문서: dry-run 미리보기
- 사용자 문서: 미리보기

보존할 식별자:
- `dry_run`
- `ToolDryRunResponse`
- `DryRunSummary`
- `PlannedBlocker`

피할 표현:
- dry-run 출력을 커밋된 상태로 보는 표현
- dry-run 출력을 저장된 차단 사유로 보는 표현
- `PlannedBlocker`를 `CloseReadinessBlocker`로 보는 표현

담당 문서:
- [API 코어 스키마](api/schema-core.md)
- [API 메서드](api/methods.md)
- [API 오류 처리 경로](api/error-routing.md)
- [저장 효과](storage-effects.md)

설명:
- `dry-run`은 선택된 동작의 유효한 미리보기 경로이며 쓰기를 커밋하거나 담당 기록을 만들지 않습니다.

### blocked result

영어:
- blocked result

한국어:
- 참조 문서: 차단 결과
- 사용자 문서: 차단 결과

보존할 식별자:
- `CloseTaskResult(close_state=blocked)`
- `decision=blocked`
- `WriteDecisionReason`
- `CloseReadinessBlocker`

피할 표현:
- 거부 응답
- 공개 오류
- `STATE_VERSION_CONFLICT`를 차단 코드로 쓰는 표현

담당 문서:
- [API 오류 처리 경로](api/error-routing.md)
- [쓰기 준비 메서드](api/method-prepare-write.md)
- [Task 닫기 메서드](api/method-close-task.md)
- [저장 효과](storage-effects.md)

설명:
- 차단 결과는 메서드별 결과이며 공개 전송 오류나 스키마 거부가 아닙니다.

### rejected response

영어:
- rejected response

한국어:
- 참조 문서: 거부 응답
- 사용자 문서: 거부 응답

보존할 식별자:
- `ToolRejectedResponse`
- `ToolError`
- `ErrorCode`

피할 표현:
- 차단 결과
- 닫기 차단 사유
- 커밋된 결과

담당 문서:
- [API 코어 스키마](api/schema-core.md)
- [API 오류 처리 경로](api/error-routing.md)
- [저장 효과](storage-effects.md)

설명:
- 거부 응답은 메서드가 커밋 동작으로 진행하기 전에 실패했다는 뜻입니다.

### lifecycle

영어:
- lifecycle

한국어:
- 참조 문서: 생명주기
- 사용자 문서: 생명주기

보존할 식별자:
- `Task.lifecycle_phase`
- `artifact_staging.status`

피할 표현:
- 생명주기 의미를 영어로 남긴 표현

담당 문서:
- [Core 모델](core-model.md)
- [API 값 집합](api/schema-value-sets.md)
- [아티팩트 저장소](storage-artifacts.md)

설명:
- 생명주기는 `Task`나 아티팩트 핸들 같은 개념에서 허용되는 단계 진행입니다.
