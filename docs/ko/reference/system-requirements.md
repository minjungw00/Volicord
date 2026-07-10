# 시스템 요구사항

이 문서는 Volicord 실행 파일을 설치하거나 MCP 호스트를 연결하기 전에 독자가 확인해야 하는 환경 적용 가능성과 전제 조건을 담당합니다. 이 저장소에서 확인할 수 있는 증거를 기준으로 운영 환경, 셸, 도구 체인, 실행 파일 배치, 파일시스템 접근, Runtime Home, Product Repository, MCP 호스트 전제 조건을 분류합니다.

이 문서는 관리 명령 동작, MCP stdio 동작, 저장 효과, 호스트 신뢰, 공개 API 동작, 스키마, 보안 보장을 정의하지 않습니다. 정확한 동작은 [관리 CLI](admin-cli.md), [MCP 전송](mcp-transport.md), [런타임 경계](runtime-boundaries.md), [Agent Connection](agent-connection.md)이 계속 담당합니다.

## 상태 용어

| 상태 | 이 문서에서의 의미 |
|---|---|
| 지원됨 | 관련 담당 문서가 기준 경로를 문서화했고 설치 전에 확인할 수 있습니다. 지원은 명시된 요구사항으로 제한되며, 이 문서가 따로 말하지 않는 한 운영체제 지원 약속이 아닙니다. |
| 검증됨 | 워크스페이스 메타데이터, 유지되는 예시, 소스 점검, 테스트, 저장소에 포함된 검증 도구처럼 해당 문장을 뒷받침하는 직접 증거가 있습니다. |
| 미검증 | 동작할 수는 있지만, 이 저장소에는 지원 또는 검증 상태로 문서화할 만큼 충분한 증거가 없습니다. |
| 지원 범위 밖 | 유지되는 기준 범위에 포함되지 않거나, 담당 문서가 명시적으로 거부하거나, 이 저장소에 없는 절차 설명이 필요합니다. |

Rust 이식성만으로 지원을 추론하지 마세요. 어떤 Rust 크레이트가 원칙적으로 이식 가능하다는 사실은 이 저장소가 특정 운영체제, 셸, 패키지 관리자, 컨테이너 이미지, 원격 호스트, 에이전트 호스트 버전을 검증한다는 증거가 아닙니다.

## 적용 가능성 표

