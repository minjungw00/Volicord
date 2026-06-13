# 저장소 기록

이 문서는 기준 범위의 영속 저장소 기록 배치와 저장되는 기록 계열을 담당합니다. 영속 기록은 Core가 나중에 `Harness Runtime Home` 안에서 다시 읽을 수 있도록 커밋한 로컬 기록입니다.

영속 기록은 변조 불가능한 저장소, 위조 방지 증명, 외부 감사 보장, `Product Repository` 쓰기 권한을 뜻하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 기준 범위 영속 기록 계열과 로컬 저장소 모델 안에서의 위치
- 기록 계열 배치, 저장 범주, 테이블/파일 위치
- 저장소 소유 값 집합과 저장소 소유 JSON `TEXT` 배치
- 지원되지 않는 영속 계열에 대한 기록 수준 제외 경계

이 문서가 담당하지 않는 것은 아래 항목입니다.

- 메서드별 저장 효과: [저장 효과](storage-effects.md)
- 아티팩트 스테이징, 승격, 연결, 본문 읽기, 보존, 무결성 생명주기: [아티팩트 저장소](storage-artifacts.md)
- `project_state.state_version`, 멱등성, 이벤트 의미, 잠금, 마이그레이션: [저장소 버전 관리](storage-versioning.md)
- API 요청 또는 응답 스키마: [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md)
- API 메서드 동작: [API 메서드](api/methods.md)와 메서드 담당 문서
- 런타임 위치와 저장소 경계: [런타임 경계](runtime-boundaries.md)
- 보안 보장 수준과 보안 비주장: [보안](security.md)

## 저장소 모델

하네스는 기준 범위 기록을 로컬 `Harness Runtime Home` 하나와 등록된 프로젝트별 로컬 상태 데이터베이스 하나에 저장합니다. 기본 기준 루트는 `~/.harness`이며, 구현은 같은 역할을 하는 설정 루트를 선택할 수 있습니다.

```text
~/.harness/
  registry.sqlite
  projects/
    PRJ-0001/
      project.yaml
      state.sqlite
      artifacts/
        tmp/
```

저장 위치의 의미는 아래와 같습니다.

- `registry.sqlite`는 런타임 홈 식별 정보와 최소 프로젝트 등록을 저장합니다.
- `projects/{project_id}/`는 등록된 프로젝트 하나의 하네스 프로젝트 홈입니다. `repo_root`와 같은 위치나 권한이 아닙니다.
- `project.yaml`은 정적 프로젝트 설정만 저장합니다.
- `state.sqlite`는 등록된 프로젝트의 프로젝트별 로컬 Core 상태를 저장합니다.
- `artifacts/`는 프로젝트 아티팩트 저장소입니다. `artifacts/tmp/`는 임시 스테이징 공간이며 증거 권한이 아닙니다.

런타임 홈 식별은 파일시스템 경로에만 의존하면 안 됩니다. 복사되거나 이동된 런타임 홈은 같은 저장된 `runtime_home_id`를 가질 수 있고, 새 런타임 홈은 새 식별자를 가져야 합니다. 이 식별자는 의심스러운 복사본, 중복 등록, 경로 변경을 감지하는 데 도움이 될 수 있지만 보안 보장은 아닙니다.

런타임 홈 파일은 로컬 운영 제어 데이터이고 민감한 지원 데이터를 담을 수 있습니다. 제품 소스 파일, 빌드 산출물, 생성된 상태 보기, 수락 기록, 잔여 위험 기록, 닫기 기록이 아니며 제품 저장소 쓰기 권한을 대신하지 않습니다.

## 저장소 기록과 API 스키마

저장소 기록과 API 스키마는 서로 다른 담당 문서 경계를 가집니다.

- API 스키마 담당 문서는 요청/응답 데이터 형태, 공개 API 값, 공개 오류, 응답 분기를 정의합니다.
- 저장소 기록은 영속 기록 계열, 파일/테이블 배치, 저장소 소유 JSON `TEXT` 배치, 저장 관계, 커밋 시점 검증 기대를 정의합니다.
- 비슷한 이름이 같은 권한을 뜻하지 않습니다. `ArtifactRef`는 API 형태이고 `artifacts`와 `artifact_links`는 저장소 기록입니다. `CloseReadinessBlocker`는 API 형태이고 `blockers`는 저장되는 차단 사유 계열입니다.
- 렌더링된 상태 카드, 판단 요청, 실행/증거 요약, 닫기 결과, 에이전트 맥락 패킷은 읽는 시점의 보기입니다. 템플릿 문구는 [템플릿 본문](template-bodies.md)이 담당하고, 상태 보기 권한은 [상태 보기 권한 참조](projection-and-templates.md)가 담당합니다.

