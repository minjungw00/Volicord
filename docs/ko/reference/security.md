# 보안 참조

이 문서는 Volicord의 보안 보장 표현, 로컬 연결 가정, 민감 동작 승인 경계, `operation_category` 보안 의미, 명시적 보안 비주장을 담당합니다.

## 담당하는 것 / 담당하지 않는 것

| 이 문서가 담당하는 것 | 이 문서가 담당하지 않는 것 |
|---|---|
| `cooperative`와 연결 관찰 기반 `detective` 표현의 지원 보장 의미. | API 메서드 요청/응답 스키마나 메서드별 동작. |
| 기준 범위에 지원되는 예방형 보장이 없다는 경계. | 저장소 기록 배치, 아티팩트 생명주기 세부사항, 잠금, 해시, 마이그레이션. |
| 로컬 연결 가정, `operation_category` 비주장, 접근 경계 비주장. | 커넥터 구현이나 호스트별 운영 레시피. |
| 보안과 맞닿아 있는 사용자 소유 판단으로서 민감 동작 승인 경계. | OS 권한, 배포 통제, 임의 도구 샌드박싱, 호스트 정책. |
| 로컬 파일, 생성된 표시, 복사된 식별자, 대화 텍스트, 에이전트 기억이 권한이 아니라는 규칙. | 런타임 위치 정의. 위치 정의는 [런타임 경계](runtime-boundaries.md)가 담당합니다. |
| Agent Connection의 호스트 신뢰, 호스트 승인, 안내 비보장. | Codex 또는 Claude Code 호스트 설정 문법. 해당 문법은 [관리 CLI](admin-cli.md)가 담당합니다. |

## 경계 요약

Volicord 보안 표현은 문서화된 Volicord 경로 안의 기록과 정책 경계를 설명합니다. 운영체제 샌드박스, 악성코드 검사기, 네트워크 격리 계층, 완전한 호스트 신뢰 강제 시스템, 일반 호스트 정책 엔진을 설명하지 않습니다.

| 표면 | 지원되는 보안 의미 | 보장하지 않는 것 |
|---|---|---|
| `Volicord Runtime Home` | 저장소/런타임 담당 문서는 어떤 Volicord 운영 기록이 그 안에 있고 어떻게 검증되는지 정의합니다. | Runtime Home 배치는 OS 샌드박싱, 변조 방지 격리, 호스트 신뢰, 네트워크 격리, 악성코드 검사, 비밀값 검사가 아닙니다. |
| `Product Repository` | 제품 파일은 입력으로 검사될 수 있고, 호환되는 제품 파일 쓰기는 담당 문서가 정의한 Core, 사용자 판단, `Write Check` 경로의 지배를 받을 수 있습니다. | 제품 파일은 Volicord 상태가 아니며, Volicord는 임의 제품 파일 편집 권한, 악성코드 검사, 비밀값 검사, 전역 파일시스템 가로채기를 제공하지 않습니다. |
| Agent Connection과 호스트 설정 | 현재 호출이 등록된 연결과 맞을 때 Agent Connection은 문서화된 연결 맥락, `actor_source` 출처, 연결 의도, 모드, Connection Projects 허용 목록을 제공합니다. | 연결 설정은 OS 권한, 호스트 신뢰, 사용자 신원, 외부 호스트가 `volicord mcp --stdio`를 로드하거나 노출했다는 증거가 아닙니다. |
| `volicord mcp --stdio` | 어댑터는 MCP 호출을 Agent Connection 점검, Runtime Home 상태, Core, Store를 통해 라우팅합니다. | 이 프로세스 자체는 임의 제품 파일 편집 권한을 부여하거나, 권한을 지니는 사용자 판단을 기록하거나, 호스트 신뢰를 강제하거나, 명령을 차단하거나, 네트워크를 차단하거나, 도구를 격리하지 않습니다. |
| `volicord` CLI | 관리 명령은 설정, registry 상태, 지원되는 호스트 통합 상태를 관리합니다. | CLI는 공개 API 보안 경계, 호스트 신뢰 제어기, OS 권한 메커니즘, 포괄적 쓰기 승인이 아닙니다. |

## 지원되는 보안 보장

<a id="honest-guarantee-display"></a>
Volicord가 어떤 보장을 설명하려면 [범위](scope.md)와 이 보안 담당 문서가 모두 그 보장 수준을 지원해야 합니다. 보장 표시는 현재 `operation_category`, 관련되는 경우 현재 Agent Connection 또는 `User Channel` 출처, 기록된 관찰 사실, 지원되는 기준 범위에서 파생됩니다. 주장이 관찰된 연결 결과에 의존한다면 이름 붙은 연결 또는 증거 출처와 관찰 범위에 대해 관련 관찰이 기록되어 있어야 합니다.