| 영역 | 상태 | 저장소 증거 | 계속하기 전에 |
|---|---|---|---|
| 릴리스 바이너리 패키징과 설치 | **지원됨, 검증됨.** 이 상태는 표에 나온 대상 트리플에 적용됩니다. 실제 설치에는 서로 맞는 게시 자산 세트도 필요합니다. | `.github/workflows/release.yml`은 대상 이름의 릴리스 압축 파일을 빌드하고, 각 바이너리에 스모크 테스트를 실행하며, `.sha256` 파일을 생성합니다. POSIX 대상은 `volicord` 하나만 담은 `.tar.gz`이고, 네이티브 Windows 대상은 `volicord.exe` 하나만 담은 `.zip`입니다. 내려받은 `install.sh`와 `install.ps1`이 이 대상 이름을 선택합니다. | 선택한 릴리스 저장소, 태그, 미러가 설치 스크립트, 대상 압축 파일, 체크섬을 제공하는지 확인합니다. 그렇지 않으면 소스 빌드, 로컬 Docker 빌드, 기존에 설치한 실행 파일을 사용합니다. |
| Linux x86_64 | 릴리스 대상 `x86_64-unknown-linux-gnu`로 **지원되고 검증되었습니다.** | 릴리스 워크플로는 `ubuntu-24.04`에서 빌드하고 `volicord-x86_64-unknown-linux-gnu.tar.gz`를 패키징합니다. | Linux x86_64 환경에서 POSIX 스타일 셸과 아래 설치 스크립트 도구를 사용합니다. |
| Linux aarch64 | 릴리스 대상 `aarch64-unknown-linux-gnu`로 **지원되고 검증되었습니다.** | 릴리스 워크플로는 네이티브 `ubuntu-24.04-arm` 실행 환경에서 빌드하고 `volicord-aarch64-unknown-linux-gnu.tar.gz`를 패키징합니다. | Linux aarch64 환경에서 POSIX 스타일 셸과 아래 설치 스크립트 도구를 사용합니다. |
| WSL2 | `uname`이 `Linux`를 보고하고 아키텍처가 `x86_64` 또는 `aarch64`이면 Linux 환경으로 **지원됩니다.** | POSIX 설치 스크립트는 관찰되는 플랫폼이 Linux 사용자 공간이므로 WSL2를 Linux로 처리합니다. 네이티브 Windows는 별도 PowerShell 설치 스크립트와 Windows 대상을 사용합니다. | WSL2와 대응 Linux 아키텍처를 사용합니다. WSL 경로를 네이티브 Windows Volicord 프로세스에 전달하지 않습니다. |
| macOS arm64 | 릴리스 대상 `aarch64-apple-darwin`으로 **지원되고 검증되었습니다.** | 릴리스 워크플로는 macOS arm64 실행 환경에서 빌드하고 `volicord-aarch64-apple-darwin.tar.gz`를 패키징합니다. | macOS arm64 환경에서 POSIX 스타일 셸과 아래 설치 스크립트 도구를 사용합니다. |
| macOS x86_64 | 릴리스 대상 `x86_64-apple-darwin`으로 **지원되고 검증되었습니다.** | 릴리스 워크플로는 macOS Intel 실행 환경에서 빌드하고 `volicord-x86_64-apple-darwin.tar.gz`를 패키징합니다. | macOS x86_64 환경에서 POSIX 스타일 셸과 아래 설치 스크립트 도구를 사용합니다. |
| Docker | 체크인된 `Dockerfile`을 쓰는 로컬 런타임 선택지로 **지원되고 검증되었습니다.** 외부 이미지 레지스트리는 주장하지 않습니다. | `Dockerfile`은 릴리스 CLI를 Debian 런타임 이미지에 빌드합니다. 릴리스 워크플로는 이미지를 빌드하고 `volicord --help`와 `volicord serve --help` 스모크 테스트를 실행합니다. 설치 문서는 로컬 `docker build`와 호스트 루프백 `docker run` 사용을 설명합니다. | 이 저장소 또는 신뢰하는 소스 사본에서 이미지를 빌드합니다. 유지되는 기준 범위에는 레지스트리 이미지가 없습니다. |
| 네이티브 Windows x86_64 Record profile | `record` 프로필과 릴리스 대상 `x86_64-pc-windows-msvc`로 **지원되고 검증되었습니다.** | 릴리스 워크플로는 `windows-2022`에서 빌드하고, `target/x86_64-pc-windows-msvc/release/volicord.exe`를 스모크 테스트하며, `volicord-x86_64-pc-windows-msvc.zip`과 `.sha256`을 만듭니다. 네이티브 Windows에서 `cargo test --workspace --all-targets --all-features`도 실행합니다. 내려받은 `install.ps1`은 기본적으로 사용자 로컬 디렉터리에 해당 바이너리를 설치합니다. | 네이티브 Windows x86_64에서 PowerShell을 사용합니다. `volicord init --host HOST --repo PATH --profile record`를 사용합니다. |
| 네이티브 Windows Detective profile | **지원 범위 밖입니다.** | Detective 설정은 현재 검증된 어댑터에 POSIX `sh` 훅 래퍼를 씁니다. CLI는 네이티브 Windows의 `volicord init --profile detective`를 `DETECTIVE_WINDOWS_UNSUPPORTED`로 거부합니다. | 네이티브 Windows에서는 `--profile record`를 사용합니다. 또는 선택한 호스트 훅 계약이 지원되는 WSL2, Linux, macOS에서 Volicord를 실행합니다. |
| 소스 빌드 도구 체인 | Cargo가 포함된 Rust 1.85 이상으로 **지원되고 검증되었습니다.** | 워크스페이스 루트 `Cargo.toml`이 `rust-version = "1.85"`를 설정하고 모든 워크스페이스 패키지가 이 값을 상속합니다. 설치 문서는 소스 빌드 경로를 설명합니다. | 소스 빌드 경로를 사용할 때 Cargo가 포함된 Rust 1.85 이상을 설치하거나 선택합니다. |
| 셸 문법 | Linux, WSL2, macOS의 유지되는 POSIX 스타일 예시와 네이티브 Windows의 PowerShell 예시는 **지원됩니다.** 다른 셸은 이 예시에 대해 **미검증입니다.** | POSIX 설치 예시는 `sh` 호환 환경 변수 지정, 임시 설치 스크립트 경로, `~/.local/bin`을 사용합니다. 네이티브 Windows 설치 예시는 내려받은 `install.ps1`, PowerShell 매개변수 또는 환경 변수, `%LOCALAPPDATA%\Volicord\bin`을 사용합니다. CLI 통합 테스트는 `#[cfg(unix)]` 아래에서 `#!/bin/sh` 가짜 실행 파일을 만들며, 릴리스 워크플로는 Windows에서 PowerShell 스모크 테스트를 실행합니다. | 선택한 운영 환경에 맞는 셸 문법을 사용하고, 설치된 명령을 확인한 뒤 계속합니다. |
| 실행 파일 역할 이름 | **지원되고 검증되었습니다.** | 참조 담당 문서는 `volicord`를 관리 CLI 명령과 로컬 MCP stdio 어댑터가 사용하는 `mcp` 하위 명령을 제공하는 설치 실행 파일로 정의합니다. | `volicord`를 빌드하거나 설치합니다. 호스트 설정은 MCP를 `volicord mcp --stdio ...`로 시작해야 합니다. |
| 패키지 관리자 설치 | **지원 범위 밖입니다.** | 이 저장소는 Homebrew 탭이나 포뮬러, Linux 패키지 관리자 패키지, 외부 패키지 레지스트리를 제공한다고 주장하지 않습니다. | 소스 빌드, 로컬 Docker 빌드, 기존 `volicord` 실행 파일, 또는 검증된 게시 자산 세트가 뒷받침하는 릴리스 설치 스크립트를 사용합니다. |
| Codex와 Claude Code 호스트 최소 버전 | 안정적인 호스트 최소 버전은 정의되어 있지 않습니다. 호스트 호환성은 문서화된 버전 하한이 아니라 운영 점검으로 확인합니다. | Codex 검증은 `PATH`에서 `codex`를 찾고 `codex --version`을 실행합니다. Claude Code 검증은 `claude mcp get <server_name>`으로 호스트 상태를 조사합니다. 관리 검증은 최종 결과 상태를 담당합니다. | 설치 후 `volicord connection verify HOST [--repo PATH] [--shared|--global]`을 사용합니다. 문서화되지 않은 Codex 또는 Claude Code 최소 버전에 의존하지 않습니다. |
| Codex Detective profile의 호스트 훅 루트 결정 | 로컬 Git 작업 트리에서 **지원되고 검증되었습니다.** | 생성된 Codex 탐지형 호스트 훅 명령은 Volicord 관리 래퍼를 호출하기 전에 `git rev-parse --show-toplevel`로 Git 작업 트리 루트를 찾습니다. 초기화는 이 방법을 지원할 수 없으면 Detective profile 설정을 거부합니다. | Codex Detective profile에는 `.git` 작업 트리 루트가 있는 Product Repository를 사용합니다. Codex 훅 환경은 저장소 하위 디렉터리에서도 `git`을 실행할 수 있어야 합니다. 이 전제 조건이 없으면 `--profile record`를 사용합니다. |

