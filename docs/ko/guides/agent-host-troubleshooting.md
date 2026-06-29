# 에이전트 호스트 문제 해결

`volicord setup`, `volicord connect`, `volicord connection ...`,
`volicord export mcp-config`가 호스트 설정 문제를 보고할 때 이 가이드를
사용합니다. 이 가이드는 Volicord가 저장소 프로젝트를 감지하고 내부 ID를 관리하는
단순화된 명령 모델을 전제로 합니다.

정확한 결과 상태 의미는
[관리 CLI 참조](../reference/admin-cli.md#agent-connection-result-states)가 담당합니다.

## 변경 전에

현재 로컬 상태를 모읍니다.

```sh
volicord doctor
volicord project current
volicord connections
```

명령을 의도한 저장소 밖에서 실행하고 있다면 그 저장소로 `cd`하거나, 확인하려는
project, connection, export, user 명령에 `--repo PATH`를 추가합니다.

## Setup이 완료되지 않음

관찰 증상: 일반 project, connection, export, MCP, user workflow가 선택된
`Volicord Runtime Home`에 setup이 완료되지 않았다고 말합니다.

제한된 복구:

```sh
volicord setup
volicord doctor
```

소스에서 빌드했고 명령 링크가 필요하다면:

```sh
./target/debug/volicord setup --link-bin ~/.local/bin
volicord doctor
```

그래도 `volicord`를 찾지 못하면 링크 디렉터리를 셸 설정에 추가하고 새 셸이나 MCP
호스트를 시작합니다. Setup은 필요한 `PATH` 동작을 보고할 수 있지만 부모 셸을
영구적으로 바꿀 수는 없습니다.

Runtime Home 파일을 직접 만들지 않습니다. Registry와 setup 프로필이 함께 만들어지도록
setup을 사용합니다.

## 저장소가 감지되지 않음

관찰 증상: project 또는 connection 명령이 Git 저장소 루트를 찾지 못했다고 말합니다.

제한된 복구:

```sh
cd /work/acme-api
volicord project current
volicord project use
```

또는 저장소를 명시적으로 선택합니다.

```sh
volicord project use /work/acme-api
volicord connect codex --repo /work/acme-api
```

사용자에게 보이는 프로젝트 이름은 저장소 디렉터리에서 나옵니다. 내부 프로젝트
ID는 복구 입력이 아닙니다.

## 호스트를 선택할 수 없음

관찰 증상: `volicord connect` 또는 `volicord connection ...`이 호스트를 추론하지
못하거나 호스트 값이 지원되지 않습니다.

제한된 복구: 호스트를 명시적으로 전달합니다.

```sh
volicord connect codex
volicord connect claude-code
volicord connection status codex
```

연결에 사용한 의도 선택자도 함께 사용합니다.

```sh
volicord connection status codex --shared
volicord connection verify claude-code --global
```

Codex는 personal과 shared 연결 의도를 지원합니다. Claude Code는 personal, shared,
global 연결 의도를 지원합니다.

## `action_required`

관찰 증상: connection status 또는 verification이 `status: action_required`를
보고합니다.

제한된 복구:

```sh
volicord connection status codex
volicord connection verify codex
```

보고된 동작을 읽고 그 호스트 소유 단계만 완료합니다. 흔한 동작에는 호스트 항목
신뢰, 프로젝트 MCP 항목 승인, 호스트 로그인, 호스트 reload, 호스트 restart,
setup 재실행이 있습니다. 그런 다음 verification을 다시 실행합니다.

`action_required`를 치명적 실패로 다루지 않습니다. 오래 유지되는 Volicord 쪽 상태가
이미 있을 수 있습니다.

## `failed`

관찰 증상: setup, connect, export, verification이 `failed`를 보고하거나 런타임 오류로
종료합니다.

제한된 복구:

1. `volicord doctor`를 실행합니다.
2. 이 명령이 이름 붙인 첫 실패 setup 또는 실행 파일 점검을 고칩니다.
3. 원래 명령이 지원한다면 `--dry-run`으로 다시 실행합니다.
4. Dry-run 계획이 기대한 호스트와 저장소를 이름 붙인 뒤에만 실제 명령을 다시
   실행합니다.

정확한 실패 문구를 사용해 다음 동작을 고릅니다. 담당 문서나 인간 운영자가 의도한
복구라고 식별하지 않은 한 Runtime Home 상태나 호스트 설정을 직접 삭제하지 않습니다.

## MCP 명령을 사용할 수 없음

관찰 증상: setup 또는 verification이 `volicord-mcp`를 찾거나 시작하거나 초기화할 수
없다고 보고합니다.

제한된 복구:

```sh
cargo build --workspace --bins
./target/debug/volicord setup --link-bin ~/.local/bin
volicord doctor
volicord connection verify codex
```

Setup은 관리 호스트 설정과 generic export가 사용할 MCP 명령을 기록하는 위치입니다.
일반 `connect` 명령은 사용자가 MCP 명령 경로를 전달하도록 요구하지 않습니다.
실행 파일이 sibling 조회나 `PATH`로 찾을 수 없는 위치에 설치되어 있다면
`--mcp-command PATH`로 setup을 다시 실행합니다.

## Shared 연결에 호스트 승인이 필요함

관찰 증상: shared 연결이 프로젝트 통합 파일을 쓰거나 갱신했지만 호스트가 여전히
Volicord 도구를 로드하지 않습니다.

제한된 복구:

```sh
volicord connection status codex --shared
volicord connection verify codex --shared
```

명령이 이름 붙인 호스트 소유 프로젝트 승인 또는 reload 동작을 완료합니다.
`Product Repository` 통합 파일은 Core 권한이 아니며, 호스트가 MCP 서버를 로드,
신뢰, 노출했다는 증거도 아닙니다.

## Generic Export가 호스트에 나타나지 않음

관찰 증상: `volicord export mcp-config`가 파일을 만들었지만 외부 호스트에 Volicord
도구가 보이지 않습니다.

제한된 복구:

```sh
volicord export mcp-config --output /tmp/volicord.mcp.json
volicord doctor
```

그다음 외부 호스트의 자체 설정 절차로 export 파일을 로드하거나 reload합니다. 내보낸
파일은 export 뒤에도 사용자 관리 파일입니다.

## 제거가 일부만 완료됨

관찰 증상: `volicord connection remove ...`가 호스트 설정을 제거하지 못했다고
보고하거나, 다른 저장소에 대한 연결이 계속 보입니다.

제한된 복구:

```sh
volicord connection remove codex --dry-run
volicord connection status codex
volicord connections
```

제거는 먼저 선택된 저장소 멤버십을 제거합니다. 소유 멤버십이 남지 않고 안전 점검이
허용할 때만 Agent Connection과 관리 호스트 설정을 제거합니다. `Product Repository`,
프로젝트 상태, Core 기록, 아티팩트 저장소, 관련 없는 호스트 항목을 제거하면 안
됩니다.

## 보안 경계

Volicord setup과 verification은 로컬 진단입니다. 외부 호스트가 안전하다거나, 모델이
Volicord 도구를 사용할 것이라거나, 파일 쓰기가 안전하다는 증명이 아닙니다. 정확한
보안 표현은 [보안](../reference/security.md)을 사용합니다.