보장 표시는 그 표시를 정당화하는 연결, 작업, 증거 관찰에 묶여 있어야 합니다. 협력형 Run 보고나 협력적 `agent_report` 관찰은 별도 지원 관찰 또는 외부 결과가 기록되고 인용되지 않는 한 `detective`나 외부 관찰 사실이 아닙니다.

지원되는 보장 표시 라벨은 `cooperative`와 `detective`입니다. 값 이름은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.

### `cooperative`

`cooperative`는 기준 범위의 기본 보안 보장입니다.

조건:
- 호출자, Agent Connection, User Channel, 로컬 관리 경로, 커넥터가 문서화된 Volicord 계약을 따릅니다.
- 주장이 문서화된 Core, API, 저장소, 런타임, 사용자 판단 경계 안에 머뭅니다.

주장할 수 있는 것:
- Volicord 기록, 쓰기 호환성, 증거 요약, 사용자 소유 판단, 닫기 준비 상태 결과는 담당 계약의 지배를 받습니다.
- 관련 담당 계약이 현재 상태와 호환되지 않는다고 정의하면 Volicord는 거부하거나, 진행을 막는 결과를 반환하거나, 집중된 사용자 소유 판단을 요구할 수 있습니다.

주장하면 안 되는 것:
- `cooperative`가 Volicord 소유 경로 밖의 임의 도구 동작, 호스트 명령, 네트워크 접근, 비밀값 접근, 제품 파일 편집을 막는다는 주장.
- `cooperative`가 OS 권한 강제, 샌드박싱, 변조 불가능한 격리, 완전한 보안 격리를 제공한다는 주장.

### 연결 관찰 기반 `detective`

`detective`는 제한적이고 관찰로 뒷받침되는 주장으로만 지원됩니다.

조건:
- 주장이 Agent Connection, User Channel, 외부 증거 출처, 또는 담당 문서가 지원하는 다른 관찰 출처를 이름으로 밝힙니다.
- 관련 `operation_category`와 담당 문서가 지원하는 관찰 경로가 그 주장을 지원합니다.
- 관련 관찰 또는 강제 확인이 통과했고 관찰된 동작에 대해 지원되는 사실을 만들었습니다.
- 관찰 범위가 문서화되어 있습니다.
- 변경 경로 표현은 기록된 관찰이 관련 동작의 변경 경로를 보고할 때만 사용합니다.

주장할 수 있는 것:
- 확인된 관찰 출처는 문서화된 관찰 범위 안에서 제한적인 관찰이나 불일치 보고를 뒷받침합니다.
- 보고 조건이 맞으면 관찰된 변경 경로에 대한 제한적 탐지 주장을 할 수 있습니다.
- 관찰 지원이 없거나 부족하면 관련 담당 문서가 정의한 문서화된 오류 동작으로 이어집니다.

주장하면 안 되는 것:
- 복사된 `connection_id`, `operation_category`, 커넥터 설명, `Projection`, 생성된 표시, 대화 메시지, 에이전트 기억이 역량이나 관찰을 증명한다는 주장.
- 연결 선언만으로 보장이 `cooperative`보다 높아진다는 주장.
- 협력형 Run 보고, 협력적 `agent_report`, 검증되지 않은 주장이 지원 관찰 사실 없이 표시를 `cooperative`보다 높인다는 주장.
- `detective` 표현이 예방, 샌드박싱, OS 권한 강제, 전체 모니터링, 변조 방지 저장소가 된다는 주장.

### 예방형 보장

기준 범위 계약은 지원되는 예방형 보장을 정의하지 않습니다.

주장하면 안 되는 것:
- Volicord가 임의 도구 실행을 예방한다는 주장.
- Volicord가 보편적인 도구 실행 전 차단을 제공한다는 주장.
- Volicord가 기본적으로 명령, 네트워크, 비밀값 접근을 관찰하거나 차단한다는 주장.
- Volicord가 OS 샌드박싱, 호스트 권한 강제, 더 강한 격리를 제공한다는 주장.

## 민감 동작 승인 경계

민감 동작 승인은 경계가 정해진 `SensitiveActionScope` 안에서 이름 붙은 민감 단계에 대한 사용자 소유 판단입니다.

