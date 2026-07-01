# 빠른 시작

이 튜토리얼은 [설치](installation.md)를 통해 `volicord`가 `PATH`에서 사용 가능해진
뒤 작동하는 Agent Connection 하나까지 가는 경로입니다. 에이전트에게 작업을 요청할
Product Repository로 사용할 일반 Git 저장소에 로컬 호스트를 연결한다고 가정합니다.

정확한 명령 계약은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
Agent Connection 의미는 [Agent Connection 참조](../reference/agent-connection.md)가
담당합니다.

## 빠른 경로

```sh
volicord init --host codex --repo /path/to/your-product-repo --mode mcp-only
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다. `volicord init`은 첫 실행에서 저장소를 설정하고 호스트를 연결하는 기본
명령입니다. 필요하면 Runtime Home과 설치 프로필을 만들거나 재사용하고, 선택한
저장소를 등록하며, 선택한 호스트의 프로젝트 범위 MCP 설정을 설치하고, Volicord가
관리하는 지침과 policy 메타데이터를 쓰고, guard 설치 상태를 기록합니다. 생성된 호스트
설정은 단일 공개 실행 파일을 `volicord mcp --stdio`로 시작합니다.

이 빠른 경로는 호스트 lifecycle hook 설치를 요구하지 않는 `--mode mcp-only`를
사용합니다. 기본 `guarded` 또는 `managed` init은 모든 필수 호스트 hook phase에 대한
검증된 지원이 필요합니다. 지원이 빠져 있으면 degraded guard 파일과 누락 hook 진단을
명시적으로 원할 때만 `--allow-degraded`를 사용합니다. 정확한 프로젝트 이름, guard
mode 동작, 연결 기본값, 내부 식별 정보 동작은 [관리 CLI 참조](../reference/admin-cli.md)가
담당합니다.

## 설정 확인하기

```sh
volicord doctor
volicord project current
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
```

완료 상태: status나 verification이 `complete`를 보고하면 연결 준비가 끝난 것입니다.
`action_required`를 보고하면 이름 붙은 호스트 소유 동작이나 로컬 복구 동작을 완료한
뒤 verification을 다시 실행합니다. 정확한 결과 상태 의미는 [관리 CLI
참조](../reference/admin-cli.md#agent-connection-result-states)가 담당합니다.

## 호스트 의도 선택하기

personal, global, read-only 변형을 직접 써야 할 때만 낮은 수준의
`volicord connect` 명령을 사용합니다. 일반 `init` 흐름 없이 `volicord connect`로
프로젝트 공유 통합 파일을 관리할 때만 `--shared`를 추가하고, `--global`은 사용자
전체 설정을 지원하는 호스트 경로에만 사용합니다. 정확한 의도 의미는
[관리 CLI 참조](../reference/admin-cli.md#connection-intents-and-hosts)가 담당하고,
호스트 가용성 요구사항은 [시스템
요구사항](../reference/system-requirements.md#host-configuration-requirements)이 담당합니다.

읽기 중심 동작만 노출해야 할 때만 `--read-only`를 사용합니다.

```sh
volicord connect codex --read-only
```

낮은 수준의 연결 관리에서는 현재 디렉터리가 연결 대상 Product Repository가 아닐 때
`--repo PATH`를 사용합니다.

```sh
volicord connect codex --repo /path/to/your-product-repo
```

`volicord connect`는 personal, shared, global, read-only 변형을 위한 낮은 수준의
연결 관리 명령으로 계속 지원됩니다. 일반적인 첫 실행 경로에서는
`volicord init --host HOST --repo PATH --mode mcp-only`를 우선 사용합니다.

## 연결 조회 또는 변경하기

```sh
volicord connections
volicord connection status codex --repo /path/to/your-product-repo
volicord connection verify codex --repo /path/to/your-product-repo
volicord connection mode codex read-only
volicord connection mode codex workflow
```

선택한 Product Repository를 연결에서 제거할 때도 같은 호스트와 의도 선택을 사용합니다.

```sh
volicord connection remove codex --dry-run
volicord connection remove codex
```

`--dry-run`은 지속 변경 없이 계획을 보고합니다.

## Generic MCP 설정 내보내기

Volicord가 직접 관리하지 않는 MCP 호스트에는 호스트 중립 설정을 내보냅니다.

```sh
volicord export mcp-config --output /tmp/volicord.mcp.json
```

내보내기는 감지된 Product Repository와 설치 프로필을 사용합니다. 정확한 출력 기본값은
[관리 CLI 참조](../reference/admin-cli.md#generic-mcp-config-export)가 담당합니다. 내보낸
파일은 내보내기 뒤에도 사용자가 관리합니다. Volicord는 임의 외부 호스트가 이 파일을
로드하거나 승인했다고 주장하지 않습니다.

## 사용자 판단 기록하기

Agent Connection은 초점이 맞춰진 판단 필요를 요청하거나 보여 줄 수 있지만,
권한을 지니는 사용자 답변은 로컬 `User Channel`을 거칩니다.

호스트와 클라이언트가 지원하면 MCP 어댑터는 대기 판단에 MCP elicitation을 사용할 수
있습니다. guarded prompt-capture hook이 설정되어 있으면 일반 채팅 경로는
`Volicord: answer J-3 1 #AB7K` 같은 엄격한 prompt 명령입니다. elicitation이나 prompt
capture를 사용할 수 없을 때는 아래 터미널 명령을 안정적인 복구 경로로 사용합니다.

```sh
volicord user status
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

현재 Product Repository와 다른 Product Repository에 답해야 할 때만 `--repo PATH`를
사용합니다. 활성 작업이 의도한 작업이 아닐 때는 `--task ID`를 사용합니다.

## 다음 단계

| 필요 | 읽을 문서 |
|---|---|
| 호스트 설정 세부사항 | [에이전트 호스트 설정](../guides/agent-host-setup.md) |
| `action_required` 또는 `failed` 문제 해결 | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md) |
| 사용자 작업 흐름과 판단 경계 | [사용자 가이드](../guides/user-workflow.md) |
| 에이전트 작업 흐름 경계 | [에이전트 가이드](../guides/agent-workflow.md) |
