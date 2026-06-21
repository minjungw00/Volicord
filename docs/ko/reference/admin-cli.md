# 관리 CLI 참조

이 문서는 로컬 `harness` 관리/부트스트랩 CLI 계약을 담당합니다. 이 CLI는 `Harness Runtime Home`을 초기화하고, 로컬 프로젝트와 로컬 접점을 등록합니다. 이 명령들은 공개 하네스 API 메서드가 아닙니다.

이 문서는 공개 API 메서드 동작, API 스키마, 접근 등급 값 의미, 저장소 기록 배치, 보안 보장, Core 권한 의미, MCP stdio 전송 동작을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `harness` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- `harness` 관리 명령의 Runtime Home 경로 선택
- 관리용 프로젝트와 접점 등록 기본값
- 로컬 MCP 설정 오케스트레이션, 선택적 대화형 설정 프런트엔드, 설정 옵션 기본값, 충돌 처리, dry-run 동작, 미리보기와 취소 보장, 저장소 준비와 마이그레이션 순서, 준비 뒤 재검증, 프로젝트 ID 검증, 생성 설정 경로 검증, 부분 실패 보고, 출력 형식, 호스트 중립 설정 생성
- `baseline-workflow` 로컬 등록 프로필 확장
- 관리 명령과 공개 하네스 API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 하네스 API 메서드: [API 메서드](api/methods.md)
- `access_class` 값의 API 값 의미: [API 값 집합](api/schema-value-sets.md#access-class-values)
- 접점 등록 의미, 확인된 접점 맥락, 행위자 출처, 역량 선언 경계: [에이전트 통합](agent-integration.md)
- 런타임 데이터 경계 의미: [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio 프레이밍, 와이어 동작, 응답 래핑, 사전 점검 내부 동작, 종료: [MCP 전송](mcp-transport.md)
- 외부 MCP 호스트 설치 스키마나 호스트별 설정 위치
- 저장소 기록 배치, SQLite DDL, 일반 저장소 마이그레이션 정의, Core 권한 의미, 보안 보장 의미

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

`harness setup local-mcp [OPTIONS]`는 일반적인 `Product Repository` 루트 로컬 MCP 설정 경로를 위한 로컬 관리 오케스트레이션 명령입니다. 이 명령은 비대화식 명령 경로와 선택적 대화형 프런트엔드를 지원합니다. 낮은 수준의 `harness init`, `harness project register`, `harness surface register` 명령은 그대로 유지됩니다.

지원 옵션:

```text
--interactive
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

참/거짓 옵션은 존재 여부로 켜지는 플래그입니다. `--dry-run=true` 같은 형식은 사용법 오류입니다.

기본값:

- `--output`의 기본값은 `text`입니다.
- `--interactive`는 있을 때만 켜집니다.
- `--with-user-interaction`이 있을 때만 사용자 상호작용 설정을 수행합니다.
- 에이전트 MCP 접점 대상은 `surface_id=agent_mcp`, `surface_instance_id=agent_mcp_local`, `surface_kind=mcp`, `interaction_role=agent`이고 `baseline-workflow` 접근 집합을 사용합니다.
- 선택적 사용자 상호작용 MCP 접점 대상은 `surface_id=user_ui`, `surface_instance_id=user_ui_local`, `surface_kind=mcp`, `interaction_role=user_interaction`이고 `read_status`와 `core_mutation`을 사용합니다.

선택 규칙:

- 비대화식 설정에는 `--repo-root`가 필수입니다. 현재 `Product Repository`를 선택하려면 명시 형식인 `--repo-root .`을 사용합니다.
- 대화형 설정에서 `--repo-root`가 없으면 `Product Repository`를 입력하라는 프롬프트를 표시합니다.
- 설정에서 명시한 `--runtime-home` 값은 절대 경로여야 합니다. `--runtime-home`을 생략하면 [Runtime Home 설정 선택](#runtime-home-setup-selection)에서 설명하는 `HARNESS_HOME` 또는 공유 사용자 홈 대체 경로를 계속 사용합니다.
- 선택된 `Harness Runtime Home`과 `Product Repository`는 [런타임 경계](runtime-boundaries.md#runtime-home-product-repository-separation)가 담당하는 경로 분리 계약을 만족해야 합니다.

### 대화형 설정 프런트엔드

`--interactive`는 같은 설정 명령을 위한 텍스트 전용 마법사를 시작합니다. 마법사는 설정 입력을 수집하거나 확인하고, 계획된 바인딩과 접근 등급을 보여 주며, 최종 확인 전에 파괴적 결정을 따로 묻고, 그 뒤 비대화식 실행이 쓰는 것과 같은 설정 계획 및 적용 경로를 호출합니다. 이는 선택 사항이며 유일한 지원 온보딩 경로가 되면 안 됩니다.

대화형 모드 규칙:

- `--interactive`는 텍스트 출력만 사용합니다.
- `--interactive --output json`은 사용법 오류입니다.
- `--interactive`는 `--dry-run`과 함께 쓸 수 있습니다.
- 명시한 설정 옵션은 마법사의 기본값이 됩니다.
- 설정 적용이나 dry-run 출력 전에 최종 계획을 항상 보여 줍니다.
- 최종 긍정 확인 전까지 마법사는 dry-run과 같은 읽기 전용 계획 경로를 사용합니다. 저장소를 초기화하거나 마이그레이션하지 않고, 사전 점검을 실행하지 않으며, 설정 디렉터리나 파일을 만들지 않고, 프로젝트나 접점을 등록하지 않으며, 애플리케이션 기록을 만들거나 프로젝트 `state_version`을 바꾸지 않습니다.
- 취소하면 `0`으로 끝나고 stdout에 `setup: cancelled`를 쓰며 영속 설정 변경을 수행하지 않습니다. 사용자가 충돌 접점 교체를 거절하거나, 설정 덮어쓰기를 거절하거나, 최종 계획을 거절하거나, 마법사를 취소하거나, 대화형 dry-run을 사용할 때도 같은 영속 변경 없음 보장이 적용됩니다. 이 보장은 화면 출력이나 일반 프로세스 로컬 프롬프트 상태가 변하지 않는다고 주장하지 않습니다.

터미널과 스트림 동작:

- 대화형 모드는 바이너리 경계에서 표준 터미널 감지로 확인한 사용 가능한 대화형 터미널 입력이 필요합니다.
- 대화형 입력을 사용할 수 없으면 명령은 stderr에 사용법 진단을 쓰고, 비대화식 플래그를 제안하고, `2`로 끝나며, 입력을 기다리지 않고 상태를 바꾸지 않습니다.
- 프롬프트, 접근 검토, 충돌 확인, 최종 확인은 stderr에 씁니다.
- 정상 최종 설정 출력은 기존 텍스트 렌더러를 통해 stdout에 씁니다.
- 대화형 프롬프트는 JSON 출력에 섞이면 안 되며, 마법사는 비밀값이나 관련 없는 원시 환경 값을 출력하면 안 됩니다.

마법사 프롬프트 순서:

1. Runtime Home
2. Product Repository
3. project ID
4. 에이전트 바인딩과 접근 검토
5. 사용자 상호작용 커넥터 선택
6. 설정 출력 위치
7. 필요한 경우 충돌 결정
8. 전체 계획 검토
9. 최종 확인

프롬프트 동작:

- Runtime Home 프롬프트는 설정 우선순위가 선택한 경로를 기본값으로 보여 줍니다. 빈 입력은 기본값을 받아들입니다. 입력한 설정 재정의 값은 절대 경로여야 하며, 프롬프트 중에는 경로를 만들지 않습니다.
- 저장소 프롬프트는 명시한 `--repo-root` 값을 기본값으로 보여 줍니다. 빈 입력은 `--repo-root`가 제공된 경우에만 그 기본값을 받아들입니다. `--repo-root`가 없으면 프롬프트는 입력을 요구하고 프로세스 현재 작업 디렉터리를 조용히 미리 선택하지 않습니다. `.` 입력은 현재 `Product Repository`를 명시적으로 선택하는 형식입니다. 입력한 경로는 검증하고 정규화하며, 접근할 수 없거나 디렉터리가 아니면 상태를 바꾸지 않고 다시 입력하게 합니다.
- 저장소 선택 뒤 프로젝트 프롬프트는 설정 플래너를 사용해 정확히 하나의 일치 프로젝트 ID가 있으면 그것을 제안하고, 없으면 유효할 때 최종 디렉터리 이름을 제안합니다. 유효한 제안이 없으면 명시 입력을 요구하고, 여러 일치 항목 중 하나를 고르지 않고 모호함을 드러내며, 프로젝트 ID와 저장소의 충돌을 분명히 보여 줍니다. 프로젝트 재바인딩은 계속 지원하지 않습니다.
- 에이전트 바인딩 검토는 `surface_id=agent_mcp`, `surface_instance_id=agent_mcp_local`, `interaction_role=agent`와 접근 등급 `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, `run_recording`을 보여 줍니다. 이 목록은 등록 입력이지 사용자 신원, 신뢰, Core 권한이 아닙니다.
- 사용자 상호작용 커넥터 프롬프트의 기본값은 no입니다. 선택하면 `surface_id=user_ui`, `surface_instance_id=user_ui_local`, `interaction_role=user_interaction`과 `read_status`, `core_mutation`을 가진 별도 바인딩을 보여 줍니다. 프롬프트는 이것이 별도 커넥터 바인딩이며 에이전트 역할의 확장이 아니고, 실제 사용자 대상 UI나 커넥터가 사용자 동작을 제출할 때만 필요하며, `actor_kind=user`만으로는 사용자 권한이 성립하지 않고, 그 설정은 에이전트 설정과 분리되어 남는다고 설명합니다.
- 설정 프롬프트는 `--config-dir`가 기본값을 주지 않는 한 stdout 전용을 기본값으로 씁니다. 설정 디렉터리를 받을 수 있습니다. 제3자 호스트 설정 경로를 묻거나 추론하지 않습니다.

충돌과 최종 확인 동작:

- 접점 교체 확인은 설정 계획이 만든 구조화 충돌을 사용합니다. 호환되지 않는 각 대상 접점마다 마법사는 현재 역할, 종류, 정규화된 접근 등급과 원하는 역할, 종류, 정규화된 접근 등급을 보여 준 뒤, 정확히 그 대상 접점을 교체할지 따로 묻습니다. 기본값은 no이며, 명시한 파괴적 플래그가 있으면 제안 답변의 기본값으로 사용할 수 있습니다.
- 기존 생성 설정 파일은 정확한 경로로 보여 주고, 덮어쓸지 따로 확인해야 합니다. 기본값은 no이며, `--overwrite-config`가 있으면 제안 답변의 기본값으로 사용할 수 있습니다.
- 일반 최종 확인만으로는 파괴적인 접점 교체나 설정 덮어쓰기를 승인한 것으로 충분하지 않습니다.
- 필요한 파괴적 동작을 거절하면 설정은 저장소 준비, 등록, 사전 점검, 설정 파일 생성 전에 취소됩니다.
- 최종 계획은 Runtime Home, 저장소, 프로젝트 ID와 작업, 각 접점과 작업, MCP 실행 파일, 사전 점검 바인딩, 설정 대상, dry-run 여부, 파괴적 갱신을 보여 줍니다. 최종 확인의 기본값은 no입니다.

`--interactive --dry-run`에서는 마법사가 같은 입력을 수집하고 확인하며 계획을 보여 주고, 영속 설정 변경이나 마이그레이션을 수행하지 않으며, 사전 점검을 실행하지 않고, 최종 확인 뒤 정상 dry-run 출력을 냅니다.

<a id="runtime-home-setup-selection"></a>
### Runtime Home 설정 선택

`harness setup local-mcp`의 Runtime Home 선택 순서는 아래와 같습니다.

1. 절대 경로 `--runtime-home`
2. `HARNESS_HOME`
3. [Runtime Home 선택](#runtime-home-selection)에서 정의한 공유 사용자 홈 대체 경로

명시한 `--runtime-home` 값이 비어 있거나 상대 경로이면 무효입니다. `--runtime-home`이 없으면 `HARNESS_HOME`과 공유 사용자 홈 대체 경로는 공유 Runtime Home 해석 규칙을 따릅니다.

최종 선택 경로는 절대 경로이며 설정 전에 존재할 필요는 없습니다.

설정 명령은 Runtime Home이 아직 초기화되지 않았다면 초기화합니다. 이미 유효한 Runtime Home 등록은 보존합니다.

### 프로젝트 선택

비대화식 설정에는 `--repo-root`가 필수이며, 명령은 프로세스 현재 작업 디렉터리를 추론하지 않습니다. 대화형 설정에서 `--repo-root`가 없으면 `Product Repository`를 입력하라는 프롬프트를 표시합니다. `--repo-root .`은 유효하며 현재 `Product Repository`를 명시적으로 선택합니다.

선택된 `repo_root`는 이미 존재하고 접근 가능한 디렉터리여야 하며, 비교 전에 정규화해야 합니다.

명시 프로젝트 ID와 파생 프로젝트 ID는 Runtime Home 초기화, 저장소 마이그레이션, 프로젝트 등록, 접점 등록, MCP 사전 점검, 설정 파일 생성 전에 검증됩니다. 파생한 경로 구성 요소 ID가 무효이면 명시적으로 유효한 `--project-id`를 제공해야 합니다.

프로젝트 ID 검증 규칙:

- 빈 값과 공백뿐인 값은 무효입니다.
- `.`와 `..`는 무효입니다.
- `/`, `\`, `NUL` 문자는 무효입니다.
- 설정은 무효 ID의 공백을 조용히 잘라내거나, slugify하거나, 다시 쓰거나, 대체하지 않습니다.
- `harness setup local-mcp`와 낮은 수준의 프로젝트 등록 경로는 같은 검증을 공유합니다.
- 설정은 구현된 경로 구성 요소 제외 규칙보다 넓은 문자 허용 목록을 정의하지 않습니다.

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

설정이 기존 프로젝트 등록을 재사용하기 전에는 저장된 `Product Repository`가 선택된 `Harness Runtime Home`과의 [Runtime Home/Product Repository 분리 계약](runtime-boundaries.md#runtime-home-product-repository-separation)을 계속 만족해야 합니다. 이 관계를 위반하는 이력 등록은 저장소 준비, 접점 등록, MCP 사전 점검, 설정 출력 전에 실패합니다. 설정은 해당 registry 행을 복구하거나, 갱신하거나, 삭제하거나, 다른 행으로 대체하지 않습니다. 지원되는 복구 사실은 별도의 `Harness Runtime Home`을 선택하고 그곳에서 `Product Repository`를 설정하는 것입니다.

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

### 미리보기 검사와 이력 스키마

설정 계획과 dry-run은 기존 Runtime Home 데이터베이스를 읽기 전용, 마이그레이션 없음 경로로 검사합니다. 지원되는 이력 스키마는 마이그레이션 없이 검사할 수 있으며, 나중의 실제 설정 경로에서 마이그레이션이 필요하다고 내부적으로 분류될 수 있습니다. 이 검사는 권한을 만들거나, 등록 의미를 바꾸거나, 저장소 복구를 적용하지 않습니다.

지원하지 않는 스키마, 일관성이 없는 스키마, 손상된 스키마, 안전하게 검사할 수 없는 스키마는 복구나 수정 없이 계획 단계에서 실패합니다. 읽기 전용 검사는 기존 마이그레이션이 모호하지 않은 기본값을 정의한 경우에만 이력 등록 의미를 정규화합니다. 예를 들어 저장된 `interaction_role`이 없는 오래된 접점 행은 호환성 분석을 위해 데이터베이스를 바꾸지 않고 `agent`로 검사됩니다.

자세한 저장소 마이그레이션 의미는 [저장소 버전 관리](storage-versioning.md)가 담당합니다. 저장소 기록 계열과 기록 배치 담당 경계는 [저장소 기록](storage-records.md)이 담당합니다.

### 실제 승인된 설정 순서

dry-run이 아닌 설정 경로는 비대화식 실행 또는 대화형 최종 긍정 확인 뒤에만 변경 단계에 도달합니다. 순서는 아래와 같습니다.

```text
read-only planning
-> execution approval
-> required recognized storage initialization or migration
-> refreshed planning and conflict validation
-> project and surface registration
-> MCP preflight
-> generated configuration output
```

규칙:

- 마이그레이션은 실제 승인된 설정 경로에서만 일어납니다.
- 마이그레이션 전 계획을 그대로 적용하지 않습니다.
- 저장소 준비 또는 마이그레이션 뒤 계획을 다시 만듭니다.
- 새로 관찰된 충돌은 프로젝트와 접점 등록을 중지합니다.
- 완료된 마이그레이션은 이후 재검증, 등록, 사전 점검, 설정 출력이 실패했다는 이유만으로 롤백되지 않습니다.
- 관련 진단은 완료된 저장소 준비나 마이그레이션을 식별합니다.
- 명령은 부분 실패 뒤 다시 실행할 수 있어야 합니다.
- 데이터베이스, 파일, 시스템을 가로지르는 롤백 보장은 없습니다.
- 마이그레이션 자체는 Core 권한, 사용자 신원, 접점 신뢰, 사용자 소유 판단을 만들지 않습니다.

### 멱등성과 부분 실패

정확히 반복한 설정은 아래 성질을 갖습니다.

- Runtime Home, 프로젝트, 접점 기록을 중복으로 만들지 않습니다.
- 호환되는 기록은 `reused`로 보고합니다.
- 재사용한 프로젝트나 접점 메타데이터를 다시 쓰지 않습니다.
- 기존 `Task`나 Core 워크플로 기록을 수정하지 않습니다.
- 프로젝트 `state_version`을 증가시키지 않습니다.
- 결정적인 호스트 설정을 생성합니다.

등록 변경 전 명령은 현재 관찰 가능한 결정적 입력 오류, 등록 충돌, 실행 파일 탐색 실패, 설정 렌더링 실패, 출력 경로 구조 충돌을 감지합니다. 비대화식 `--repo-root` 누락과 무효인 명시 `--runtime-home` 값은 실행 파일 탐색 전에 실패합니다. 프로젝트 ID 검증과 생성 설정 경로 구조 검증은 저장소 초기화나 마이그레이션 전에 일어납니다.

이 변경 전 검증은 경쟁 상태가 없거나 실패가 없다는 보장이 아닙니다. 외부 파일시스템 변경, 권한 변경, 저장 공간 고갈, 운영체제 오류, MCP 사전 점검 실패, 그 밖의 런타임 실패는 저장소 준비나 등록이 시작된 뒤에도 발생할 수 있습니다. 파일 검사는 time-of-check/time-of-use 경쟁 상태를 제거할 수 없습니다. 나중 실패는 관련될 때 완료된 작업을 보고하고, 생성 대상 파일은 계속 같은 디렉터리의 임시 파일 교체 방식을 사용하며, 시스템 간 트랜잭션은 약속하지 않습니다.

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

<a id="host-neutral-configuration"></a>
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

- 저장소 준비나 등록 전에 `--config-dir`가 기존 디렉터리인지, 또는 기존 디렉터리 조상 아래에 만들 수 있는지 검증합니다.
- 저장소 준비나 등록 전에 기존 조상이 디렉터리가 아니거나 지원하지 않는 파일시스템 객체이면 거절합니다.
- 저장소 준비나 등록 전에 대상 경로가 디렉터리이거나 지원하지 않는 파일시스템 객체이면 거절합니다.
- 저장소 준비나 등록 전에 대상 파일이 이미 있는지 확인합니다.
- 저장소 준비나 등록 전에 기존 일반 대상 파일에는 명시적 덮어쓰기 승인을 요구합니다.
- 어떤 대상도 쓰기 전에 요청된 모든 에이전트 및 사용자 상호작용 대상을 검증합니다.
- 필요하면 대상 디렉터리를 만듭니다.
- 유효하고 결정적인 JSON을 씁니다.
- 대상 파일을 제자리에서 잘라내지 말고 같은 디렉터리의 교체용 파일을 사용합니다.
- 기본적으로 기존 파일을 덮어쓰지 않습니다.
- 기존 생성 파일을 교체하려면 `--overwrite-config`가 필요합니다.
- `--config-dir` 없는 `--overwrite-config`는 사용법 오류입니다.
- dry-run은 같은 구조 검사를 수행하지만 디렉터리나 파일을 만들지 않습니다.

이 명령은 지원 플랫폼 전체에서 구현이 제공할 수 있는 것보다 강한 atomic 동작을 주장하면 안 됩니다. 적어도 부분적으로 쓰인 대상 파일이 완료된 설정으로 노출되면 안 됩니다.

### Dry run

`--dry-run`은 아래를 수행합니다.

- 경로 해석
- 저장소 정규화
- 프로젝트 선택
- 접점 호환성과 충돌 분석
- 마이그레이션 없는 읽기 전용 검사 경로를 통한 기존 Runtime Home 데이터베이스 검사
- 나중의 실제 설정 경로에서 마이그레이션이 필요할 수 있는 지원 이력 스키마 식별
- MCP 실행 파일 탐색
- 설정 렌더링
- 설정 출력 구조와 대상 파일 충돌 분석

수행하지 않는 것:

- Runtime Home 생성
- SQLite 데이터베이스 생성 또는 수정
- 마이그레이션 적용
- `schema_migrations` 변경
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
- 선택된 Runtime Home과 `--repo-root`는 등록이 기록되기 전에 [Runtime Home/Product Repository 분리 계약](runtime-boundaries.md#runtime-home-product-repository-separation)을 만족해야 합니다.

`harness project list`는 선택된 Runtime Home의 등록된 프로젝트를 나열합니다.

`harness project list`는 registry 수준 검사입니다. Runtime Home/Product Repository 분리 계약을 위반하는 이력 프로젝트 기록을 진단 목적으로 보여 줄 수 있습니다. 목록에 보인다는 사실만으로 그 기록이 프로젝트 상태 데이터베이스 접근, 접점 관리, Core 실행, 설정 재사용, MCP 시작에 적격해지지는 않습니다.

`Product Repository`와 `Harness Runtime Home`의 구분을 포함한 런타임 위치 경계는 [런타임 경계](runtime-boundaries.md#runtime-home-product-repository-separation)가 담당합니다.

## 접점 등록

`harness surface register`는 등록된 프로젝트에 로컬 접점 인스턴스 하나를 기록합니다.

접점 등록과 목록 조회는 프로젝트 등록이 [런타임 경계](runtime-boundaries.md#runtime-home-product-repository-separation)가 담당하는 Runtime Home/Product Repository 분리 계약에 따라 계속 적격해야 합니다.

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
