# 설치

이 튜토리얼은 로컬 `volicord`와 `volicord-mcp` 실행 파일을 준비하고, 이후
프로젝트, 연결, 내보내기, `User Channel` 명령이 사용할 설치 프로필을 기록합니다.
[빠른 시작](quickstart.md) 전에 수행하는 설정 단계입니다.

정확한 명령 동작은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
런타임 위치와 저장소 분리는 [런타임 경계](../reference/runtime-boundaries.md)가
담당합니다.

## 전제 조건

- [시스템 요구사항](../reference/system-requirements.md)에 적힌 Rust 1.85 이상.
- Cargo와 로컬 바이너리를 실행할 수 있는 셸.
- 호스트를 연결할 준비가 되었을 때 사용할 Git 기반 제품 저장소.

## 소스에서 빌드하기

Volicord 소스 저장소에서 실행합니다.

```sh
cargo build --workspace --bins
```

이 명령은 두 로컬 실행 파일을 빌드합니다.

- `./target/debug/volicord`
- `./target/debug/volicord-mcp`

그다음 방금 빌드한 CLI에서 안내형 setup을 실행합니다.

```sh
./target/debug/volicord setup
```

`volicord setup`은 선택된 `Volicord Runtime Home`을 만들거나 검증하고 설치
프로필을 저장합니다. 실행 중인 `volicord` 실행 파일을 발견하고 `volicord-mcp`를
찾으며, 이후 터미널과 에이전트 호스트에서 선택된 명령을 `PATH`로 사용할 수 있는지
확인합니다. 정확한 `volicord setup` 옵션, MCP 명령 찾기 순서, 출력 동작은 [관리 CLI
참조](../reference/admin-cli.md#runtime-home-selection)가 담당합니다.

대화형 터미널에서 선택된 실행 파일이 `PATH`에 준비되어 있지 않으면 setup은 명령
가용성 선택지를 제시할 수 있습니다.

- 제안된 디렉터리에 명령 링크를 만듭니다.
- 명령 링크를 만들고, 명시적 승인을 받은 뒤 지원되는 셸 시작 파일에 관리되는
  `PATH` 블록을 추가합니다.
- 명령 링크를 만들고 사용자가 직접 실행할 셸 명령을 출력합니다.
- 수동 `PATH` 복구용 셸 명령을 출력합니다.
- 지금은 명령 링크를 건너뜁니다.

셸 시작 파일 변경은 암시적으로 이루어지지 않습니다. Setup이 지원되는 셸 시작 파일을
식별할 수 있으면 대상 파일과 관리되는 블록을 보여 주고 쓰기 전에 승인을 요청합니다.
관리되는 블록은 Volicord가 소유하는 블록이며 관련 없는 셸 설정을 다시 쓰지
않습니다. 지원되지 않는 셸이나 플랫폼에서는 수동 동작이 필요합니다.

Setup은 부모 셸의 현재 `PATH`를 바꿀 수 없습니다. 출력된 `export PATH=...` 명령은
그 명령을 실행한 터미널에만 영향을 줍니다. Setup이 셸 시작 파일을 쓰거나 갱신하라고
요청했다면, 새 셸을 열거나 기존 에이전트 호스트 프로세스를 restart 또는 reload한
뒤 명령을 찾을 수 있다고 기대해야 합니다.

자동화나 결정적인 로컬 배치가 필요할 때는 명시적 setup 옵션을 사용합니다.

| 옵션 | 사용할 때 |
|---|---|
| `--link-bin PATH` | 특정 디렉터리에 명령 링크를 만들거나 갱신합니다. 이 옵션 자체가 셸 시작 파일을 편집하지는 않습니다. |
| `--mcp-command PATH` | sibling 찾기나 `PATH` 조회가 잘못된 명령을 고르거나 명령을 찾지 못할 때 특정 `volicord-mcp` 실행 파일을 저장합니다. |
| `--home PATH` | 기본값이 아닌 `Volicord Runtime Home`을 선택합니다. |

예를 들어 비대화식 링크 단계에서 링크 디렉터리를 지정할 수 있습니다.

```sh
./target/debug/volicord setup --link-bin ~/.local/bin
```

프롬프트나 `action_required`가 이름 붙인 명령 가용성 단계를 완료한 뒤 설정 준비
상태를 확인합니다.

```sh
volicord doctor
```

설정을 사용할 수 있으면 `doctor`가 `complete`를 보고합니다. `action_required`는
`volicord setup` 재실행이나 실행 파일 경로 수정처럼 구체적인 로컬 복구 동작을 이름 붙입니다.

## 설치된 실행 파일 사용하기

`volicord`와 `volicord-mcp`가 이미 `PATH`에 있다면 아래처럼 실행합니다.

```sh
volicord setup
volicord doctor
```

실행 파일을 소스에서 빌드했든 설치된 명령 디렉터리에서 가져왔든 setup은 같은 설치
프로필 계약을 사용합니다. CLI 참조가 설명하는 기본 찾기 방식이 사용하려는
`volicord-mcp` 실행 파일을 찾을 수 없을 때만 `volicord setup --mcp-command PATH`를
사용합니다. Setup이 `action_required`를 보고하면 새 터미널이나 에이전트 호스트를
시작하기 전에 이름 붙은 로컬 동작을 완료합니다. 일반 `volicord connect` 명령은
저장된 설치 프로필을 사용합니다.

## 설정이 하지 않는 일

`volicord setup`은 제품 저장소를 등록하지 않고 호스트 설정을 설치하지도 않습니다. 프로젝트
등록은 Git 저장소 안에서 `volicord project use`나 `volicord connect` 같은 명령을
실행할 때 이루어집니다.

프로젝트 이름과 내부 식별 정보 동작은 [관리 CLI
참조](../reference/admin-cli.md#project-commands)가 담당합니다. 내부 식별 정보는
Volicord가 저장하며 첫 설정 입력이 아닙니다.

## 다음 단계

제품 저장소로 이동해 호스트를 연결합니다.

```sh
cd /path/to/your-product-repo
volicord connect codex
```

`/path/to/your-product-repo`는 호스트가 작업할 Git 제품 저장소를 가리키는 자리표시자입니다.

전체 첫 실행 경로는 [빠른 시작](quickstart.md)을 계속 읽습니다. 호스트별
세부사항은 [에이전트 호스트 설정](../guides/agent-host-setup.md)을 봅니다.
