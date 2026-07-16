# 보안

이 문서는 Volicord의 보안 보장 표현, 로컬 연결 가정, 민감 동작 승인 경계, `operation_category`의 보안 의미, 명시적으로 보장하지 않는 범위를 담당합니다.

## 담당하는 것 / 담당하지 않는 것

| 이 문서가 담당하는 것 | 이 문서가 담당하지 않는 것 |
|---|---|
| `cooperative`와 연결 관찰 기반 `detective` 표현의 지원 보장 의미. | API 메서드 요청/응답 스키마나 메서드별 동작. |
| 기준 범위에 지원되는 예방형 보장이 없다는 경계. | 저장소 기록 배치, 아티팩트 생명주기 세부사항, 잠금, 해시, 스키마 초기화. |
| 로컬 연결 가정, `operation_category`로 추론할 수 없는 것, 접근 경계에서 보장하지 않는 것. | 커넥터 구현이나 호스트별 운영 절차. |
| 보안과 맞닿아 있는 사용자 소유 판단으로서 민감 동작 승인 경계. | OS 권한, 배포 통제, 임의 도구 샌드박싱, 호스트 정책. |
| 닫기 상태, 검토, 배포, QA, 위험에 대한 비보장. | Task 닫기 메서드 동작이나 상태 스키마 형태. |
| 공개 네트워크, SaaS, 다중 사용자, 보안 경계에 대한 Local HTTP transport 비보장. | HTTP 와이어 동작. 해당 동작은 [MCP 전송](mcp-transport.md)이 담당합니다. |
| 로컬 파일, 생성된 표시, 복사된 식별자, 대화 텍스트, 에이전트 기억이 권한이 아니라는 규칙. | 런타임 위치 정의. 위치 정의는 [런타임 경계](runtime-boundaries.md)가 담당합니다. |
| Agent Connection이 호스트 신뢰, 호스트 승인, 안내 준수를 보장하지 않는다는 경계. | Codex 또는 Claude Code 호스트 설정 문법. 해당 문법은 [관리 CLI](admin-cli.md)가 담당합니다. |

## 경계 요약

Volicord 보안 표현은 문서화된 Volicord 경로 안의 기록과 정책 경계를 설명합니다. 운영체제 샌드박스, 악성코드 검사기, 네트워크 격리 계층, 완전한 호스트 신뢰 강제 시스템, 일반 호스트 정책 엔진을 설명하지 않습니다.

| 표면 | 지원되는 보안 의미 | 보장하지 않는 것 |
|---|---|---|
| `Volicord Runtime Home` | 저장소/런타임 담당 문서는 어떤 Volicord 운영 기록이 그 안에 있고 어떻게 검증되는지 정의합니다. | Runtime Home 배치는 OS 샌드박싱, 변조 방지 격리, 호스트 신뢰, 네트워크 격리, 악성코드 검사, 비밀값 검사가 아닙니다. |
| `Product Repository` | 제품 파일은 입력으로 검사될 수 있고, 호환되는 제품 파일 쓰기는 담당 문서가 정의한 Core, 사용자 행동, 쓰기 티켓 경로의 지배를 받을 수 있습니다. | 제품 파일은 Volicord 상태가 아니며, Volicord는 임의 제품 파일 편집 권한, 악성코드 검사, 비밀값 검사, 전역 파일시스템 가로채기를 제공하지 않습니다. |
| Agent Connection과 호스트 설정 | 현재 호출이 등록된 연결과 맞을 때 Agent Connection은 문서화된 연결 맥락, `actor_source` 출처, 연결 의도, 모드, Connection Projects 허용 목록을 제공합니다. | 연결 설정은 OS 권한, 호스트 신뢰, 사용자 신원, 외부 호스트가 `volicord mcp --stdio`를 로드하거나 노출했다는 증거가 아닙니다. |
| `volicord mcp --stdio` | 어댑터는 Agent Connection 점검, Runtime Home 상태, Core, Store를 거쳐 MCP 호출을 처리합니다. | 이 프로세스 자체는 임의 제품 파일 편집 권한을 부여하거나, 권한 효력이 있는 사용자 행동 resolution을 기록하거나, 호스트 신뢰를 강제하거나, 명령을 차단하거나, 네트워크를 차단하거나, 도구를 격리하지 않습니다. |
| Local HTTP transport | `volicord serve --transport local-http`는 베어러 토큰과 Origin 검사가 있는 문서화된 로컬 MCP-over-HTTP 부분 구현을 `localhost`와 Docker 호스트 루프백에 노출할 수 있습니다. 베어러 토큰은 `volicord serve` 프로세스의 로컬 비밀값입니다. 로컬 웹 동의 경로는 대기 중인 사용자 행동 하나를 위한 일회성 토큰이 있는 루프백 User Channel 입력 페이지를 노출할 수 있습니다. | Local HTTP transport와 로컬 웹 동의는 공개 네트워크 API, SaaS 엔드포인트, 다중 사용자 서버, 보안 경계, 공개 호스트 인터페이스 리스너, 원격 서비스, 인증·인가 서비스, 전체 MCP Streamable HTTP 구현이 아닙니다. |
| `volicord` CLI | 관리 명령은 설정, 레지스트리 상태, 관리 호스트 통합 상태를 관리합니다. | CLI는 공개 API 보안 경계, 호스트 신뢰 제어기, OS 권한 메커니즘, 포괄적 쓰기 승인이 아닙니다. |