주장할 수 있는 것:
- 관련 담당 문서가 요구사항을 정의하면 쓰기 호환성, 실행 기록, 닫기 전에 민감 동작 승인이 필요할 수 있습니다.
- 승인된 민감 단계는 사용자가 판단하도록 질문받은 프롬프트, `SensitiveActionScope`, 영향받는 대상, 보이는 결과에 묶입니다.

주장하면 안 되는 것:
- 민감 동작 승인이 `Write Check`, `WriteCheckAttemptScope`, OS 권한, 셸 권한, 명령 승인, 배포 승인, 최종 수락, 잔여 위험 수락, 제품 정확성이라는 주장.
- 민감 동작 승인이 제품 파일 쓰기, 명령, 호스트, 네트워크, 비밀값, 배포, 파괴적 동작, 포괄적인 활동을 승인한다는 주장.
- 포괄적 승인이 필요한 민감 동작 승인, 최종 수락, 잔여 위험 수락, 범위 결정, `Write Check`을 대신한다는 주장.

담당 문서 링크:
- [Core 모델](core-model.md): 사용자 소유 판단과 비대체 규칙.
- [API 판단 스키마](api/schema-judgment.md): `SensitiveActionScope` 형태.
- [쓰기 준비 메서드](api/method-prepare-write.md): `volicord.prepare_write` 동작.

## 로컬 연결 가정

Volicord 보안 주장은 로컬 행위자가 Volicord 상태, 기록, 아티팩트, 쓰기 호환성, 사용자 소유 판단에 대해 문서화된 Volicord 계약을 사용한다는 가정에 놓입니다.

주장할 수 있는 것:
- 로컬 제품 파일은 Volicord 확인이나 사용자 소유 판단의 입력이 될 수 있습니다.
- 로컬 런타임 데이터 위치는 저장소/런타임 담당 문서가 정의할 수 있습니다.
- Agent Connection은 [Agent Connection 참조](agent-connection.md), 메서드 담당 문서, 이 보안 담당 문서가 허용할 때 `actor_source=agent_connection:<connection_id>` 출처를 제공할 수 있습니다. 그 출처 문자열의 `connection_id` 부분은 프로세스 바인딩/출처 표기이지 사용자 대상 권한 토큰이나 저장 필드 이름이 아닙니다.
- `User Channel`은 Core와 메서드 담당 문서가 요구할 때 권한을 지니는 사용자 판단에 대해 `actor_source=local_user` 출처를 제공할 수 있습니다.
- Connection Projects는 Agent Connection에 명시적으로 허용된 `project_internal_id` 목록을 정의합니다. 사용자 대상 명령은 저장소 루트, 프로젝트 이름, alias, 또는 Volicord가 반환한 `project_selector`로 프로젝트를 선택합니다.
- `operation_category`는 작업을 `read`, `agent_workflow`, `user_only`, `admin_local`로 분류합니다.
- 기준 행위자 출처는 협력적 로컬 출처이지 암호학적 인간 신원 증명이 아닙니다.

주장하면 안 되는 것:
- 로컬 파일시스템 접근이 Volicord 권한을 증명한다는 주장.
- 로컬 경로, 디렉터리 이름, 복사된 식별자, 표시된 식별자, 렌더링된 텍스트가 보안 토큰이라는 주장.
- 문서화된 Volicord 계약 밖의 직접 로컬 수정이 유효한 Volicord 기록, 증거, 수락, 잔여 위험 수락, `Write Check`, 아티팩트 권한을 만든다는 주장.
- `Volicord Runtime Home`이 자동으로 OS 보안 경계, 샌드박스, 격리 계층이라는 주장.
- 호출자가 제공한 `verified` 플래그, 요청된 `operation_category`, 복사된 `actor_source`, 공개 요청 필드, 환경 변수가 Volicord 권한을 부여하거나 신뢰된 출처를 제공한다는 주장.
- `actor_source=agent_connection:<connection_id>`가 인간 신원을 증명하거나 사용자 권한을 제공한다는 주장.
- 호스트 설정 쓰기가 호스트가 MCP 서버를 신뢰, 승인, 로드, 초기화, 노출했다는 사실을 증명한다는 주장.
- 저장소 안내, MCP 서버 instructions, 호스트 규칙 파일이 모델 동작을 강제하거나 에이전트가 Volicord 도구를 선택한다고 보장한다는 주장.

## 권한 경계

### Volicord 기록

Volicord 기록은 그 기록을 만들고, 검증하고, 갱신하는 담당 계약을 통해서만 권한을 가집니다.

