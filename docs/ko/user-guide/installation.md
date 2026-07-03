# 설치

이 튜토리얼은 로컬 `volicord` 실행 파일을 준비합니다. 일반적인 첫 실행 경로는
[빠른 시작](quickstart.md)의 `volicord init --host HOST --repo PATH --profile record`를
실행하면서 설치 프로필을 기록합니다. 저장된 설치 프로필을 확인해야 할 때는
`volicord doctor`를 사용합니다.

정확한 명령 동작은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
런타임 위치와 저장소 분리는 [런타임 경계](../reference/runtime-boundaries.md)가
담당합니다.

## 전제 조건

- [시스템 요구사항](../reference/system-requirements.md)에 적힌 지원 릴리스 바이너리
  환경, 또는 아래 Docker 경로를 사용할 때의 Docker.
- Linux, WSL2, macOS에서는 `curl` 또는 `wget`, `tar`, 쓰기 가능한 설치 디렉터리를 사용할 수 있는 POSIX 스타일 셸. Native Windows에서는 PowerShell.
- 호스트를 연결할 준비가 되었을 때 Product Repository로 사용할 Git 저장소.

## 릴리스 바이너리 설치하기

기본 사용자 경로는 릴리스 바이너리입니다. POSIX 설치 스크립트는 Linux, WSL2, macOS를
감지하고 맞는 릴리스 tarball을 선택하며, 대응 `.sha256` 파일을 내려받을 수 있으면
검증한 뒤 `volicord` 실행 파일 하나만 설치합니다. Native Windows PowerShell 설치
스크립트는 `x86_64-pc-windows-msvc` zip archive를 선택하고, 대응 `.sha256` 파일을
내려받을 수 있으면 검증한 뒤 `volicord.exe` 하나만 설치합니다. 두 스크립트 모두 셸
시작 파일을 암시적으로 편집하지 않습니다.

Linux, WSL2, macOS에서는 Volicord 릴리스 자산을 게시하는 같은 저장소에서
`scripts/install.sh`를 내려받거나 복사한 뒤, 릴리스 저장소를 명시해서 실행합니다.

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh
```

`OWNER/REPO`는 이 체크아웃의 릴리스 자산을 호스팅하는 GitHub 저장소입니다. 기본값은
그 저장소의 latest release에서 내려받는 것입니다. 특정 태그를 설치하려면
`VOLICORD_VERSION`을 설정합니다.

```sh
VOLICORD_REPO=OWNER/REPO VOLICORD_VERSION=v0.1.0 sh ./scripts/install.sh
```

GitHub가 아닌 릴리스 mirror에서는 target 이름이 붙은 tarball과 checksum이 들어 있는
디렉터리를 제공합니다.

```sh
VOLICORD_RELEASE_BASE_URL=https://example.invalid/releases/v0.1.0 sh ./scripts/install.sh
```

기본 설치 디렉터리는 `~/.local/bin`입니다. 한 번 실행할 때만 바꾸려면
`--install-dir PATH`를 사용하고, 환경 변수 기반 자동화에는
`VOLICORD_INSTALL_DIR`을 사용합니다.

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh --install-dir /usr/local/bin
```

파일을 내려받거나 쓰지 않고 감지된 target, 릴리스 asset, checksum 계획, 설치
디렉터리, 바이너리 이름을 미리 보려면 `--dry-run`을 추가합니다. 자동화가 target
식별자만 필요할 때는 `--print-target`을 사용합니다.

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh --dry-run
sh ./scripts/install.sh --print-target
```

Native Windows x86_64에서는 Volicord 릴리스 자산을 게시하는 같은 저장소에서
`scripts/install.ps1`을 내려받거나 복사한 뒤 PowerShell에서 실행합니다.

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO
```

특정 태그를 설치하려면 아래처럼 실행합니다.

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -Version v0.1.0
```

GitHub가 아닌 릴리스 mirror에서는 아래처럼 실행합니다.

```powershell
.\scripts\install.ps1 -ReleaseBaseUrl https://example.invalid/releases/v0.1.0
```

기본 native Windows 설치 디렉터리는 `%LOCALAPPDATA%\Volicord\bin`입니다. 다른
사용자 로컬 디렉터리를 쓰려면 `-InstallDir`를 사용합니다.

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -InstallDir "$env:LOCALAPPDATA\Volicord\bin"
```