## 지원되는 보안 보장

<a id="honest-guarantee-display"></a>
Volicord가 어떤 보장을 설명하려면 [범위](scope.md)와 이 보안 담당 문서가 모두 그 보장 수준을 지원해야 합니다. 보장 표시는 현재 `operation_category`, 관련되는 경우 현재 Agent Connection 또는 `User Channel` 출처, 기록된 관찰 사실, 지원되는 기준 범위에서 파생됩니다. 주장이 관찰된 연결 결과에 의존한다면 이름 붙은 연결 또는 증거 출처와 관찰 범위에 대해 관련 관찰이 기록되어 있어야 합니다.

보장 표시는 그 표시를 뒷받침하는 연결, 작업, 증거 관찰에 묶여 있어야 합니다. 협력형 `Run` 보고나 `agent_report` 관찰은 별도로 지원되는 관찰 또는 외부 결과가 기록되고 인용되지 않는 한 `detective`나 외부 관찰 사실이 아닙니다.

지원되는 보장 표시 라벨은 `cooperative`와 `detective`입니다. 값 이름은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.

### `cooperative`

`cooperative`는 기준 범위의 기본 보안 보장입니다.

조건:
- 호출자, Agent Connection, User Channel, 로컬 관리 경로, 커넥터가 문서화된 Volicord 계약을 따릅니다.
- 주장이 문서화된 Core, API, 저장소, 런타임, 사용자 행동 경계 안에 머뭅니다.

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
- 협력형 `Run` 보고, 협력적 `agent_report`, 검증되지 않은 주장이 지원 관찰 사실 없이 표시를 `cooperative`보다 높인다는 주장.
- `detective` 표현이 예방, 샌드박싱, OS 권한 강제, 전체 모니터링, 변조 방지 저장소가 된다는 주장.

세션 감시기의 `detective` 표현은 기록된 감시 범위 시작 뒤의 한정된 `Product Repository`
스냅샷 비교로 제한됩니다. 감시기는 `.git/`, `.volicord/`, `target/`,
`node_modules/`, `dist/`, `build/`, `coverage/`, `vendor/` 같은 기본 정책 경로를
건너뜁니다. Runtime Home/Product Repository 분리 규칙에 따라 선택된 `Volicord Runtime Home`은
스캔 대상 저장소 밖에 있으며, 감시기는 기본적으로 심볼릭 링크를 따라가지 않습니다. 상태형
출력은 확인할 수 있는 파일 수 제한, 파일 크기 제한, 읽을 수 없는 경로, 정책상 건너뛴
경로, 건너뛴 심볼릭 링크처럼 감시 범위를 건너뛰거나 저하한 이유를 보여 줘야 합니다. 이 사실은
전체 파일시스템 감시, 행위자 귀속, 쓰기 방지, 변조 불가능 감사, OS 강제, 보안 격리가
아닙니다.

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
- 민감 동작 승인이 쓰기 티켓, `WriteTicketScope`, OS 권한, 셸 권한, 명령 승인, 배포 승인, 최종 수락, 잔여 위험 수락, 제품 정확성이라는 주장.
- 민감 동작 승인이 제품 파일 쓰기, 명령, 호스트, 네트워크, 비밀값, 배포, 파괴적 동작, 포괄적인 활동을 승인한다는 주장.
- 포괄적 승인이 필요한 민감 동작 승인, 최종 수락, 잔여 위험 수락, 범위 결정, 쓰기 티켓을 대신한다는 주장.
- 호출자가 보고한 `performed_operation`과 티켓의 `intended_operation`이
  정확히 같다는 사실이 외부 동작의 실행이나 효과를 입증하거나 그 동작을 특정
  행위자에게 귀속한다는 주장. 이는 호출자가 제공한 좌표의 호환성 검사일 뿐입니다.

