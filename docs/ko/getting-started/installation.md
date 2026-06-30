# 설치

이 튜토리얼은 로컬 `volicord` 실행 파일을 준비하고, 이후 프로젝트, 연결, 내보내기,
MCP, `User Channel` 명령이 사용할 설치 프로필을 기록합니다.
[빠른 시작](quickstart.md) 전에 수행하는 설정 단계입니다.

정확한 명령 동작은 [관리 CLI 참조](../reference/admin-cli.md)가 담당합니다.
런타임 위치와 저장소 분리는 [런타임 경계](../reference/runtime-boundaries.md)가
담당합니다.

## 전제 조건

- [시스템 요구사항](../reference/system-requirements.md)에 적힌 지원 릴리스 바이너리
  환경, 또는 아래 Docker 경로를 사용할 때의 Docker.
- `curl` 또는 `wget`, `tar`, 쓰기 가능한 설치 디렉터리를 사용할 수 있는 POSIX 스타일 셸.
- 호스트를 연결할 준비가 되었을 때 Product Repository로 사용할 Git 저장소.

## 릴리스 바이너리 설치하기

기본 사용자 경로는 릴리스 바이너리입니다. 설치 스크립트는 Linux, WSL2, macOS를
감지하고 맞는 릴리스 tarball을 선택하며, 대응 `.sha256` 파일을 내려받을 수 있으면
검증한 뒤 `volicord` 실행 파일 하나만 설치합니다. 셸 시작 파일은 편집하지 않습니다.

Volicord 릴리스 자산을 게시하는 같은 저장소에서 `scripts/install.sh`를 내려받거나
복사한 뒤, 릴리스 저장소를 명시해서 실행합니다.

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

기본 설치 디렉터리는 `~/.local/bin`입니다. 다른 디렉터리를 쓰려면
`VOLICORD_INSTALL_DIR`을 사용합니다.

```sh
VOLICORD_REPO=OWNER/REPO VOLICORD_INSTALL_DIR=/usr/local/bin sh ./scripts/install.sh
```

지원되지 않는 운영체제나 CPU 아키텍처에서는 스크립트가 내려받기 전에 실패합니다.
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
`volicord init --host HOST --repo PATH`로 이어갑니다. `volicord init`은 선택한
Product Repository를 연결하고 guarded 호스트 통합 파일을 쓰는 동안 Runtime Home과
설치 프로필을 초기화할 수 있습니다.

저장소를 연결하기 전에 설치 프로필만 준비하거나 복구하려면 `volicord setup`을
사용합니다.

```sh
volicord setup
```