주장하면 안 되는 것:
- 로컬 파일 내용이 Volicord 데이터를 설명하거나 저장한다는 이유로 변조 방지된다는 주장.
- 제품 텍스트, 생성된 텍스트, 기록처럼 보이는 복사 텍스트가 Volicord 기록을 직접 바꾼다는 주장.

### `Product Repository` 파일

[런타임 경계](runtime-boundaries.md)는 `Product Repository`를 제품 파일 경계로 정의합니다. 이 절은 그 경계에 대한 보안 주장과 비주장만 담당합니다.

주장할 수 있는 것:
- 제품 파일은 입력으로 검사될 수 있습니다.
- 호환되는 제품 파일 쓰기는 현재 적용 범위, 현재 적용 Change Unit 호환성, 사용자 소유 판단, 그리고 쓰기 담당 문서가 요구하는 `Write Check`의 지배를 받을 수 있습니다.

주장하면 안 되는 것:
- 제품 파일이 Volicord 상태라는 주장.
- 제품 파일이 Volicord 권한을 증명한다는 주장.
- 주변에 Volicord 메타데이터가 있다는 이유로 제품 파일이 Volicord 기록이 된다는 주장.

### `Volicord Runtime Home`

보안 표현에서는 `Volicord Runtime Home`을 런타임/저장소 담당 문서가 정의하는 운영 데이터 위치로 다룹니다.

런타임 위치 정의는 [런타임 경계](runtime-boundaries.md)가 담당합니다. 이 절은 그 위치에 대한 보안 비주장만 담당합니다.

주장할 수 있는 것:
- 저장소/런타임 담당 문서는 어떤 Volicord 운영 데이터가 여기에 속하고 어떻게 검증되는지 정의합니다.

주장하면 안 되는 것:
- `Volicord Runtime Home`이 `Product Repository`라는 주장.
- `Volicord Runtime Home`이 자동으로 보안 경계라는 주장.
- 데이터를 `Volicord Runtime Home` 아래에 둔다는 사실이 보안 권한이나 격리를 증명한다는 주장.

### Agent Connection, User Channel, 작업 범주

연결 식별자, 사용자 채널 출처, 작업 범주는 주장할 수 있는 범위를 제한합니다.

주장할 수 있는 것:
- `connection_internal_id`, 연결 의도, `connection.mode`, Connection Projects, `operation_category`, `actor_source`는 현재 호출이 문서화된 연결 맥락에 맞은 뒤 런타임, Core, 메서드, 보안 담당 문서에 따라 사용할 수 있습니다.
- `actor_source`는 Core와 메서드 담당 문서가 현재 권한 해결 동작에 대해 그 값을 받아들일 때만 지속 출처를 제공할 수 있습니다.
- 권한을 지니는 사용자 판단에는 `User Channel`을 통한 `actor_source=local_user`가 필요합니다.

주장하면 안 되는 것:
- `connection_id` 자체가 권한 토큰이라는 주장.
- 복사된 내부 연결 식별자가 역량이나 사용자 권한을 증명한다는 주장.
- `connection.mode=workflow`가 OS 권한이나 포괄적 권한이라는 주장.
- `personal`, `shared`, `global` 연결 의도가 OS 권한, 호스트 신뢰, 포괄적 권한이라는 주장.
- `operation_category`가 OS 권한, 호스트 신뢰, 포괄적 권한이라는 주장.
- 텍스트에서 복사한 `actor_source`가 호출자 권한 토큰이라는 주장.
- 환경으로 제어되는 라벨, 공개 요청 필드, 임의 호출자 텍스트가 신뢰된 권한, 감사 사실, 검증 근거 입력이라는 주장.

### 호스트 신뢰와 안내

호스트 신뢰와 승인 결정은 외부 호스트와 사용자가 소유합니다. Volicord는 지원되는 설정을 설치하고 추가 사용자 동작이 필요한지 보고할 수 있지만, 호스트의 신뢰 결정을 통제하지 않습니다.

주장할 수 있는 것:
- 관리 CLI가 필요한 확인을 관찰할 수 있으면 managed host configuration state 검증은 `complete`를 `action_required`, `failed`와 구분할 수 있습니다.
- `action_required`는 설치 프로필 복구, 명령 링크 복구, 호스트 신뢰, 승인, restart, reload, 또는 그와 비슷한 사용자 통제 동작이 남은 관찰 가능한 차단 사유일 때 그 동작을 이름 붙일 수 있습니다.
- MCP 서버 instructions와 선택적 저장소 안내는 에이전트가 프로젝트와 도구를 선택하는 방법을 설명할 수 있습니다.

