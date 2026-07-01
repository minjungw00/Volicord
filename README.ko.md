# Volicord

**AI가 움직여도, 판단은 사용자에게.**

[English](README.md) | **[한국어](README.ko.md)**

## 개요

Volicord(볼리코드)는 AI 지원 제품 작업을 위한 로컬 작업 권한 시스템입니다. Codex나
Claude Code 같은 에이전트 호스트가, 대화에만 남기면 안 되는 작업 사실을 로컬 기록으로
다룰 수 있게 합니다. 어떤 작업이 활성 상태인지, 현재 범위에서 어떤 쓰기가 호환되는지,
어떤 증거가 있는지, 어떤 판단이 아직 사용자에게 남아 있는지, 정직한 닫기를 무엇이 막는지
기록합니다.

Volicord는 에디터, 셸, 테스트, 코드 리뷰, 사용자 판단을 대체하지 않습니다. Volicord는
에이전트가 그런 도구를 쓰는 동안 범위, 증거, 사용자 결정, 닫기 차단 사유를 다듬어진
요약 안에 숨기지 않도록 돕는 보호된 로컬 권한 계층입니다.

Core는 Volicord 상태의 로컬 기준 기록입니다. 대화 메시지, 생성된 Markdown, 상태 요약,
상태 보기는 Core 상태를 설명할 수 있지만 대신하지는 않습니다.

## Volicord가 존재하는 이유

Volicord는 AI 지원 제품 작업 중 아래 질문들이 분명하게 남아 있도록 돕습니다.

- 에이전트가 하려는 일은 무엇인가?
- 무엇이 범위 안이고 범위 밖인가?
- 현재 주장을 뒷받침하는 증거는 무엇인가?
- 현재 적용 범위에서 쓰기는 준비되었는가?
- 에이전트가 무엇을 실행하거나 기록했는가?
- 아직 필요한 사용자 소유 판단은 무엇인가?
- 정직하게 닫는 것을 아직 막는 것은 무엇인가?

AI 에이전트는 사람이 모든 경계를 작업 기억에 붙잡아 두는 속도보다 빠르게 파일을
살피고, 도구를 실행하고, 코드를 고치고, 결과를 요약할 수 있습니다.

그 속도는 유용하지만, 오래 남는 기록이 대화에만 있으면 경계가 흐려질 수 있습니다.
범위가 조금씩 넓어지고, 수락이 암시된 것처럼 보이고, 잔여 위험이 대화에서 사라지고,
제품 결정이 구현 단계 안에 묻힐 수 있습니다.

Volicord는 범위, 증거, 쓰기 준비 상태,
사용자 판단, 실행 기록, 닫기 준비 상태가 서로 다른 작업 사실로 계속 보이도록
존재합니다.

## 짧은 모델

README의 나머지 내용을 읽을 때는 아래 모델을 사용합니다.

| 개념 | 첫 사용자에게 필요한 의미 |
|---|---|
| `Task` | 구체화되거나, 작업 중이거나, 막혀 있거나, 닫히는 사용자 가치 단위입니다. 현재 목표, 범위, 범위 밖 항목, 현재 작업 경계를 담습니다. |
| 쓰기 | 제품 파일 변경은 현재 `Task`와 현재 범위에 호환되어야 합니다. `Write Check`은 제안된 쓰기 하나에 대한 좁은 Volicord 호환성 기록이며, OS 권한이나 최종 승인이 아닙니다. |
| 증거 | 실행, 관찰, 아티팩트 참조처럼 특정 주장을 뒷받침하도록 기록된 자료입니다. 증거는 주장을 돕지만 사용자 판단이나 정확성 증명이 되지는 않습니다. |
| 사용자 판단 | 제품 방향, 중요한 기술 방향, 범위, 민감 동작, 최종 수락, 잔여 위험 수락, 취소처럼 사용자에게 속한 결정입니다. |
| 닫기 | 현재 `Task`를 미해결 요구사항을 숨기지 않고 정직하게 끝낼 수 있는지 확인하는 일입니다. 닫기 준비 상태는 판단을 돕는 자료이지 제품 결과가 옳다는 증명이 아닙니다. |