## 영속 기록 계열

기준 범위 저장소는 기준 범위 저장소 계약에서 정의한 지원되는 Core 기록 계열만 영속합니다. 어떤 분기가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지는 [저장 효과](storage-effects.md)가 담당합니다.

저장되는 계열은 아래와 같습니다.

- `registry.sqlite`의 런타임 홈 식별 정보.
- `registry.sqlite`의 프로젝트 등록.
- `project.yaml`의 정적 프로젝트 설정.
- `state.sqlite`의 프로젝트별 로컬 상태 기록.
- [아티팩트 저장소](storage-artifacts.md)가 정의하는 프로젝트 아티팩트 저장소의 아티팩트 메타데이터, 영속 아티팩트 본문, 임시 스테이징 바이트.

지원되지 않는 영속 계열은 아래와 같습니다.

- [범위](scope.md)와 관련 저장소 담당 문서가 승격하지 않은 다른 영속 테이블 계열이나 임시 핸들 계열은 기준 범위가 아닙니다.
- 지원되지 않는 계획 기록과 지원되지 않는 보조 작업 흐름 테이블을 다른 이름으로 도입하면 안 됩니다.
- 생성된 상태 보기 본문, 확장된 증거 패키지, QA 작업 흐름 기록, 수락 기록, 잔여 위험 기록, 닫기 기록은 별도 기준 범위 저장소 계열이 아닙니다.

## 기록 계열 개요

아래 표는 기준 범위의 저장소 기록 계열을 이름 붙입니다. 전체 DDL이 아니며 API 스키마, 메서드 효과, 아티팩트 생명주기 규칙, 렌더링된 템플릿 본문을 복사하지 않습니다.

| 기록 계열 | 저장 위치 | 저장 범주 |
|---|---|---|
| 런타임 홈 식별 정보 | `registry.sqlite` | 런타임 홈 식별자, 스키마/저장 프로필, 로컬 레지스트리 메타데이터. |
| 프로젝트 등록 | `registry.sqlite` | 등록된 프로젝트를 `repo_root`와 `project_home`에 연결하는 매핑. |
| `project.yaml` | 프로젝트 홈 | 등록된 프로젝트 하나의 정적 프로젝트 설정. |
| `project_state` | `state.sqlite` | 프로젝트별 로컬 상태 헤더, 저장 프로필, 상태 시계 필드, 현재 적용 `Task` 포인터, 기본 접점 포인터. |
| `surfaces` | `state.sqlite` | API 요청 래퍼 호환성, 기능 표시, 로컬 접근 상태에 필요한 등록된 로컬 접점 사실. |
| `tasks` | `state.sqlite` | 사용자 가치 작업 단위, 구체화 요약, 생명주기/결과/닫기 요약, 현재 적용 `CompletionPolicy`, 현재 적용 Change Unit 포인터. |
| `change_units` | `state.sqlite` | 범위 있는 작업 경계, 쓰기/닫기 근거, 범위 요약, Change Unit 생명주기. |
| `user_judgments` | `state.sqlite` | 대기 중이거나 해결된 사용자 소유 판단, 필요한 경우 별도 민감 동작 승인 범위. |
| `write_authorizations` | `state.sqlite` | 단일 사용 협력형 `Write Authorization` 기록, 기준 버전, 시도 범위, 만료, 소비 상태. |
| `runs` | `state.sqlite` | 커밋된 실행 또는 관찰 기록, 호환되는 승인 소비, 간결한 증거 갱신. |
| `artifact_staging` | `state.sqlite`와 `artifacts/tmp/` | 임시 스테이징 아티팩트 핸들, 안전한 스테이징 메타데이터, 임시 바이트 또는 알림. |
| `artifacts` | `state.sqlite`와 아티팩트 저장소 | 영속 아티팩트 메타데이터 또는 본문 위치, 무결성, 가림 처리, 보존, 생산자, 가용성 사실. |
| `artifact_links` | `state.sqlite` | 아티팩트와 지원되는 Core/API 기록 사이의 담당 관계. |
| `evidence_summaries` | `state.sqlite` | 간결한 증거 범위, 지원 참조, 공백 참조. |
| `blockers` | `state.sqlite` | 다음 행동, 쓰기 호환성, 증거 공백, 닫기 준비 상태, 복구를 위한 구조화된 차단 사유 상태. |
| `task_events` | `state.sqlite` | 커밋된 Core 변경의 추가 전용 순서 및 감사 기록. |
| `tool_invocations` | `state.sqlite` | 저장 효과 담당 문서가 재실행을 만들도록 한 커밋된 `dry_run=false` Core 메서드 결과의 재실행 행. |