주장하면 안 되는 것:
- Codex 또는 Claude Code 설정 설치가 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 재시작, 다시 로드, 그 밖의 호스트 통제 동작을 우회한다는 주장.
- 설정은 설치되었지만 호스트가 여전히 사용자 통제 신뢰나 승인을 요구하는 경우 `action_required`가 실패한 설치라는 주장.
- 에이전트 instructions, `AGENTS.md` 블록, `CLAUDE.md`, `.claude/rules/` 파일, MCP 서버 instructions가 접근 제어, 보안 강제, 사용자 판단, `Write Check`, 또는 모델이 이를 따랐다는 증명이라는 주장.

### 생성된 표시와 텍스트

생성된 표시, 렌더링된 템플릿, 대화 텍스트, 커넥터 설명, 에이전트 기억은 독자가 원천 기록을 이해하도록 도울 수 있습니다.

주장하면 안 되는 것:
- 렌더링된 표시, `Projection`, 상태 카드, 템플릿 출력, 대화 메시지, 커넥터 설명, 에이전트 기억이 새로운 권한 원천이라는 주장.
- 표시된 `ArtifactRef`, `UserJudgment`, `Write Check`, `connection_id` 텍스트가 그 식별자가 가리키는 권한을 만든다는 주장.

## 명시적 비보장

### 운영체제와 격리

Volicord는 아래를 보장하지 않습니다.

- OS 수준 샌드박싱.
- OS 권한 강제.
- 네트워크 격리.
- 변조 불가능한 격리.
- 완전한 보안 격리.
- 로컬 사용자, 프로세스, 도구, 호스트 사이의 격리.

### 모니터링과 예방

Volicord는 아래를 보장하지 않습니다.

- 전체 파일시스템 모니터링.
- 기본 명령 모니터링.
- 기본 네트워크 모니터링.
- 기본 네트워크 차단.
- 기본 비밀값 접근 모니터링.
- 악성코드 검사.
- 비밀값 검사.
- 보편적 도구 실행 전 차단.
- Volicord 소유 경로 밖에서 이루어지는 악의적 에이전트 동작의 예방.

### 호스트 신뢰와 통합

Volicord는 아래를 보장하지 않습니다.

- 완전한 호스트 신뢰 강제.
- 외부 호스트가 `volicord mcp --stdio`를 신뢰, 승인, 로드, 초기화, 노출했다는 것.
- 호스트 instructions, 저장소 안내, MCP 서버 instructions가 모델 또는 도구 동작을 강제한다는 것.

### 저장소와 아티팩트 권한

Volicord는 아래를 보장하지 않습니다.

- 변조 방지 저장소.
- 기준 보장으로서의 Agent Connection 자체 아티팩트 캡처.
- 표시된 식별자만으로 생기는 아티팩트 권한.
- 복사된 아티팩트, 실행 기록, 증거, 판단 텍스트에서 생기는 검증이나 수락.

### 포괄적 권한 추론

Volicord는 독자나 에이전트가 아래에서 권한을 추론하도록 허용하지 않습니다.

- 포괄적 승인.
- 로컬 경로 이름.
- 복사된 `connection_id` 프로세스 바인딩 값.
- 표시된 `ArtifactRef` 값.
- 렌더링된 `Projection` 출력.
- `Product Repository` 텍스트.
- 커넥터 설명.
- 대화 텍스트나 에이전트 기억.

## 관련 담당 문서

- [범위](scope.md): 기준 범위 포함/제외와 지원되는 보장 경계.
- [Agent Connection 참조](agent-connection.md): Agent Connection, Connection Projects, 현재 연결 맥락, Agent Connection/User Channel 권한 경계.
- [런타임 경계](runtime-boundaries.md): User Channel 위치, Volicord 소스 저장소/설치 파일, 실행 파일 프로세스, `Product Repository`, `Volicord Runtime Home`, 외부 MCP 호스트 설정 경계.
- [API 값 집합](api/schema-value-sets.md): `GuaranteeDisplay.level`, `operation_category`, 그 밖의 값 이름.
- [API 오류 처리 경로](api/error-routing.md): 공개 오류 처리 경로.
- [Core 모델](core-model.md): 사용자 소유 판단, `Write Check`, 수락, 잔여 위험, 비대체 규칙.
- [API 판단 스키마](api/schema-judgment.md): `SensitiveActionScope`와 사용자 소유 판단 스키마 형태.
- [저장 효과](storage-effects.md), [저장소 기록](storage-records.md), [아티팩트 저장소](storage-artifacts.md): 저장 효과, 기록 배치, 아티팩트 권한 세부사항.
