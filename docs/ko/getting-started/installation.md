# 설치

이 튜토리얼은 첫 호스트 설정에 사용할 `Harness Server` 실행 파일을 준비합니다.
실행 파일 출처를 고르고, `harness`와 `harness-mcp`를 확인하며, 선택한 바이너리가
[빠른 시작](quickstart.md)에 사용할 준비가 되었는지 판단하는 절차를 다룹니다.
패키지 관리자 배포, 운영체제 지원, 공개 API 동작, 저장 효과, `Product Repository`
등록, 호스트 신뢰, MCP 와이어 동작은 정의하지 않습니다.

## 독자, 목표, 완료 상태

독자: 에이전트 호스트를 연결하기 전에 동작하는 로컬 `harness` 관리 CLI와
`harness-mcp` MCP 어댑터가 필요한 첫 사용자, 운영자, 구현자입니다.

목표: 소스 빌드 산출물 디렉터리 또는 별도로 설치된 실행 파일 디렉터리 중 하나를
선택하고, 같은 POSIX 스타일 셸에서 두 실행 파일을 실행할 수 있음을 확인합니다.

완료 상태: `HARNESS_BIN`이 실행 가능한 `harness`와 `harness-mcp` 파일이 함께 들어
있는 절대 디렉터리 하나를 가리키고, 아래 버전/도움말 확인이 모두 성공합니다. 이는
실행 파일이 호스트 설정에 사용할 준비가 되었다는 뜻입니다. `Harness Runtime Home`,
`Product Repository`, 호스트 설정이 만들어졌다는 뜻은 아닙니다.

## 전제 조건

경로를 고르기 전에 [시스템 요구사항](../reference/system-requirements.md)을 읽습니다.
이 문서의 명령 예시는 `export`, `$(pwd)`, 따옴표로 감싼 변수 확장, 인라인
`PATH=...`, `test -x`를 사용하는 POSIX 스타일 셸 문법을 전제로 합니다. 사용 중인
셸이 이 문법을 실행할 수 없다면 예시를 직접 옮기고, 옮긴 각 명령을 확인한 뒤
계속합니다.

아래 경로 중 하나를 사용합니다.

| 경로 | 사용할 때 | 계속하기 전에 |
|---|---|---|
| 소스 빌드 | 이 저장소 체크아웃이 있고 현재 워크스페이스 실행 파일을 빌드하려는 경우. | Cargo가 포함된 Rust 1.85 이상을 사용할 수 있고 Cargo가 워크스페이스 의존성을 해석할 수 있습니다. |
| 별도 설치 실행 파일 | 소스 체크아웃과 별개로 `Harness Server` 설치 디렉터리가 이미 있는 경우. | 절대 디렉터리 하나에 `harness`와 `harness-mcp`가 모두 들어 있습니다. |

다음 설정 단계에는 로컬 `Product Repository`, 별도의 `Harness Runtime Home`, Codex나
Claude Code 같은 지원 호스트 경로도 필요합니다.

## 경로 A: 소스에서 빌드

작업 디렉터리: `Harness Server` 소스 저장소 루트.

먼저 변경하지 않는 도구 체인 점검을 실행합니다.

```sh
cargo --version
rustc --version
```

둘 중 하나를 사용할 수 없거나 선택된 Rust 컴파일러가 1.85보다 오래되었다면, 빌드하기
전에 도구 체인을 고칩니다.

디버그 빌드:

```sh
cargo build -p harness-cli -p harness-mcp
export HARNESS_BIN="$(pwd)/target/debug"
```

릴리스 빌드:

```sh
cargo build --release -p harness-cli -p harness-mcp
export HARNESS_BIN="$(pwd)/target/release"
```

나머지 셸 세션에서 사용할 빌드 산출물 하나를 선택합니다. Cargo 패키지 이름은
`harness-cli`와 `harness-mcp`이고, 실행 파일 이름은 `harness`와 `harness-mcp`입니다.

## 경로 B: 설치된 실행 파일 선택

실행 파일이 소스 체크아웃과 별도로 설치되어 있다면 이 경로를 사용합니다.

```sh
export HARNESS_BIN="/absolute/path/to/installed/bin"
```

`/absolute/path/to/installed/bin`은 두 실행 파일이 들어 있는 실제 절대 디렉터리로
바꿉니다. 예시 값을 그대로 복사하지 않습니다.

<a id="verify-the-selected-directory"></a>

## 선택한 디렉터리 확인

`HARNESS_BIN`을 설정한 같은 셸에서 실행합니다.

```sh
test -x "$HARNESS_BIN/harness"
test -x "$HARNESS_BIN/harness-mcp"

"$HARNESS_BIN/harness" --version
"$HARNESS_BIN/harness" agent --help
"$HARNESS_BIN/harness-mcp" --version
"$HARNESS_BIN/harness-mcp" --help
```