담당 문서 링크:
- [Core 모델](core-model.md): 사용자 소유 판단과 비대체 규칙.
- [API 판단 스키마](api/schema-judgment.md): `SensitiveActionScope` 형태.
- [쓰기 준비 메서드](api/method-prepare-write.md): `volicord.prepare_write` 동작.

## 로컬 연결 가정

Volicord 보안 주장은 로컬 행위자가 Volicord 상태, 기록, 아티팩트, 쓰기 호환성, 사용자 소유 행동에 대해 문서화된 Volicord 계약을 사용한다는 가정에 놓입니다.

주장할 수 있는 것:
- 로컬 제품 파일은 Volicord 확인이나 사용자 소유 행동의 입력이 될 수 있습니다.
- 로컬 런타임 데이터 위치는 저장소/런타임 담당 문서가 정의할 수 있습니다.
- Agent Connection은 [Agent Connection 참조](agent-connection.md), 메서드 담당 문서, 이 보안 담당 문서가 허용할 때 `actor_source=agent_connection:<connection_id>` 출처를 제공할 수 있습니다. 그 출처 문자열의 `connection_id` 부분은 프로세스 바인딩/출처 표기이지 사용자 대상 권한 토큰이나 저장 필드 이름이 아닙니다.
- `User Channel`은 Core와 메서드 담당 문서가 요구할 때 판단과 Evidence 관찰을 포함한 권한 효력이 있는 사용자 행동 resolution에 대해 `actor_source=local_user` 출처를 제공할 수 있습니다.
- Connection Projects는 Agent Connection에 명시적으로 허용된 `project_internal_id` 목록을 정의합니다. 사용자 대상 명령은 저장소 루트, 프로젝트 이름, 별칭, 또는 Volicord가 반환한 `project_selector`로 프로젝트를 선택합니다.
- `operation_category`는 작업을 `read`, `agent_workflow`, `user_only`, `admin_local`, `local_recovery`로 분류합니다.
- 기준 행위자 출처는 협력적 로컬 출처이지 암호학적 인간 신원 증명이 아닙니다.

Core가 도출한 영속 사용자 행동 요청·본문·근거, 완전한 inbox 항목, canonical 캡처 form,
캡처 경로, credential은 검증된 User Channel renderer에만 반환합니다. Agent가 자신이
작성한 draft text를 이미 알고 있을 수 있습니다. 이 규칙은 권한 있는 저장 projection의
비공개 규칙이지 Agent가 자신의 입력을 본 적이 없다는 주장이 아닙니다. Agent Connection
결과는 정규 대기 요청 요약과 안전한 현재 resolution projection만 담습니다. 이는 콘텐츠
가림 처리가 아니라 projection 경계입니다. 완전한 form은
처음부터 에이전트 결과, 정확한 replay, operation-result byte에 기록하지 않습니다.
사용자 전용 표면은 완전한 canonical 폼을 계속 표시합니다. Presentation safety 분류가
추가로 풍부한 host 입력 경로를 거절할 수 있지만 이 규칙은 일반 비밀값 검사, 콘텐츠
격리, 악성코드 탐지, host 강제, 임의의 비밀값을 찾았거나 배제했다는 증명이 아닙니다.

주장하면 안 되는 것:
- 로컬 파일시스템 접근이 Volicord 권한을 증명한다는 주장.
- 로컬 경로, 디렉터리 이름, 복사된 식별자, 표시된 non-credential 식별자, 일반 렌더링
  text가 보안 token이라는 주장. 원문 local-web token과 이를 포함한 완전한 URL은 bearer
  credential이며 이 비주장에 포함되지 않습니다.
- 문서화된 Volicord 계약 밖의 직접 로컬 수정이 유효한 Volicord 기록, 증거, 수락, 잔여 위험 수락, 쓰기 티켓, 아티팩트 권한을 만든다는 주장.
- `Volicord Runtime Home`이 자동으로 OS 보안 경계, 샌드박스, 격리 계층이라는 주장.
- 호출자가 제공한 `verified` 플래그, 요청된 `operation_category`, 복사된 `actor_source`, 공개 요청 필드, 환경 변수가 Volicord 권한을 부여하거나 신뢰된 출처를 제공한다는 주장.
- `actor_source=agent_connection:<connection_id>`가 인간 신원을 증명하거나 사용자 권한을 제공한다는 주장.
- 호스트 설정 쓰기가 호스트가 MCP 서버를 신뢰, 승인, 로드, 초기화, 노출했다는 사실을 증명한다는 주장.
- 저장소 안내, MCP 서버 지침, 호스트 규칙 파일이 모델 동작을 강제하거나 에이전트가 Volicord 도구를 선택한다고 보장한다는 주장.

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
- 호환되는 제품 파일 쓰기는 현재 적용 범위, 현재 적용 Change Unit 호환성, 사용자 소유 판단, 그리고 쓰기 담당 문서가 요구하는 쓰기 티켓의 지배를 받을 수 있습니다.

