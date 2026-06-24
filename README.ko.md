# Volicord

[English](README.md) | **[한국어](README.ko.md)**

**AI가 움직여도, 판단은 사용자에게.**

Volicord(볼리코드)는 AI 지원 제품 작업을 위한 로컬 작업 권한 시스템입니다.

Volicord는 사용자와 에이전트가 작업의 중요한 부분, 즉 범위, 사용자 소유 판단,
증거, 검증 기준, 최종 수락, 잔여 위험 수락, 닫기 준비 상태를 계속 볼 수 있게
돕습니다.

이 README는 현재 저장소를 처음 쓰는 독자를 위한 경로입니다. Volicord가
무엇인지, 로컬 실행 파일과 호스트 설정이 어떻게 맞물리는지, 실행 파일을
빌드하고 확인하는 방법, 지원되는 호스트 경로를 고르는 방법, 설정 뒤 첫
Volicord 지원 상호작용을 시도하는 방법을 설명합니다.

정확한 계약은 이 페이지 곳곳에서 연결하는 유지 참조 문서에 남아 있습니다.

## 목차

- [개요](#overview)
- [Volicord가 필요한 이유](#why-volicord-exists)
- [구체적인 시나리오](#scenario)
- [구성 요소가 맞물리는 방식](#how-the-pieces-fit)
- [첫 설정을 위한 용어](#terms)
- [현재 기능과 경계](#support)
- [시스템과 셸 요구사항](#system-requirements)
- [예시 값과 경로](#example-values)
- [실행 파일 빌드와 확인](#executable-installation)
- [호스트 경로 선택](#host-selection)
- [Codex 사용자 범위 설정](#codex)
- [Claude Code 프로젝트 범위 설정](#claude-code)
- [Generic export](#generic-export)
- [상태와 검증](#verification)
- [호스트에서 처음 사용하기](#first-use)
- [데이터 소유와 쓰기 경계](#data-boundaries)
- [첫 설치 문제 해결](#troubleshooting)
- [문서 경로](#documentation-routes)

<a id="overview"></a>
## 개요

Volicord는 AI 에이전트가 변경을 살펴보고, 계획하고, 작성하고, 확인하고,
요약하는 로컬 제품 작업을 위한 도구입니다. Volicord의 역할은 모든 에이전트
행동을 자동화하는 것이 아닙니다. 작업이 진행되는 동안 권한의 근거를 보이는
상태로 유지하는 것입니다.

실제로 Volicord는 에이전트가 다음과 같은 질문을 계속 묻고 답하게 돕습니다.

- 무엇이 범위 안에 있고, 무엇이 명시적으로 범위 밖인가?
- 어떤 결정이 사용자에게 속하는가?
- 어떤 증거가 이 주장을 뒷받침하는가?
- 실제로 실행한 점검은 무엇인가?
- 사용자는 최종 결과를 수락하는가, 아니면 다음 쓰기만 승인하는가?
- 남은 위험이 사용자가 수락하거나 거부할 수 있을 만큼 분명하게 이름 붙어
  있는가?
- 정직하게 닫는 데 아직 무엇이 걸려 있는가?

Volicord 자체는 로컬 기준 기록이 아닙니다. Core가 Volicord 상태를 위한 로컬 기준
기록입니다. Volicord는 그 기록을 둘러싼 더 넓은 제품이자 시스템이며, 로컬
런타임 구성 요소, 호스트 통합, 문서, 작업 흐름을 포함합니다.

<a id="why-volicord-exists"></a>
## Volicord가 필요한 이유

AI 지원 제품 작업은 중요한 구분이 흐려질 만큼 빠르게 움직일 수 있습니다.
통과한 테스트가 최종 수락처럼 들리기 시작할 수 있습니다. 그럴듯한 구현
세부사항이 말하지 않은 제품 결정이 될 수 있습니다. 넓은 의미의 "looks good"이
범위 확장, 민감 동작, 잔여 위험, 닫기에 대한 승인으로 오해될 수 있습니다.

Volicord는 흐려지면 안 되는 부분만 늦추기 위해 존재합니다. 에이전트는 여전히
빠르게 움직일 수 있지만, 범위, 사용자 판단, 증거, 검증 기준, 수락, 잔여
위험, 닫기 준비 상태는 서로 분리되어 남아 있습니다.

<a id="scenario"></a>
## 구체적인 시나리오

사용자가 이렇게 요청한다고 해 봅시다.

```text
이메일 로그인을 추가해줘. 비밀번호 재설정과 계정 생성은 범위 밖으로 둬.
먼저 계획을 세우고, 내가 첫 변경을 승인하기 전까지는 파일에 아무 변경도 하지 마.
```

Volicord 지원 작업 흐름은 다음 사실들을 서로 구분해 보이게 유지해야 합니다.

| 작업 항목 | 계속 보여야 하는 것 |
|---|---|
| 범위 | 이메일 로그인은 범위 안에 있고, 비밀번호 재설정과 계정 생성은 범위 밖입니다. |
| 사용자 소유 판단 | 제품 동작, 위험 절충, 최종 수락은 여전히 사용자가 결정합니다. |
| 증거 | diff, 테스트 출력, 로그, 소스 인용은 특정 주장만 뒷받침합니다. |
| 검증 기준 | 요청한 작업을 위한 보이는 점검은 증거나 수락과 같은 것이 아닙니다. |
| 쓰기 승인 | 이름 붙은 쓰기 시도 하나에 대한 허가는 이후 모든 쓰기나 민감 동작에 대한 승인이 아닙니다. |
| 최종 수락 | 통과한 점검은 수락 판단에 정보를 줄 수 있지만, 사용자의 수락을 대체하지 않습니다. |
| 잔여 위험 | 사용자가 수락하도록 요청받는 이름 붙은 남은 위험은 계속 보여야 합니다. |
| 닫기 준비 상태 | 현재 상태에 대해 닫기 근거가 정직할 때만 작업을 닫아야 합니다. |

핵심은 평범합니다. 에이전트가 속도로 사용자의 판단을 대체하면 안 됩니다.

<a id="how-the-pieces-fit"></a>
## 구성 요소가 맞물리는 방식

현재 로컬 설정에는 네 가지 분리된 위치 또는 행위자가 있습니다.

```text
AI host
  Codex, Claude Code, or a user-managed MCP host
        |
        | starts a local child process
        v
volicord-mcp --integration <integration_id>
        |
        | uses the selected Volicord Runtime Home and allowed Product Repository
        v
Volicord Runtime Home                    Product Repository
  /Users/alex/.volicord                    /work/acme-api

volicord
  administrative CLI used for install, status, verification, and guidance
```

`volicord-mcp`는 호스트가 시작하는 로컬 stdio 자식 프로세스입니다. TCP, HTTP,
소켓 또는 다른 네트워크 리스너가 아닙니다. `volicord-mcp` 프로세스 하나는
`--integration <integration_id>`로 하나의 Agent Integration Profile에 묶입니다.
프로젝트 선택은 공개 Volicord 도구 호출마다 이루어집니다.

`volicord`는 관리 CLI입니다. 설정 상태를 만들고, 호스트 구성을 설치하거나
내보내고, 상태를 살펴보고, 검증을 새로 고치는 데 사용합니다. 장기 실행
서버가 아니며 공개 Volicord API 메서드 표면도 아닙니다.

<a id="terms"></a>
## 첫 설정을 위한 용어

| 용어 | 처음 읽는 독자를 위한 의미 | 더 자세한 내용 |
|---|---|---|
| Volicord | AI 지원 제품 작업을 위한 로컬 작업 권한 제품이자 시스템입니다. | [시작하기 개요](docs/ko/getting-started/overview.md) |
| Core | Volicord 상태를 위한 로컬 기준 기록입니다. | [Core 모델](docs/ko/reference/core-model.md) |
| Volicord 구현 | 이 저장소가 유지하는 구현 집합입니다. Core, 저장소, 타입, `volicord` CLI, `volicord-mcp`, 테스트, 문서, 검증 도구를 포함합니다. | [런타임 경계](docs/ko/reference/runtime-boundaries.md) |
| `volicord` | `volicord-cli` 패키지의 관리 CLI 실행 파일입니다. | [관리 CLI](docs/ko/reference/admin-cli.md) |
| `volicord-mcp` | AI 호스트가 시작하는 로컬 MCP stdio 실행 파일입니다. | [MCP 전송](docs/ko/reference/mcp-transport.md) |
| `Volicord Runtime Home` | Volicord 기록과 운영 데이터를 위한 로컬 런타임 저장소입니다. | [런타임 경계](docs/ko/reference/runtime-boundaries.md) |
| `Product Repository` | 사용자의 프로젝트 작업 공간과 제품 파일 경계입니다. | [런타임 경계](docs/ko/reference/runtime-boundaries.md) |
| 에이전트 호스트 | `volicord-mcp`를 시작할 수 있는 Codex, Claude Code, 또는 사용자 관리 MCP 호스트입니다. | [에이전트 통합](docs/ko/reference/agent-integration.md) |
| Agent Integration Profile | `integration_id`로 선택되는 지속되는 Volicord 통합 기록입니다. | [에이전트 통합](docs/ko/reference/agent-integration.md) |
| Host Installation | 호스트 설정과 마지막 검증 상태를 위한 Volicord 관리 인벤토리입니다. | [에이전트 통합](docs/ko/reference/agent-integration.md) |

<a id="support"></a>
## 현재 기능과 경계

현재 이 저장소에는 다음이 들어 있습니다.

- Cargo Rust 워크스페이스
- `volicord-cli`의 `volicord` 관리 실행 파일
- `volicord-mcp`의 로컬 stdio `volicord-mcp` 실행 파일
- `docs/` 아래의 유지되는 영어와 한국어 문서
- Codex와 Claude Code에 대한 직접 설정 지원
- 사용자 관리 호스트를 위한 일반 MCP 구성 내보내기
- 구현, 통합, 적합성 테스트 경로
- `docs/doc-index.yaml`의 문서 메타데이터

현재 첫 설정 지원은 의도적으로 로컬에 한정됩니다.

| 영역 | 현재 기준 |
|---|---|
| 실행 파일 원천 | 이 소스 체크아웃에서 빌드하거나, `volicord`와 `volicord-mcp`가 모두 들어 있는 이미 사용 가능한 Volicord 설치 디렉터리를 사용합니다. |
| 직접 호스트 설정 | Codex와 Claude Code에는 지원되는 직접 `volicord agent install` 경로가 있습니다. |
| 일반 MCP 호스트 | Generic export는 사용자 관리 호스트를 위한 구성을 렌더링합니다. Volicord는 그 호스트에 직접 설치하거나 호스트가 그것을 로드했음을 증명하지 않습니다. |
| MCP 전송 | 기준 프로세스는 로컬 stdio입니다. 호스트가 `volicord-mcp`를 자식 프로세스로 시작합니다. |
| 패키지 관리자 | 현재 담당 문서에는 패키지 관리자 설치 경로가 문서화되어 있지 않습니다. |
| 이름 붙은 운영체제 | 이 체크아웃에서 일반적으로 지원된다고 선언된 OS 계열은 없습니다. 유지되는 예시는 POSIX 스타일 셸 문법을 사용합니다. |
| 원격 호스트와 컨테이너 | 현재 담당 문서는 이를 지원되는 기준 설정 경로로 문서화하지 않습니다. |

Codex와 Claude Code 설정은 관리 작업으로 성공할 수 있지만, 프로젝트 trust,
프로젝트 MCP 승인, reload, restart, OAuth, 실행 파일 가용성 같은 호스트 소유
동작이 여전히 필요할 수 있습니다. Generic export는 보통 `action_required`로
남습니다. Volicord가 외부 호스트가 내보낸 구성을 로드했는지 관찰할 수 없기
때문입니다.

<a id="system-requirements"></a>
## 시스템과 셸 요구사항

설치 명령을 실행하기 전에 다음을 확인합니다.

| 요구사항 | 현재 규칙 |
|---|---|
| 소스 빌드를 위한 Rust 도구 체인 | Cargo가 포함된 Rust 1.85 이상입니다. 워크스페이스 루트 `Cargo.toml`은 `rust-version = "1.85"`를 선언합니다. |
| 셸 예시 | 유지되는 명령은 `export`, `$(pwd)`, 따옴표로 감싼 변수, 인라인 환경 할당, 콜론으로 구분한 `PATH`, `test -x` 같은 POSIX 스타일 문법을 사용합니다. |
| 실행 파일 배치 | 선택한 디렉터리 하나에 실행 가능한 `volicord`와 `volicord-mcp`가 모두 들어 있어야 합니다. |
| Runtime Home | 선택한 사용자와 이후 호스트 프로세스가 읽고 쓸 수 있는 로컬 `Volicord Runtime Home`을 선택합니다. |
| Product Repository | 이미 존재하는 로컬 `Product Repository` 디렉터리를 선택합니다. Runtime Home과 분리되어야 합니다. |
| 호스트 가용성 | 직접 설정에는 Codex 또는 Claude Code를 사용하고, generic export에는 사용자 관리 MCP 호스트를 사용합니다. 고정된 최소 호스트 버전은 문서화되어 있지 않습니다. |

셸이 POSIX 스타일 예시를 실행할 수 없다면 신중하게 변환하고, 계속하기 전에
변환한 각 명령을 확인합니다. Rust 이식성을 이 저장소가 이름 붙은 OS,
PowerShell, `cmd.exe`, 컨테이너 이미지, 원격 호스트를 지원한다는 주장으로
취급하지 마세요.

전체 요구사항 계약은 [시스템 요구사항](docs/ko/reference/system-requirements.md)을
사용합니다.

<a id="example-values"></a>
## 예시 값과 경로

아래 명령은 하나의 일관된 예시 집합을 사용합니다. 모든 예시 경로와 ID를 실제
값으로 바꾸세요.

| 예시 값 | 의미 |
|---|---|
| `VOLICORD_BIN="/absolute/path/to/selected/bin"` | 두 실행 파일이 모두 들어 있는 디렉터리 하나를 가리키는 셸 편의 변수입니다. |
| `"$VOLICORD_BIN/volicord"` | 관리 CLI 호출입니다. |
| `"$VOLICORD_BIN/volicord-mcp"` | 사용자/로컬 범위 호스트 구성과 generic export에 쓰는 절대 `volicord-mcp` 경로입니다. |
| `/Users/alex/.volicord` | 예시 `Volicord Runtime Home`입니다. |
| `/work/acme-api` | 예시 `Product Repository`입니다. |
| `acme-api` | 예시 `project_id`입니다. |
| `int-codex-team` | 예시 Codex `integration_id`입니다. |
| `int-claude-acme` | 예시 Claude Code `integration_id`입니다. |
| `int-generic-acme` | 예시 generic export `integration_id`입니다. |

`VOLICORD_BIN`은 이 예시에서만 쓰는 셸 변수입니다. Volicord는 이것을 구성으로
읽지 않습니다. 새 셸마다 다시 설정하거나 절대 경로를 직접 사용하세요.

`VOLICORD_HOME`은 다릅니다. 관리 명령과, 기본 home에서 파생되는 Runtime Home이
의도한 값이 아닐 때 이후 `volicord-mcp` 프로세스 시작에 쓰는 실제 Runtime Home
선택 입력입니다.

`Volicord Runtime Home`과 `Product Repository`는 파일시스템 경로를 실제 위치로
해석한 뒤에도 서로 다른 위치여야 합니다. 어느 쪽도 다른 쪽 안에 두지 마세요.

<a id="executable-installation"></a>
## 실행 파일 빌드와 확인

이 저장소에서 빌드할 때는 경로 A를 사용합니다. 두 실행 파일이 모두 들어 있는
Volicord 설치 디렉터리가 이미 있을 때는 경로 B를 사용합니다.

### 경로 A: 소스에서 빌드

작업 디렉터리: Volicord 소스 저장소 루트.

먼저 도구 체인을 확인합니다.

```sh
cargo --version
rustc --version
```

둘 중 하나를 사용할 수 없거나 선택한 Rust 컴파일러가 1.85보다 오래되었다면,
빌드하기 전에 도구 체인을 고칩니다.

디버그 빌드:

```sh
cargo build -p volicord-cli -p volicord-mcp
export VOLICORD_BIN="$(pwd)/target/debug"
```

릴리스 빌드:

```sh
cargo build --release -p volicord-cli -p volicord-mcp
export VOLICORD_BIN="$(pwd)/target/release"
```

### 경로 B: 설치된 실행 파일 선택

소스 체크아웃 밖에서 실행 파일을 이미 사용할 수 있을 때 이 경로를 사용합니다.

```sh
export VOLICORD_BIN="/absolute/path/to/installed/bin"
```

예시 경로를 `volicord`와 `volicord-mcp`가 모두 들어 있는 절대 디렉터리로
바꿉니다.

### 선택한 디렉터리 확인

`VOLICORD_BIN`이 설정된 같은 셸에서 실행합니다.

```sh
test -x "$VOLICORD_BIN/volicord"
test -x "$VOLICORD_BIN/volicord-mcp"

"$VOLICORD_BIN/volicord" --version
"$VOLICORD_BIN/volicord" agent --help
"$VOLICORD_BIN/volicord-mcp" --version
"$VOLICORD_BIN/volicord-mcp" --help
```

버전 명령은 `volicord <version>`과 `volicord-mcp <version>`을 출력해야 합니다.
도움말 명령은 `volicord agent` 명령군과 통합에 묶인 `volicord-mcp
--integration <integration_id>` 프로세스 사용법을 보여야 합니다.

두 실행 파일이 같은 선택 디렉터리에서 실행된 뒤에만 계속합니다. 이것은 실행
파일이 호스트 설정에 준비되었음을 증명합니다. Runtime Home을 만들거나,
Product Repository를 등록하거나, 호스트 구성을 설치하지는 않습니다.

집중 튜토리얼은 [설치](docs/ko/getting-started/installation.md)를 봅니다.

<a id="host-selection"></a>
## 호스트 경로 선택

첫 설정에는 경로 하나를 선택합니다. 다른 경로는 나중에 추가할 수 있습니다.

| 경로 | 사용할 때 | Volicord가 확인할 수 있는 것 |
|---|---|---|
| Codex `user` 범위 | 개인 Codex MCP 항목 하나가 지금 이 저장소를 처리해야 하고, 나중에 명시적으로 허용된 저장소를 더 처리할 수 있어야 할 때. | 직접 설정은 Codex 사용자 구성을 설치하고 관리 검증을 실행할 수 있습니다. |
| Claude Code `project` 범위 | Product Repository가 팀 공유 Claude Code `.mcp.json` 항목을 가져야 할 때. | 직접 설정은 권한이 부여되면 프로젝트 파일을 쓴 뒤, 호스트 소유 승인 또는 완료 상태를 보고할 수 있습니다. |
| Generic `export` 범위 | 다른 MCP 호스트를 사용하고 그 구성을 직접 관리할 때. | Volicord는 구성을 렌더링할 수 있습니다. 외부 호스트가 그것을 로드했는지는 증명할 수 없습니다. |

아래 예시는 의도적으로 Codex 경로 하나, Claude Code 경로 하나, generic export
경로 하나를 보여 줍니다. 더 많은 호스트와 범위 조합은
[에이전트 호스트 설정](docs/ko/guides/agent-host-setup.md)에 문서화되어
있습니다.

<a id="codex"></a>
## Codex 사용자 범위 설정

개인 Codex 구성 하나가 Codex 프로젝트들에서 같은 Volicord 통합을 로드해야 할
때 이 경로를 사용합니다.

실행하기 전에:

- `VOLICORD_BIN`은 확인된 실행 파일 디렉터리를 가리킵니다.
- Codex는 `CODEX_HOME` 또는 `HOME`을 통해 사용자 `config.toml`을 읽을 수
  있습니다.
- 호환성 확인을 위해 `codex` 실행 파일을 관리 명령 `PATH`에서 사용할 수
  있습니다.
- `/Users/alex/.volicord`와 `/work/acme-api`는 분리된 경로입니다.
- 이 첫 설치는 `/work/acme-api`를 새 프로젝트 등록으로 도입하므로
  `--project-id acme-api`와 `--repo-root /work/acme-api`를 모두 제공합니다.
  프로젝트 ID는 사용자가 선택하는 안정적인 논리 식별자입니다.
- `--integration-id`, `--runtime-home`, 절대 `--mcp-command`는 일반적으로 선택
  사항이지만, 이 예시에서는 후속 명령과 생성되는 호스트 구성이 예측 가능한 값을
  쓰도록 고정합니다. 전체 인자 규칙은 [관리 CLI](docs/ko/reference/admin-cli.md#volicord-agent-install)를
  봅니다.

설치:

```sh
"$VOLICORD_BIN/volicord" agent install \
  --host codex \
  --scope user \
  --integration-id int-codex-team \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.volicord \
  --mcp-command "$VOLICORD_BIN/volicord-mcp"
```

`--default-project-id`와 `--server-name`이 생략되었으므로 새 통합은 선택한
프로젝트를 기본값으로 사용하고, CLI는 `integration_id`에서
`volicord-int-codex-team` 같은 안정적인 호스트 MCP 서버 이름을 파생합니다.

예상되는 첫 결과에는 다음이 포함됩니다.

```text
status: complete
integration_id: int-codex-team
host_kind: codex
host_scope: user
server_name: volicord-int-codex-team
verification: complete
```

설정은 `/Users/alex/.volicord` 아래의 Runtime Home 기록과 Codex 사용자 MCP
항목을 쓸 수 있습니다. 선택적 저장소 지침을 별도로 선택하고 명시적으로 권한을
부여하지 않는 한 `/work/acme-api`에는 쓰지 않습니다.

독립 완료 확인:

```sh
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord
```

검증이 `status: complete`를 보고하면 이 경로는 완료된 것입니다.
`action_required`를 보고하면 이름 붙은 호스트 소유 동작을 완료하고 검증을
다시 실행합니다.

<a id="claude-code"></a>
## Claude Code 프로젝트 범위 설정

`/work/acme-api`가 팀 공유 Claude Code `.mcp.json` 항목을 가져야 할 때 이
경로를 사용합니다.

실행하기 전에:

- `VOLICORD_BIN`은 확인된 실행 파일 디렉터리를 가리킵니다.
- `/Users/alex/.volicord`와 `/work/acme-api`는 분리된 경로입니다.
- 이후 Claude Code 프로세스가 사용하는 `PATH`에서 `volicord-mcp`를 사용할 수
  있습니다.
- Claude Code가 기본적으로 `/Users/alex/.volicord`를 쓰지 않는다면, 이후
  Claude Code 실행 환경은 `VOLICORD_HOME=/Users/alex/.volicord`를 제공해야
  합니다.
- 관리 명령이 `/work/acme-api/.mcp.json`을 쓰도록 의도적으로 허용합니다.
- `--integration-id`는 선택 사항이지만 verify 명령과 생성되는 서버 이름을 예측할
  수 있게 고정합니다. 프로젝트 범위는 기본값이 이식 가능한 `volicord-mcp`
  명령이므로 `--mcp-command`를 생략합니다.
- `--dry-run`은 선택적인 zero-write 미리보기이고 `--output json`은 미리보기 출력
  형식만 바꿉니다. 실제 적용 명령은 의도한 프로젝트 파일 쓰기를 권한 부여하기
  위해 `--allow-repository-write`를 유지합니다.

선택적 dry-run:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --dry-run \
  --output json
```

설정 적용:

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent install \
  --host claude-code \
  --scope project \
  --integration-id int-claude-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --allow-repository-write
```

호스트 승인 전 예상되는 첫 결과에는 다음이 포함될 수 있습니다.

```text
status: action_required
integration_id: int-claude-acme
host_kind: claude_code
host_scope: project
server_name: volicord-int-claude-acme
verification: action_required
```

`action_required`는 성공한 관리 결과입니다. Claude Code 프로젝트 MCP 승인,
reload, restart 같은 이름 붙은 호스트 소유 동작이 남아 있다는 뜻입니다.

생성된 `.mcp.json` 항목은 의도적으로 이식 가능한 명령 `volicord-mcp`를 저장하고
개인 `VOLICORD_HOME`을 넣지 않습니다. 위의 인라인 `VOLICORD_HOME`과 `PATH` 값은
관리 명령에만 적용됩니다. 나중에 Claude Code가 서버를 시작할 때에는 Claude
Code 자체 환경이 `volicord-mcp`를 찾을 수 있어야 하고, 기본값이 다르다면 의도한
Runtime Home을 선택할 수 있어야 합니다.

호스트 소유 승인 또는 reload 단계를 완료한 뒤 확인합니다.

```sh
VOLICORD_HOME=/Users/alex/.volicord \
PATH="$VOLICORD_BIN:$PATH" \
"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-claude-acme
```

검증이 `status: complete`를 보고하고 선택된 Host Installation이
`final_status: complete`를 보고하면 이 경로는 완료된 것입니다.

<a id="generic-export"></a>
## Generic export

Generic export는 Volicord가 직접 설치하지 않는 호스트에만 사용합니다. 이 경로는
외부 호스트 자체의 설정 흐름에 적용할 구성을 렌더링합니다.

필수 선택은 호스트와 범위입니다. 이 예시는 프로젝트 선택을 명시적으로 만들려고
`--project-id acme-api`와 `--repo-root /work/acme-api`를 모두 제공합니다. 전체 생략
규칙은 [관리 CLI](docs/ko/reference/admin-cli.md#volicord-agent-install)에 남아
있습니다. 선택 사항인 `--integration-id`, `--runtime-home`, 명시적 `--mcp-command`,
`--export-dir`는 내보낸 서버 이름, Runtime Home 환경, 명령 경로, 대상 위치를 재현
가능하게 하려고 유지합니다.

```sh
"$VOLICORD_BIN/volicord" agent install \
  --host generic \
  --scope export \
  --integration-id int-generic-acme \
  --project-id acme-api \
  --repo-root /work/acme-api \
  --runtime-home /Users/alex/.volicord \
  --mcp-command "$VOLICORD_BIN/volicord-mcp" \
  --export-dir /tmp/volicord-mcp-export
```

내보내기는 다음 형태의 호스트 중립 MCP 서버 객체를 포함합니다.

```json
{
  "mcpServers": {
    "volicord-int-generic-acme": {
      "command": "/absolute/path/to/selected/bin/volicord-mcp",
      "args": ["--integration", "int-generic-acme"],
      "env": {
        "VOLICORD_HOME": "/Users/alex/.volicord"
      }
    }
  }
}
```

그 구성을 외부 호스트 자체의 지침에 따라 적용합니다. Volicord는 그것을 직접
설치하거나, 호스트를 reload하거나, 호스트가 그것을 로드했는지 확인하지
않습니다. Generic export는 그 이유로 `action_required`로 남을 수 있습니다.

<a id="verification"></a>
## 상태와 검증

다음 점검은 서로 다른 뜻을 가집니다.

| 명령 | 알려 주는 것 | 증명하지 않는 것 |
|---|---|---|
| `volicord agent status` | 레지스트리 상태, 허용된 프로젝트, Host Installation 인벤토리, 마지막 검증 상태, 지침 상태입니다. | 호스트가 MCP 서버를 로드했거나 노출했음을 증명하지 않습니다. |
| `volicord agent verify` | 선택된 Host Installation에 대한 관리 검증입니다. 관찰 가능한 경우 시작 점검과 호스트별 gate를 포함합니다. | 호스트 소유 trust 또는 승인 결정을 사용자 대신 하지 않습니다. |
| `volicord-mcp --check --integration <integration_id>` | 로컬 `volicord-mcp` 프로세스와 선택된 통합의 시작 검증입니다. | 완전한 호스트 통합이 아니며 Codex, Claude Code, generic 호스트가 그것을 로드했음을 증명하지 않습니다. |

유용한 점검:

```sh
"$VOLICORD_BIN/volicord" agent status \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord

"$VOLICORD_BIN/volicord" agent verify \
  --integration-id int-codex-team \
  --runtime-home /Users/alex/.volicord

VOLICORD_HOME=/Users/alex/.volicord \
"$VOLICORD_BIN/volicord-mcp" --check --integration int-codex-team
```

온보딩 수준의 설정 결과 상태:

| 상태 | 의미 |
|---|---|
| `complete` | 선택된 설치에서 관리 설정, 관련 호스트 소유 gate, MCP 초기화, 도구 발견이 모두 성공했습니다. |
| `action_required` | 명령은 성공했지만 이름 붙은 호스트 소유 동작이 남았습니다. 그 동작을 완료한 뒤 `volicord agent verify`를 실행합니다. |
| `partial_failure` | 뒤의 단계가 실패하기 전에 일부 지속 관리 동작이 성공했을 수 있습니다. 다시 시도하기 전에 `effects`와 `residual_effects`를 읽으세요. |
| `failed` | 요청한 설정 또는 검증이 사용할 수 있는 지속 통합 상태나 호스트 구성을 만들지 못했습니다. 다시 시도하기 전에 보고된 오류를 고칩니다. |

성공한 MCP 시작은 호스트가 Volicord를 로드했거나 계속 일관되게 사용할 것임을
증명하지 않습니다. 도구 발견도 AI 모델이 모든 요청에 대해 Volicord를 선택한다는
보장이 아닙니다.

정확한 결과 상태 동작은 [관리 CLI](docs/ko/reference/admin-cli.md)에 속합니다.

<a id="first-use"></a>
## 호스트에서 처음 사용하기

선택한 호스트 경로를 설치하고 검증한 뒤에는 호스트를 평소처럼 사용합니다. MCP
메서드를 직접 호출할 필요는 없습니다.

좋은 첫 요청은 에이전트에게 경계를 계속 보이게 해 달라고 요청하는 자연어
요청입니다.

```text
구현 전에 이 계획을 더 구체화해줘. 현재 적용 범위, 범위 밖 항목,
알려지지 않은 사항, 안전하게 수행할 수 있는 첫 행동을 보여줘.
```

```text
무엇을 알고 있고, 아직 무엇이 막혀 있고, 다음에 안전하게 할 수 있는 일은 뭐야?
```

```text
무엇이 바뀌었고, 무엇을 확인했으며, 어떤 잔여 위험이 보이고, 아직 무엇이 닫기를
막는지 보여줘.
```

통합에 둘 이상의 프로젝트가 허용되어 있고 에이전트가 어느 프로젝트를 사용할지
확신하지 못한다면, 폴더 이름, 호스트 라벨, 채팅 기억으로 추측하지 말고 허용된
프로젝트를 나열한 뒤 명시적 프로젝트 선택으로 다시 시도해야 합니다.

사용자 작업 흐름은 [사용자 가이드](docs/ko/guides/user-workflow.md)를 봅니다.

<a id="data-boundaries"></a>
## 데이터 소유와 쓰기 경계

다음 위치를 분리해서 유지합니다.

| 위치 | 소유자 | 그곳에 속하는 것 | 설정이 쓸 수 있는 것 |
|---|---|---|---|
| Volicord 소스 저장소 또는 설치 | Volicord 구현 유지보수자 또는 설치자 | 소스 체크아웃, 설치된 실행 파일, 빌드 출력, 문서, 테스트, 필수 실행 파일 리소스입니다. | 소스 빌드는 Cargo 출력을 `target/` 아래에 씁니다. |
| `Volicord Runtime Home` | 로컬 Volicord 운영자 | Volicord 레지스트리, 통합 상태, 프로젝트 상태, 런타임 기록, 저장소 담당 문서가 정의한 런타임 데이터입니다. | 에이전트 설정은 그곳에 Volicord 기록을 만들거나 재사용합니다. |
| `Product Repository` | 제품 프로젝트 소유자 | 제품 파일과 명시적으로 선택된 프로젝트 범위 통합 파일입니다. | `.codex/config.toml`, `.mcp.json`, `AGENTS.md` 지침, `.claude/rules/` 지침 같은 명시적으로 선택되고 권한이 부여된 통합 파일 또는 지침만 씁니다. |
| Codex 또는 Claude Code 구성 | 호스트 운영자 | `volicord-mcp --integration <integration_id>`를 시작하는 호스트 소유 설정입니다. | 직접 설정은 선택한 호스트와 범위가 요구하는 위치에 관리되는 호스트 구성을 쓸 수 있습니다. |
| Generic export 대상 | 사용자 관리 호스트 운영자 | 다른 호스트를 위한 내보낸 MCP 구성입니다. | `/tmp/volicord-mcp-export` 같은 사용자가 선택한 내보내기 파일 또는 디렉터리입니다. |

Volicord 런타임 데이터베이스, 런타임 기록, 생성 기록, 로그, Projection, QA 결과,
수락 기록, 닫기 준비 상태, 잔여 위험 기록은 `Product Repository`에 저장되지
않습니다.

설정 중 저장소 쓰기는 명시적으로 선택한 통합 구성이나 지침으로 제한되며,
비대화식 프로젝트 범위 쓰기에는 `--allow-repository-write`가 필요합니다. 그
파일들은 호스트 구성 또는 조언 맥락입니다. Core 권한, 증거, 최종 수락, 닫기
준비 상태, 잔여 위험 수락, 보안 보장이 아닙니다.

정확한 위치 규칙은 [런타임 경계](docs/ko/reference/runtime-boundaries.md)를
사용합니다.

<a id="troubleshooting"></a>
## 첫 설치 문제 해결

| 증상 | 첫 안전 대응 | 경로 |
|---|---|---|
| `cargo` 또는 `rustc`를 사용할 수 없거나 Rust가 1.85보다 오래되었습니다. | Cargo가 포함된 Rust 1.85 이상을 선택한 뒤 도구 체인 점검을 다시 실행합니다. | [시스템 요구사항](docs/ko/reference/system-requirements.md) |
| `target/debug` 또는 `target/release`에 두 실행 파일이 모두 없습니다. | 어떤 빌드 명령이 성공했는지 확인하고, 그에 맞는 출력 디렉터리를 선택한 뒤 모든 실행 파일 점검을 다시 실행합니다. | [설치](docs/ko/getting-started/installation.md) |
| 도움말 또는 버전 명령이 실패합니다. | 실제로 실행 가능한 `volicord`와 `volicord-mcp`가 들어 있는 디렉터리를 선택합니다. | [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| Runtime Home과 Product Repository가 겹칩니다. | 조상-자손 관계가 없는 별도 경로를 선택합니다. SQLite를 편집해서 고치지 마세요. | [런타임 경계](docs/ko/reference/runtime-boundaries.md) |
| 프로젝트 범위 설정이 `.mcp.json` 또는 `.codex/config.toml` 쓰기를 거부합니다. | 저장소 쓰기가 의도한 일인지 결정한 뒤에만 `--allow-repository-write`를 포함해 다시 실행합니다. | [관리 CLI](docs/ko/reference/admin-cli.md) |
| 결과가 `action_required`입니다. | 이름 붙은 호스트 소유 trust, approval, reload, restart, OAuth, 또는 실행 파일 가용성 동작을 완료한 뒤 `volicord agent verify`를 실행합니다. | [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| 결과가 `partial_failure`입니다. | `effects`와 `residual_effects`를 읽고, 다시 시도하기 전에 이름 붙은 문제만 고칩니다. | [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| 결과가 `failed`입니다. | 보고된 오류를 고치고, 가능하면 다른 쓰기 전에 dry-run을 실행한 뒤 install 또는 verify를 다시 시도합니다. | [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| 프로젝트 범위 호스트가 `volicord-mcp`를 찾지 못합니다. | 프로젝트 파일은 이식 가능한 형태로 유지하고 이후 호스트 프로세스의 `PATH`를 고칩니다. | [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| Generic export가 계속 `action_required`로 남습니다. | 내보낸 구성을 외부 호스트에 직접 적용합니다. Volicord는 그 호스트의 로드 상태를 관찰할 수 없습니다. | [에이전트 호스트 설정](docs/ko/guides/agent-host-setup.md) |

설정 오류의 첫 대응으로 Runtime Home, Product Repository, 아티팩트 저장소, Core
기록, 관련 없는 호스트 항목, 사용자가 편집한 지침을 삭제하지 마세요. 구체적인
문제를 이름 붙이는 status, dry-run, verification 명령을 선호합니다.

<a id="documentation-routes"></a>
## 문서 경로

| 필요 | 경로 |
|---|---|
| 영어 문서 홈 | [docs/en/README.md](docs/en/README.md) |
| 한국어 문서 홈 | [docs/ko/README.md](docs/ko/README.md) |
| 문서 디렉터리 안내 | [docs/README.md](docs/README.md) |
| 첫 제품 방향 잡기 | [시작하기 개요](docs/ko/getting-started/overview.md) |
| 빌드와 실행 파일 확인 | [설치](docs/ko/getting-started/installation.md) |
| 가장 짧은 첫 호스트 경로 | [빠른 시작](docs/ko/getting-started/quickstart.md) |
| 전체 호스트 설정과 generic export | [에이전트 호스트 설정](docs/ko/guides/agent-host-setup.md) |
| 첫 설치 복구 | [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| 사용자 작업 흐름 | [사용자 가이드](docs/ko/guides/user-workflow.md) |
| 여러 저장소 | [다중 저장소 에이전트 설정](docs/ko/guides/multi-repository-agent-setup.md) |
| 에이전트 작업 흐름 | [에이전트 가이드](docs/ko/guides/agent-workflow.md) |
| 소스 코드 학습 | [개발자 문서](docs/ko/development/README.md) |
| 참조 계약 | [참조 색인](docs/ko/reference/README.md) |
| 관리 CLI 계약 | [관리 CLI](docs/ko/reference/admin-cli.md) |
| MCP 프로세스 계약 | [MCP 전송](docs/ko/reference/mcp-transport.md) |
| 런타임 위치 경계 | [런타임 경계](docs/ko/reference/runtime-boundaries.md) |
| 공개 API 메서드 목록 | [API 메서드](docs/ko/reference/api/methods.md) |

`docs/doc-index.yaml`은 담당 경로, 유지되는 경로, 적용 가능성, 의존성, 한영
유지보수를 위한 유지보수 메타데이터입니다. 일반 런타임 구성이 아니며 새
사용자가 처음 읽어야 하는 문서도 아닙니다.
