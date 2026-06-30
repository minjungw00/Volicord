# 에이전트 호스트 문제 해결

`volicord setup`, `volicord connect`, `volicord connection ...`,
`volicord export mcp-config`가 호스트 설정 문제를 보고할 때 이 가이드를
사용합니다. 이 가이드는 Volicord가 Product Repository를 감지하고 내부 식별 정보를 관리하는
단순화된 명령 모델을 전제로 합니다.

정확한 setup, doctor, 연결 결과 상태 의미는
[관리 CLI 참조](../reference/admin-cli.md#runtime-home-selection)와
[연결 결과 상태](../reference/admin-cli.md#agent-connection-result-states)가 담당합니다.

## 변경 전에

현재 로컬 상태를 모읍니다.

```sh
volicord doctor
volicord project current
volicord connections
```

명령을 의도한 Product Repository 밖에서 실행하고 있다면 그 저장소로 `cd`하거나,
확인하려는 project, connection, export, user 명령에 `--repo PATH`를 추가합니다.

`volicord setup`과 `volicord doctor`는 서로 다른 상태 질문에 답합니다. setup은
안내형 첫 실행 설정 경험에 사용자 동작이 아직 필요한지를 보고합니다. doctor는 저장된
설치 프로필을 사용할 수 있는지를 보고합니다. 따라서 프로필을 사용할 수 있으면
doctor가 `complete`를 보고하면서도 이후 셸이나 에이전트 호스트를 위한 명령 가용성
경고 또는 권장 `PATH`와 명령 링크 동작을 함께 보여 줄 수 있습니다.

## 설정이 완료되지 않음

관찰 증상: 일반 project, connection, export, MCP, user workflow가 선택된
`Volicord Runtime Home`에 설정이 완료되지 않았다고 말합니다.

제한된 복구:

`volicord`를 이미 사용할 수 있다면:

```sh
volicord setup
volicord doctor
```

`volicord`를 사용할 수 없다면 [설치](../getting-started/installation.md)의 릴리스
바이너리 경로를 다시 실행합니다. 의도적으로 개발용 소스 체크아웃에서 작업 중이라면:

```sh
cargo build --workspace --bins
./target/debug/volicord setup
```

Setup이 `volicord`를 사용할 수 있게 만드는 방법을 묻거나
`action_required`를 보고하면 그 안내를 따릅니다. 셸 명령을 출력했다면 setup을 계속할
터미널에서 그 명령을 실행합니다. 셸 시작 파일을 쓰거나 갱신하라고 했다면 새 셸을
열거나 에이전트 호스트를 restart 또는 reload한 뒤 다시 확인합니다.

```sh
volicord doctor
```

자동화나 특수한 로컬 배치 때문에 결정적인 명령 링크 디렉터리가 필요할 때만
`--link-bin PATH`를 사용합니다. `volicord setup`은 필요한 `PATH` 동작을 보고할 수
있고, 그 디렉터리에 쓸 수 없으면 복구 동작을 보고할 수 있지만 부모 셸을 영구적으로
바꿀 수는 없습니다.

Runtime Home 파일을 직접 만들지 않습니다. Registry와 설치 프로필이 함께 만들어지도록
`volicord setup`을 사용합니다.

## setup이 `~/.local/bin`을 제안하지 않음

관찰 증상: 대화형 setup이 명령을 `PATH`에서 사용할 수 없다고 보고하지만
`~/.local/bin` 만들기를 제안하지 않습니다.

제한된 복구:

Setup은 `HOME` 아래에서 안전한 후보를 식별할 수 있고, 그 디렉터리가 없을 때 안전하게
만들 수 있으며, 명령 링크를 만들기 전에 쓰기 가능 여부를 확인할 수 있을 때만 관례적
사용자 명령 디렉터리를 제안합니다. `HOME`이 없거나 쓸 수 없거나, 셸 또는 플랫폼이
그 안내형 선택을 지원하지 않거나, 후보 경로가 기존의 안전하지 않은 항목과 충돌하거나,
setup이 JSON, 비-TTY, 명시적 `--link-bin` 모드로 실행 중이면 수동 `PATH` 동작으로
남길 수 있습니다.

안전한 다음 단계:

- 대화형 터미널에서 `volicord setup`을 다시 실행하고 프롬프트나 `action_required`
  출력을 따릅니다.
- 사용자가 제어하는 명령 디렉터리로 `volicord setup --link-bin PATH`를 실행합니다.
  Setup은 필요하면 디렉터리를 만들고 쓰기 가능 여부를 확인하며, 이 옵션 자체가 셸
  시작 파일을 편집하지는 않습니다.
- `~/.local/bin`이 사용자가 제어하려는 명령 디렉터리일 때만 직접 만든 뒤 setup을 다시
  실행합니다.

Setup이 셸 명령을 출력하거나 `PATH` 동작을 이름 붙이면 그 명령이 필요한 터미널에서
실행하거나 setup이 이름 붙인 지원 시작 파일을 갱신합니다. Volicord는 명령을 `PATH`에서
사용할 수 있게 도울 수 있지만 현재 부모 셸 환경을 직접 바꿀 수는 없습니다. 이미
실행 중인 에이전트 호스트는 새 명령 디렉터리를 보려면 restart 또는 reload가 필요할 수
있습니다.

## 저장소가 감지되지 않음

관찰 증상: project 또는 connection 명령이 Git 저장소 루트를 찾지 못했다고 말합니다.

제한된 복구:

```sh
cd /path/to/your-product-repo
volicord project current
volicord project use
```

또는 Product Repository를 명시적으로 선택합니다.

```sh
volicord project use /path/to/your-product-repo
volicord connect codex --repo /path/to/your-product-repo
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다. 사용자에게 보이는 프로젝트 이름은 저장소 디렉터리에서 나옵니다. 내부
프로젝트 식별 정보는 복구 입력이 아닙니다.

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
`volicord setup` 재실행이 있습니다. 그런 다음 verification을 다시 실행합니다.

`action_required`를 치명적 실패로 다루지 않습니다. 오래 유지되는 Volicord 쪽 상태가
이미 있을 수 있습니다.

## `failed`

관찰 증상: setup, connect, export, verification이 `failed`를 보고하거나 런타임 오류로
종료합니다.

제한된 복구:

1. `volicord doctor`를 실행합니다.
2. 이 명령이 이름 붙인 첫 실패 setup 또는 실행 파일 점검을 고칩니다.
3. 원래 명령이 지원한다면 `--dry-run`으로 다시 실행합니다.
4. Dry-run 계획이 기대한 호스트와 Product Repository를 이름 붙인 뒤에만 실제 명령을 다시
   실행합니다.

정확한 실패 문구를 사용해 다음 동작을 고릅니다. 담당 문서나 인간 운영자가 의도한
복구라고 식별하지 않은 한 Runtime Home 상태나 호스트 설정을 직접 삭제하지 않습니다.

## MCP 명령을 사용할 수 없음

관찰 증상: setup 또는 verification이 `volicord mcp --stdio`를 찾거나 시작하거나 초기화할 수
없다고 보고합니다.

제한된 복구:

설치된 릴리스 바이너리로 setup을 다시 실행합니다.

```sh
volicord setup
```

의도적으로 개발용 소스 체크아웃에서 작업 중이라면:

```sh
cargo build --workspace --bins
./target/debug/volicord setup
```

Setup 프롬프트나 `action_required`가 이름 붙인 명령 가용성 단계를 완료한 뒤 설치와
연결을 다시 확인합니다.

```sh
volicord doctor
volicord connection verify codex
```

`volicord setup`은 관리 호스트 설정과 generic export가 사용할 MCP 명령을 기록하는
위치입니다. 일반 `connect` 명령은 사용자가 MCP 명령 경로를 전달하도록 요구하지
않습니다. 실행 파일이 sibling 조회나 `PATH`로 찾을 수 없는 위치에 설치되어 있다면
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
보고하거나, 다른 Product Repository에 대한 연결이 계속 보입니다.

제한된 복구:

```sh
volicord connection remove codex --dry-run
volicord connection status codex
volicord connections
```

제거는 먼저 선택된 Product Repository 멤버십을 제거합니다. 소유 멤버십이 남지 않고 안전 점검이
허용할 때만 Agent Connection과 관리 호스트 설정을 제거합니다. `Product Repository`,
프로젝트 상태, Core 기록, 아티팩트 저장소, 관련 없는 호스트 항목을 제거하면 안
됩니다.

## 보안 경계

Volicord setup과 verification은 로컬 진단입니다. 외부 호스트가 안전하다거나, 모델이
Volicord 도구를 사용할 것이라거나, 파일 쓰기가 안전하다는 증명이 아닙니다. 정확한
보안 표현은 [보안](../reference/security.md)을 사용합니다.