주장하면 안 되는 것:
- 제품 파일이 Volicord 상태라는 주장.
- 제품 파일이 Volicord 권한을 증명한다는 주장.
- 주변에 Volicord 메타데이터가 있다는 이유로 제품 파일이 Volicord 기록이 된다는 주장.

### `Volicord Runtime Home`

보안 표현에서는 `Volicord Runtime Home`을 런타임/저장소 담당 문서가 정의하는 운영 데이터 위치로 다룹니다.

런타임 위치 정의는 [런타임 경계](runtime-boundaries.md)가 담당합니다. 이 절은 그 위치에 대한 보안 비주장만 담당합니다.

주장할 수 있는 것:
- 저장소/런타임 담당 문서는 어떤 Volicord 운영 데이터가 여기에 속하고 어떻게 검증되는지 정의합니다.
- 관리 진단은 저장된 행 본문을 출력하지 않고 레지스트리, 프로젝트 상태, 아티팩트,
  User Channel, `guard`, 세션 감시 메타데이터 같은 Runtime Home 개인정보 저장 범위를
  범주와 개수로 요약할 수 있습니다.

주장하면 안 되는 것:
- `Volicord Runtime Home`이 `Product Repository`라는 주장.
- `Volicord Runtime Home`이 자동으로 보안 경계라는 주장.
- 데이터를 `Volicord Runtime Home` 아래에 둔다는 사실이 보안 권한이나 격리를 증명한다는 주장.
- Runtime Home 기록이 행위자 귀속, 쓰기 방지, 변조 불가능 감사, 전체 파일시스템 감시,
  OS 강제, 정확성, 테스트 충분성, 검토 완료, 최종 수락, 잔여 위험 수락을 증명한다는
  주장.

### Agent Connection, User Channel, 작업 범주

연결 식별자, 사용자 채널 출처, 작업 범주는 주장할 수 있는 범위를 제한합니다.

주장할 수 있는 것:
- `connection_internal_id`, 연결 의도, `connection.mode`, Connection Projects, `operation_category`, `actor_source`는 현재 호출이 문서화된 연결 맥락에 맞은 뒤 런타임, Core, 메서드, 보안 담당 문서에 따라 사용할 수 있습니다.
- `actor_source`는 Core와 메서드 담당 문서가 현재 권한 해결 동작에 대해 그 값을 받아들일 때만 지속되는 출처 정보를 제공할 수 있습니다.
- 판단과 Evidence 관찰을 포함한 권한 효력이 있는 사용자 행동 resolution에는 `User Channel`을 통한 `actor_source=local_user`가 필요합니다.
- 원문 User Channel bearer token이나 credential을 포함한 URL은 `content`,
  `structuredContent`, 호환·진단 text, 정확한 replay, operation-result byte를 포함한 Agent
  대상·모델 맥락 또는 공개 출력 projection에 들어가면 안 됩니다. Local-web 전달에는
  관리되는 generic이 아닌 stdio 호스트, 준비된 loopback listener,
  `params.capabilities.experimental["io.volicord/user-channel"].model_invisible_user_surface`의
  정확한 boolean `true`, 변경 불가능한 결과가 `outcome=passed`이고 만료되지 않은 현재의
  정확히 일치하는 영속 호스트 역량 상태가 모두 필요합니다. 검증 구간은
  `observed_at <= created_at < expires_at <= observed_at + 86,400 seconds`를 만족해야
  합니다. 24시간은 기본 수명, 신원 증명, attestation 기간이 아니라 최대 최신성 구간일
  뿐입니다.
  `evidence_artifact_sha256` 예상값에는 [호스트 릴리스 증거](host-release-evidence.md)가
  정의한 외부 `volicord-host-release-manifest-v3`를 신뢰해 획득하는 운영 경로가
  필요합니다. 그
  manifest는 같은 역량, 호스트·클라이언트, 어댑터, 빌드, source, target, 실행 파일
  다이제스트에 결속되어야 합니다. Manifest가 없거나, 알 수 없거나, 잘못됐거나, 검증되지
  않았거나, 일치하지 않으면 닫힌 상태로 실패하며 영속 행이나 빌드 메타데이터가 예상값을
  자기 선언할 수 없습니다. 현재 어댑터에는 신뢰된 manifest 획득 경로가 없고 릴리스
  아티팩트 자체도 런타임 신뢰 입력이 아니므로 운영
  local-web 자격은 사용할 수 없고 CLI inbox를 사용합니다.
  V1 검증 `metadata_json`은 엄격한 정규 `{}`만 허용합니다. 임의 메타데이터가 다른 신뢰
  입력이 되거나 token, prompt, transcript, 원문 호스트 아티팩트, 비공개 운영자 데이터를
  담을 수 없습니다. 유일한 host-only 예외로 URL을
  `outputSchema`와 모델 맥락 밖의 namespaced 최상위 tool-result `_meta` handoff에만 둡니다.
  입력이 없거나, 통과하지 않았거나, 만료·취소·손상·불일치 상태이면 token을 발급하지 않고
  CLI inbox 복구를 남깁니다.
