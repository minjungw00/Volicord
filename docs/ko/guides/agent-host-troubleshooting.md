# 에이전트 호스트 문제 해결

Codex, Claude Code, 또는 generic MCP 호스트 통합이 `harness agent install`,
`harness agent verify`, `harness agent status`, 프로젝트 멤버십 변경, uninstall 뒤
기대한 상태에 도달하지 않을 때 이 가이드를 사용합니다.

일반 설정 경로는 [에이전트 호스트 설정](agent-host-setup.md)을 사용합니다. 하나의
사용자 범위 통합이 여러 저장소를 처리해야 하면
[다중 저장소 에이전트 설정](multi-repository-agent-setup.md)을 사용합니다.

이 가이드는 관찰된 상태를 식별하고, 가능할 때 추가 변경 없이 원인을 점검하고,
범위가 제한된 복구 동작을 수행하고, 결과를 확인하는 데 도움을 줍니다. CLI 동작,
MCP 프로세스 동작, 저장 효과, 호스트 어댑터 동작, 보안 보장을 다시 정의하지
않습니다. 정확한 동작은 [관리 CLI](../reference/admin-cli.md),
[MCP 전송](../reference/mcp-transport.md),
[런타임 경계](../reference/runtime-boundaries.md),
[에이전트 통합](../reference/agent-integration.md), 그리고
[저장소](../reference/storage.md)가 안내하는 저장소 담당 문서에 남습니다.

## 변경하기 전에

설정할 때 사용한 같은 자리표시자 값을 유지합니다.

- `HARNESS_BIN`은 `harness`와 `harness-mcp`가 들어 있는 선택된 디렉터리입니다.
- `HARNESS_HOME` 또는 `--runtime-home`은 선택된 `Harness Runtime Home`입니다.
- `<integration_id>`, `<project_id>`, `<repo_root>`, `<installation_id>`,
  `<server_name>`은 설정 출력에 나온 실제 값입니다.

읽기 전용이거나 변경하지 않는 점검이 있으면 거기서 시작합니다.

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id <integration_id> \
  --runtime-home <runtime_home>