`volicord setup`은 선택된 `Volicord Runtime Home`을 만들거나 검증하고 설치
프로필을 저장합니다. 실행 중인 `volicord` 실행 파일을 발견하고 MCP 시작 명령을
저장하며, 이후 터미널과 에이전트 호스트에서 선택된 명령을 `PATH`로 사용할 수 있는지
확인합니다. 정확한 `volicord setup` 옵션, MCP 시작 명령 동작, 출력 동작은 [관리 CLI
참조](../reference/admin-cli.md#runtime-home-selection)가 담당합니다.
이 상태는 안내형 첫 실행 설정 경험에 이름 붙은 사용자 동작이 아직 필요한지를
답하므로, 설치 프로필이 저장된 뒤에도 `action_required`가 나타날 수 있습니다.

대화형 터미널에서 선택된 실행 파일이 `PATH`에 준비되어 있지 않으면 setup은 명령
가용성 선택지를 제시할 수 있습니다.

- setup이 쓰기 가능하다고 확인한 제안 디렉터리에 명령 링크를 만듭니다.
- `~/.local/bin` 같은 관례적 사용자 명령 디렉터리가 없고 안전하게 만들 수 있을 때
  그 디렉터리를 만든 뒤, 쓰기 가능 여부를 확인하고 링크합니다.
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
| `--link-bin PATH` | 필요하면 디렉터리를 만들고 쓰기 가능 여부를 확인한 뒤 그곳에 명령 링크를 만들거나 갱신합니다. 이 옵션 자체가 셸 시작 파일을 편집하지는 않습니다. |
| `--mcp-command PATH` | 생성된 MCP 시작 항목이 실행 중인 실행 파일 대신 특정 `volicord` 명령을 사용해야 할 때 그 명령을 저장합니다. |
| `--home PATH` | 기본값이 아닌 `Volicord Runtime Home`을 선택합니다. |

예를 들어 비대화식 setup 단계에서 결정적인 명령 링크 디렉터리를 계속 지정할 수
있습니다.

```sh
volicord setup --link-bin ~/.local/bin
```

프롬프트나 `action_required`가 이름 붙인 명령 가용성 단계를 완료한 뒤 설정 준비
상태를 확인합니다.

```sh
volicord doctor
```

`doctor`는 첫 실행 setup 진행도가 아니라 설치 프로필 상태를 보고합니다. 저장된
프로필을 사용할 수 있으면, 이후 셸이나 에이전트 호스트를 위한 명령 가용성 경고 또는
권장 `PATH`와 명령 링크 동작을 함께 보고하더라도 `complete`를 보고합니다.
`action_required`는 `volicord setup` 재실행이나 실행 파일 경로 수정처럼 차단하는
로컬 복구 동작을 이름 붙입니다.

## 기존 설치 실행 파일 사용하기

`volicord`가 이미 `PATH`에 있고 저장소를 연결하기 전에 설치 프로필만 준비하거나
점검하려면 아래처럼 실행합니다.

```sh
volicord setup
volicord doctor
```

실행 파일을 릴리스로 설치했든, 개발용 소스 빌드에서 가져왔든, 다른 설치 명령
디렉터리에서 가져왔든 setup은 같은 설치 프로필 계약을 사용합니다. 생성된 호스트
설정이 다른 `volicord` 명령 경로로 MCP를 시작해야 할 때만
`volicord setup --mcp-command PATH`를 사용합니다. Setup이 `action_required`를 보고하면 새 터미널이나 에이전트 호스트를
시작하기 전에 이름 붙은 로컬 동작을 완료합니다. 일반 `volicord init`과
`volicord connect` 명령은 저장된 설치 프로필을 사용합니다.

## 개발용 소스 빌드

소스 빌드는 구현자와 로컬 개발자를 위한 경로이며 기본 사용자 설치 경로가 아닙니다.
Volicord 소스 저장소에서 실행합니다.

```sh
cargo build --workspace --bins
./target/debug/volicord --version
./target/debug/volicord setup
```

이 경로는 로컬 개발 실행 파일 `./target/debug/volicord`를 빌드하고 실행합니다.
이 경로의 Rust 도구 체인 요구사항은
[시스템 요구사항](../reference/system-requirements.md#toolchain-requirements)에 있습니다.

## Docker 이미지

Docker 지원은 로컬 컨테이너 배치와 localhost MCP 접근을 위한 것입니다. Volicord 소스
저장소에서 이미지를 빌드합니다.

```sh
docker build -t volicord:local .
```

setup, init, project, connection, serve 명령을 실행할 때는 Runtime Home 볼륨을
사용하고 Product Repository를 같은 컨테이너 경로에 마운트합니다. 프로젝트 등록은
저장소 루트를 저장하므로, 한 경로 배치에서 준비한 Runtime Home을 다른 컨테이너
workspace 경로와 함께 재사용하면 안 됩니다.

예를 들어 같은 마운트로 Docker Runtime Home을 준비하거나 점검합니다.

```sh
docker run --rm -it \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local setup
```

Runtime Home에 serve할 프로젝트 등록과 Agent Connection이 들어간 뒤, 예를 들어 같은
mount를 사용한 `volicord init` 또는 `volicord connect` 실행 뒤, 운영자가 제공한
token으로 로컬 HTTP MCP endpoint를 시작합니다.

```sh
VOLICORD_HTTP_TOKEN="$(openssl rand -hex 32)"
docker run --rm \
  -p 127.0.0.1:8765:8765 \
  -v volicord-home:/var/lib/volicord \
  -v "$PWD:/workspace" \
  volicord:local serve --transport streamable-http \
    --listen 0.0.0.0:8765 \
    --allow-nonlocal-listen \
    --token "$VOLICORD_HTTP_TOKEN" \
    --project /workspace
```

컨테이너는 Docker가 포트를 publish할 수 있도록 Docker 내부에서만 `0.0.0.0`에
리스닝합니다. 호스트 publish 주소는 `127.0.0.1`로 남으며 Volicord는 여전히
`--allow-nonlocal-listen`과 bearer 인증을 요구합니다. `VOLICORD_HTTP_TOKEN`을 저장소
파일에 저장하지 마세요.

## 설정이 하지 않는 일

`volicord setup`은 Product Repository를 등록하지 않고 호스트 설정을 설치하지도 않습니다.
프로젝트 등록은 Git 저장소 안에서 `volicord project use`,
`volicord init --host HOST --repo PATH`, `volicord connect` 같은 명령을 실행할 때
이루어집니다.

프로젝트 이름과 내부 식별 정보 동작은 [관리 CLI
참조](../reference/admin-cli.md#project-commands)가 담당합니다. 내부 식별 정보는
Volicord가 저장하며 첫 설정 입력이 아닙니다.

## 다음 단계

Product Repository에 호스트를 연결합니다.

```sh
volicord init --host codex --repo /path/to/your-product-repo
```

`/path/to/your-product-repo`는 에이전트에게 작업을 요청할 Product Repository의 경로
예시입니다.

전체 첫 실행 경로는 [빠른 시작](quickstart.md)을 계속 읽습니다. 호스트별
세부사항은 [에이전트 호스트 설정](../guides/agent-host-setup.md)을 봅니다.