<a id="toolchain-requirements"></a>

## 도구 체인 요구사항

게시된 자산에서 릴리스를 설치할 때는 Rust나 Cargo가 필요하지 않습니다.

소스 빌드 경로에는 아래가 필요합니다.

- Rust 1.85 이상
- 선택한 Rust 도구 체인의 Cargo
- 이 저장소의 로컬 체크아웃
- Cargo가 워크스페이스 의존성을 해석할 수 있게 하는 네트워크 또는 로컬 의존성 가용성

Rust 1.85는 이 워크스페이스의 컴파일러 요구사항입니다. 게시된 릴리스 자산에서
설치할 때는 필요하지 않으며 운영체제 지원 주장이 아닙니다.

이 요구사항을 읽거나 사용하는 것만으로 Rust 구현 검증이 필요한 것은 아닙니다. Rust 소스, Cargo 매니페스트, 테스트, 픽스처, 빌드 설정을 편집하는 유지보수자는 저장소 작업 규칙의 Rust 검증 정책을 따릅니다.

## 셸과 경로 요구사항

Linux, WSL2, macOS 릴리스 설치 예시는 아래를 제공하는 POSIX 스타일 셸을 가정합니다.

- `VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"` 같은 명령 앞 환경 변수 지정
- 설치 스크립트 자산을 임시 경로로 내려받기 위한 `curl`
- 설치 스크립트가 관리하는 릴리스 자산 다운로드를 위한 `curl` 또는 `wget`
- 임시 설치 스크립트 경로를 만들기 위한 `mktemp`
- 대상 이름의 릴리스 압축 파일을 풀기 위한 `tar`
- 체크섬과 압축 파일 형태를 확인하기 위한 `awk`, `wc`, `tr`, `sed`
- 체크섬을 검증할 수 있을 때 사용할 `sha256sum` 또는 `shasum`
- 설정 과정에서 셸 명령을 출력했을 때의 현재 세션 `PATH` 갱신
- `~/.local/bin` 같은 홈 기준 경로
- `PATH`를 통한 명령 찾기
- 예시의 슬래시 경로