## 설치와 초기화

일반 사용자 경로는 설치된 `volicord` 실행 파일 하나를 사용하는 것입니다. 시스템이 지원
target과 맞으면 릴리스 바이너리 설치가 기본 경로입니다. 소스 빌드는 개발용입니다.

Volicord 릴리스 자산을 게시하는 저장소에서 `scripts/install.sh`를 내려받거나 복사한 뒤,
릴리스 바이너리를 설치합니다.

```sh
VOLICORD_REPO=OWNER/REPO sh ./scripts/install.sh
volicord --version
```

`OWNER/REPO`는 이 체크아웃의 Volicord 릴리스 자산을 호스팅하는 GitHub 저장소입니다.
스크립트는 지원되는 Linux, WSL2, macOS target을 감지하고, target 이름이 붙은 tarball을
내려받으며, 사용할 수 있을 때 `.sha256` 파일을 검증하고, `volicord` 하나만 설치합니다.
셸 시작 파일은 편집하지 않습니다. 이 체크아웃에는 Homebrew tap, Homebrew formula, Linux
패키지, 외부 패키지 registry 설치 경로가 없습니다.

미래의 에이전트 호스트가 `PATH`를 통해 `volicord`를 실행할 수 있게 한 뒤, 에이전트에게
작업을 요청할 Product Repository를 초기화합니다.

```sh
volicord init --host codex --repo /path/to/your-product-repo
```

Claude Code에는 `--host claude-code`를 사용합니다.

```sh
volicord init --host claude-code --repo /path/to/your-product-repo
```

`volicord init`은 대화 중심 사용을 위한 기본 첫 실행 설정 및 연결 명령입니다. 필요하면
Runtime Home을 초기화하고, 설치 프로필을 기록하며, 선택한 Product Repository를 등록하거나
재사용하고, Agent Connection을 만들며, `volicord mcp --stdio`를 시작하는 프로젝트 범위
MCP 설정을 씁니다. 또한 Volicord가 관리하는 `AGENTS.md` 안내, `.volicord/policy.json`,
그리고 호스트가 지원하는 프로젝트 로컬 rule 관례가 있을 때 지원 호스트 rule 파일을
씁니다.

명령이 `action_required`를 보고하면 이름 붙은 호스트 통제 동작이나 로컬 동작을 따릅니다.
예를 들면 호스트 restart 또는 reload, 프로젝트 MCP 설정 승인, 프로젝트 trust, 명령
가용성 복구가 있습니다. 그런 뒤 확인합니다.

```sh
volicord connection verify codex --repo /path/to/your-product-repo
```

정확한 명령 동작은 [관리 CLI 참조](docs/ko/reference/admin-cli.md)가 담당합니다. 환경
지원은 [시스템 요구사항](docs/ko/reference/system-requirements.md)이 담당합니다.

## 개발용 소스 빌드

Volicord 자체를 개발하거나 로컬 개발 바이너리가 필요할 때만 소스 빌드 경로를 사용합니다.

```sh
cargo build --workspace --bins
./target/debug/volicord --version
./target/debug/volicord init --host codex --repo /path/to/your-product-repo
```