## 기록 배치 규칙

### 식별자와 범위

기준 범위 기록은 불투명하고 안정적인 식별자를 기본 키 또는 동등한 고유 키로 사용합니다. 고유성은 담당 기록 계열의 소유 범위 안에서 적용됩니다.

- 런타임 홈 식별 정보는 그 런타임 홈의 `runtime_home_id` 하나를 저장합니다.
- 프로젝트 등록에는 고유한 프로젝트 식별자와 고유한 프로젝트 홈이 필요합니다.
- 프로젝트 범위 행은 등록된 프로젝트에 속합니다.
- `Task` 범위 행은 자신을 소유한 `tasks` 행과 같은 프로젝트와 같은 `Task`에 속합니다.
- 현재 적용 포인터, 기본 접점 포인터, 담당 참조는 같은 프로젝트의 기록을 가리켜야 합니다.
- `Task` 하나에는 현재 적용 Change Unit이 최대 하나만 있습니다.
- 소비된 `Write Authorization` 행, 소비된 스테이징 핸들, 승격된 스테이징 아티팩트, 아티팩트 담당 연결, 재실행 키처럼 단일 사용 관계는 여러 커밋 의미로 갈라지면 안 됩니다.

### 현재 행, 이벤트 행, 재실행 행

현재 기록 계열은 일반 읽기에 쓰는 현재 Core 상태를 담습니다. `task_events`는 커밋된 Core 변경의 추가 전용 순서 및 감사 기록입니다. `tool_invocations`는 [저장 효과](storage-effects.md)가 재실행 생성을 정의한 경우에만 커밋된 재실행 행을 저장합니다.

이벤트 의미, 멱등성, 재실행 충돌 처리, 잠금, 마이그레이션 동작은 [저장소 버전 관리](storage-versioning.md)가 담당합니다.

### 관계 검증

저장소는 커밋 전에 저장 관계를 검증해야 합니다. 검증에는 아래 항목이 포함됩니다.

- 같은 프로젝트와 같은 `Task` 소유 관계
- 현재 적용 포인터 대상
- 호환되는 `Write Authorization` 소비
- 아티팩트 스테이징 소비와 승격 대상
- 아티팩트 담당 관계
- SQLite가 직접 외래 키로 표현할 수 없는 JSON 참조 배열

### 삭제 비주장

일반적인 기준 범위 Core 동작은 권한 행을 하드 삭제하지 않습니다. 행은 상태 또는 생명주기 필드로 이동하고, Core는 이벤트를 추가하며, 재실행 행과 아티팩트 메타데이터는 감사와 복구에 사용할 수 있게 남습니다.

`Task`를 완료, 취소, 대체해도 `tasks`, `change_units`, `user_judgments`, `write_authorizations`, `runs`, `artifacts`, `artifact_links`, `evidence_summaries`, `blockers`, `task_events`, `tool_invocations` 같은 권한 행이 연쇄 삭제되면 안 됩니다.

소비되지 않았거나 만료된 `artifact_staging` 행과 `artifacts/tmp/`의 스테이징 바이트 또는 알림은 `expired` 또는 `discarded`로 표시할 수 있고, 영속 아티팩트 등록 전 임시 바이트는 정리할 수 있습니다. `artifacts` 행이 커밋된 뒤의 보존 삭제, 프로젝트 해체, 파괴적 정리는 담당 문서가 정의한 경로가 필요합니다.

## 저장소 소유 값

닫힌 저장소 소유 값 집합은 영속 제약입니다. 알 수 없는 값은 커밋할 수 없습니다.

저장소가 담당하는 기준 범위 저장 값은 아래와 같습니다.

- 프로젝트 등록 `status`: `active`.
- `change_units.status`: `proposed`, `active`, `replaced`, `closed`.
- `write_authorizations.status`: `active`, `consumed`, `expired`, `stale`, `revoked`.
- `artifact_staging.status`: `staged`, `consumed`, `expired`, `discarded`.
- `artifacts.status`: `available`, `missing`, `integrity_failed`, `unavailable`.
- `artifact_links.owner_record_kind`: `task`, `change_unit`, `run`, `user_judgment`, `evidence_summary`, `blocker`.
- `blockers.status`: `active`, `resolved`, `superseded`.
- `tool_invocations.status`: `committed`.