네이티브 Windows 릴리스 설치 예시는 아래를 제공하는 PowerShell을 가정합니다.

- 임시 경로에서 실행하는 내려받은 `install.ps1` 릴리스 자산
- 설치 스크립트와 릴리스 자산 다운로드를 위한 `Invoke-WebRequest`
- 대상 이름의 `.zip`을 풀기 위한 `Expand-Archive`
- 체크섬을 검증할 수 있을 때 사용할 `Get-FileHash -Algorithm SHA256`
- `-UpdateUserPath`를 명시적으로 요청했을 때만 수행되는 사용자 수준 `PATH` 갱신
- Runtime Home과 Product Repository 위치에 사용할 로컬 드라이브 문자 경로

설치 스크립트는 내려받은 `.sha256` 체크섬 파일을 사용할 수 있을 때 이를
검증합니다. 체크섬 파일이 있지만 검증할 수 없으면 스크립트는 실패합니다. 체크섬
파일을 사용할 수 없으면 경고하고 계속 진행합니다. 이때 `VOLICORD_REQUIRE_CHECKSUM=1`이
설정되어 있으면 실패합니다.

현재 세션 `PATH` 예시는 실행한 셸에만 영향을 줍니다. 이후 셸이나 MCP 호스트에
명령을 지속적으로 설치하지 않습니다.

네이티브 Windows에서는 내려받은 PowerShell 설치 스크립트에 `-UpdateUserPath`를 사용하면
설치 디렉터리가 아직 없을 때 사용자 수준 `PATH` 값에만 그 디렉터리를 추가합니다. 이
스크립트는 시스템 수준 `PATH`를 바꾸지 않습니다. `-UpdateUserPath`를 사용하지 않으면
현재 세션용 `PATH` 명령과 설치된 실행 파일 경로를 출력합니다.

CLI는 부모 셸의 `PATH`를 영구적으로 수정할 수 없습니다. 설정 중 Volicord는 명령
링크, 안전할 때 없는 `~/.local/bin` 같은 관례적 사용자 명령 디렉터리 만들기, 출력된
셸 명령, 지원되는 셸에서 명시적으로 사용자가 확인한 관리 셸 시작 블록 같은 안전한 선택지를
제공해 명령을 `PATH`에서 사용할 수 있도록 도울 수 있습니다. 설정은 명령 링크를
놓기 전에 쓰기 가능 여부를 확인합니다. 기존 셸과 MCP 호스트는 변경된 시작 파일이나
명령 링크 디렉터리를 보려면 재시작하거나 다시 로드해야 할 수 있습니다.

`VOLICORD_HOME`은 다릅니다. `VOLICORD_HOME`은 담당 문서가 정의한 `volicord` 관리 명령과 `volicord mcp --stdio` 프로세스 시작의 실제 Runtime Home 선택 입력입니다.

<a id="executable-layout-and-discovery"></a>

## 실행 파일 배치와 찾기

설치된 실행 파일을 사용하려면 선택한 실행 파일 위치에서 아래 명령을 제공해야 합니다.