- Workflow Agent Connection은 현재 evidence-capture intent를 만들 수 있습니다. 등록된
  local source만 이를 fulfillment할 수 있고, receipt를 producer와 observation으로
  finalization할 수 있는 메서드는 `record_run`뿐입니다. MCP receipt-fulfillment
  도구는 없습니다.

주장하면 안 되는 것:
- `connection_id` 자체가 권한 토큰이라는 주장.
- 복사된 내부 연결 식별자가 역량이나 사용자 권한을 증명한다는 주장.
- `connection.mode=workflow`가 OS 권한이나 포괄적 권한이라는 주장.
- `personal`, `shared`, `global` 연결 의도가 OS 권한, 호스트 신뢰, 포괄적 권한이라는 주장.
- `operation_category`가 OS 권한, 호스트 신뢰, 포괄적 권한이라는 주장.
- 텍스트에서 복사한 `actor_source`가 호출자 권한 토큰이라는 주장.
- 환경으로 제어되는 라벨, 공개 요청 필드, 임의 호출자 텍스트가 신뢰된 권한, 감사 사실, 검증 근거 입력이라는 주장.
- 스스로 선언한 capability, `clientInfo` 이름·버전, 환경 marker, 프로세스 인자, 연결의
  `complete` 결과, 복사한 증거 다이제스트, fixture 결과가 credential 전달 자격을 만든다는
  주장. 이 값은 집중 담당 문서가 요구하는 곳에서 일치 대조 입력일 뿐입니다.
- 실행 파일에 내장한 릴리스 증거 다이제스트가 유효한 신뢰 원천이라는 주장. 실제 호스트
  증거는 최종 실행 파일이 생긴 뒤에 생성되며, 다이제스트를 내장하려고 다시 빌드하면 실행
  파일 다이제스트가 바뀌어 재귀 결속이 생깁니다.
- 호스트 역량 검증이 암호학적 host attestation, 현재 사용자 신원, 호스트 격리, 이후
  호스트 실행에서 `_meta`를 모델 맥락 밖에 두었다는 증명이라는 주장. 한 호스트, 버전,
  target, 어댑터 프로필, 연결, 지문, 빌드, 유효 기간의 결과를 다른 tuple로 일반화하면
  안 됩니다.
- 등록된 guard event, session-watcher observation, Volicord command runner가
  cryptographic host signature, local-principal attestation, 위조 방지 경계,
  actor-identity 증명이라는 주장. 정확한 digest와 완전성을 검사해도 이 source는
  협력적 local integration입니다.

관리 호스트 세션 상관관계에는 [호스트 릴리스 증거](host-release-evidence.md)가 정의하는
domain 분리 불투명 `managed_host_session_id`만 사용합니다. 검토된 Codex `0.144.4`에서
관리 기술 정보는 시작 출처일 뿐입니다. 정확한 `clientInfo`, 엄격한 호출별
`_meta.threadId`, `_meta["x-codex-turn-metadata"]`가 root 세션과 변경 불가능한
프로세스 로컬 thread 다이제스트를 결속합니다. 대기 상태는 관리 영속 효과, Core 효과,
token, local-web 효과를 만들지 않습니다. `CODEX_THREAD_ID`, PID, cwd, 프로세스 조상
관계, 시각, 훅 이벤트와의 근접성은 이 결속을 확립하거나 복구할 수 없습니다. 결속이 관리
세션 감시 기준선을 구체화할 때는 검증된 1바이트 이상 256바이트 이하의 정확한 initialize
`client_name`과 `client_version` 문자열만 릴리스 기록용 최상위 메타데이터로 보존할 수
있습니다. 이는 협력적 일치 대조 좌표이지 신원 증명이 아니며 호스트, probe, 설정,
프로토콜, 상수, 다른 세션에서 추론할 수 없습니다. 원본 native session identifier와 event,
tool-call, capture, turn, invocation identifier 및 원본 initialize 또는
프로토콜·세션·thread·turn payload는 영속 저장, 로그, 진단, 증거 첨부를 하지 않습니다.
`mhs_` namespace는 예약되며 호스트 하나와 등록 연결
하나에 변경 불가능하게 결속됩니다. 잘못되거나 충돌하는 marker는 영속 상태 없이
실패합니다. 매핑 값은 상관관계 메타데이터일 뿐 사용자 신원, host attestation, 권한이
아니며 결속이 없거나 일치하지 않으면 Strong Evidence를 만들 수 없습니다.
이 정확한 상관관계가 `local_web_user_channel`을 승격하지는 않습니다. 이 기능에는 별도
credential 전달 조건과 현재 model-separation 및 advertised-and-exercised probe가 모두
필요합니다. 검토된 Codex `0.144.4` 좌표는 지원을 증명하지도
`unsupported_by_host`를 만들지도 않습니다. 현재 capability의 명시적 부재만 그 상태를
만듭니다.