공개 API 스키마 값을 반영하는 행은 API 스키마 담당 문서와 정확히 맞아야 합니다. 이 문서는 `tasks.mode`, `tasks.lifecycle_phase`, `tasks.result`, `runs.kind`, `runs.status`, `user_judgments.status`, `evidence_summaries.status` 같은 필드의 공개 API 값을 다시 정의하지 않습니다. 공개 API 값은 [API 값 집합](api/schema-value-sets.md), [API 상태 스키마](api/schema-state.md), 메서드 담당 문서를 봅니다.

## 저장소 소유 JSON

JSON을 저장하는 SQLite `TEXT` 열은 저장 표현 선택일 뿐이며 임의 JSON을 저장해도 된다는 뜻이 아닙니다.

규칙:

- Core는 커밋 전에 JSON을 파싱하고 검증해야 합니다.
- API 형태의 저장 JSON은 API 스키마 담당 문서를 기준으로 검증합니다.
- 저장소 전용 JSON은 이 문서나 이 문서가 가리키는 담당 문서를 기준으로 검증합니다.
- `'{}'`, `'[]'` 같은 SQLite 기본값은 저장 기본값일 뿐이며 API 필드를 선택 필드로 만들지 않습니다.

기준 범위 JSON `TEXT` 열은 아래 항목에 대한 간결한 담당 형태 데이터를 저장합니다.

- 접점 기능 프로필 데이터
- `Task`와 Change Unit의 구체화 요약, 제한된 목록, 자율성 경계, `CompletionPolicy`
- 사용자 판단 요청, 맥락, 선택지, 영향 참조, 아티팩트 참조, 민감 동작 승인 범위, 해결 데이터
- `Write Authorization` 시도 범위
- 실행 관찰과 증거 갱신 데이터
- 증거 범위와 공백 참조
- 차단 사유의 담당 참조와 관련 참조
- 이벤트 페이로드
- 커밋된 재실행 응답

`Task`와 Change Unit 구체화 JSON은 간결한 요약과 제한된 목록만 저장합니다. 지원되지 않는 계획 기록, 생성된 상태 보기 본문, 확장된 증거 패키지 본문, QA 작업 흐름 기록, 수락 기록, 잔여 위험 기록, 닫기 기록을 다른 이름으로 저장하면 안 됩니다.

## 기준 범위 / 지원 범위 밖 경계

프로필 조건부 저장소는 [범위](scope.md)와 관련 저장소 담당 문서가 대체 동작과 증명 경로 기대치를 갖춘 지원 계약을 정의하지 않는 한 기준 범위 밖에 있습니다. 참조 스키마에 존재한다는 사실만으로 저장소가 지원되지는 않습니다.

기준 범위 저장소는 지원 범위 밖 기능 계열, 생성된 운영 출력, 확장된 증거 패키지, 호스팅 서비스, 접점 간 조율, 지원되지 않는 계획 기록, 지원되지 않는 보조 작업 흐름 테이블, 장기 설계 지원 기록을 제외합니다.

상태, 닫기 준비 상태, 실행/증거 요약, 다음 행동, 읽기용 카드, `agent-context-packet`, 보장 표시는 기준 범위 영속 기록 위에서 읽는 시점에 파생하는 보기입니다. 이런 보기는 오래되었거나, 없거나, 실패했을 수 있고, 다시 계산되어도 저장소 권한을 바꾸지 않습니다.

## 관련 담당 문서

- [저장 효과](storage-effects.md): 어떤 메서드가 기록을 만들거나, 바꾸거나, 관찰하거나, 건드리지 않는지.
- [아티팩트 저장소](storage-artifacts.md): 아티팩트 전용 저장 생명주기.
- [저장소 버전 관리](storage-versioning.md): 시계, 멱등성, 잠금, 이벤트 의미, 재실행, 마이그레이션 의미.
- [API 메서드](api/methods.md)와 메서드 담당 문서: 기록을 사용하는 공개 메서드 동작.
- [API 코어 스키마](api/schema-core.md), [API 상태 스키마](api/schema-state.md), [API 아티팩트 스키마](api/schema-artifacts.md), [API 판단 스키마](api/schema-judgment.md), [API 값 집합](api/schema-value-sets.md): 요청/응답 형태와 공개 API 값.
- [템플릿 본문](template-bodies.md): 사용자에게 보이는 상태 카드, 판단 요청, 실행/증거 요약, 닫기 결과, 에이전트 맥락 패킷의 표시 본문.
- [상태 보기 권한 참조](projection-and-templates.md): 읽기 전용 상태 보기 권한, 원천 기록, 최신성 경계.
- [런타임 경계](runtime-boundaries.md): 런타임 홈과 제품 저장소 경계.
- [보안](security.md): 보안 비주장과 보장 수준.
