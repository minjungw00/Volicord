# 관리 CLI 참조

이 문서는 로컬 `harness` 관리/부트스트랩 CLI 계약을 담당합니다. 이 CLI는 `Harness Runtime Home`을 초기화하고, 로컬 프로젝트와 로컬 접점을 등록합니다. 이 명령들은 공개 하네스 API 메서드가 아닙니다.

이 문서는 공개 API 메서드 동작, API 스키마, 접근 등급 값 의미, 저장소 기록 배치, 보안 보장, Core 권한 의미, MCP stdio 전송 동작을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `harness` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- `harness` 관리 명령의 Runtime Home 경로 선택
- 관리용 프로젝트와 접점 등록 기본값
- 로컬 MCP 설정 오케스트레이션, 설정 옵션 기본값, 충돌 처리, dry-run 동작, 출력 형식, 호스트 중립 설정 생성
- `baseline-workflow` 로컬 등록 프로필 확장
- 관리 명령과 공개 하네스 API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 하네스 API 메서드: [API 메서드](api/methods.md)
- `access_class` 값의 API 값 의미: [API 값 집합](api/schema-value-sets.md#access-class-values)
- 접점 등록 의미, 확인된 접점 맥락, 행위자 출처, 역량 선언 경계: [에이전트 통합](agent-integration.md)
- 런타임 데이터 경계 의미: [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio 프레이밍, 와이어 동작, 응답 래핑, 사전 점검 내부 동작, 종료: [MCP 전송](mcp-transport.md)
- 외부 MCP 호스트 설치 스키마나 호스트별 설정 위치
- 저장소 기록 배치, Core 권한 의미, 보안 보장 의미

## 명령 모델

`harness`는 로컬 관리/부트스트랩 실행 파일입니다. 장기 실행 서버가 아니며 공개 하네스 API를 직접 노출하지 않습니다.

지원되는 기준 명령은 아래와 같습니다.

```text
harness --help
harness --version
harness setup local-mcp [OPTIONS]
harness init [--runtime-home-id ID]
harness project register --project-id ID --repo-root PATH [--status active]
harness project list
harness surface register --project-id ID --surface-id ID [--surface-instance-id ID] [--kind KIND] [--name NAME] [--interaction-role agent|user_interaction] [--access-class ACCESS_CLASS ...] [--profile baseline-workflow] [--capability-profile JSON]
harness surface list --project-id ID
```

종료 코드와 스트림 동작:

- 성공한 명령은 성공 출력을 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- `harness --version`은 stdout에 `harness <version>`을 쓰며 Runtime Home 해석을 요구하지 않습니다.
- 사용법 오류는 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- 런타임, 환경, 저장소 오류는 진단을 stderr에 쓰고 종료 코드 `1`로 끝납니다.

지원하지 않는 것:

- CLI에는 `serve`, `server`, `connect` 명령이 없습니다.
- 관리 명령은 공개 하네스 API 메서드가 아니며 공개 메서드 목록에 추가되면 안 됩니다.

<a id="runtime-home-selection"></a>
## Runtime Home 선택

`harness` 관리 CLI는 아래 Runtime Home 경로 해석 규칙을 사용합니다. `harness-mcp` 프로세스 환경과 현재 MCP Runtime Home 경로 해석은 [MCP 전송](mcp-transport.md#process-environment)이 담당합니다.

해석 순서:

1. `HARNESS_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 절대 경로 `HARNESS_HOME`은 제공된 그대로 사용합니다.
3. 상대 경로 `HARNESS_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
4. `HARNESS_HOME`이 없으면 `HOME`, `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 결합 순서로 첫 번째 비어 있지 않은 홈 소스를 사용합니다.
5. 선택한 사용자 홈에 `.harness`를 붙입니다.
6. 선택한 홈이 상대 경로이면 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
7. `harness init` 전에 정규화를 요구하지 않습니다.

`harness init`은 선택된 Runtime Home registry를 만들거나 검증할 수 있습니다. 다른 관리 명령은 선택된 Runtime Home에 요청 작업에 필요한 기록이 있어야 합니다.

`harness setup local-mcp`는 명령별 `--runtime-home` 재정의를 추가합니다. 전체 선택 순서는 [로컬 MCP 설정 오케스트레이션](#local-mcp-setup-orchestration)에서 정의합니다.

<a id="local-mcp-setup-orchestration"></a>
## 로컬 MCP 설정 오케스트레이션

`harness setup local-mcp [OPTIONS]`는 일반적인 `Product Repository` 루트 로컬 MCP 설정 경로를 위한 비대화식 로컬 관리 오케스트레이션 명령입니다. 낮은 수준의 `harness init`, `harness project register`, `harness surface register` 명령은 그대로 유지됩니다.

지원 옵션:

```text
--runtime-home PATH
--repo-root PATH
--project-id ID
--with-user-interaction
--mcp-command PATH
--config-dir PATH
--output text|json
--dry-run
--replace-conflicting-surfaces
--overwrite-config
```

참/거짓 옵션은 존재 여부로 켜지는 플래그입니다. `--dry-run=true` 같은 형식은 사용법 오류입니다. `--interactive`는 이 계약에 포함되지 않습니다.

기본값:

- `--repo-root`의 기본값은 프로세스 현재 작업 디렉터리입니다.
- `--output`의 기본값은 `text`입니다.
- `--with-user-interaction`이 있을 때만 사용자 상호작용 설정을 수행합니다.
- 에이전트 MCP 접점 대상은 `surface_id=agent_mcp`, `surface_instance_id=agent_mcp_local`, `surface_kind=mcp`, `interaction_role=agent`이고 `baseline-workflow` 접근 집합을 사용합니다.
- 선택적 사용자 상호작용 MCP 접점 대상은 `surface_id=user_ui`, `surface_instance_id=user_ui_local`, `surface_kind=mcp`, `interaction_role=user_interaction`이고 `read_status`와 `core_mutation`을 사용합니다.

### Runtime Home 설정 선택

`harness setup local-mcp`의 Runtime Home 선택 순서는 아래와 같습니다.

1. `--runtime-home`
2. `HARNESS_HOME`
3. [Runtime Home 선택](#runtime-home-selection)에서 정의한 공유 사용자 홈 대체 경로

선택된 경로는 공유 Runtime Home 해석 규칙을 따릅니다.

- 명시 값이 비어 있으면 무효입니다.
- 상대 경로는 프로세스 현재 작업 디렉터리를 기준으로 해석합니다.
- 최종 경로는 절대 경로입니다.
- 설정 전에 경로가 존재할 필요는 없습니다.

설정 명령은 Runtime Home이 아직 초기화되지 않았다면 초기화합니다. 이미 유효한 Runtime Home 등록은 보존합니다.

### 프로젝트 선택

선택된 `repo_root`는 이미 존재하고 접근 가능한 디렉터리여야 하며, 비교 전에 정규화해야 합니다.

`--project-id`가 있을 때:

- 그 프로젝트 ID를 사용합니다.
- 해당 ID가 등록되어 있지 않으면 선택된 저장소에 대해 생성합니다.
- 해당 ID가 같은 정규화 저장소를 가리키고 `active`이면 재사용합니다.
- 해당 ID가 다른 저장소를 가리키면 등록을 바꾸지 않고 실패합니다.
- 해당 프로젝트가 `inactive`이면 조용히 활성화하지 않고 실패합니다.

`--project-id`가 없을 때:

1. 정규화된 `repo_root`가 선택된 저장소와 정확히 일치하는 프로젝트를 찾습니다.
2. 일치 항목이 정확히 하나이면 그 프로젝트를 재사용합니다.
3. 일치 항목이 둘 이상이면 모호함으로 실패합니다.
4. 일치 항목이 없으면 최종 저장소 디렉터리 이름에서 프로젝트 ID를 파생합니다.
5. 파일시스템 루트 같은 경우를 포함해 유효한 UTF-8 디렉터리 이름을 얻을 수 없으면 `--project-id`를 요구합니다.
6. 파생한 ID가 이미 다른 저장소에 등록되어 있으면 실패합니다.

기존 프로젝트 ID를 다른 저장소로 강제 재바인딩하는 설정 옵션은 없습니다.

### 접점 호환성과 충돌

각 대상 접점 인스턴스에 대해 설정 명령은 아래처럼 동작합니다.

- 없으면 생성합니다.
- 기존 등록이 호환되면 쓰지 않고 재사용합니다.
- 기존 등록이 다르면 충돌을 보고합니다.
- `--replace-conflicting-surfaces`가 있을 때만 교체합니다.

호환성은 raw JSON 바이트 동일성이 아니라 정규화된 의미를 비교합니다.

- 정확한 대상 프로젝트, 접점, 인스턴스 ID
- `surface_kind`
- `interaction_role`
- 정규화된 등록 접근 등급 집합
- MCP 시작 검증이 요구하는 유효한 JSON 객체 메타데이터

관련 없는 표시 텍스트 차이나 기존 비권위 설정 메타데이터 차이는 권한 변경을 일으키면 안 됩니다. 기존의 읽기 전용 에이전트 접점은 `--replace-conflicting-surfaces` 없이 `baseline-workflow`로 승격되면 안 됩니다.

`--replace-conflicting-surfaces`는 고정된 대상 접점 인스턴스에만 적용됩니다. 프로젝트 재바인딩이나 공개 하네스 권한 규칙 변경을 허용하지 않습니다.

### 멱등성과 부분 실패

정확히 반복한 설정은 아래 성질을 갖습니다.

- Runtime Home, 프로젝트, 접점 기록을 중복으로 만들지 않습니다.
- 호환되는 기록은 `reused`로 보고합니다.
- 재사용한 프로젝트나 접점 메타데이터를 다시 쓰지 않습니다.
- 기존 `Task`나 Core 워크플로 기록을 수정하지 않습니다.
- 프로젝트 `state_version`을 증가시키지 않습니다.
- 결정적인 호스트 설정을 생성합니다.

명령은 발견 가능한 모든 검증을 쓰기 전에 수행합니다.

등록은 둘 이상의 SQLite 데이터베이스에 걸쳐 있으므로 이 명령은 데이터베이스 간 롤백 보장을 주장하지 않습니다. 등록 뒤의 늦은 사전 점검이 실패하면 설정은 실패하고, 완료된 작업을 보고하며, 안전하게 다시 실행할 수 있어야 합니다.

설정이 새로 만든 기록은 아래와 동등한 비권위 진단 메타데이터를 사용할 수 있습니다.

```json
{
  "created_by": "harness_cli_setup",
  "setup_profile": "local_mcp_v1"
}
```

이 메타데이터는 일반 등록 메타데이터로 보존됩니다. Core 권한, 사용자 신원, 접점 신뢰, 접근 부여, 보안 속성 증명으로 해석하면 안 됩니다. 호환되어 재사용한 기록은 기존 메타데이터를 유지합니다.

### MCP 실행 파일과 사전 점검

MCP 실행 파일 탐색 우선순위는 아래와 같습니다.

1. `--mcp-command PATH`
2. 실행 중인 `harness` 실행 파일 옆의 `harness-mcp` 실행 파일
3. `PATH`에서 발견한 `harness-mcp`

생성된 호스트 설정에 쓰는 선택 실행 파일 경로는 절대 경로여야 합니다. 실행 파일 탐색과 기본 경로 검증은 등록 쓰기 전에 이루어져야 합니다.

설정 명령은 관리 CLI에서 MCP 어댑터 구현으로 가는 의존성을 추가하면 안 됩니다. 선택한 실행 파일을 별도 프로세스로 호출합니다.

등록을 적용한 뒤 설정 명령은 명시적 환경 바인딩으로 `harness-mcp --check`와 동등한 사전 점검을 호출합니다. 에이전트 사전 점검은 아래를 확인해야 합니다.

```text
configuration: valid
interaction_role: agent
baseline_workflow_access: full
```

`--with-user-interaction`이 있으면 별도 사전 점검을 실행해 아래를 확인합니다.

```text
configuration: valid
interaction_role: user_interaction
baseline_workflow_access: not_applicable
```

사전 점검 실패는 설정 실패이며 종료 코드 `1`로 끝납니다. 사전 점검이 실패한 뒤에는 호스트 설정 파일을 쓰면 안 됩니다. 정확한 MCP 사전 점검 동작은 [MCP 전송](mcp-transport.md#configuration-preflight)이 계속 담당합니다.

### 호스트 중립 설정

생성되는 MCP 설정은 호스트 중립입니다. 설정 명령은 알 수 없는 외부 호스트의 설정 파일을 추측하거나 편집하면 안 됩니다.

`--config-dir`가 없을 때 text 출력은 아래와 동등한 복사 가능한 호스트 중립 에이전트 설정을 포함해야 합니다.

```json
{
  "mcpServers": {
    "harness-agent": {
      "command": "/absolute/path/to/harness-mcp",
      "env": {
        "HARNESS_HOME": "/absolute/path/to/runtime-home",
        "HARNESS_PROJECT_ID": "project-id",
        "HARNESS_SURFACE_ID": "agent_mcp",
        "HARNESS_SURFACE_INSTANCE_ID": "agent_mcp_local"
      }
    }
  }
}
```

사용자 상호작용을 요청하면 `harness-user-interaction` 바인딩만 담은 별도 설정을 출력하거나 생성합니다. 에이전트와 사용자 상호작용 바인딩을 하나의 생성 파일로 합치면 안 되며, 일반 에이전트 호스트가 사용자 상호작용 바인딩을 받아야 한다고 암시하면 안 됩니다.

`--config-dir PATH`가 있으면 설정 명령은 아래 파일을 생성합니다.

```text
harness-agent.mcp.json
harness-user-interaction.mcp.json
```

`harness-user-interaction.mcp.json`은 `--with-user-interaction`이 있을 때만 생성합니다.

설정 디렉터리 규칙:

- 필요하면 대상 디렉터리를 만듭니다.
- 유효하고 결정적인 JSON을 씁니다.
- 대상 파일을 제자리에서 잘라내지 말고 같은 디렉터리의 교체용 파일을 사용합니다.
- 기본적으로 기존 파일을 덮어쓰지 않습니다.
- 기존 생성 파일을 교체하려면 `--overwrite-config`가 필요합니다.
- 모든 대상 파일 충돌을 등록 쓰기 전에 검증합니다.
- `--config-dir` 없는 `--overwrite-config`는 사용법 오류입니다.

이 명령은 지원 플랫폼 전체에서 구현이 제공할 수 있는 것보다 강한 atomic 동작을 주장하면 안 됩니다. 적어도 부분적으로 쓰인 대상 파일이 완료된 설정으로 노출되면 안 됩니다.

### Dry run

`--dry-run`은 아래를 수행합니다.

- 경로 해석
- 저장소 정규화
- 프로젝트 선택
- 접점 호환성과 충돌 분석
- MCP 실행 파일 탐색
- 설정 렌더링
- 설정 파일 충돌 분석

수행하지 않는 것:

- Runtime Home 생성
- SQLite 기록 쓰기
- 프로젝트 등록이나 갱신
- 접점 등록이나 갱신
- 설정 디렉터리나 파일 생성
- `harness-mcp --check` 호출
- `Task` 또는 애플리케이션 기록 생성
- `state_version` 변경

Dry run 출력은 사전 점검을 `passed`가 아니라 `planned`로 보고합니다.

### 설정 출력

Text 출력은 사람이 읽을 수 있어야 하며 최소한 아래를 포함해야 합니다.

```text
setup: complete|dry_run
runtime_home: ...
project_id: ...
repo_root: ...
agent_surface_id: agent_mcp
agent_surface_instance_id: agent_mcp_local
mcp_command: ...
preflight: passed|planned
```

각 리소스 작업은 `created`, `reused`, `updated`, `skipped` 중 하나로 식별해야 합니다.

`--output json`은 stdout에 유효한 JSON 문서 정확히 하나를 쓰며 사람용 설명을 stdout에 섞지 않습니다. 오류는 기존 CLI 종료 코드 모델에 따라 stderr 진단으로 남습니다. JSON 출력은 관리 CLI 출력이지 공개 하네스 API 응답 스키마가 아닙니다.

JSON 성공 출력은 아래 최상위 키를 갖습니다.

```text
status
runtime_home
project
surfaces
mcp_command
preflight
generated_configs
actions
warnings
```

필수 JSON 값:

- `status`: `complete` 또는 `dry_run`
- 프로젝트 작업: `created` 또는 `reused`
- 접점 작업: `created`, `reused`, 또는 `updated`
- 사전 점검 상태: `passed` 또는 `planned`
- 바인딩 이름: `agent` 또는 `user_interaction`

각 `generated_configs` 항목은 아래를 포함합니다.

- 바인딩 이름
- 출력 경로 또는 `null`
- 파싱된 JSON 설정 객체

### 사용법 오류와 종료 코드

사용법 오류에는 아래가 포함됩니다.

- 알 수 없는 옵션
- 반복할 수 없는 옵션의 중복
- 옵션 값 누락
- 지원하지 않는 출력 형식
- `--config-dir` 없는 `--overwrite-config`
- 빈 명시 경로 또는 ID
- 호환되지 않는 boolean 값 구문

종료 동작:

- 성공은 `0`으로 끝납니다.
- 런타임, 저장소, 사전 점검, 충돌 실패는 `1`로 끝납니다.
- 사용법 실패는 `2`로 끝납니다.

### 설정 관리 경계

`harness setup local-mcp`는 로컬 관리 오케스트레이션입니다. 공개 하네스 API 메서드가 아니며 공개 메서드 목록에 추가되면 안 됩니다.

설정 명령이 하지 않는 것:

- `Product Repository` 파일을 편집하지 않습니다.
- `Task`를 만들지 않습니다.
- Core 권한을 부여하지 않습니다.
- 에이전트와 사용자 상호작용 출처를 합치지 않습니다.
- 알 수 없는 외부 MCP 호스트를 설치하지 않습니다.
- 접근 등급 의미, 공개 요청/응답 스키마, 저장소 DDL, 보안 보장, 사용자 판단 권한 규칙을 바꾸지 않습니다.

## 프로젝트 등록

`harness project register --project-id ID --repo-root PATH [--status active]`는 로컬 `Product Repository`를 선택된 Runtime Home에 등록합니다.

규칙:

- `--project-id`는 필수입니다.
- `--repo-root`는 필수입니다.
- `--status`의 기본값은 `active`입니다.
- 기준 등록은 `status=active`를 받습니다.
- `--repo-root`는 프로젝트 등록에 쓰는 로컬 저장소 루트를 식별합니다.

`harness project list`는 선택된 Runtime Home의 등록된 프로젝트를 나열합니다.

`Product Repository`와 `Harness Runtime Home`의 구분을 포함한 런타임 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당합니다.

## 접점 등록

`harness surface register`는 등록된 프로젝트에 로컬 접점 인스턴스 하나를 기록합니다.

기본값:

- `surface_kind` 기본값은 `cli`입니다.
- `interaction_role` 기본값은 `agent`입니다.
- 기본 접근은 `read_status`뿐입니다.
- 생성되는 Runtime Home ID와 생성되는 `surface_instance_id` 값은 구현이 생성하는 불투명 값입니다.

등록 프로필:

- `--profile baseline-workflow`는 명시적으로 선택해야 합니다.
- `baseline-workflow`는 `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, `run_recording`으로 확장됩니다.
- 명시 접근 등급과 프로필에서 파생된 접근 등급은 결정적이고 중복 제거된 합집합을 이룹니다.
- `baseline-workflow` 프로필은 `artifact_read`를 포함하지 않습니다.

`user_interaction` 제약:

- `user_interaction`에는 `core_mutation`이 필요합니다.
- `user_interaction`은 `read_status`와 `core_mutation`만 가질 수 있습니다.
- 따라서 `baseline-workflow`는 `user_interaction` 접점에 유효하지 않습니다.

MCP 등록 지침:

- MCP 프로세스 등록에는 명시적인 `--kind mcp`를 사용합니다.
- 등록된 접점 인스턴스를 `HARNESS_SURFACE_INSTANCE_ID`로 참조할 때는 명시적인 `--surface-instance-id`를 사용합니다.

접근 등급 값 이름과 의미는 [API 값 집합](api/schema-value-sets.md#access-class-values)이 담당합니다. 접점 등록 의미와 확인된 맥락 경계는 [에이전트 통합](agent-integration.md)이 담당합니다.

## 접점 목록

`harness surface list --project-id ID`는 선택된 Runtime Home에서 한 프로젝트의 등록된 접점을 나열합니다.

규칙:

- `--project-id`는 필수입니다.
- 목록 출력은 진단용 등록 정보입니다.
- 목록 출력은 권한을 부여하거나, 로컬 도달 가능성을 증명하거나, 담당 결과가 반환한 확인된 접점 맥락을 대신하지 않습니다.

## 관리 경계

관리 CLI는 로컬 리소스를 초기화하고 등록할 수 있습니다. 그 자체로 공개 하네스 API 메서드를 만들지 않으며 Core 권한, `Write Authorization`, 증거 충분성, 닫기 준비 상태, 사용자 소유 판단, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을 만들지 않습니다.

담당 문서 경로:

- 공개 메서드 목록과 메서드 경로: [API 메서드](api/methods.md).
- 공통 요청/응답 스키마: [API 코어 스키마](api/schema-core.md).
- 접근 등급 값: [API 값 집합](api/schema-value-sets.md#access-class-values).
- 접점과 행위자 맥락 의미: [에이전트 통합](agent-integration.md).
- 런타임 위치 경계: [런타임 경계](runtime-boundaries.md).
