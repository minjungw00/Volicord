# 설치

이 문서는 첫 설정 단계, 즉 `Harness Server` 실행 파일 준비를 담당합니다. 현재 저장소 실행 파일의 소스 전제 조건, 빌드 명령, 실행 파일 경로, 빌드 확인을 다룹니다. 패키지 관리자 배포, 운영체제 지원, 공개 API 동작, 저장 효과, `Product Repository` 등록, 호스트 신뢰, MCP 와이어 동작은 정의하지 않습니다.

## 전제 조건

설치 전 환경 분류의 담당 문서는 [시스템 요구사항](../reference/system-requirements.md)입니다.

소스 빌드 경로에는 아래가 필요합니다.

- 이 저장소의 로컬 복제본
- Cargo가 포함된 Rust 1.85 이상. Rust 1.85는 현재 워크스페이스에 대해 검증된 최소 컴파일러 버전입니다.
- Cargo와 로컬 실행 파일을 실행할 수 있는 셸

다음 설정 단계에는 아래도 필요합니다.

- 로컬 `Product Repository` 디렉터리
- 별도의 `Harness Runtime Home`
- 에이전트 호스트에 연결할 때 사용할 Codex, Claude Code, 또는 다른 MCP 호스트

## 저장소 루트에서 빌드

작업 디렉터리: `Harness Server` 소스 저장소 루트.

디버그 소스 빌드:

```sh
cargo build -p harness-cli -p harness-mcp
export HARNESS_BIN="$(pwd)/target/debug"

test -x "$HARNESS_BIN/harness"
test -x "$HARNESS_BIN/harness-mcp"
```

릴리스 소스 빌드:

```sh
cargo build --release -p harness-cli -p harness-mcp
export HARNESS_BIN="$(pwd)/target/release"
```

별도로 설치된 실행 파일을 사용할 때:

```sh
export HARNESS_BIN="/absolute/path/to/installed/bin"
```

`harness`와 `harness-mcp`가 함께 들어 있는 절대 디렉터리 하나를 선택합니다. `HARNESS_BIN`은 이 예시들이 쓰는 셸 편의 변수입니다. Harness가 설정으로 읽는 값이 아닙니다. Cargo 패키지 이름은 `harness-cli`와 `harness-mcp`입니다. 실행 파일 이름은 `harness`와 `harness-mcp`입니다.

## 빌드 확인

`HARNESS_BIN`을 선택한 뒤 같은 셸에서 실행 파일을 확인합니다.

```sh
"$HARNESS_BIN/harness" --version
"$HARNESS_BIN/harness" agent --help
"$HARNESS_BIN/harness-mcp" --version
"$HARNESS_BIN/harness-mcp" --help
```

버전 명령은 `harness <version>`과 `harness-mcp <version>`을 출력합니다. 도움말 명령은 `harness agent` 명령군과 `harness-mcp --integration <integration_id>` 기반 프로세스 사용법을 출력해야 합니다.

## 설정 중 실행 파일 찾기

`harness agent install`은 `harness-mcp --integration <integration_id>`를 시작하는 호스트 설정을 설치하거나 내보냅니다.

사용자 범위 Codex 설정이나 사용자/로컬 범위 Claude Code 설정에서는 선택한 절대 실행 파일 경로를 `--mcp-command "$HARNESS_BIN/harness-mcp"`로 전달합니다. 또는 CLI가 찾을 수 있도록 `harness-mcp`를 `harness` 옆이나 `PATH`에 둡니다. 저장되는 호스트 설정에는 셸 변수가 아니라 해석된 절대 명령 경로가 들어갑니다.

프로젝트 범위 Codex 또는 Claude Code 설정에서는 생성되는 프로젝트 파일이 공유 가능해야 합니다. `PATH="$HARNESS_BIN:$PATH"`를 붙여 설정을 실행하고 `--mcp-command harness-mcp`를 사용하거나 `--mcp-command`를 생략합니다. 프로젝트 파일은 이식 가능한 명령 이름을 유지하며, 호스트 환경의 `PATH`에서 `harness-mcp`를 찾을 수 있어야 합니다.

설치 위치는 런타임 상태가 아닙니다. `Harness Server` 소스 또는 설치 파일은 실행 파일을 담고, `Harness Runtime Home`은 하네스 런타임 기록을 담으며, `Product Repository`는 제품 파일과 선택한 프로젝트 범위 통합 파일을 담습니다. 에이전트 호스트는 자기 실제 설정과 신뢰 상태를 소유합니다.

## 다음 단계

[빠른 시작](quickstart.md)으로 이어집니다. 빠른 시작은 Codex 또는 Claude Code의 실제 지원 호스트 경로에서 시작합니다.

정확한 명령 동작은 [관리 CLI](../reference/admin-cli.md)가 담당합니다. 정확한 `harness-mcp` 시작, 환경, stdio 전송, 사전 점검, 종료 동작은 [MCP 전송](../reference/mcp-transport.md)이 담당합니다.