버전 명령은 `harness <version>`과 `harness-mcp <version>`을 출력합니다. 도움말 명령은
`harness agent` 명령군과 `harness-mcp --integration <integration_id>` 기반 프로세스
사용법을 보여줘야 합니다.

`HARNESS_BIN`은 이 예시들이 쓰는 셸 편의 변수일 뿐입니다. Harness는 이를 설정으로
읽지 않으며 생성된 호스트 설정에 저장하지 않습니다. 새 셸을 열면 다시 설정하거나
절대 경로를 직접 사용합니다.

## 호스트 설정에서 이 선택을 쓰는 방식

`harness agent install`은 `harness-mcp --integration <integration_id>`를 시작하는 호스트
설정을 설치하거나 내보냅니다.

사용자 범위 Codex 설정이나 사용자/로컬 범위 Claude Code 설정에서는 선택한 절대 실행
파일 경로를 `--mcp-command "$HARNESS_BIN/harness-mcp"`로 전달합니다. 또는 CLI가 찾을 수
있도록 `harness-mcp`를 `harness` 옆이나 `PATH`에 둡니다. 저장되는 호스트 설정에는 셸
변수가 아니라 해석된 절대 명령 경로가 들어갑니다.

프로젝트 범위 Codex 또는 Claude Code 설정에서는 생성되는 프로젝트 파일이 공유 가능해야
합니다. `PATH="$HARNESS_BIN:$PATH"`를 붙여 설정을 실행하고 `--mcp-command harness-mcp`를
사용하거나 `--mcp-command`를 생략합니다. 프로젝트 파일은 이식 가능한 명령 이름을
유지하며, 이후 호스트 프로세스가 자기 `PATH`에서 `harness-mcp`를 찾을 수 있어야 합니다.

설치 위치는 런타임 상태가 아닙니다. `Harness Server` 소스 또는 설치 파일은 실행 파일을
담고, `Harness Runtime Home`은 Harness 런타임 기록을 담으며, `Product Repository`는 제품
파일과 선택한 프로젝트 범위 통합 파일을 담습니다. 에이전트 호스트는 자기 실제 설정과
신뢰 상태를 소유합니다.

## 실패 경로

| 증상 | 안전한 다음 행동 | 경로 |
|---|---|---|
| `cargo` 또는 `rustc`를 사용할 수 없습니다. | Cargo가 포함된 Rust 1.85 이상을 설치하거나 선택한 뒤 사전 점검을 다시 실행합니다. | [시스템 요구사항](../reference/system-requirements.md#toolchain-requirements) |
| Rust가 1.85보다 오래되었습니다. | `cargo build`를 실행하기 전에 Rust 1.85 이상 도구 체인을 선택합니다. | [시스템 요구사항](../reference/system-requirements.md#toolchain-requirements) |
| `cargo build`가 실패합니다. | Cargo 진단을 읽고 보고된 도구 체인, 의존성, 소스 문제를 고친 뒤 같은 빌드 명령을 다시 실행합니다. 첫 대응으로 Runtime Home이나 Product Repository를 삭제하지 않습니다. | [시스템 요구사항](../reference/system-requirements.md#toolchain-requirements) |
| `target/debug` 또는 `target/release`에 두 실행 파일이 모두 없습니다. | 성공한 빌드 명령을 확인하고 그에 맞는 산출물 디렉터리를 선택한 뒤 `test -x` 점검을 다시 실행합니다. | [시스템 요구사항](../reference/system-requirements.md#executable-layout-and-discovery) |
| `test -x` 또는 도움말/버전 명령이 실패합니다. | 실제로 실행 가능한 `harness`와 `harness-mcp`가 들어 있는 디렉터리를 선택하거나 선택된 사용자의 실행 권한을 고칩니다. | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md#missing-harness-mcp) |
| `HARNESS_BIN`이 잘못된 디렉터리를 가리킵니다. | 같은 셸에서 올바른 절대 디렉터리를 내보낸 뒤 모든 확인 명령을 다시 실행합니다. | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md#wrong-absolute-mcp-command) |
| 이후 프로젝트 범위 호스트가 `harness-mcp`를 찾지 못합니다. | 프로젝트 파일은 이식 가능한 형태로 유지하고 호스트 시작 환경의 `PATH`를 고칩니다. | [에이전트 호스트 문제 해결](../guides/agent-host-troubleshooting.md#portable-project-command-not-on-path) |

## 다음 단계

이 문서의 모든 확인 명령이 성공하면 [빠른 시작](quickstart.md)으로 이어집니다.

정확한 명령 동작은 [관리 CLI](../reference/admin-cli.md)가 담당합니다. 정확한
`harness-mcp` 시작, 환경, stdio 전송, 사전 점검, 종료 동작은
[MCP 전송](../reference/mcp-transport.md)이 담당합니다.
