# 다중 저장소 에이전트 설정

하나의 사용자 범위 통합이 명시적으로 허용된 여러 `Product Repository` 등록을 처리해야 할 때 이 가이드를 사용합니다.

기준 토폴로지는 다음과 같습니다.

```mermaid
flowchart LR
  host[Codex 사용자 MCP 항목]
  process["harness-mcp --integration int-codex-team"]
  allowlist[명시적 integration project allowlist]
  a["project_id: acme-api<br/>/work/acme-api"]
  b["project_id: billing-api<br/>/work/billing-api"]

  host --> process
  process --> allowlist
  allowlist --> a
  allowlist --> b
```

호스트 MCP 항목 하나, `harness-mcp --integration <integration_id>` 프로세스 하나, 명시적 allowlist 하나가 있고, 도구 호출마다 여러 저장소 중 하나를 선택합니다. 프로젝트를 추가해도 모든 Runtime Home 프로젝트가 허용되지는 않습니다. 접근 제거는 호스트 항목을 다시 쓰지 않아도 registry 상태를 통해 적용됩니다.

프로젝트 및 로컬 호스트 범위는 단일 저장소 범위로 남습니다. 이 토폴로지에는 사용자 범위를 사용합니다.

## 실행 파일 선택 규칙

아래 명령 예시는 `harness`와 `harness-mcp`가 함께 들어 있는 절대 디렉터리 하나를 선택하고 현재 셸에서 내보냈다고 가정합니다.

```sh
export HARNESS_BIN="/absolute/path/to/selected/bin"
```

`Harness Server` 소스 저장소 루트에서 빌드한다면 디버그 빌드는 아래 값을 사용할 수 있습니다.

```sh
export HARNESS_BIN="$(pwd)/target/debug"
```

`/absolute/path/to/selected/bin`은 그대로 복사할 경로가 아니라 실제 선택한 디렉터리로 바꿉니다. `HARNESS_BIN`은 이 예시들을 위한 셸 편의 변수일 뿐입니다. Harness는 이를 런타임 설정이나 호스트 설정으로 읽지 않습니다. 릴리스 빌드와 설치 디렉터리 선택지는 [설치](../getting-started/installation.md)와 [에이전트 호스트 설정](agent-host-setup.md)을 봅니다.

관리 명령은 `"$HARNESS_BIN/harness"`를 사용합니다. 사용자 범위 Codex 설치는 `--mcp-command "$HARNESS_BIN/harness-mcp"`를 전달해 생성 설정이 문자 그대로의 `HARNESS_BIN` 변수가 아니라 해석된 절대 실행 파일 경로를 저장하게 합니다.

## Product Repository A 설치

```sh
"$HARNESS_BIN/harness" agent install \
  --host codex \
  --scope user \
  --server-name harness-main \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --default-project-id acme-api \
  --runtime-home /Users/alex/.harness \
  --mcp-command "$HARNESS_BIN/harness-mcp"
```

이 예시는 호스트 항목이 짧고 예측 가능한 키를 갖도록 `--server-name harness-main`을 고정합니다. 이 옵션은 필수가 아닙니다. 생략하면 `integration_id`에서 안정적인 이름을 파생합니다.

호스트 설정에는 서버 항목 하나가 있습니다.

```toml
[mcp_servers.harness-main]
command = "/absolute/path/to/selected/bin/harness-mcp"
args = ["--integration", "int-codex-team"]

[mcp_servers.harness-main.env]
HARNESS_HOME = "/Users/alex/.harness"
```

실제 생성되는 `command` 값은 `HARNESS_BIN`으로 선택한 경로가 해석된 절대 경로입니다. 생성된 TOML에는 `HARNESS_BIN`이 들어가지 않습니다.

## Product Repository B 추가

```sh
"$HARNESS_BIN/harness" agent project add \
  --integration-id int-codex-team \
  --project-id billing-api \
  --repo-root /work/billing-api \
  --runtime-home /Users/alex/.harness
```

`harness agent project add`는 선택된 Runtime Home에 `billing-api`가 이미 등록되어 있으면 그 등록을 재사용합니다. 아직 등록되어 있지 않다면 필요한 `--repo-root /work/billing-api` 값이 제공되었으므로 이 명령이 프로젝트를 등록한 뒤 통합 멤버십을 추가할 수 있습니다. 이 명령은 호스트 설정을 다시 쓰지 않습니다. 자세한 명령 계약은 [관리 CLI](../reference/admin-cli.md)에 있습니다.

