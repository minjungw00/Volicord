# 설치

이 튜토리얼은 로컬 `volicord` 실행 파일을 준비합니다. 일반적인 첫 실행 경로는
[빠른 시작](quickstart.md)의 `volicord init --shared --host HOST --repo PATH --profile record`를
실행하면서 설치 프로필을 기록합니다. 저장된 설치 프로필을 확인해야 할 때는
`volicord doctor`를 사용합니다.

정확한 명령 동작은 [관리 CLI 참조](../reference/admin-cli.md)를 보세요.
런타임 위치와 저장소 분리는 [런타임 경계](../reference/runtime-boundaries.md)에
있습니다.

## 전제 조건

- 소스 빌드 경로에는 Cargo가 포함된 Rust 1.85 이상, 릴리스 설치 경로에는 완전한
  게시 릴리스 자산 세트, 로컬 컨테이너 경로에는 Docker가 필요합니다. 자세한 내용은
  [시스템 요구사항](../reference/system-requirements.md)을 보세요.
- 게시 릴리스를 설치할 때 Linux, WSL2, macOS에서는 `curl` 또는 `wget`, `tar`, 쓰기
  가능한 설치 디렉터리를 사용할 수 있는 POSIX 스타일 셸이 필요합니다. Native
  Windows에서는 PowerShell이 필요합니다.
- 호스트를 연결할 준비가 되었을 때 Product Repository로 사용할 Git 저장소.

## 소스에서 빌드하기

이 체크아웃에서 직접 재현할 수 있는 네이티브 경로는 소스 빌드입니다.

```sh
cargo build --locked --release -p volicord-cli --bin volicord
./target/release/volicord --version
```

빌드한 실행 파일을 사용자 명령 디렉터리에 설치합니다. 필요하면
`$HOME/.local/bin`을 이미 `PATH`에 있는 다른 디렉터리로 바꿉니다.

```sh
mkdir -p "$HOME/.local/bin"
install -m 0755 target/release/volicord "$HOME/.local/bin/volicord"
volicord --version
```