- `volicord`

POSIX 릴리스 tar 압축 파일은 아래 하나만 담아야 합니다.

- `volicord`

네이티브 Windows 릴리스 zip 압축 파일은 아래 하나만 담아야 합니다.

- `volicord.exe`

설치 스크립트는 그 실행 파일 하나만 설치합니다. 소스 빌드에서는 디버그 실행
파일이 `target/debug` 아래에, 릴리스 실행 파일이 `target/release` 아래에 있어야 합니다.
별도로 설치된 실행 파일을 사용할 때는 명시적 설정 옵션이나 `PATH`를 통해 설정 과정이
`volicord`를 찾을 수 있는 설치 배치를 선택합니다.

릴리스 바이너리나 다른 설치 명령 디렉터리에서 첫 연결을 하기 전에는 같은 셸에서 설치된
실행 파일을 확인합니다.

```sh
volicord --version
volicord --help
volicord mcp --help
volicord init --help
volicord status --help
volicord connection --help
volicord inbox --help
volicord serve --help
```

설치 가이드가 설명하는 릴리스 모드 소스 빌드에서 첫 연결을 하기 전에는 같은 셸에서
빌드된 실행 파일을 확인합니다.

```sh
./target/release/volicord --version
./target/release/volicord --help
./target/release/volicord mcp --help
```

`init` 또는 프로필 복구 안내로 명령이 보이게 된 뒤에는 일반 명령 찾기를 확인합니다.

```sh
volicord --version
volicord init --help
volicord status --help
volicord connection add --help
volicord mcp --version
volicord mcp --help
```