<a id="historical-operation-result-access"></a>
### 과거 동작 결과 접근

`OperationResultRef`는 조회할 수 있는 변경 불가능한 과거 Core 변경 응답을
가리키는 조회 식별자이며, 소지만으로 권한을 주는 베어러 자격 증명이 아닙니다.
참조나 페이지 커서를 가지고 있거나 복사했다는 사실만으로 조회가 허용되거나
변경이 재실행되거나 현재 권한이 생기지 않습니다.

`volicord.get_operation_result`의 모든 페이지는 현재 활성화된 Agent Connection,
선택 프로젝트에 대한 현재 Connection Projects 멤버십, 원래
`operation_category=agent_workflow` 호출에 저장된 행위자와 정확히 일치하는
검증된 `actor_source`를 요구합니다. 각 페이지마다 이 조건을 다시 확인합니다.
비활성 연결, 제거된 프로젝트 멤버십, 다른 프로젝트, 다른 행위자에게 과거 응답
바이트를 노출하면 안 됩니다.

`operation_category=user_only` 결과는 이 Agent Connection 조회 경로에서
제외합니다. 특히 정확한 `volicord.resolve_user_action` 응답, 사용자의 자유 형식
`note`, Evidence 관찰 `summary`를 `volicord.get_operation_result`로 반환하면 안
됩니다. 호스트가 중개한 사용자 행동 흐름은 에이전트 소유 요청에 대해 MCP 전송
담당 문서가 정의한 에이전트용 상태 보기만 노출할 수 있습니다. Compact, full, resume,
page 단위 과거 형태는 대기 요청에 대해 요청 ID, 과거 `pending` 상태,
`next_actor=user`만 담습니다. 완전한 요청, inbox 폼, 비공개 `note`, Evidence 관찰
`summary`, User Channel credential, user-only 작업 ref, 정확한 응답 본문을 생략합니다.
이 안전 형태에 맞지 않는 보정 전 저장 결과는 일부를 반환하지 않고 unavailable입니다.

조회한 바이트는 과거 결과를 설명합니다. `AuthorityReceipt`, 현재 상태, 증거,
쓰기 티켓이 아니며 과거 상태가 여전히 현재라는 증명도 아닙니다. 현재 권한을
주장하기 전에는 `volicord.status`를 별도로 읽어야 합니다.

### 호스트 신뢰와 안내

호스트 신뢰와 승인 결정은 외부 호스트와 사용자가 소유합니다. Volicord는 지원되는 설정을 설치하고 추가 사용자 동작이 필요한지 보고할 수 있지만, 호스트의 신뢰 결정을 통제하지 않습니다.

주장할 수 있는 것:
- 관리 CLI가 필요한 확인을 관찰할 수 있으면 `managed host configuration state` 검증은 `complete`를 `action_required`, `failed`와 구분할 수 있습니다.
- `action_required`는 설치 프로필 복구, 명령 링크 복구, 호스트 신뢰, 승인, 재시작, 다시 로드처럼 사용자가 통제하는 동작이 남은 관찰 가능한 차단 사유일 때 그 동작을 이름 붙일 수 있습니다.
- 훅 경로 안전성 진단은 생성된 호스트 훅 명령이 현재 작업 디렉터리와 무관하고 하위 디렉터리에서도 안전하며, 예상한 Volicord 관리 래퍼를 가리키는지 보고할 수 있습니다.
- MCP 서버 지침과 선택적 저장소 안내는 에이전트가 프로젝트와 도구를 선택하는 방법을 설명할 수 있습니다.