예상 결과:

```text
status: complete
allowed_projects:
  acme-api
  billing-api
verification_detail: project-specific startup preflight passed
```

호스트에 MCP 서버 항목이 여전히 하나인지 확인합니다. Codex 설정에는 이 통합에 대해 `mcp_servers.harness-main`만 있어야 하며, 프로젝트마다 서버 항목이 하나씩 늘어나면 안 됩니다.

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

Status는 `allowed_projects` 아래에 `acme-api`와 `billing-api`를 모두 보여줘야 합니다.

## 에이전트가 해야 할 일

사용자가 어떤 저장소를 사용할 수 있는지 묻는다면, 에이전트는 어댑터 유틸리티를 호출합니다.

```json
{"name":"harness.list_projects","arguments":{}}
```

MCP 결과에는 다음과 비슷한 JSON 객체가 텍스트로 들어 있습니다.

```json
{
  "integration_id": "int-codex-team",
  "default_project_id": "acme-api",
  "projects": [
    {
      "project_id": "acme-api",
      "repo_root": "/work/acme-api",
      "available": true,
      "is_default": true
    },
    {
      "project_id": "billing-api",
      "repo_root": "/work/billing-api",
      "available": true,
      "is_default": false
    }
  ]
}
```

Product Repository A에 대해 에이전트는 공개 메서드 envelope에 `project_id: "acme-api"`를 제공합니다.

```json
{
  "name": "harness.status",
  "arguments": {
    "envelope": {
      "project_id": "acme-api",
      "actor_kind": "agent",
      "request_id": "req_status_acme",
      "idempotency_key": null,
      "expected_state_version": null,
      "dry_run": false,
      "locale": "en-US",
      "task_id": null
    },
    "include": {
      "task": true,
      "pending_user_judgments": true,
      "write_authority": false,
      "evidence": false,
      "close": true,
      "guarantees": true
    }
  }
}
```

Product Repository B에 대한 이후 호출은 명시적 프로젝트 선택자와 request id만 바꿉니다.

```json
{
  "name": "harness.status",
  "arguments": {
    "envelope": {
      "project_id": "billing-api",
      "actor_kind": "agent",
      "request_id": "req_status_billing",
      "idempotency_key": null,
      "expected_state_version": null,
      "dry_run": false,
      "locale": "en-US",
      "task_id": null
    },
    "include": {
      "task": true,
      "pending_user_judgments": true,
      "write_authority": false,
      "evidence": false,
      "close": true,
      "guarantees": true
    }
  }
}
```

에이전트는 폴더 이름, 현재 작업 디렉터리, MCP roots, 호스트 라벨, 기억에서 project ID를 추측하면 안 됩니다. 여러 프로젝트를 사용할 수 있고 명시적 프로젝트나 유효한 기본값이 없으면, 어댑터는 Core 실행 전에 호출을 거부하고 다음과 같은 실행 가능한 텍스트를 반환합니다.

```text
project selection is ambiguous; call harness.list_projects and retry with envelope.project_id
```

## 기본값과 모호성

유효한 명시적 `default_project_id`가 있으면 어댑터가 생략된 `project_id`를 그 기본값으로 보낼 수 있습니다. 기본값은 편의이지 권한이 아닙니다. 기본값은 허용된 프로젝트를 가리켜야 하며, 그 프로젝트가 비활성 또는 실행 불가 상태가 되면 사용할 수 없게 될 수 있습니다.

사용자 요청이 저장소를 이름 붙이면, 에이전트는 그래도 일치하는 `project_id`를 명시적으로 사용해야 합니다. 명시적 프로젝트 선택은 다중 저장소 작업에서 가장 분명하며, 실수로 기본 프로젝트에 대해 작업하는 일을 막습니다.

호스트 설정을 다시 쓰지 않고 기본값을 설정하거나 바꿉니다.