이 경로에는 [시스템 요구사항](docs/ko/reference/system-requirements.md#toolchain-requirements)이
이름 붙인 Rust 도구 체인이 필요합니다. 첫 사용자의 기본 설치 경로가 아닙니다.

## 일반 사용은 대화입니다

초기화 뒤에는 Product Repository에서 평소처럼 에이전트 호스트와 대화로 일합니다.
터미널에서 워크플로를 직접 몰고 갈 필요가 없습니다.

예를 들어 대화에서 이렇게 요청합니다.

```text
결제 생성에 idempotency key 지원을 추가하고, 테스트를 갱신한 뒤, 닫기를 아직 막는 것이 무엇인지 알려줘.
```

호스트는 계속 사용자의 대화/에디터 에이전트입니다. Volicord는 오래 남는 작업 상태가
필요할 때 호스트가 호출할 수 있는 로컬 MCP 도구를 제공합니다.

- `Task` 만들기 또는 갱신
- 현재 범위, 차단 사유, 증거, 대기 판단 보여 주기
- 제안된 제품 파일 쓰기 준비
- 아티팩트 스테이징과 실행 또는 관찰 기록
- 초점이 맞춰진 사용자 판단 요청
- 에이전트가 완료를 주장하기 전에 닫기 준비 상태 확인

에이전트는 사용할 수 있을 때 Volicord 상태를 사용하고, 사용할 수 없으면 그 사실을
명시적으로 말해야 합니다. Volicord 도구, MCP 서버 instructions, 호스트 rule,
`AGENTS.md` 안내는 에이전트를 유도하지만 모델 동작을 절대적으로 강제하지 않습니다.

## Guarded 모드

`volicord init`의 기본값은 `--mode guarded`입니다.

Guarded 모드는 MCP 워크플로 주변에 협력적이고 탐지적인 guard 표면을 더합니다.

| 표면 | 기여하는 것 |
|---|---|
| MCP | 저장된 Agent Connection과 허용된 Product Repository에 묶인 로컬 `volicord.*` 도구를 `volicord mcp --stdio`로 호스트에 제공합니다. |
| `AGENTS.md` | 상태 확인, 작업 시작, 쓰기 준비, 사용자 판단 요청, 닫기 확인, Volicord 도구 사용 불가 보고를 에이전트에게 요구하는 Volicord 관리 안내 블록을 더합니다. |
| `.volicord/policy.json` | session start, pre-tool, post-tool, prompt capture, stop 같은 지원 lifecycle hook을 위한 기계 판독 guard 명령 policy를 기록합니다. |
| 호스트 hook과 rule | 호스트가 지원하고 생성된 설정을 로드하면 hook은 맥락을 주입하고, 도구 시도를 분류하고, 안전하지 않아 보이는 일부 작업을 경고하거나 거부하고, 관찰된 미기록 변경을 기록하고, 엄격한 채팅 판단 명령을 캡처하고, 닫기 차단 사유가 남아 있을 때 stop을 막을 수 있습니다. Claude Code rule 같은 호스트 rule 파일은 호스트를 policy로 안내합니다. |

다른 모드도 있습니다.

- `--mode mcp-only`는 MCP 설정과 안내를 쓰지만 policy 메타데이터에서 guard 명령을
  비활성화합니다.
- `--mode managed`는 현재 `guarded`와 같은 설정 표면을 사용하며, 이를 구분하는 통합을
  위해 managed guard 모드를 기록합니다.

Guarded 모드는 호스트가 설정된 hook을 실제로 실행하고 rule을 존중할 때 우회를 줄입니다.
그래도 OS 수준 강제는 아닙니다. 도구를 샌드박스하지 않고, 모든 파일을 감시하지 않으며,
모든 명령을 차단하지 않고, 네트워크를 격리하지 않으며, 모델이 지침을 따랐다는 것을
증명하지 않습니다.

Guard 설치에는 파일 설치와 활성화라는 별도 단계가 있습니다. `volicord init`은 호스트
설정, Volicord 관리 `AGENTS.md` 안내, `.volicord/policy.json`, 호스트 hook 또는 rule
파일, guard 상태를 설치하거나 갱신합니다. 그래도 호스트가 그 파일을 실행하려면 reload,
restart, trust, approval이 필요할 수 있습니다. 처음으로 일치하는 guard hook 이벤트가
관찰되면 설치가 활성화됩니다. `volicord connection verify`와 `volicord doctor`는 파일
상태, 필요한 호스트 동작, 관찰된 활성화를 분리해서 보고합니다. 파일이 설치되었다는
사실만으로 hook이 활성 상태임이 증명되지는 않습니다.

## 미기록 변경과 닫기 차단 사유

Guarded hook은 제품 파일 변경이 대응되는 예상 쓰기와 맞지 않을 때 미기록
Product Repository 변경을 보고할 수 있습니다. 이런 항목은 조정될 때까지 guard 찾기로
남으며, 미해결 찾기는 닫기를 막습니다.

조정은 호환되는 `Write Check`나 기록된 실행이 이미 다루는 찾기처럼 결정적으로 해결할
수 있는 경우를 해결할 수 있습니다. 수락이 필요하면 Volicord는 초점이 맞춰진 사용자 소유
판단을 만듭니다. 사용자는 MCP elicitation, 엄격한 채팅 명령, CLI 복구 경로로 답합니다.
에이전트는 Product Repository 우회 찾기를 조용히 무시하거나 사용자를 대신해 수락한
것으로 표시할 수 없습니다.

채팅에서는 에이전트에게 `volicord.reconcile_changes` 결과와 다음 행동을 보여 달라고
요청합니다. CLI 복구 경로는 `volicord changes reconcile`입니다.

## 사용자 판단 캡처

사용자 판단은 사용자에게 남습니다. Agent Connection은 판단을 요청할 수 있지만,
권한을 지니는 사용자 답변을 사용자처럼 기록하면 안 됩니다.

지원되는 캡처 경로는 아래와 같습니다.

| 경로 | 쓰이는 때 |
|---|---|
| MCP elicitation | 초기화된 MCP client가 `capabilities.elicitation`을 선언하면 Volicord는 초점이 맞춰진 대기 판단에 대해 `elicitation/create` 요청을 보낼 수 있습니다. 유효한 응답은 사용자 출처로 로컬 `User Channel`을 통해 기록됩니다. |
| 채팅 prompt capture | elicitation을 사용할 수 없고 guarded prompt capture가 활성화되어 있으면 Volicord는 `Volicord: answer J-3 1 #AB7K`, `Volicord: answer J-3 reject #AB7K`, `Volicord: answer J-3 defer #AB7K`, `Volicord: note J-3 "text" #AB7K` 같은 정확한 채팅 명령을 반환합니다. prompt-capture hook은 현재 검증 코드가 있는 엄격하게 유효한 명령만 기록합니다. |
| CLI fallback | 채팅 캡처를 사용할 수 없거나 비활성화되어 있거나 수동 점검이 필요하면 Product Repository에서 `volicord user`를 사용합니다. |

CLI fallback 예시:

```sh
volicord user status
volicord user judgments
volicord user judgment show 1
volicord user judgment answer 1 1
```

이 체크아웃에는 별도의 로컬 웹 판단 UI가 문서화되어 있지 않습니다. 실험적 HTTP MCP serve
모드도 HTTP elicitation을 구현하지 않습니다.

## Volicord가 보장하지 않는 것

Volicord는 작업 권한을 보이게 하지만 일반 보안 제품이나 정확성 판정기가 아닙니다.
아래를 Volicord에 기대하면 안 됩니다.

- OS 수준 샌드박싱 또는 OS 권한 강제
- 악성코드 방어, 악성코드 검사, 비밀값 검사
- 네트워크 격리, 네트워크 모니터링, 네트워크 차단
- 모든 제품 파일 쓰기 예방
- 보편적 도구 실행 전 차단 또는 전체 파일시스템 모니터링
- 코드가 옳다는 증명
- 테스트가 충분하다는 증명
- 사람 리뷰, QA, 릴리스 판단, 위험 판단의 대체
- 외부 호스트가 `volicord mcp --stdio`를 신뢰, 승인, 로드, 초기화, 노출했다는 증명
- `AGENTS.md`, 호스트 rule, MCP instructions가 모델 동작을 강제했다는 증명

Guarded 모드는 설정된 hook을 통해 `warn` 또는 `deny` 결정을 반환할 수 있고, 닫기/쓰기
확인은 차단 사유를 드러낼 수 있습니다. 이것은 협력적인 로컬 제어이지 커널 수준 강제나
Volicord를 아는 경로 밖에서 도구가 파일을 쓸 수 없다는 보장이 아닙니다.

정확한 보장 표현과 명시적 비보장은 [보안 참조](docs/ko/reference/security.md)를 봅니다.

## Docker와 로컬 HTTP MCP

체크인된 `Dockerfile`을 통한 로컬 컨테이너 배치용 Docker 지원이 있습니다.

```sh
docker build -t volicord:local .
```

로컬 HTTP MCP 모드는 아래 명령으로 구현되어 있습니다.

```sh
volicord serve --transport streamable-http
```

이 모드는 Docker와 localhost MCP 사용을 위한 명시적 고급 모드이며 기본 호스트 설정
경로가 아닙니다. 기본값은 loopback이고, bearer 인증을 요구하며, `POST /mcp`를 노출합니다.
server-sent event 스트림, HTTP elicitation, 전체 MCP Streamable HTTP 호환성은 구현하지
않습니다. 인증 없는 네트워크 서비스처럼 다루면 안 됩니다.

자세한 Docker와 HTTP 경계는 [설치](docs/ko/getting-started/installation.md)와
[MCP 전송](docs/ko/reference/mcp-transport.md)을 사용합니다.

## 문제 해결

| 증상 | 할 일 |
|---|---|
| `volicord`를 찾지 못함 | 설치 디렉터리를 `PATH`에 넣거나 이미 `PATH`에 있는 디렉터리에 설치한 뒤 `volicord --version`을 다시 실행합니다. 미래의 에이전트 호스트도 `volicord`를 시작할 수 있어야 합니다. |
| `init`이 `action_required`를 보고함 | 호스트 restart 또는 reload, 프로젝트 trust, MCP approval, OAuth, 명령 링크 복구, 설치 프로필 복구처럼 이름 붙은 동작을 완료한 뒤 `volicord connection verify HOST --repo PATH`를 다시 실행합니다. |
| 호스트가 MCP를 시작하지 못함 | 같은 명령 경로로 호스트가 `volicord mcp --help`를 실행할 수 있는지 확인합니다. 설치 프로필 상태는 `volicord doctor`로 확인합니다. |
| Product Repository가 감지되지 않음 | `--repo /path/to/your-product-repo`를 넘기고, 그 경로가 Runtime Home과 분리된 기존 로컬 저장소인지 확인합니다. |
| 판단이 대기 중임 | 가능하면 호스트의 MCP elicitation이나 정확한 채팅 prompt-capture 명령을 우선 사용합니다. CLI fallback으로 `volicord user judgments`와 `volicord user judgment answer`를 사용합니다. |
| 닫기가 막힘 | 에이전트에게 `volicord.check_close` 결과, 대기 중인 사용자 판단, 빠진 증거, 미해결 미기록 변경, 잔여 위험을 보여 달라고 합니다. 요약으로 닫지 말고 이름 붙은 차단 사유를 처리합니다. |

## 더 읽을 문서

| 필요 | 읽을 문서 |
|---|---|
| 설치 세부사항과 Docker 예시 | [설치](docs/ko/getting-started/installation.md) |
| 지원 환경 | [시스템 요구사항](docs/ko/reference/system-requirements.md) |
| 사용자 작업 흐름과 판단 경계 | [사용자 가이드](docs/ko/guides/user-workflow.md) |
| 호스트 설정과 복구 | [에이전트 호스트 설정](docs/ko/guides/agent-host-setup.md)과 [에이전트 호스트 문제 해결](docs/ko/guides/agent-host-troubleshooting.md) |
| 정확한 CLI 동작 | [관리 CLI 참조](docs/ko/reference/admin-cli.md) |
| MCP stdio와 HTTP 전송 | [MCP 전송](docs/ko/reference/mcp-transport.md) |
| Agent Connection과 User Channel 경계 | [Agent Connection 참조](docs/ko/reference/agent-connection.md) |
| Core 권한 개념 | [Core 모델](docs/ko/reference/core-model.md) |
| 보안 표현과 비보장 | [보안 참조](docs/ko/reference/security.md) |
| 공개 API 메서드와 스키마 | [참조 색인](docs/ko/reference/README.md) |

Volicord 명령은 로컬 관리 명령이며 공개 Volicord API 메서드가 아닙니다. 정확한 공개 API
동작은 참조 문서가 담당합니다.