주장하면 안 되는 것:
- Codex 또는 Claude Code 설정 설치가 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 재시작, 다시 로드, 그 밖의 호스트 통제 동작을 우회한다는 주장.
- 설정은 설치되었지만 호스트가 여전히 사용자 통제 신뢰나 승인을 요구하는 경우 `action_required`가 실패한 설치라는 주장.
- 현재 작업 디렉터리와 무관한 훅 경로, 래퍼 검증, `hook_path_safety=ok`가 OS 샌드박싱, 전역 파일시스템 가로채기, 포괄적 명령 차단, 네트워크 차단, 비밀값 차단, 또는 구현된 호스트 훅 밖에서 쓰기가 일어나지 않는다는 증거라는 주장.
- 에이전트 지침, `AGENTS.md` 블록, `CLAUDE.md`, `.claude/rules/` 파일, MCP 서버 지침이 접근 제어, 보안 강제, 사용자 판단, 쓰기 티켓, 또는 모델이 이를 따랐다는 증명이라는 주장.
- 최종 출력 어댑터 설정을 쓰거나 검증한 사실이 호스트가 어댑터를 로드하고 이벤트를 전달해 고정 UI 출력을 표시했다는 증명이라는 주장.

<a id="generated-displays-and-text"></a>
### 생성된 표시와 텍스트

생성된 표시, 렌더링된 템플릿, 대화 텍스트, 커넥터 설명, 에이전트 기억은 독자가 원천 기록을 이해하도록 도울 수 있습니다.

주장하면 안 되는 것:
- 렌더링된 표시, `Projection`, 상태 카드, 템플릿 출력, 대화 메시지, 커넥터 설명, 에이전트 기억이 새로운 권한 원천이라는 주장.
- 표시된 `ArtifactRef`, `UserActionRequest`, `UserActionResolution`, 쓰기 티켓, `connection_id` 텍스트가 그 식별자가 가리키는 권한을 만든다는 주장.
- 최종 출력 `AuthorityReceipt` 상태 보기가 두 번째 권한 기록, Core 변경, 호스트 관찰, 또는 모델이 작성한 최종 산문이 현재 권한을 사용했다는 증명이라는 주장.

<a id="detective-observation-confidence"></a>
### Detective 관찰 confidence와 종료

Detective가 하드 deny할 수 있는 경우는 호스트가 구조화한 결정적 직접 Product Repository
쓰기이고 구체적인 정규화 경로가 confirmed이며, 현재 Task, 정확히 하나의 일치하는 활성
쓰기 티켓, 티켓 범위, 필요한 sensitive 승인이 누락된 때뿐입니다. 셸 텍스트, 넓은 명령
이름, 누락된 감시기 데이터, 모호한 대상, heuristic 추론은 uncertain입니다. 경고는 만들 수
있지만 하드 보안 주장이나 정상 작업 deny의 근거가 될 수 없습니다.

PostTool 관찰은 구조화된 변경 경로, 감시기 before/after 비교, 크기가 제한된 안전한 Git
diff, heuristic 신호 순으로 사용합니다. 신뢰할 수 있는 before/after Evidence가 있는 알려진
경로 변경은 `confirmed`, 감시기를 사용할 수 없거나 heuristic뿐인 신호는 `suspected`입니다.
confirmed 미기록 또는 범위 밖 변경만 해당 닫기 차단 사유를 만들 수 있습니다. suspected
변경은 이후 관찰이 confirmed로 올리거나 변경 없음으로 해소할 때까지 경고입니다. 어느
분류도 행위자 신원, 악의적 의도, 완전한 파일시스템 가로채기, 방지를 증명하지 않습니다.

Stop은 호스트 세션 종료를 항상 허용합니다. 대기 사용자 행동, confirmed 미기록 변경,
누락된 Evidence나 다른 닫기 요건, 권한 상태 갱신 실패는
`completion_claim_allowed=false`로 만들 뿐 Stop deny나 강제 retry를 정당화하지 않습니다.
영속 종료 receipt는 내용이 없습니다. 모델 산문, 프롬프트, 명령, 경로, 파일 내용, 사용자
답변, 원시 이벤트, 오류 본문을 저장하지 않습니다.

작업 흐름 지표에도 같은 개인정보 경계가 적용됩니다. 크기가 제한된 집계 횟수, 시간,
byte 크기, 범주형 결과만 저장할 수 있으며 프롬프트, 답변, 명령, 경로, 파일 내용, 모델
출력, 원시 호스트 이벤트는 저장하지 않습니다. 지표와 정확한 또는 자체 보고 호스트·
클라이언트 버전은 진단이나 Evidence 좌표일 뿐 권한을 부여하거나 신원을 확정하거나 버전
동등성으로 런타임 capability를 gate하거나 방지·닫기 준비 상태를 증명하지 않습니다.

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
- 호스트 지침, 저장소 안내, MCP 서버 지침이 모델 또는 도구 동작을 강제한다는 것.