파일을 내려받거나 설치하거나 사용자 `PATH`를 바꾸지 않고 감지된 target, 릴리스
asset, checksum 계획, 설치 디렉터리, 바이너리 이름, 요청된 `PATH` 동작을 미리 보려면
`-DryRun`을 추가합니다. 자동화가 target 식별자만 필요할 때는 `-PrintTarget`을
사용합니다.

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -DryRun
.\scripts\install.ps1 -PrintTarget
```

설치 디렉터리가 아직 `PATH`에 없으면 Windows 설치 스크립트는 현재 세션용 `PATH`
명령을 출력합니다. 설치 디렉터리를 사용자 수준 `PATH`에 추가하려면 `-UpdateUserPath`로
다시 실행합니다.

```powershell
.\scripts\install.ps1 -Repo OWNER/REPO -UpdateUserPath
```

지원되지 않는 운영체제나 CPU 아키텍처에서는 각 스크립트가 내려받기 전에 실패합니다.
Checksum 파일이 있는데 검증할 수 없으면 실패합니다. Checksum 파일을 사용할 수 없으면
경고를 출력합니다. 이 경우에도 반드시 실패해야 한다면 `VOLICORD_REQUIRE_CHECKSUM=1`을
설정합니다.

이 저장소는 그에 맞는 저장소 아티팩트가 추가되기 전까지 Homebrew tap, 패키지 관리자
패키지, 외부 패키지 registry가 있다고 주장하지 않습니다.

설치 뒤 설치된 명령을 확인합니다.

```sh
volicord --version
volicord --help
volicord mcp --help
volicord init --help
```

일반적인 첫 저장소 연결은 [빠른 시작](quickstart.md)의
`volicord init --host HOST --repo PATH --profile record`로 이어갑니다. `volicord init`은
선택한 Product Repository를 연결하고, 프로젝트 범위 MCP 설정을 쓰며, 통합 상태를
기록하는 동안 Runtime Home과 설치 프로필을 초기화할 수 있습니다. Detective 설정에는
[관리 CLI 참조](../reference/admin-cli.md#agent-host-setup-and-init)에 설명된 검증된 host
hook 및 session watcher 요구사항이 적용됩니다.
Native Windows에서는 `--profile record`를 사용합니다. `--profile detective`는 Windows
host hook과 watcher 동작이 구현되고 테스트되기 전까지 unsupported-platform 진단으로
실패합니다.

`volicord init`은 선택된 `Volicord Runtime Home`을 만들거나 검증하고 설치 프로필을
저장하면서 저장소를 연결합니다. 실행 중인 `volicord` 실행 파일을 발견하고 MCP 시작
명령을 저장하며, 이후 터미널과 에이전트 호스트에서 선택된 명령을 `PATH`로 사용할 수
있는지 확인합니다. 정확한 Runtime Home 선택, MCP 시작 명령 동작, 출력 동작은
[관리 CLI 참조](../reference/admin-cli.md#runtime-home-selection)가 담당합니다. 이 상태는
setup에 이름 붙은 사용자 또는 호스트 동작이 아직 필요한지를 답하므로, 오래 유지되는
로컬 상태가 저장된 뒤에도 `action_required`가 나타날 수 있습니다.

호스트 설정을 실행하기 전에 설치된 `volicord` 바이너리가 `PATH`에 있어야 합니다.
셸 시작 파일 변경은 암시적으로 이루어지지 않습니다. 셸 시작 파일을 통해 `PATH`를
갱신했다면, 새 셸을 열거나 기존 에이전트 호스트 프로세스를 restart 또는 reload한 뒤
명령을 찾을 수 있다고 기대해야 합니다.

자동화나 결정적인 로컬 배치가 필요할 때는 명시적 init 옵션을 사용합니다.

| 옵션 | 사용할 때 |
|---|---|
| `--mcp-command PATH` | 생성된 MCP 시작 항목이 실행 중인 실행 파일 대신 특정 `volicord` 명령을 사용해야 할 때 그 명령을 저장합니다. |
| `--home PATH` | 기본값이 아닌 `Volicord Runtime Home`을 선택합니다. |

프롬프트나 `action_required`가 이름 붙인 명령 가용성 단계를 완료한 뒤 설정 준비
상태를 확인합니다.

```sh
volicord doctor
```

`doctor`는 기본 `init` 진행도가 아니라 설치 프로필 상태를 보고합니다. 저장된
프로필을 사용할 수 있으면, 이후 셸이나 에이전트 호스트를 위한 명령 가용성 경고 또는
권장 `PATH`와 명령 링크 동작을 함께 보고하더라도 `complete`를 보고합니다.
`action_required`는 실행 파일 경로 수정처럼 차단하는 로컬 복구 동작을 이름 붙입니다.

## 기존 설치 실행 파일 사용하기

`volicord`가 이미 `PATH`에 있으면 바로 [빠른 시작](quickstart.md)으로 갈 수 있습니다.
설치 프로필을 점검해야 할 때는 아래처럼 실행합니다.

```sh
volicord doctor
```

실행 파일을 릴리스로 설치했든, 개발용 소스 빌드에서 가져왔든, 다른 설치 명령
디렉터리에서 가져왔든 init은 같은 설치 프로필 계약을 사용합니다. 생성된 호스트
설정이 다른 `volicord` 명령 경로로 MCP를 시작해야 할 때만
`volicord init --mcp-command PATH ...`를 사용합니다. Init이 `action_required`를
보고하면 새 터미널이나 에이전트 호스트를 시작하기 전에 이름 붙은 로컬 또는 호스트
동작을 완료합니다. 일반 `volicord init`과 `volicord connection add` 명령은 저장된
설치 프로필을 사용합니다.

## 개발용 소스 빌드

소스 빌드는 구현자와 로컬 개발자를 위한 경로이며 기본 사용자 설치 경로가 아닙니다.
Volicord 소스 저장소에서 실행합니다.

```sh
cargo build --workspace --bins
./target/debug/volicord --version
./target/debug/volicord init --host codex --repo /path/to/your-product-repo --profile record
```

이 경로는 로컬 개발 실행 파일 `./target/debug/volicord`를 빌드하고 실행합니다. 호스트가
개발 실행 파일을 사용하려면 선택한 `volicord` 명령을 호스트 프로세스에서 찾을 수 있게
하거나, 일반 호스트 설정에는 설치된 릴리스 바이너리를 사용합니다. 이 경로의 Rust 도구 체인 요구사항은
[시스템 요구사항](../reference/system-requirements.md#toolchain-requirements)에 있습니다.

## Docker 이미지

Docker 지원은 로컬 컨테이너 배치와 localhost MCP 접근을 위한 것입니다. Volicord 소스
저장소에서 이미지를 빌드합니다.

```sh
docker build -t volicord:local .
```

`init`, `project`, `connection`, `doctor`, `connection verify`, `serve` 명령을 실행할
때는 Runtime Home 볼륨을 사용하고 Product Repository를 같은 컨테이너 경로에 마운트합니다.
프로젝트 등록은 저장소 루트를 저장하므로, 한 경로 배치에서 준비한 Runtime Home을 다른
컨테이너 workspace 경로와 함께 재사용하면 안 됩니다.

예를 들어 마운트한 저장소에 대한 record 프로필 설치를 만들거나 재사용합니다.

```sh
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local init --host codex --repo /workspace --profile record
```

같은 마운트로 Docker 설치 프로필을 점검하고 선택한 Agent Connection을 검증합니다.

```sh
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local doctor

docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local connection verify codex --repo /workspace
```

Detective Docker 설정에는 컨테이너를 쓰지 않을 때와 같은 검증된 host hook 및 session
watcher 요구사항이 적용됩니다. Runtime Home에 serve할 프로젝트 등록과 Agent Connection이
들어간 뒤, 예를 들어 그와 일치하는 `init` 실행이나 낮은 수준의 `connection add` 실행 뒤,
운영자가 제공한 token 파일로 로컬 HTTP MCP endpoint를 시작합니다.

```sh
umask 077
VOLICORD_HTTP_TOKEN_FILE="$(mktemp)"
openssl rand -hex 32 > "$VOLICORD_HTTP_TOKEN_FILE"
docker run --rm \
  -p 127.0.0.1:8765:8765 \
  -v "$VOLICORD_HTTP_TOKEN_FILE:/tmp/volicord-http-token:ro" \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local serve --transport local-http \
    --container-listen 0.0.0.0:8765 \
    --token-file /tmp/volicord-http-token \
    --project /workspace
```

`-p 127.0.0.1:8765:8765`는 컨테이너 포트를 호스트 loopback 인터페이스에만 노출합니다.
`--container-listen 0.0.0.0:8765`는 이 Docker 노출 형태를 위한 옵션입니다. 네이티브 로컬
실행은 대신 기본 loopback `--listen` 동작을 사용해야 합니다. 컨테이너 포트를 `0.0.0.0`,
공개 호스트 인터페이스, 원격 호스트에 노출하지 말고, token 파일을 저장소 파일에 저장하지
마세요. 이것은 로컬/Docker 전송일 뿐 공개 네트워크 API, SaaS endpoint, 다중
사용자 서버, 보안 경계가 아닙니다.

## 설치가 하지 않는 일

바이너리 설치만으로는 Product Repository를 등록하지 않고 호스트 설정을 설치하지도
않습니다. 프로젝트 등록은 Git 저장소 안에서 `volicord project use`,
`volicord init --host HOST --repo PATH --profile record`, `volicord connection add` 같은 명령을
실행할 때 이루어집니다.

프로젝트 이름과 내부 식별 정보 동작은 [관리 CLI
참조](../reference/admin-cli.md#project-commands)가 담당합니다. 내부 식별 정보는
Volicord가 저장하며 첫 설정 입력이 아닙니다.

## 다음 단계

Product Repository에 호스트를 연결합니다.

```sh
volicord init --host codex --repo /path/to/your-product-repo --profile record
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다. 선택한 호스트, 플랫폼, 저장소 설정이 검증된 detective 전제조건을 만족할 때만
`--profile detective`를 사용합니다. Native Windows에서는 `detective` 프로필이 지원되지
않으므로 `--profile record`를 사용합니다.

전체 첫 실행 경로는 [빠른 시작](quickstart.md)을 계속 읽습니다. 호스트별
세부사항은 [에이전트 호스트 설정](../user-guide/agent-host-setup.md)을 봅니다.