호스트 설정은 보통 `volicord init`이 마련한 MCP 명령 정보를 사용합니다. 정확한
`--mcp-command`, 찾기 순서, 연결, 일반 MCP 호스트 설정 동작은 [관리
CLI](admin-cli.md#runtime-home-selection)를 사용합니다.

요구사항 요약:

- 설치는 찾을 수 있는 `volicord` 명령을 식별해야 합니다.
- 설정을 로드하는 호스트 프로세스는 설정된 `volicord` 명령을 `mcp --stdio --connection <connection_id>`
  인자와 함께 시작할 수 있어야 합니다.
- `shared` 프로젝트 호스트 설정은 개인 Runtime Home 경로를 포함하면 안 됩니다.
  호스트 환경이 `PATH`로 해석해야 하는 명령 이름 `volicord`를 사용합니다.
- 사용자가 관리하는 일반 MCP 호스트 설정에는 호스트별로 관찰 가능한 로드 가능 여부
  점검이 없습니다.

## Runtime Home 요구사항

사용 가능한 `Volicord Runtime Home`은 요청한 관리 또는 MCP 작업이 런타임 기록을 필요로 할 때 선택한 프로세스가 만들고, 읽고, 쓸 수 있는 로컬 파일시스템 위치여야 합니다.

설치 전에 아래를 확인합니다.

- Runtime Home은 `Product Repository`가 아니어야 하며, `Product Repository` 안이나 위에 있지 않아야 합니다.
- 네이티브 Windows에서는 로컬 드라이브 문자 Runtime Home 경로를 선택합니다. UNC 경로,
  `\\wsl$\...` 같은 WSL UNC 경로, `/mnt/c/...` 같은 WSL 마운트 경로는 네이티브
  Windows Runtime Home 경로로 지원되지 않습니다.
- 선택한 사용자가 `volicord init`, `volicord project use`, `volicord connection add`, `volicord connection verify`를 실행할 때 디렉터리를 만들거나 그 안에 쓸 수 있어야 합니다.
- 기본 `$HOME/.volicord`가 의도한 위치가 아니라면 `volicord mcp --stdio`를 시작하는 호스트 프로세스도 같은 Runtime Home 선택을 받아야 합니다. `shared` 프로젝트 호스트 설정은 개인 Runtime Home 경로를 담으면 안 됩니다. 각 사용자는 기본값이 아닌 Runtime Home을 자신의 로컬 초기화 또는 환경으로 제공해야 합니다.

Runtime Home 선택과 정확한 생성 동작은 [관리 CLI](admin-cli.md)와 [MCP 전송](mcp-transport.md)이 담당합니다. 런타임 위치와 분리 규칙은 [런타임 경계](runtime-boundaries.md)가 담당합니다.

## Product Repository 요구사항

`Product Repository`는 프로젝트 등록, 프로젝트 선택, `shared` 연결 의도의 호스트 설정에 쓰이는 기존 로컬 디렉터리여야 합니다. `Volicord Runtime Home`과 분리되어 있어야 합니다. 네이티브 Windows에서는 Product Repository에 로컬 드라이브 문자 경로를 사용합니다. UNC 경로와 WSL 경로는 네이티브 Windows 프로젝트 등록에 지원되지 않습니다.

Volicord가 등록된 프로젝트를 검증하거나 사용할 때는 읽기 접근이 필요합니다. `Product Repository` 쓰기 접근은 담당 문서가 정의한 제품 파일 쓰기나 명시적으로 요청한 통합 파일에만 필요합니다. 여기에는 아래가 포함됩니다.

- 프로젝트 범위 Codex `.codex/config.toml`
- 프로젝트 범위 Claude Code `.mcp.json`
- Volicord 관리 `AGENTS.md` 지침 블록
- `.volicord/policy.json` 탐지형 호스트 훅 정책 파일
- Codex `.codex/hooks.json` 훅 설정과 `.codex/hooks/` 아래의 Volicord 관리 래퍼 스크립트
- `.claude/settings.json` 안의 Volicord 관리 Claude Code 훅 항목
- `.claude/hooks/` 아래의 Volicord 관리 Claude Code 훅 래퍼 스크립트
- `.claude/rules/` 아래의 Volicord 관리 Claude Code 규칙 파일

이 목록의 생성된 `guard` 통합 파일을 적용하려면 선택한 파일시스템과 프로세스가 같은
디렉터리의 조건부 커밋도 지원해야 합니다. 이 요구사항은 관리 지침, 정책, 호스트 훅,
래퍼, 규칙 파일에 적용됩니다. 프로젝트 범위 MCP 설정은 해당 호스트 어댑터를 통해
적용되며 이 `guard` 통합 커밋 보장을 그대로 적용하지 않습니다.

- 해석된 Product Repository 경로와 대상의 부모 경로 연결은 심볼릭 링크를 따라가지
  않고 열 수 있는 디렉터리로 유지되어야 합니다. 기존 대상은 일반 파일이어야 하며,
  대상 디렉터리에서 전용 스테이징 항목을 만들고 제거할 수 있어야 합니다.
- Linux와 macOS에서 기존 파일을 갱신하려면 같은 디렉터리에서 기존 대상을 덮어쓰지
  않는 생성 연산과 맞바꾸기 연산을 운영체제가 제공해야 합니다. 프로세스는 이전
  파일의 POSIX 모드, 사용자 ID, 그룹 ID, 플랫폼 인터페이스가 노출하는 모든 확장
  속성을 읽고, 다시 적용하고, 검증할 수 있어야 합니다.
- 네이티브 Windows에서 생성에는 기존 대상을 덮어쓰지 않는 `MoveFileExW` 이동 권한이
  필요합니다. 기존 파일 갱신에는 같은 볼륨의 하드 링크를 지원하는 로컬 NTFS 볼륨,
  이전 파일에 대한 새 쓰기 공유를 차단할 수 있는 권한, 미리 예약한 백업 항목을 사용하는
  `ReplaceFileW` 교체 권한도 필요합니다. 속성과 ACL 병합은 Windows 고유 동작을
  사용합니다. ReFS와 네트워크 파일시스템은 이 기존 파일 갱신 경로에서 지원하지 않으며,
  보존 하드 링크를 만들 수 없으면 갱신에 실패합니다.
- 지원되는 운영체제 대상이라고 해서 모든 네트워크, 가상, 사용자 공간, 마운트
  파일시스템이 이런 이름 공간과 메타데이터 의미를 제공하는 것은 아닙니다. 그런
  파일시스템에서 관리 파일 갱신은 미검증입니다. 필요한 연산이나 메타데이터 재현을
  사용할 수 없으면 CLI는 관리 갱신이 성공했다고 보고하지 않고 쓰기에 실패합니다.

Codex Detective profile을 사용하려면 선택한 Product Repository가 Git 작업 트리여야
합니다. 생성된 훅이 호스트 세션의 현재 작업 디렉터리에 의존하지 않고 프로젝트 루트를
찾기 위해서입니다. 이 Git 루트 요구사항은 Codex 탐지형 호스트 훅의 경로 안전성에만
적용됩니다. 통합 파일을 Volicord 런타임 상태로 만들거나 OS 수준 샌드박싱을 추가하지
않습니다. Record profile은 Codex 탐지형 호스트 훅 설치를 요구하지 않습니다. 네이티브
Windows의 Record profile은 지원되지만, Windows 호스트 훅과 감시기 동작을 사용할 수
없으므로 Detective profile은 거부됩니다.

비대화형 `shared` 연결 의도 호스트 설정 또는 지침 쓰기에는 [관리 CLI](admin-cli.md#noninteractive-approval-behavior)가 정의한 명시적 `--shared` 명령 경로가 필요합니다. 런타임 기록, SQLite 데이터베이스, 생성 기록, 로그, 상태 보기, QA 결과, 수락 기록, 닫기 준비 상태, 잔여 위험 기록은 `Product Repository`에 속하지 않습니다.

<a id="host-configuration-requirements"></a>
## 호스트 설정 요구사항

직접 호스트 설정을 구성할 때는 선택한 호스트와 연결 의도가 필요로 할 때 관리 프로세스가 대상 호스트 설정을 조사하고 관리 설정을 쓸 수 있어야 합니다.

기준 호스트와 연결 의도 요구사항:

| 호스트 | 연결 의도 | 환경 전제 조건 |
|---|---|---|
| Codex | `personal` | `CODEX_HOME` 또는 `HOME`이 사용자 Codex 설정 위치를 식별해야 합니다. 가용성 점검을 위해 `codex`가 `PATH`에서 사용 가능해야 합니다. |
| Codex | `shared` | `.codex/config.toml`을 적용할 때 선택한 `Product Repository`에 쓸 수 있어야 합니다. Codex 호스트는 `PATH`를 통해 프로젝트에 묶인 `volicord mcp --stdio`를 시작할 수 있어야 합니다. `shared` 파일은 개인 Runtime Home 경로를 포함하면 안 됩니다. Codex 프로젝트 신뢰가 여전히 필요할 수 있습니다. |
| Claude Code | `personal`, `global` | Volicord가 `claude mcp` 명령을 사용할 수 있도록 관리 프로세스가 `claude` 실행 파일을 시작할 수 있어야 합니다. |
| Claude Code | `shared` | `.mcp.json`을 적용할 때 선택한 `Product Repository`에 쓸 수 있어야 합니다. Claude Code 호스트는 `PATH`를 통해 프로젝트에 묶인 `volicord mcp --stdio`를 시작할 수 있어야 합니다. `shared` 파일은 개인 Runtime Home 경로를 포함하면 안 됩니다. 프로젝트 MCP 승인이 여전히 필요할 수 있습니다. |
| 일반 MCP 호스트 | 사용자 관리 | Volicord는 일반 MCP 호스트 설정을 쓰지 않습니다. 외부 호스트를 수동으로 설정하려면 먼저 지원되는 Agent Connection이 있어야 합니다. 외부 호스트는 호스트별 방식으로 로드되고 점검되기 전까지 사용자 관리 상태이며 미검증입니다. |

호스트 설정을 썼다는 사실은 호스트가 `volicord mcp --stdio`를 신뢰, 승인, 로드, 초기화, 노출했다는 증거가 아닙니다. `managed host configuration state`의 의미와 호스트 신뢰 경계는 [Agent Connection](agent-connection.md)이 담당합니다.

## MCP 호스트 환경 요구사항

기준 MCP 호스트 환경은 `volicord mcp --stdio --connection <connection_id> [--project <project_id>]`를
로컬 자식 프로세스로 시작하고 `stdin`/`stdout`으로 통신할 수 있어야 합니다.
`connection_id` 프로세스 인자는 생성된 호스트 설정이 기록했거나 사용자가 관리하는 일반
호스트 설정을 위해 선택된 저장 `connection_internal_id`를 가리킵니다. 선택적 `project_id` 프로세스 인자는 그
연결에 허용된 저장 `project_internal_id`를 가리킵니다. 둘 다 공개 MCP 도구 인자가
아닙니다. 이것은 네트워크 리스너 요구사항이 아닙니다.

호스트 프로세스 환경은 아래를 제공해야 합니다.

- 설정된 명령 경로나 `PATH`에 따른 실행 가능한 `volicord` 명령
- 의도한 Runtime Home이 기본 홈에서 유도되는 위치가 아니고 호스트 설정이 개인 환경 값을 담을 수 있을 때의 `VOLICORD_HOME`
- Runtime Home과 명시적으로 허용된 각 `Product Repository`에 대한 로컬 파일시스템 접근

`volicord mcp --check --connection <connection_id>`는 그 프로세스 바인딩에 대한 시작
검증 점검입니다. 전체 호스트 통합 검증이 아닙니다. 전체 호스트 검증에는 [관리
CLI](admin-cli.md)가 정의한 관리 결과 기준이 필요합니다.

## 중지 기준

아래 조건 중 하나라도 해당하면 설치 전에 멈춥니다.

- 소스 빌드 경로를 사용하는데 Cargo가 포함된 Rust 1.85 이상을 사용할 수 없습니다.
- 선택한 릴리스 출처가 문서화된 릴리스 경로에 필요한 설치 스크립트, 맞는 대상 압축 파일, 체크섬을 제공하지 않습니다.
- 게시된 릴리스 자산을 사용하는데 운영체제와 CPU 아키텍처에 맞는 지원 대상이 없습니다.
- 설치 스크립트가 지원되지 않는 플랫폼 또는 지원되지 않는 CPU 아키텍처를 보고합니다.
- 로컬에서 체크섬 검증을 요구하지만 체크섬 파일을 내려받거나 검증할 수 없습니다.
- 선택한 환경에 맞는 유지되는 셸 예시를 실행하거나 안정적으로 맞춰 쓸 수 없습니다.
- `volicord`가 없거나, 선택한 사용자가 실행할 수 없거나, 도움말과 버전 출력을 낼 수 없습니다.
- 선택한 Runtime Home을 필요한 프로세스가 만들고, 읽고, 쓸 수 없습니다.
- Runtime Home과 Product Repository가 같은 경로이거나 한쪽이 다른 한쪽을 포함합니다.
- 네이티브 Windows 설정에서 Runtime Home 또는 Product Repository에 UNC 경로, WSL UNC 경로,
  WSL 마운트 경로를 사용합니다.
- Product Repository가 없거나, 디렉터리가 아니거나, 요청한 프로젝트 범위 설정 또는 지침 쓰기에 필요한 쓰기가 불가능합니다.
- 요청된 `guard` 통합 관리 파일 쓰기가 심볼릭 링크를 따라가지 않고 대상을 안전하게
  순회할 수 없거나, 필요한 같은 디렉터리 이름 공간 연산을 사용할 수 없거나, 필요한
  기존 파일 메타데이터를 재현할 수 없습니다.
- `shared` 연결 의도 호스트 설정이 호스트 환경의 `PATH`에서 `volicord mcp --stdio`를 시작할 수 없습니다.
- 선택한 호스트 경로에 Codex 또는 Claude Code가 필요한데 관리 호환성 점검이 호스트를 시작하거나 해석할 수 없습니다.
- 네이티브 Windows 설정에서 `--profile detective`를 요청합니다.
- 필요한 호스트 신뢰, 프로젝트 신뢰, 프로젝트 MCP 승인, OAuth, 다시 로드, 재시작, 또는 비슷한 호스트 소유 동작이 남아 있고 운영자가 이를 완료할 수 없습니다.
- 선택한 환경이 이 저장소가 문서화하지 않는 패키지 관리자, Homebrew 탭, 게시된 Docker 레지스트리 이미지, 원격 호스트, 네트워크 리스너, 호스트 버전 약속에 의존합니다.

저장소 증거가 충분하지 않다면 그 환경을 미검증으로 분류하고, 그 환경에 의존하기 전에 담당 문서가 정의한 검증 명령을 사용합니다.