### 저장소와 아티팩트 권한

Volicord는 아래를 보장하지 않습니다.

- 변조 방지 저장소.
- 권한 번들, `manifest.json`, SHA-256 체크섬이 `Volicord Runtime Home`이 내보내기 전에
  한 번도 수정되지 않았음을 증명한다는 것.
- MCP나 임의 Agent Connection payload를 통한 native evidence-receipt fulfillment.
  지원 source 경로는 local registered fulfillment 뒤의 Core finalization입니다.
- 표시된 식별자만으로 생기는 아티팩트 권한.
- 복사된 아티팩트, 실행 기록, 증거, 판단 텍스트에서 생기는 검증이나 수락.
- 권한 번들이 정확성, 테스트 충분성, 검토 완료, 배포 성공, 최종 수락, 잔여 위험
  수락을 증명한다는 것.

### 닫기 상태, QA, 배포, 검토

Volicord는 아래를 보장하지 않습니다.

- 닫기 상태만으로 판단한 제품 정확성.
- 닫기 상태만으로 판단한 테스트 충분성.
- QA 완료.
- 배포 성공.
- 사람 검토 완료 또는 대체.
- 무위험 완료.
- 최종 수락이나 잔여 위험 수락이 빠진 필수 증거를 보충한다는 것.

### Local HTTP transport

Volicord Local HTTP transport는 아래를 보장하지 않습니다.

- 공개 네트워크 API.
- SaaS 엔드포인트.
- 다중 사용자 서버.
- 보안 경계.
- 인증 서비스 또는 인가 서비스.
- 공개 호스트 인터페이스 리스너 또는 원격 서비스.
- 전체 MCP Streamable HTTP 호환성.
- 로컬 동의 URL, 페이지, 기록된 답변이 정확성, 테스트 충분성, 배포 성공, 검토 완료,
  보안 강제, 닫기 준비 상태를 증명한다는 것.

베어러 토큰과 Origin 검사는 로컬 HTTP 프로세스에 묶인 전송 검사입니다. 이 검사가
엔드포인트를 공개 노출에 적합하게 만들지는 않습니다. 엔드포인트는 호스트 루프백 또는
의도한 Docker 호스트 루프백 노출 경계에 두어야 합니다.
모든 local-web `POST /consent`에는 필수 동일 출처 `Origin` 점검이 적용되며, `GET
/consent`에는 `Origin`이 필요하지 않습니다. 정확한 헤더 개수, 검증, HTTP 거절,
우선순위는 [MCP 전송](mcp-transport.md#local-web-consent-fallback)이 담당합니다. 이는
브라우저 교차 출처 제출에 대한 심층 방어이며 사용자 인증이나 사용자 의도 증명이
아닙니다.

일회성 local-web consent token은 일시적인 bearer secret으로 남습니다. 영속 상태는 원문
token이 아니라 domain-separated hash와 digest-only submission/replay identity를
저장합니다. Core는 제출 identity를 정확한 프로젝트, 요청, 예상 Agent Connection,
폐쇄형 완료 맥락에 결속하고 replay 또는 커밋 전에 다시 검증합니다. 이 점검은 서로 다른
로컬 credential이나 맥락이 해당 replay를 여는 것을 막지만 사람 신원을 증명하거나
listener를 인증·인가 서비스로 바꾸지는 않습니다.
원문 token은 협상된 모델 비가시적 host 표면에 대해서만 발급하고 Agent 대상·모델 맥락
또는 공개 출력에 들어가면 안 됩니다. 폐기된 전달 계약으로 만든 token에는 필수
delivery-surface marker가 없으므로 수정된 코드에서 영구적으로 사용할 수 없습니다. GET과
POST는 표시나 효과 없이 닫힌 상태로 실패합니다. 그 행은 upgrade하지 않으며 대기 행동은
CLI 같은 다른 유효한 User Channel로 계속 해결할 수 있습니다.

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
- [Core 모델](core-model.md): 사용자 소유 판단, 쓰기 티켓, 수락, 잔여 위험, 비대체 규칙.
- [API 판단 스키마](api/schema-judgment.md): `SensitiveActionScope`와 사용자 소유 판단 스키마 형태.
- [저장 효과](storage-effects.md), [저장소 기록](storage-records.md), [아티팩트 저장소](storage-artifacts.md): 저장 효과, 기록 배치, 아티팩트 권한 세부사항.