HARNESS_HOME=<runtime_home> \
"$HARNESS_BIN/harness-mcp" --check --integration <integration_id>
```

`harness agent status`는 registry와 Host Installation inventory를 보고합니다. Codex
또는 Claude Code가 MCP 서버를 로드했다는 증명이 아닙니다. `harness-mcp --check`는
MCP 프로세스 시작만 검증합니다. 전체 호스트 검증에는
[관리 CLI](../reference/admin-cli.md#agent-setup-result-states)가 정의한 관리 검증
gate가 필요합니다.

## 실행 파일과 환경 문제

<a id="missing-harness-mcp"></a>
### `harness-mcp`가 없거나, 실행 불가능하거나, 해석되지 않음

- **관찰 증상:** 설정, 검증, 또는 호스트 시작이 `harness-mcp`가 없거나, 사용할 수
  없거나, 실행할 수 없거나, `PATH`에서 찾을 수 없다고 보고합니다.
- **가장 가능성 높은 원인:** 선택한 실행 파일 디렉터리에 `harness`와 `harness-mcp`
  둘 다 있지 않습니다. 파일을 선택한 사용자가 실행할 수 없습니다. 또는 프로젝트
  범위 호스트 설정은 `harness-mcp`를 저장했지만 미래의 호스트 프로세스가 이를 찾을
  `PATH`를 받지 못합니다.
- **진단 점검:** 절대 명령이면 `test -x "$HARNESS_BIN/harness-mcp"`와
  `"$HARNESS_BIN/harness-mcp" --version`을 실행합니다. 프로젝트 범위의 이식 가능한
  명령이면 호스트를 시작할 같은 셸, 실행기, 서비스 환경에서 `command -v
  harness-mcp`를 실행합니다.
- **제한된 복구 동작:** 두 실행 파일을 모두 포함하는 실행 파일 디렉터리를 선택하거나
  빌드합니다. 사용자, 로컬, generic export 범위에서는 절대 `--mcp-command`로 install
  또는 verify를 다시 실행합니다. 프로젝트 범위에서는 생성된 호스트 항목을 이식 가능한
  형태로 유지하고 호스트 시작 `PATH`를 고칩니다.
- **검증:** 의도한 `HARNESS_HOME`으로 `harness-mcp --check --integration
  <integration_id>`를 다시 실행한 뒤, 영향받은 통합 또는 설치에 대해 `harness agent
  verify`를 다시 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 지속 설정이 시작된 뒤 실패했다면 Runtime Home
  기록, 프로젝트 멤버십, Host Installation inventory, 호스트 설정, guidance가 이미
  있을 수 있습니다. 설치를 반복하기 전에 `effects`와 `residual_effects`를 읽습니다.
- **건드리지 말아야 할 상태 또는 파일:** 실행 파일을 해석하지 못했다는 이유만으로
  Runtime Home, 프로젝트 상태, Product Repository 파일, 관련 없는 호스트 설정을
  삭제하지 않습니다.
- **담당 문서 링크:** [시스템 요구사항](../reference/system-requirements.md),
  [관리 CLI](../reference/admin-cli.md), [MCP 전송](../reference/mcp-transport.md#configuration-preflight).

<a id="wrong-absolute-mcp-command"></a>
### 절대 `--mcp-command`가 잘못됨

- **관찰 증상:** CLI가 `--mcp-command`를 거절하거나, 이후 검증이 설정된 명령이
  없거나, 바뀌었거나, 사용할 수 없거나, 시작할 수 없다고 보고합니다.
- **가장 가능성 높은 원인:** 경로가 절대 경로가 아니거나, 오래된 빌드 산출물을
  가리키거나, `harness-mcp`가 아니라 `harness`를 가리키거나, 재빌드 또는 이동 뒤 더
  이상 존재하지 않습니다.
- **진단 점검:** 호스트 설정을 바꾸지 않고 `test -x /absolute/path/to/harness-mcp`와
  `/absolute/path/to/harness-mcp --help`를 실행합니다.
- **제한된 복구 동작:** 같은 `integration_id`, 호스트, 범위, 서버 이름으로
  `harness agent install`을 다시 실행하되 올바른 절대 `--mcp-command`를 제공합니다.
  기존 관리 항목이 사용자가 바꾼 상태라면, 관리 지문이나 소유 marker가 그 내용이
  여전히 하네스 관리 내용임을 보여 줄 때만 교체를 사용합니다.
- **검증:** `harness agent verify --integration-id <integration_id> --runtime-home
  <runtime_home>`를 다시 실행합니다. 정확한 대상을 확인해야 하면 JSON 출력의 `host`와
  `installation_verifications`를 봅니다.
- **이미 존재할 수 있는 지속 효과:** 교체가 성공하기 전까지 Host Installation
  inventory가 이전 설정 대상과 관리 지문을 계속 가리킬 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** `registry.sqlite`를 직접 편집하거나 관련
  없는 호스트 항목을 덮어쓰지 않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md),
  [에이전트 통합](../reference/agent-integration.md#host-installation),
  [런타임 경계](../reference/runtime-boundaries.md#runtime-location-server-installation).

<a id="portable-project-command-not-on-path"></a>
### 프로젝트 범위의 이식 가능한 명령이 호스트 `PATH`에서 보이지 않음

- **관찰 증상:** 프로젝트 범위 Codex 또는 Claude Code 설정에 `command =
  "harness-mcp"` 또는 `"command": "harness-mcp"`가 있지만, 이후 호스트 세션이
  하네스를 시작하지 못합니다. 또는 관리 명령에 `PATH="$HARNESS_BIN:$PATH"`를 더할
  때만 검증이 성공합니다.
- **가장 가능성 높은 원인:** 프로젝트 범위 설정은 의도적으로 개인 빌드 경로와 개인
  `HARNESS_HOME`을 생략합니다. 미래의 호스트 프로세스가 `harness-mcp`를 찾을 수 없는
  환경에서 시작되었습니다.
- **진단 점검:** 호스트 시작 환경에서 `command -v harness-mcp`를 실행합니다. 의도한
  Runtime Home이 기본값이 아니라면 같은 시작 환경이 `HARNESS_HOME`도 제공하는지
  확인합니다.
- **제한된 복구 동작:** 호스트 시작 환경, 셸 시작 파일, 서비스 설정, 또는 그에
  준하는 호스트 소유 경로가 `harness-mcp`를 찾을 수 있게 합니다. 프로젝트 범위
  호스트 파일은 이식 가능한 형태로 유지합니다.
- **검증:** 그 환경에서 호스트를 시작하거나 다시 로드한 뒤, 관리 검증 명령의 `PATH`에
  선택한 디렉터리를 둔 상태로 `harness agent verify`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 프로젝트 범위 `.codex/config.toml` 또는
  `.mcp.json`, Runtime Home 기록, Host Installation inventory가 이미 있고 올바를 수
  있습니다.
- **건드리지 말아야 할 상태 또는 파일:** 공유 Product Repository 파일 안의 프로젝트
  범위 `harness-mcp` 명령을 개인 절대 빌드 경로로 바꾸지 않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md),
  [시스템 요구사항](../reference/system-requirements.md),
  [런타임 경계](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories).

## 위치와 파일 문제

<a id="runtime-home-product-repository-overlap"></a>
### Runtime Home과 Product Repository가 잘못 배치되었거나 겹침

- **관찰 증상:** 프로젝트 등록, 설정 재사용, MCP 시작, 프로젝트 목록 조회, 공개 도구
  라우팅이 경로 분리 또는 등록 불변식 오류를 보고합니다. 같은 경로 또는 상위/하위
  관계 오류가 여기에 해당합니다.
- **가장 가능성 높은 원인:** 선택한 `Harness Runtime Home`이 Product Repository
  자체이거나, 그 안에 있거나, 그 위에 있습니다. 또는 저장된 등록이 등록된 프로젝트
  홈과 더 이상 맞지 않는 프로젝트 상태 경로를 가리킵니다.
- **진단 점검:** 어떤 것도 바꾸기 전에 해석된 Runtime Home 경로와 저장소 루트를
  비교합니다. 등록된 상태를 보려면 `harness agent status` 또는 `harness project
  list`를 사용합니다. 유효하지 않은 운영 행은 정상 프로젝트로 반환되지 않고
  거절됩니다.
- **제한된 복구 동작:** 서로 분리된 Runtime Home과 Product Repository를 선택합니다.
  관리 CLI를 통해 수정된 경로로 등록 또는 설치합니다. 예전의 유효하지 않은 행이
  있으면 진단할 데이터로 취급하고 SQLite를 직접 고치지 않습니다.
- **검증:** 수정된 설정을 적용한 뒤 `harness agent install --dry-run` 또는
  `harness project list`를 다시 실행하고, 이후 `harness-mcp --check --integration
  <integration_id>`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 운영 조회가 거절하더라도 원시 registry 내용은
  진단용으로 남아 있을 수 있습니다. Runtime Home과 프로젝트 상태 위치도 이미 존재할
  수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** 경로를 맞추려고 Product Repository 내용이나
  Runtime Home 데이터베이스를 직접 이동, 삭제, 재작성하지 않습니다.
- **담당 문서 링크:** [런타임 경계](../reference/runtime-boundaries.md#runtime-home-product-repository-separation),
  [저장소 기록](../reference/storage-records.md),
  [관리 CLI](../reference/admin-cli.md).

<a id="host-config-read-write-failure"></a>
### 호스트 설정 파일을 읽거나 쓸 수 없음

- **관찰 증상:** install, verify, guidance, uninstall이 설정 대상이 디렉터리이거나,
  UTF-8 텍스트가 아니거나, 잘못된 JSON 또는 TOML이거나, 지원하지 않는 파일시스템
  타입이거나, 계획 뒤 바뀌었거나, 읽기, 만들기, 쓰기, 이동, 제거가 불가능하다고
  보고합니다.
- **가장 가능성 높은 원인:** 호스트 대상 경로가 일반 파일이 아니거나, 부모
  디렉터리를 사용할 수 없거나, 선택된 사용자의 권한이 부족하거나, 계획과 쓰기 사이에
  파일이 바뀌었거나, 기존 호스트 형식이 잘못되었습니다.
- **진단 점검:** `harness agent install --dry-run --output json` 또는 `harness agent
  uninstall --dry-run --output json`으로 정확한 대상 경로를 미리 봅니다. 그 경로를
  일반 파일시스템 도구로 검사하되 편집하지 않습니다.
- **제한된 복구 동작:** 호스트 소유 파일 또는 디렉터리 상태를 고친 뒤 같은 관리
  명령을 다시 실행합니다. 내용이 바뀌었다면 관련 없는 항목은 보존하고 marker나
  fingerprint가 맞는 하네스 관리 내용만 교체하거나 제거합니다.
- **검증:** inventory를 확인하려면 `harness agent status`를 다시 실행하고, 호스트
  대상이 다시 읽기 가능해지면 `harness agent verify`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 읽기 또는 쓰기 실패 전에 Runtime Home 상태,
  Host Installation inventory, 호스트 설정, guidance가 적용되었을 수 있습니다.
  정확한 대상은 `effects`와 `residual_effects`에서 확인합니다.
- **건드리지 말아야 할 상태 또는 파일:** 호스트 설정 파일 전체나 Product Repository를
  삭제하지 않습니다. 관련 없는 호스트 항목, 사용자 편집, 관리되지 않는 guidance를
  보존합니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md#setup-output),
  [런타임 경계](../reference/runtime-boundaries.md),
  [에이전트 통합](../reference/agent-integration.md#host-installation).

<a id="managed-fingerprint-conflict"></a>
### 관리 설정 fingerprint 충돌 또는 사용자가 바꾼 관리 내용

- **관찰 증상:** install, verify, guidance, uninstall이 같은 서버 이름에 대해 변경된
  관리 항목, fingerprint 불일치, 관련 없는 항목, 또는 충돌을 보고합니다.
- **가장 가능성 높은 원인:** 하네스가 마지막으로 지문을 기록한 뒤 사용자나 호스트가
  하네스 관리 블록 또는 MCP 항목을 바꿨습니다. 또는 같은 `<server_name>`을 관리되지
  않는 호스트 설정이 이미 사용합니다.
- **진단 점검:** `harness agent status --output json`을 실행하고 보고된 호스트 대상,
  `managed_fingerprint`, `fingerprint_state`, 경고 문구를 확인합니다. 이름 붙은 호스트
  항목이나 관리 블록만 비교합니다.
- **제한된 복구 동작:** 현재 내용이 여전히 하네스 관리 내용이고 교체하려는 의도가
  분명하면 install 또는 guidance apply를 `--replace-managed`와 함께 다시 실행합니다.
  관리 내용을 제거하려면 소유권 점검이 허용할 때만 uninstall 또는 guidance remove를
  `--remove-managed`와 함께 사용합니다. 그렇지 않으면 다른 `--server-name`을 고르거나
  사용자 소유 항목을 보존합니다.
- **검증:** 교체 뒤에는 `harness agent verify`를 다시 실행합니다. 보존 또는 제거 뒤에는
  `harness agent status`를 다시 실행합니다.
- **이미 존재할 수 있는 지속 효과:** Host Installation inventory가 이전 지문을 유지할
  수 있고, 검증은 변경된 설치에 대해 `failed`를 기록할 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** 하네스 지문을 맞추기 위해 관련 없는 호스트
  설정, 관리되지 않는 호스트 항목, 사용자가 편집한 guidance를 덮어쓰거나 제거하지
  않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md#noninteractive-approval-behavior),
  [에이전트 통합](../reference/agent-integration.md#host-installation),
  [런타임 경계](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories).

## 호스트 소유 후속 조치

<a id="codex-availability-or-trust"></a>
### Codex 실행 파일 가용성 또는 프로젝트 trust가 완료되지 않음

- **관찰 증상:** 결과 상태가 `action_required`입니다. 검증은 Codex 실행 파일을 사용할
  수 없거나, `codex --version`이 실패했거나, 프로젝트 trust가 확인되지 않았다고
  보여 줍니다.
- **가장 가능성 높은 원인:** 관리 프로세스 `PATH`에 `codex`가 없거나, `codex
  --version`을 실행할 수 없거나, Codex가 프로젝트 범위 `.codex/config.toml`을 소유한
  프로젝트를 아직 신뢰하지 않았습니다.
- **진단 점검:** 관리 검증에 쓰는 환경에서 `command -v codex`와 `codex --version`을
  실행합니다. 관리 설정 대상을 확인하려면 `harness agent status`를 사용합니다.
- **제한된 복구 동작:** 선택된 사용자에게 Codex를 설치하거나 가용성을 복구한 뒤,
  프로젝트 범위를 사용한다면 Codex 안에서 프로젝트 trust 단계를 완료합니다.
- **검증:** `harness agent verify --integration-id <integration_id> --runtime-home
  <runtime_home>`를 다시 실행합니다.
- **이미 존재할 수 있는 지속 효과:** Runtime Home 기록과 Codex 설정이 이미 설치되어
  있을 수 있으며, `last_verified_status`가 `action_required`일 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** `complete`를 강제로 만들기 위해 하네스
  저장소를 편집하지 않습니다. 관리 지문이 맞는 Codex 설정은 제거하지 않습니다.
- **담당 문서 링크:** [시스템 요구사항](../reference/system-requirements.md),
  [관리 CLI](../reference/admin-cli.md#agent-setup-result-states),
  [에이전트 통합](../reference/agent-integration.md#host-installation).

<a id="claude-project-approval"></a>
### Claude Code 프로젝트 MCP 승인이 완료되지 않음

- **관찰 증상:** 프로젝트 범위 Claude Code install 또는 verify가 `status:
  action_required`를 보고하거나, `claude mcp get <server_name>`이 승인을 기다리고
  있다고 보고합니다.
- **가장 가능성 높은 원인:** 프로젝트 `.mcp.json` 항목은 있지만 Claude Code가
  프로젝트 범위 MCP 서버를 아직 승인하지 않았습니다.
- **진단 점검:** Product Repository에서 `.mcp.json`을 편집하지 않고 `claude mcp get
  <server_name>`을 실행합니다.
- **제한된 복구 동작:** Claude Code의 호스트 소유 승인 흐름으로 프로젝트 MCP 서버를
  승인합니다. Claude Code가 요구하면 호스트를 다시 로드하거나 재시작합니다.
- **검증:** `harness agent verify`를 다시 실행합니다. 승인 대기 중에도 진단 MCP
  handshake는 가능할 수 있지만, 호스트 승인 gate가 충족되기 전까지 최종 상태는
  `action_required`로 남습니다.
- **이미 존재할 수 있는 지속 효과:** `.mcp.json`, Runtime Home 기록, 프로젝트
  멤버십, Host Installation inventory가 이미 있을 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** 승인을 우회하려고 `.mcp.json`을 제거하거나
  다시 쓰지 않습니다. 관련 없는 Claude Code MCP 항목은 그대로 둡니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md#agent-setup-result-states),
  [에이전트 통합](../reference/agent-integration.md#host-installation),
  [런타임 경계](../reference/runtime-boundaries.md).

<a id="already-running-process-stale"></a>
### 이미 실행 중인 MCP 프로세스가 새 통합 상태를 반영하지 않음

- **관찰 증상:** 통합 멤버십, 기본 프로젝트, 호스트 설정, 서버 명령을 바꿨지만 열려
  있는 Codex 또는 Claude Code 세션이 예전 상태처럼 동작합니다.
- **가장 가능성 높은 원인:** 실행 중인 `harness-mcp` 프로세스는 수명 동안 하나의
  `integration_id`에 묶입니다. registry 멤버십 변경은 실행 중인 프로세스가 관찰할 수
  있지만, 바뀐 통합 바인딩, 바뀐 명령, 바뀐 호스트 설정, reload, restart는 호스트가
  소유한 동작입니다.
- **진단 점검:** 저장된 inventory는 `harness agent status`로 확인합니다. 기존 MCP
  세션에서 도구를 여전히 호출할 수 있다면 `harness.list_projects`를 호출해 그 실행
  중인 프로세스가 관찰하는 허용 프로젝트를 확인합니다.
- **제한된 복구 동작:** 멤버십만 바뀐 경우 `harness.list_projects`가 새 목록을
  반영한 뒤 명시적 `project_id`로 도구 호출을 다시 시도합니다. 호스트 설정, 명령
  경로, 서버 이름, 통합 바인딩이 바뀐 경우 호스트를 reload 또는 restart하여 새 MCP
  프로세스를 시작하게 합니다.
- **검증:** 재시작 뒤 `harness agent verify`를 실행하고, 호스트 안에서는 프로젝트
  선택이 불분명할 때 `harness.list_projects`를 사용한 뒤 프로젝트 라우팅 호출을
  수행합니다.
- **이미 존재할 수 있는 지속 효과:** 예전 프로세스가 아직 실행 중이어도 Runtime Home
  registry 변경과 호스트 설정은 이미 커밋되어 있을 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** 실행 중인 프로세스를 새로고침하려고 Host
  Installation record를 중복으로 만들거나 서버 항목을 하나 더 추가하지 않습니다.
- **담당 문서 링크:** [MCP 전송](../reference/mcp-transport.md),
  [에이전트 통합](../reference/agent-integration.md).

## 결과 상태

<a id="status-action_required"></a>
### `status: action_required`

- **관찰 증상:** `harness agent install` 또는 `harness agent verify`가 성공 종료하고
  `status: action_required`를 보고합니다.
- **가장 가능성 높은 원인:** 지속 통합 상태와 호스트 설정은 있지만 호스트 소유 trust,
  승인, OAuth, reload, restart 또는 비슷한 행동이 남았습니다.
- **진단 점검:** `--output json`에서 `action_required` 배열, `verification` 세부사항,
  host gate 필드, Host Installation inventory를 확인합니다.
- **제한된 복구 동작:** 이름 붙은 호스트 소유 동작만 수행합니다. 프로젝트를 trust하고,
  MCP 서버를 승인하고, 호스트를 reload 또는 restart하거나, 호스트 실행 파일 가용성을
  복구합니다.
- **검증:** 통합 또는 특정 `--installation-id`에 대해 `harness agent verify`를 다시
  실행합니다.
- **이미 존재할 수 있는 지속 효과:** Runtime Home 기록, 관리되는 호스트 설정, Host
  Installation inventory, 선택적 guidance가 존재하는 것이 예상 상태입니다.
- **건드리지 말아야 할 상태 또는 파일:** `action_required`가 나왔다는 이유만으로
  통합을 롤백하거나 삭제하지 않습니다. 이 상태 자체는 실패 결과가 아닙니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md#agent-setup-result-states),
  [에이전트 통합](../reference/agent-integration.md#host-installation).

<a id="status-partial_failure"></a>
### `status: partial_failure`

- **관찰 증상:** 에이전트 명령이 상태 `1`로 종료하고 `status: partial_failure`를
  보고합니다.
- **가장 가능성 높은 원인:** 일부 지속 관리 동작은 성공했지만 뒤따르는 install,
  verify, 호스트 대상, 롤백, 정리, 지속 상태 업데이트 단계가 실패했습니다.
- **진단 점검:** JSON 출력의 `effects`, `residual_effects`, warnings,
  `installation_verifications`를 읽습니다. 각 잔여 효과는 component, target, current
  state, reason, recommended action을 이름 붙입니다.
- **제한된 복구 동작:** 보고된 원인을 고친 뒤, 이름 붙은 잔여 대상만 처리합니다. 광범위한
  파일 또는 데이터베이스 삭제 대신 uninstall, guidance remove, project default, project
  membership 명령을 사용합니다.
- **검증:** 실패했던 명령을 다시 실행하거나, 설정 또는 호스트 상태를 고친 뒤 `harness
  agent verify`를 실행합니다. 잔여 효과가 사라졌거나 담당 문서가 지원하는 명령으로
  의도적으로 보존되었는지 확인합니다.
- **이미 존재할 수 있는 지속 효과:** `residual_effects`에 보고되면 호스트 설정,
  guidance, Host Installation inventory, Runtime Home 생성 또는 마이그레이션, 프로젝트
  등록, surface 등록, 통합 기록, 기본 프로젝트, 멤버십 행이 남을 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** Runtime Home 전체, Product Repository,
  아티팩트 저장소, Core 기록, 관련 없는 호스트 항목, 사용자가 편집한 guidance를
  삭제하지 않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md#setup-output),
  [런타임 경계](../reference/runtime-boundaries.md), [저장소 기록](../reference/storage-records.md).

<a id="status-failed"></a>
### `status: failed`

- **관찰 증상:** install 또는 verify가 상태 `1`로 종료하고 `status: failed`를
  보고합니다.
- **가장 가능성 높은 원인:** 요청한 동작이 사용할 수 있는 지속 통합 상태 또는 호스트
  설정을 만들지 못했습니다. 또는 검증이 선택한 Host Installation이 현재 없음, 변경됨,
  거절됨, 사용할 수 없음, 알 수 없음, 실패 상태입니다.
- **진단 점검:** `verification.details`, `warnings`, `effects`, `residual_effects`를
  읽습니다. `residual_effects`가 비어 있으면, 명령은 남아 있는 적용 효과를 알고 있지
  않습니다.
- **제한된 복구 동작:** 보고된 근본 원인을 고치고, 다음 명령이 파일을 쓸 수 있다면
  dry-run을 실행한 뒤 install 또는 verify를 다시 시도합니다. verify가 Host
  Installation이 없다고 말하면 먼저 의도한 호스트에 대해 install을 실행합니다.
- **검증:** 원인을 고친 뒤 `harness agent status`를 다시 실행하고 `harness agent
  verify`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 기존 Runtime Home, 프로젝트 상태, 호스트 설정은
  남아 있을 수 있습니다. 실패한 동작에서 새 효과가 남았다면 `effects`와
  `residual_effects`에 식별되어야 합니다.
- **건드리지 말아야 할 상태 또는 파일:** 실패가 사용자 데이터, Product Repository
  내용, 관련 없는 설정을 삭제해도 된다는 뜻이라고 추론하지 않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md#agent-setup-result-states),
  [관리 CLI](../reference/admin-cli.md#setup-output).

## 프로젝트 선택과 멤버십 문제

<a id="no-allowed-project"></a>
### 명시적으로 허용된 프로젝트가 없음

- **관찰 증상:** status가 통합에 허용 프로젝트가 없다고 경고하거나, `harness-mcp
  --check`가 시작 검증에 실패하거나, verify가 실패하거나, 마지막 프로젝트가 제거되기
  전에 시작된 프로세스에서 `harness.list_projects`가 빈 목록을 반환합니다.
- **가장 가능성 높은 원인:** 마지막 통합 프로젝트 멤버십이 제거되었거나, Agent
  Integration Profile에 대한 멤버십이 성공적으로 만들어지지 않았습니다.
- **진단 점검:** `harness agent status --integration-id <integration_id>
  --runtime-home <runtime_home>`를 실행합니다. MCP 프로세스가 이미 실행 중이면
  `harness.list_projects`를 호출해 지금 빈 allowlist를 보는지 확인합니다.
- **제한된 복구 동작:** 명시적 프로젝트 하나를 추가하거나 복구합니다. `harness agent
  project add --integration-id <integration_id> --project-id <project_id> --repo-root
  <repo_root> --runtime-home <runtime_home>`를 사용합니다. 편의 기본값이 필요할 때만
  default를 설정합니다.
- **검증:** `harness-mcp --check --integration <integration_id>`를 실행한 뒤 `harness
  agent verify`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 허용 프로젝트가 없어도 Agent Integration
  Profile, Host Installation inventory, 호스트 설정, guidance가 남을 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** allowlist가 비었다는 이유만으로 호스트 항목을
  다시 설치하거나 호스트 설정을 제거하지 않습니다.
- **담당 문서 링크:** [에이전트 통합](../reference/agent-integration.md),
  [MCP 전송](../reference/mcp-transport.md),
  [관리 CLI](../reference/admin-cli.md).

<a id="ambiguous-project-selection"></a>
### 여러 허용 프로젝트가 있지만 쓸 수 있는 selector나 default가 없음

- **관찰 증상:** 공개 MCP 도구 호출이 Core 실행 전에 거절되고, `project selection is
  ambiguous; call harness.list_projects and retry with envelope.project_id` 같은 실행
  가능한 텍스트가 반환됩니다.
- **가장 가능성 높은 원인:** 여러 프로젝트를 사용할 수 있는데 호출이
  `envelope.project_id`를 생략했고 유효한 명시적 `default_project_id`도 사용할 수
  없습니다.
- **진단 점검:** 읽기 전용 어댑터 유틸리티 `harness.list_projects`를 호출하고 반환된
  project id, 가용성, 기본값 상태를 확인합니다.
- **제한된 복구 동작:** 공개 도구 호출을 명시적 `envelope.project_id`와 함께 다시
  시도합니다. 생략 선택이 정말 필요하다면 `harness agent project default set
  --integration-id <integration_id> --project-id <project_id>`로 default를 설정합니다.
- **검증:** 거절되었던 도구 호출을 다시 시도하고 의도한 프로젝트에 도달했는지 확인합니다.
- **이미 존재할 수 있는 지속 효과:** 거절된 공개 호출에서는 없습니다. 모호한 선택은
  Core 실행 전에 거절됩니다.
- **건드리지 말아야 할 상태 또는 파일:** 폴더 이름, 현재 작업 디렉터리, MCP roots,
  호스트 라벨, 기억에서 추측하지 않습니다. 선택을 쉽게 만들려고 프로젝트를 제거하지
  않습니다.
- **담당 문서 링크:** [에이전트 통합](../reference/agent-integration.md#current-surface-context),
  [MCP 전송](../reference/mcp-transport.md).

<a id="default-project-invalid-or-blocking-removal"></a>
### 기본 프로젝트가 유효하지 않거나 제거 전에 지워야 함

- **관찰 증상:** 프로젝트 제거가 그 프로젝트가 아직 `default_project_id`라서
  실패합니다. 또는 저장된 default가 더 이상 허용, 활성, 가용, 실행 가능 프로젝트가
  아니어서 시작이나 선택에 사용할 수 없습니다.
- **가장 가능성 높은 원인:** default가 제거하려는 프로젝트를 아직 가리키거나, default가
  현재 멤버십 또는 프로젝트 가용성과 맞지 않게 오래되었습니다.
- **진단 점검:** `harness agent status --output json`을 실행하고
  `default_project_id`, `allowed_projects`, 프로젝트 가용성 경고를 확인합니다.
- **제한된 복구 동작:** 다른 허용 프로젝트가 편의 default로 남아야 하면 `harness agent
  project default set`을 실행합니다. 마지막 프로젝트를 제거한다면 먼저 `harness agent
  project default clear`를 실행한 뒤 멤버십을 제거합니다.
- **검증:** `harness agent status`를 다시 실행합니다. 새 시작 자격을 확인하려면 하나
  이상의 프로젝트를 다시 허용한 뒤 `harness-mcp --check --integration <integration_id>`를
  실행합니다.
- **이미 존재할 수 있는 지속 효과:** default를 바꾸거나 지우고 제거가 성공하기 전까지
  멤버십은 남습니다. default 변경이나 멤버십 제거는 호스트 설정을 다시 쓰지 않습니다.
- **건드리지 말아야 할 상태 또는 파일:** default 프로젝트 규칙을 우회하려고 호스트
  설정이나 registry 행을 직접 편집하지 않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md),
  [에이전트 통합](../reference/agent-integration.md).

<a id="storage-version-unsupported"></a>
### Registry 또는 project-state 저장소 버전이 지원되지 않음

- **관찰 증상:** dry-run, install, status, verify, project list, MCP 시작이 registry
  또는 project state schema version, migration row, storage profile이 지원되지 않는다고
  보고합니다.
- **가장 가능성 높은 원인:** Runtime Home 또는 프로젝트 상태 데이터베이스가 지원하지
  않는 프로필, 더 최신 빌드, 손상되거나 일부만 적용된 마이그레이션, 또는 정확한 예전
  `baseline_sqlite` 프로필로 만들어졌습니다.
- **진단 점검:** 진단 명령이 마이그레이션이나 복구를 시도하지 않도록 `harness agent
  install --dry-run --output json` 또는 `harness agent status --output json`을
  우선합니다.
- **제한된 복구 동작:** 이 체크아웃으로 해당 Runtime Home 사용을 중지합니다. 새
  기준 상태가 필요하면 명시적 새 Runtime Home을 다시 초기화하고, 기존 기록이 필요하면
  호환되는 백업에서 복원합니다.
- **검증:** 선택한 호환 Runtime Home에 대해 같은 dry-run 또는 status 명령을 다시
  실행합니다. 그 뒤 일반 설정 또는 `harness-mcp --check`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 지원되지 않는 기존 SQLite 파일은 그 위치에
  남습니다. 기준 경로는 이를 변환, 삭제, 재작성, 자동 마이그레이션하지 않습니다.
- **건드리지 말아야 할 상태 또는 파일:** migration row, storage profile 값, registry
  테이블, 프로젝트 `state.sqlite`를 직접 편집하지 않습니다.
- **담당 문서 링크:** [저장소 버전 관리](../reference/storage-versioning.md),
  [저장소 기록](../reference/storage-records.md),
  [관리 CLI](../reference/admin-cli.md#dry-run).

## 제거와 정리 문제

<a id="partial-removal"></a>
### 제거가 일부만 완료됨

- **관찰 증상:** `harness agent uninstall`이 상태 `1`로 종료하고 `status:
  partial_failure`를 보고합니다. `residual guidance preserved` 같은 경고가 함께 나올
  수 있습니다.
- **가장 가능성 높은 원인:** 관리되는 호스트 설정은 제거되었지만 관리되는 repository
  guidance를 안전하게 제거하지 못했습니다. 호스트 항목이나 guidance 블록이 계획 뒤
  바뀌었거나 정리 중 파일 작업이 실패했습니다.
- **진단 점검:** 같은 `--integration-id`, 사용했다면 `--installation-id`, 그리고
  `--allow-repository-write`, `--remove-managed` 플래그로 `harness agent uninstall
  --dry-run --output json`을 실행해 정확한 남은 대상을 미리 봅니다.
- **제한된 복구 동작:** 이름 붙은 잔여 guidance 또는 호스트 대상만 해결합니다. 소유
  marker가 여전히 맞을 때 `--remove-managed`와 함께 uninstall 또는 guidance remove를
  다시 실행합니다.
- **검증:** 남은 Host Installation inventory와 guidance 상태를 확인하려면 `harness
  agent status`를 실행합니다. 호스트 설정이 남아 있으면 그것에 의존하기 전에 `harness
  agent verify`를 실행합니다.
- **이미 존재할 수 있는 지속 효과:** 일부 Host Installation inventory가 이미 제거되었을
  수 있고, 설치가 남아 있지 않으면 Agent Integration Profile이 비활성화되었을 수
  있습니다. 일부 guidance 또는 호스트 파일은 보고된 대로 남을 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** Product Repository, 프로젝트 등록, Core 기록,
  Runtime Home 위치, 아티팩트 저장소, 관련 없는 호스트 항목을 삭제하지 않습니다.
- **담당 문서 링크:** [관리 CLI](../reference/admin-cli.md),
  [런타임 경계](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories),
  [에이전트 통합](../reference/agent-integration.md#host-installation).

<a id="host-config-remains-zero-projects"></a>
### 현재 허용 프로젝트가 없지만 호스트 설정이 남아 있음

- **관찰 증상:** `harness agent status`가 Host Installation inventory 또는 호스트 설정을
  보여 주지만 `allowed_project_count: 0`이거나 통합이 프로젝트를 추가할 때까지 실행
  가능하지 않다는 경고를 보고합니다.
- **가장 가능성 높은 원인:** 마지막 허용 프로젝트가 의도적으로 제거되었습니다. 호스트
  설정과 inventory는 남을 수 있지만 시작 자격은 아닙니다.
- **진단 점검:** `harness agent status --integration-id <integration_id>`를 실행합니다.
  이전 MCP 프로세스가 아직 살아 있다면 `harness.list_projects`를 호출해 빈 목록을
  반환하는지 확인합니다.
- **제한된 복구 동작:** 통합을 다시 사용해야 하면 `harness agent project add`로
  프로젝트를 추가합니다. 통합을 완전히 제거해야 하면 guidance 제거가 필요할 때 필수
  repository-write 플래그와 함께 `harness agent uninstall --remove-managed`를
  실행합니다.
- **검증:** 다시 사용하려면 프로젝트를 추가한 뒤 `harness-mcp --check`와 `harness
  agent verify`를 실행합니다. 제거하려면 `harness agent status`를 다시 실행하고 남은
  installation과 guidance를 확인합니다.
- **이미 존재할 수 있는 지속 효과:** allowlist가 비어도 Agent Integration Profile,
  Host Installation inventory, 호스트 설정, guidance가 남을 수 있습니다.
- **건드리지 말아야 할 상태 또는 파일:** 남은 호스트 파일을 새 시작이 성공할 수 있다는
  증거로 취급하지 않고, 관련 없는 호스트 항목을 삭제하지 않습니다.
- **담당 문서 링크:** [에이전트 통합](../reference/agent-integration.md),
  [MCP 전송](../reference/mcp-transport.md),
  [관리 CLI](../reference/admin-cli.md).