```sh
"$HARNESS_BIN/harness" agent project default set \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

예상 결과:

```text
status: complete
prior_default_project_id: acme-api
resulting_default_project_id: billing-api
```

여러 프로젝트가 남아 있는 동안 기본값을 지우면 생략된 `project_id` 호출은 모호해집니다. 에이전트는 `harness.list_projects`를 호출한 뒤 명시적 `envelope.project_id`로 다시 시도해야 합니다.

## 프로젝트 제거와 재추가

기본값을 `billing-api`로 옮긴 뒤 Product Repository A는 예전에 기본값이던 프로젝트일 뿐입니다. 통합과 호스트 MCP 항목은 유지하면서 제거합니다.

```sh
"$HARNESS_BIN/harness" agent project remove \
  --integration-id int-codex-team \
  --project-id acme-api \
  --runtime-home /Users/alex/.harness
```

예상 결과:

```text
status: complete
allowed_projects:
  billing-api
verification_detail: project membership removed; host configuration was not rewritten
```

마지막으로 남은 프로젝트를 제거하려면, 기본값이 아직 그 프로젝트를 가리킬 때 먼저 기본값을 지운 뒤 멤버십을 제거합니다.

```sh
"$HARNESS_BIN/harness" agent project default clear \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness

"$HARNESS_BIN/harness" agent project remove \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

예상 결과:

```text
status: complete
allowed_project_count: 0
not executable until one is added
```

제거 뒤 Host Installation inventory와 호스트 설정은 남을 수 있지만, 이 저장 상태는 새 시작이 가능하다는 증명이 아닙니다. 이미 실행 중이던 `harness-mcp` 프로세스는 registry 상태를 새로 읽을 수 있으므로 `harness.list_projects`가 `int-codex-team`에 대해 빈 목록을 반환할 수 있습니다. 그래도 허용 프로젝트가 없으므로 프로젝트 라우팅이 필요한 공개 도구는 진행할 수 없습니다. 새로 시작하는 `harness-mcp` 프로세스, `harness-mcp --check`, 새 MCP 시작이 필요한 검증 경로는 프로젝트가 다시 추가되고 일반 설정 점검을 통과하기 전까지 실패합니다.

프로젝트가 없는 상태를 확인합니다.

```sh
"$HARNESS_BIN/harness" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness
```

예상 상태에는 아래 내용이 포함됩니다.

```text
allowed_project_count: 0
not executable
```

호스트 항목을 다시 설치하지 않고 프로젝트를 다시 추가합니다. 이렇게 하면 일반 설정 점검을 전제로 새 시작 자격이 복구됩니다.

```sh
"$HARNESS_BIN/harness" agent project add \
  --integration-id int-codex-team \
  --project-id billing-api \
  --repo-root /work/billing-api \
  --runtime-home /Users/alex/.harness
```

다시 추가한 프로젝트를 편의 기본값으로 삼아야 한다면, 추가한 뒤 기본값으로 설정합니다.

```sh
"$HARNESS_BIN/harness" agent project default set \
  --integration-id int-codex-team \
  --project-id billing-api \
  --runtime-home /Users/alex/.harness
```

## 전체 uninstall

통합에 대해 관리되는 호스트 설정과 관리되는 guidance를 제거합니다.

```sh
"$HARNESS_BIN/harness" agent uninstall \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.harness \
  --allow-repository-write \
  --remove-managed
```

Uninstall은 소유권과 안전 점검이 허용할 때 선택된 하네스 관리 호스트 설정을 제거합니다. `--remove-managed`를 사용하면 선택되어 있고 안전하게 소유된 관리 `Product Repository` guidance도 제거합니다. 성공한 관리 제거는 해당 Host Installation inventory를 제거합니다. Agent Integration Profile에 남은 Host Installation이 없으면 프로필이 비활성화될 수 있으며, 비활성화는 삭제가 아닙니다. `Product Repository` 내용, 프로젝트 등록과 프로젝트 상태, Core의 작업, 증거, 판단, 실행, 아티팩트 관련 기록, 아티팩트 저장소, 관련 없는 호스트 항목은 담당 계약에 따라 보존됩니다.

## 참조 링크

- 정확한 호스트/범위와 명령 동작: [관리 CLI](../reference/admin-cli.md)
- 정확한 Agent Integration Profile과 프로젝트 선택 동작: [에이전트 통합](../reference/agent-integration.md)
- 정확한 `harness.list_projects` 전송 동작: [MCP 전송](../reference/mcp-transport.md)
- 정확한 Product Repository 쓰기 경계: [런타임 경계](../reference/runtime-boundaries.md#explicit-integration-files-in-product-repositories)