이 경로에는 [시스템 요구사항](../reference/system-requirements.md#toolchain-requirements)에
적힌 Rust 도구 체인이 필요합니다. 게시 릴리스 호스트에는 의존하지 않습니다.

## 게시된 릴리스 자산 설치하기

릴리스 배포처가 서로 맞는 설치 스크립트, target archive, checksum 세트를 알려진 base
URL에서 제공할 때만 이 경로를 사용합니다. 체크인된 스크립트와 패키징 워크플로는 그
자산의 동작을 정의하지만, 소스 트리에 있다는 사실만으로 특정 저장소, 태그, mirror가
자산을 게시했다는 뜻은 아닙니다. 검증한 자산 출처가 없다면 위 소스 빌드 경로를
사용합니다.

POSIX 설치 스크립트는 Linux, WSL2, macOS를
감지하고 맞는 릴리스 tarball을 선택하며, 대응 `.sha256` 파일을 내려받을 수 있으면
검증한 뒤 `volicord` 실행 파일 하나만 설치합니다. Native Windows PowerShell 설치
스크립트는 `x86_64-pc-windows-msvc` zip archive를 선택하고, 대응 `.sha256` 파일을
내려받을 수 있으면 검증한 뒤 `volicord.exe` 하나만 설치합니다. 두 스크립트 모두 셸
시작 파일을 암시적으로 편집하지 않습니다.

Linux, WSL2, macOS에서는 `install.sh` 릴리스 자산을 임시 파일로 내려받은 뒤 릴리스
자산 base URL을 명시해서 실행합니다.

```sh
repo=OWNER/REPO
base="https://github.com/$repo/releases/latest/download"
tmp="$(mktemp "${TMPDIR:-/tmp}/install-volicord.XXXXXX")"
curl -fsSL "$base/install.sh" -o "$tmp"
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"
```

`OWNER/REPO`는 Volicord 릴리스 자산을 호스팅하는 GitHub 저장소입니다. 기본 예시는 그
저장소의 latest release에서 내려받습니다. 특정 태그를 설치하려면 태그별 릴리스 자산
base URL을 사용합니다.

```sh
repo=OWNER/REPO
version=v0.8.0
base="https://github.com/$repo/releases/download/$version"
tmp="$(mktemp "${TMPDIR:-/tmp}/install-volicord.XXXXXX")"
curl -fsSL "$base/install.sh" -o "$tmp"
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"
```

GitHub가 아닌 릴리스 mirror에서는 target 이름이 붙은 tarball과 checksum이 들어 있는
디렉터리에 설치 스크립트 자산도 함께 제공합니다.

```sh
base="https://example.invalid/releases/v0.8.0"
tmp="$(mktemp "${TMPDIR:-/tmp}/install-volicord.XXXXXX")"
curl -fsSL "$base/install.sh" -o "$tmp"
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp"
```

기본 설치 디렉터리는 `~/.local/bin`입니다. 한 번 실행할 때만 바꾸려면
`--install-dir PATH`를 사용하고, 환경 변수 기반 자동화에는
`VOLICORD_INSTALL_DIR`을 사용합니다. 이 예시는 위에서 선택한 릴리스의 `$base`와
`$tmp`를 다시 사용합니다.

```sh
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp" --install-dir /usr/local/bin
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 VOLICORD_INSTALL_DIR=/usr/local/bin sh "$tmp"
```

릴리스 archive를 내려받거나 설치 파일을 쓰지 않고 감지된 target, 릴리스 asset,
checksum 계획, 설치 디렉터리, 바이너리 이름을 미리 보려면 `--dry-run`을 추가합니다.
자동화가 target 식별자만 필요할 때는 `--print-target`을 사용합니다.

```sh
VOLICORD_RELEASE_BASE_URL="$base" VOLICORD_REQUIRE_CHECKSUM=1 sh "$tmp" --dry-run
sh "$tmp" --print-target
```

Native Windows x86_64에서는 `install.ps1` 릴리스 자산을 내려받은 뒤 PowerShell에서
실행합니다.

```powershell
$repo = "OWNER/REPO"
$base = "https://github.com/$repo/releases/latest/download"
$tmp = Join-Path $env:TEMP "install-volicord.ps1"
Invoke-WebRequest "$base/install.ps1" -OutFile $tmp
& $tmp -ReleaseBaseUrl $base -RequireChecksum
```

특정 태그를 설치하려면 아래처럼 실행합니다.

```powershell
$repo = "OWNER/REPO"
$version = "v0.8.0"
$base = "https://github.com/$repo/releases/download/$version"
$tmp = Join-Path $env:TEMP "install-volicord.ps1"
Invoke-WebRequest "$base/install.ps1" -OutFile $tmp
& $tmp -ReleaseBaseUrl $base -RequireChecksum
```

GitHub가 아닌 릴리스 mirror에서는 아래처럼 실행합니다.

```powershell
$base = "https://example.invalid/releases/v0.8.0"
$tmp = Join-Path $env:TEMP "install-volicord.ps1"
Invoke-WebRequest "$base/install.ps1" -OutFile $tmp
& $tmp -ReleaseBaseUrl $base -RequireChecksum
```

기본 native Windows 설치 디렉터리는 `%LOCALAPPDATA%\Volicord\bin`입니다. 다른
사용자 로컬 디렉터리를 쓰려면 `-InstallDir`를 사용합니다. 이 예시는 위에서 선택한
릴리스의 `$base`와 `$tmp`를 다시 사용합니다.

```powershell
& $tmp -ReleaseBaseUrl $base -RequireChecksum -InstallDir "$env:LOCALAPPDATA\Volicord\bin"
```

릴리스 archive를 내려받거나 설치하거나 사용자 `PATH`를 바꾸지 않고 감지된 target,
릴리스 asset, checksum 계획, 설치 디렉터리, 바이너리 이름, 요청된 `PATH` 동작을 미리
보려면 `-DryRun`을 추가합니다. 자동화가 target 식별자만 필요할 때는 `-PrintTarget`을
사용합니다.

```powershell
& $tmp -ReleaseBaseUrl $base -RequireChecksum -DryRun
& $tmp -PrintTarget
```

설치 디렉터리가 아직 `PATH`에 없으면 Windows 설치 스크립트는 현재 세션용 `PATH`
명령을 출력합니다. 설치 디렉터리를 사용자 수준 `PATH`에 추가하려면 `-UpdateUserPath`로
다시 실행합니다.

```powershell
& $tmp -ReleaseBaseUrl $base -RequireChecksum -UpdateUserPath
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
`volicord init --shared --host codex --repo PATH --profile record`로 이어갑니다.
`volicord init`은 선택한 Runtime Home을 만들거나 재사용하고 Product Repository를
연결하며 관리 Codex stdio 구성을 쓰고 통합 상태를 기록합니다. 이름 붙은 Codex 신뢰,
다시 불러오기, 검증 단계가 끝날 때까지 `action_required`가 남을 수 있습니다.


호스트 설정을 실행하기 전에 설치된 `volicord` 바이너리가 `PATH`에 있어야 합니다.
셸 시작 파일 변경은 암시적으로 이루어지지 않습니다. 셸 시작 파일을 통해 `PATH`를
갱신했다면 새 셸을 열거나 기존 에이전트 호스트 프로세스를 재시작하거나 다시 불러온 뒤
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

실행 파일을 릴리스로 설치했든, 소스 빌드에서 가져왔든, 다른 설치 명령
디렉터리에서 가져왔든 init은 같은 설치 프로필 계약을 사용합니다. 생성된 호스트
설정이 다른 `volicord` 명령 경로로 MCP를 시작해야 할 때만
`volicord init --mcp-command PATH ...`를 사용합니다. Init이 `action_required`를
보고하면 새 터미널이나 에이전트 호스트를 시작하기 전에 이름 붙은 로컬 또는 호스트
동작을 완료합니다. 일반 `volicord init`과 `volicord connection add` 명령은 저장된
설치 프로필을 사용합니다.

## Docker 이미지

저장소 루트의 `Dockerfile`은 개발과 CI에서 사용하는 범용 소스 빌드 정의입니다.
`Dockerfile.release`는 별도의 프로덕션 릴리스 패키징 책임을 가집니다. 릴리스
워크플로가 이미 검증한 `x86_64-unknown-linux-gnu` 실행 파일을 `volicord`라는
이름으로 제공하면, 릴리스 Dockerfile은 Volicord를 다시 빌드하지 않고 그 정확한
바이트를 이미지에 복사합니다. 워크플로는 이미지 안의
`/usr/local/bin/volicord` SHA-256 다이제스트가 검증된 원시 아티팩트 다이제스트와
같아야 한다고 요구합니다.

로컬 소스 빌드에는 범용 루트 Dockerfile을 사용합니다. 폐기 가능한 Runtime Home과
의도한 Product Repository를 마운트한 뒤 관리 점검 또는 결속된 stdio 프로세스를
실행합니다. 컨테이너 이미지는 별도 공개 transport를 추가하거나 플랫폼 적용 범위를
바꾸지 않습니다.

```sh
docker build -t volicord:local .
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local doctor
```

`init`, 연결 검증, 관리 stdio에도 같은 mount를 사용해 프로세스가 같은 Runtime Home과
Product Repository를 보게 합니다.

## 설치가 하지 않는 일

바이너리 설치만으로는 Product Repository를 등록하지 않고 호스트 설정을 설치하지도
않습니다. 프로젝트 등록은 Git 저장소 안에서 `volicord project use`,
`volicord init --shared --host HOST --repo PATH --profile record`, `volicord connection add` 같은 명령을
실행할 때 이루어집니다.

프로젝트 이름과 내부 식별 정보 동작은 [관리 CLI
참조](../reference/admin-cli.md#project-commands)를 보세요. 내부 식별 정보는
Volicord가 저장하며 첫 설정 입력이 아닙니다.

## 다음 단계

Product Repository에 호스트를 연결합니다.

```sh
volicord init --shared --host codex --repo /path/to/your-product-repo --profile record
```

`/path/to/your-product-repo`는 Codex가 작업할 Product Repository의 예시 경로입니다.
최초 릴리스는 모든 지원 플랫폼에서 `record` 프로필을 사용합니다.

이 명령은 남은 host 소유 activation action을 보고하기 전에 정규 managed launcher,
Connection, 현재 Guard 설정을 적용합니다. 적용에 성공했다는 사실만으로 다시 불러온
managed host나 현재 hook이 실행됐음이 증명되지는 않습니다.

이 공유 설정에서는 init이 선택한 것과 같은 비어 있지 않은 절대 경로
`VOLICORD_HOME`을 호스트 시작 환경이 제공해야 합니다. 저장소에서 보이는 설정은 그
값을 전달하며 머신 로컬 Runtime Home 경로를 내장하지 않습니다.

Init이 `review_required_by_setup`을 보고하면 host에서 activation을 마칩니다.

1. 해당 저장소에서 Codex를 restart 또는 reload합니다.
2. Codex hook UI 또는 `/hooks`로 현재 프로젝트 hook definition을 review합니다.
3. 새 conversation을 시작합니다.
4. `Run the Volicord integration verification.`을 요청합니다.
5. 현재 connection status를 읽습니다.

In-chat agent는 `volicord.list_projects`,
`volicord.begin_integration_verification`, 반환된 `volicord.guard_probe`,
`volicord.get_integration_verification`을 이 순서로 사용해야 합니다. Tool이 노출되지
않으면 managed MCP가 unavailable이라고 보고합니다. Raw stdio, 직접 작성한 Codex
`_meta`, resource, resource template, CLI preflight를 proof로 대신하지 않습니다.
`volicord connection verify`는 선택적인 diagnostic이며 host 소유 hook review나
managed in-chat evidence를 대신하지 않습니다.

전체 첫 실행 경로는 [빠른 시작](quickstart.md)을 계속 읽습니다. 호스트별
세부사항은 [에이전트 호스트 설정](../user-guide/agent-host-setup.md)을 봅니다.
