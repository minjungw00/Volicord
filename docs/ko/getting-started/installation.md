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

그다음 설치 프로필을 만듭니다.

```sh
export PATH="$PWD/target/debug:$PATH"
volicord setup
```

`export PATH=...` 줄은 현재 터미널 세션에만 영향을 줍니다. 이 줄은 그 셸에서 방금
빌드한 `volicord`와 `volicord-mcp` 명령을 찾을 수 있게 합니다.
`volicord setup`은 선택된 `Volicord Runtime Home`을 만들거나 검증하고 설치
프로필을 저장합니다. 정확한 `volicord setup` 옵션, MCP 명령 찾기 순서, 출력 동작은
[관리 CLI 참조](../reference/admin-cli.md#runtime-home-selection)가 담당합니다.

이 소스 빌드에서 지속적인 명령 링크를 원한다면 `--link-bin`과 함께 setup을 실행합니다.

```sh
volicord setup --link-bin ~/.local/bin
```

`--link-bin`이 제공되면 setup은 가능할 때 그 디렉터리에 `volicord`와
`volicord-mcp` 명령을 모두 준비합니다. CLI는 필요한 `PATH` 동작을 보고할 수는
있지만 부모 셸 환경을 영구적으로 수정할 수 없습니다. `~/.local/bin`이 아직 셸
설정에 없다면 추가한 뒤, 그 환경에서 새 셸이나 MCP 호스트를 시작합니다.

설정 준비 상태를 확인합니다.

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
사용합니다. 일반 `volicord connect` 명령은 저장된 설치 프로필을 사용합니다.

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
