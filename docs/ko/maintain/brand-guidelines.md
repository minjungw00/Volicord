# 브랜드 지침

이 유지보수 담당 문서는 저장소 문서, README, CLI/MCP 안내, 생성되는 저장소
지침, 이후 작성 작업에서 Volicord(볼리코드) 브랜드를 어떻게 이름 붙이고 보여
줄지 정의합니다.

이 문서는 런타임 동작, 공개 API 동작, 저장 동작, 보안 보장, 스키마, Core 권한
의미를 정의하지 않습니다. 정확한 제품 동작이 중요할 때는 [담당 경로](#owner-routes)에
나열된 참조 담당 문서로 연결합니다.

<a id="official-copy"></a>
## 공식 문구

| 항목 | 공식 형태 |
|---|---|
| 브랜드 이름 | `Volicord` |
| 한국어 발음 | `볼리코드` |
| 독립된 한국어 진입 문서의 첫 언급 | `Volicord(볼리코드)` |
| 브랜드 기억 문구 | `Volition, recorded.` |
| 영어 태그라인 | `AI moves. Judgment stays yours.` |
| 한국어 태그라인 | `AI가 움직여도, 판단은 사용자에게.` |
| 영어 제품 설명 | `A local work-authority system for AI-assisted product work.` |
| 한국어 제품 설명 | `AI 지원 제품 작업을 위한 로컬 작업 권한 시스템` |

핵심 메시지:

| 영어 | 한국어 |
|---|---|
| Scope stays explicit. | 범위는 분명하게. |
| Judgment stays with the user. | 판단은 사용자에게. |
| Evidence stays visible. | 근거는 보이게. |
| Closure stays honest. | 닫기는 정직하게. |

태그라인과 핵심 메시지는 독자가 Volicord의 방향을 이해하는 데 도움이 되는
브랜드, 온보딩, 표시 맥락에서 사용합니다. 운영 참조 계약, 오류 메시지, 일상적인
CLI 출력에 태그라인을 넣지 않습니다.

<a id="spelling-rules"></a>
## 표기 규칙

- 브랜드에는 `Volicord`를 사용합니다.
- `volicord`는 정확한 소문자 기술 식별자일 때만 사용합니다.
- 제품 이름이나 약어로 `VoliCord`, `Voli Cord`, `VOLI-CORD`, `Voli`, `VC`를
  사용하지 않습니다.
- [용어 지도](../../terminology-map.yaml)와 적용되는 담당 문서가 요구하는 정확한
  식별자, 파일 경로, API 메서드, 스키마 이름, 필드 이름, 상태 값, 제품 라벨,
  코드 리터럴은 보존합니다.

<a id="product-and-component-presentation"></a>
## 제품과 구성 요소 표현

- Volicord는 제품/시스템 브랜드입니다.
- Core는 계속 제품 개념이자 기준 기록 역할입니다. Core 이름을 바꾸거나 Volicord
  자체를 기준 기록으로 설명하지 않습니다.
- `volicord`는 관리 CLI 식별자입니다. 정확한 CLI 동작은 [관리 CLI](../reference/admin-cli.md)가
  담당합니다.
- `volicord mcp --stdio`는 로컬 MCP 어댑터 프로세스 식별자입니다. 정확한 MCP 프로세스, 전송,
  응답 래핑 동작은 [MCP 전송](../reference/mcp-transport.md)이 담당합니다.
- `Volicord Runtime Home`은 제품 라벨입니다. 정확한 런타임 위치와 저장소 경계
  동작은 [런타임 경계](../reference/runtime-boundaries.md)가 담당합니다.
- `Task`, Change Unit, `Write Check`, 최종 수락, 잔여 위험 수락, 닫기
  준비 상태 같은 도메인 개념에는 Volicord에서 파생한 장식적 이름을 붙이지
  않습니다.

참조 담당 문서가 정확한 식별자나 제품 라벨을 사용할 때는 그 담당 문서가 정의한
문자열을 사용 지점에서 보존합니다. 브랜드 표현은 API 메서드, 바이너리, 저장소
식별자, 환경 변수, 파일 경로, 스키마 값을 조용히 바꾸지 않습니다.

<a id="voice-and-claim-boundaries"></a>
## 문체와 주장 경계

영어에서는 record, distinguish, preserve, show, verify, identify처럼 의미가
분명한 동사를 선호합니다. 한국어에서는 기록하다, 구분하다, 보존하다, 보여 주다,
검증하다, 식별하다처럼 행위 경계가 분명한 표현을 사용합니다.

control, guarantee, secure, protect, monitor, approve, decide 같은 넓은 주장은
적용되는 계약 담당 문서가 정확한 주장을 뒷받침할 때만 제한적으로 사용합니다.
Volicord 표현은 연결된 참조 담당 문서가 정의한 것보다 강한 범위, 보안, 런타임,
Core 권한 보장을 암시하면 안 됩니다.

Volicord가 사용자 소유 판단을 대신한다고 설명하지 않습니다. Volicord는 사용자의
판단이 필요한 경계를 기록하고, 경로를 잡고, 보존하고, 보여 주는 데 도움을 줄 수
있지만 판단은 사용자에게 남습니다.

Volicord 권한 기록을 OS 수준 강제, 샌드박스, 에이전트가 지시를 따랐다는 증명으로
설명하지 않습니다. 오래 유지될 제품 정체성 경계는
[제품 및 유지보수 헌장](product-maintenance-charter.md)을 사용하고, 정확한 보장
표현은 집중 참조 담당 문서로 보냅니다.

테스트 성공, 쓰기 승인, 최종 수락, 잔여 위험 수락을 하나의 포괄적 승인으로
합치지 않습니다. 이 개념들은 구분하고, 정확한 의미는 [Core 모델](../reference/core-model.md)과
관련 API 담당 문서로 보냅니다.

<a id="visual-principles"></a>
## 시각 원칙

이 원칙은 프로젝트 안에서 사용하는 시각 정체성 지침입니다. 로고, 아이콘, 에셋
라이브러리, 상표 계획, 웹사이트, 릴리스 계획, UI 계약, 런타임 동작을 만들지
않습니다.

명시적 분리, 기록, 결정 분기, 보이는 상태 경계를 드러내는 시각 체계를 선호합니다.

방패, 자물쇠, 감시하는 눈, 군사적 통제, 로봇 대 인간, 케이블, 단일 체크 표시
모티프는 피합니다.

색만으로 상태를 전달하지 않습니다.

초기 프로젝트 토큰:

| 토큰 | 값 |
|---|---|
| Base ink | `#171A21` |
| Base background | `#F6F4EE` |
| Volicord indigo | `#3F4FD8` |
| Secondary gray | `#68707D` |

<a id="test-harness-term-boundary"></a>
## 테스트 하네스 용어 경계

`Volicord`는 제품 브랜드입니다. 일반 기술 용어 `test harness`는 테스트 기반
구성이나 보조 실행 체계를 뜻하며, Volicord의 제품 이름이나 약어로 사용하면 안
됩니다.

일반적인 테스트 픽스처, 실행기, 지원 체계를 말할 때만 `test harness`를 사용합니다.
한국어에서는 그 일반 기술 용어에만 `테스트 하네스`를 사용합니다. `Volicord`를
`test harness`나 `테스트 하네스`로 번역하지 않습니다.

<a id="owner-routes"></a>
## 담당 경로

정확한 동작과 보장 질문에는 아래 담당 문서를 사용하고, 그 계약을 브랜드 자료에
복사하지 않습니다.

| 질문 | 담당 문서 |
|---|---|
| 제품 범위와 지원되는 기준 경계 | [범위](../reference/scope.md) |
| Core 권한 개념, 사용자 소유 판단, 증거, `Write Check`, 수락, 잔여 위험, 닫기 준비 상태 | [Core 모델](../reference/core-model.md) |
| 런타임 위치, 제품 저장소 경계, Runtime Home 경계, 구성 요소/위치 분리 | [런타임 경계](../reference/runtime-boundaries.md) |
| 보안 표현, 보장 수준, 로컬 파일 접근 가정, 명시적 비보장 | [보안](../reference/security.md) |
| 관리 CLI 명령, 인자, 출력, 호스트 설정, 명령/API 경계 | [관리 CLI](../reference/admin-cli.md) |
| 로컬 MCP 어댑터 프로세스 시작, stdio 전송, 프로토콜 처리, 응답 래핑 | [MCP 전송](../reference/mcp-transport.md) |
| 문서 담당 경로와 메타데이터 | [문서 정책](documentation-policy.md), [doc-index.yaml](../../doc-index.yaml) |
| 한영 용어와 식별자 보존 | [번역 정책](translation-policy.md), [용어 지도](../../terminology-map.yaml) |
