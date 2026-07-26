# 검증

유지 문서를 편집한 뒤에는 이 정책을 사용합니다. 이 문서는 구조 점검, 사람이
하는 의미 검토, Rust 구현 검증, 결과 보고를 구분합니다.

이 검증은 유지보수 검증입니다. Volicord 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 증명, 잔여 위험 수락이 아닙니다. 저장소 로컬 자동 문서
검증기는 아래 명령입니다.

```sh
cargo run -p xtask -- docs-check
```

Clap 명령 모델에서 관리 CLI의 문법 전용 영역을 다시 생성할 때는 다음 명령을
사용합니다.

```sh
cargo run -p xtask -- docs-sync
```

## 구조 점검

문서 메타데이터, 경로, 링크, 용어 경로를 바꿨다면 저장소 루트에서
`cargo run -p xtask -- docs-check`를 실행합니다. 이 명령은 읽기 전용이며
기계로 확인할 수 있는 형태를 검증합니다.

- `docs/doc-index.yaml`이 YAML로 파싱되고 `version: 3`을 갖습니다.
- 필요한 최상위 섹션이 있으며 지원되지 않는 최상위 필드는 거부됩니다.
- `owner_areas` 카탈로그는 안정적인 식별자와 문자열 설명을 사용합니다. 적용
  가능성 키는 밑줄로 구분한 소문자 의미 단어만 사용하고 버전 번호를 포함하지
  않습니다.
- 모든 적용 가능성 항목은 지원되는 `version_source` 하나를 선언합니다.
  `docs-check`는 현재 작업 공간 패키지와 Rust 값을 루트 `Cargo.toml`에서,
  MCP 프로덕션 revision을 `ProtocolRegistry`에서, 메타데이터 스키마 값을
  `docs/doc-index.yaml` 또는 `docs/terminology-map.yaml`에서 읽습니다.
- `default_applicability`는 비어 있지 않고 중복이 없으며 모든 값이 적용 가능성
  카탈로그로 해석되는 목록입니다.
- `entry_schema`는 현재 적용 가능성 설명, 공유·대응 필수 필드, 선택 필드,
  유지보수 필드, 문서 종류, 독자 여정, 규범 수준, 번역 정책만 정확히 선언합니다.
- 모든 공유 항목은 `doc_id`, `path`, `kind`, `summary`, `normative_level`,
  `owner_area`, `created_on`, `last_updated_on`, `last_verified_on`,
  `applies_to`, `primary_audience`, `journeys`, `canonical_for`,
  `depends_on`만 사용합니다.
- 모든 대응 항목은 `doc_id`, `path_en`, `path_ko`, `kind`, `summary`,
  `normative_level`, `translation_policy`, `owner_area`, `created_on`,
  `last_updated_on`, `last_verified_on`, `applies_to`, `primary_audience`,
  `journeys`, `canonical_for`, `depends_on`만 사용합니다.
- 공유 항목과 대응 항목에 필요한 필드가 있으며 `applies_to`는 선택
  필드입니다.
- `owner_area`는 최상위 담당 영역 카탈로그로 해석됩니다.
- `applies_to`가 있으면 비어 있지 않고 중복이 없는 추가 카탈로그 값 목록이며
  루트 기본값을 반복하지 않습니다.
- `created_on`, `last_updated_on`, `last_verified_on`은 유효한
  `YYYY-MM-DD` 달력 날짜이며 `created_on <= last_updated_on <= last_verified_on`
  순서를 지킵니다.
- `kind` 값은 `landing`, `tutorial`, `how_to`, `explanation`, `reference`,
  `maintenance`만 사용합니다.
- `normative_level` 값은 `contract`, `guide`, `example`, `maintenance`만
  사용합니다.
- 유지되는 영어/한국어 대응 쌍의 `translation_policy`는
  `semantic_parity`입니다.
- `primary_audience`, `journeys`, `canonical_for`, `depends_on`은 있을 때
  목록입니다.
- `doc_id` 값은 고유합니다.
- 색인된 모든 경로가 존재합니다.
- 모든 `depends_on` 값이 색인된 `doc_id`로 해석됩니다.
- `docs/en/`과 `docs/ko/` 아래의 모든 유지되는 대응 Markdown 파일이 같은 상대
  구조의 항목으로 색인되어 있습니다.
- 정확한 루트 쌍 `README.md`와 `README.ko.md`만 유지되는 루트 수준 의미 일치
  대응 쌍으로 허용됩니다.
- `README.ko.md`가 있으면 루트 README 쌍으로 `README.md`와 함께 색인되어야
  하며, 색인된 루트 README 경로가 없을 때는 일반 경로 존재 규칙으로 보고됩니다.
- 루트 README 쌍에도 다른 색인 경로와 같은 기존 파일 규칙과 중복 경로 규칙이
  적용됩니다.
- 대응 문서는 같은 제목 수준 순서를 유지합니다.
- `docs/terminology-map.yaml`에서 식별한 코드 리터럴은 대응 제목이나 절 의미
  단위에 남아 있어야 합니다. 이 검사는 용어 카탈로그가 지정한 정확한 식별자만
  인라인 코드나 코드 펜스에서 찾아 비교합니다.
- 상대 링크가 존재하는 파일로 해석됩니다.
- 조각 링크와 숨김 앵커가 사용되는 곳에서 해석됩니다.
- 유지되는 영어/한국어 대응 쌍의 로컬 Markdown 독자 경로 링크가, 색인된 대상은
  `doc_id`로 정규화하고 유효하지만 색인되지 않은 저장소 대상은 저장소 상대
  경로로 정규화하며 조각을 보존한 뒤 동등한지 확인됩니다. 정확한 루트 README
  쌍도 같은 로컬 의미 링크와 조각 일치 메커니즘을 사용합니다. 이 일치 점검에서는
  외부 링크, 이미지, 코드 펜스 안의 텍스트를 무시합니다.
- 실행 가능한 `volicord` 예시는 명시적인 `sh cli-example` 펜스를 사용하며 실제
  공개 Clap 명령 모델로 parsing됩니다. 일반 셸 펜스, `text` 펜스, 표시 출력만
  보고 실행 가능한 CLI 예시라고 추론하지 않습니다.
- 생성된 관리 CLI synopsis 영역은 현재 공개 Clap 명령 트리와 일치하며 숨겨진
  내부 명령을 제외합니다.
- 신원에 민감한 용어의 역할 메타데이터는 허용된 역할 집합을 사용하고, 공개
  선택자, 저장소 내부, MCP 프로세스 바인딩, 진단에 필요한 역할을 포함합니다.
- `docs/terminology-map.yaml`의 `primary_owner`와 `related_references` 경로가
  존재하고 `doc-index.yaml`에 표현되어 있습니다.
- 문서 정책이 표면 라벨을 요구하는 집중 참조 담당 문서는 표면 안정성 섹션을
  포함하고, 기준 어휘로 연결하며, `stable`, `beta`, `internal`,
  `diagnostic` 라벨만 사용합니다.
- 공개 출력 소스는 Volicord 보장을 과장할 수 있는 넓은 보안 단어를 한정 없이
  사용하지 않습니다. 정확한 보안 보장 의미는 보안 담당 문서와 브랜드 주장
  담당 문서에 남아 있습니다.
- 유지되는 산문과 공개 출력 소스는 호스트 전체, 프로필 전체, Agent Connection에 대한
  모호한 지원 형용사와 서술어를 피합니다. 이 점검은 단어 경계를 사용하므로 에이전트
  호스트, 이름 붙은 관리 호스트, 프로필, 연결 전체에 대한 주장을 다른 단어의 부분 문자열
  때문에 오인하지 않습니다. `unsupported_by_host` 같은 정확한 식별자와 상태 값, 호스트가
  미지원임을 명시하는 문장은 허용합니다.

자동 구조 검증 뒤에는 저장소 위생을 사람이 확인합니다.

- 생성된 기록, 런타임 홈, SQLite 파일, 생성 로그, 보관 사본, 변환 메모, 부수
  메모, 작업용 목록, 작업 로그가 유지 문서에 남아 있지 않습니다.

## 사람이 하는 의미 검토

한영 변경에서는 영어와 한국어를 의미 단위로 비교합니다. 독자 목적, 규범 강도,
담당 경로, 기준 범위와 지원 범위 밖 경계, 사용자 판단 경계, 부정 절, 비주장,
보장 강도, 제목, 표, 목록, 예시, 링크, 정확한 식별자를 보존합니다.

계약과 가까운 편집에서는 정확한 API 동작, 스키마 의미, 오류 의미, 저장 효과,
보안 표현, 접근 경계, 닫기 준비 상태 의미, 값 집합 의미, Core 권한 의미가 집중
참조 담당 문서에 남아 있는지 확인합니다. 담당 문서가 아닌 곳은 요약하고
링크해야 하며 두 번째 계약 본문이 되면 안 됩니다.

용어 변경에서는 정확한 식별자, 선호 표현, 피해야 할 표현, 한국어 혼합어 통제,
담당 경로 무결성을 용어 지도에서 확인합니다.

브랜드 표현이나 넓은 주장 문구를 바꿀 때는 [브랜드 지침](brand-guidelines.md)에서
Volicord 표기, 공식 한영 브랜드 문구, 구성 요소 표현, 테스트 하네스 용어 경계,
시각 원칙, 주장 제한을 확인합니다. 정확한 제품 동작, API 동작, 저장 효과,
스키마, 보안 보장, Core 권한 의미가 계속 참조 담당 문서로 연결되는지 확인합니다.

관리 호스트 주장은 각 문장을 환경 적용 가능성, 설정 또는 구성 상태, 행동 기반 연결
검증, 운영 session 권한으로 나누어 검토합니다. 각 계층은
[시스템 요구사항](../reference/system-requirements.md),
[Agent Connection](../reference/agent-connection.md#validated-agent-session)을 기준으로
확인합니다. 설정, 구성, 구현, fixture, 테스트에 관한 사실은 현재 관리 session을
성립시키지 않습니다. 성공한 관찰은 현재 구성과 환경에서 검사한 동작만 설명하며,
미래 동작에는 새로운 관찰이 필요합니다.

API와 참조 예시는 필요할 때 메서드 안의 정합성, 요청과 응답 형태, 필드 이름,
필수 필드, `null` 허용 여부, enum 형태 값, `state_version`, 참조, 아티팩트
참조, 실행 참조, 판단 참조, 닫기 차단 사유, 응답 분기, 적용되는 담당 문서
링크를 확인합니다.

코드 이동 때문에 아키텍처 가이드 문서가 바뀌었다면 관련 문서가 오래 유지될
크레이트, 모듈, 진입점, 실행 단계, 책임 경계를 설명하는지 확인합니다. 구현
세부사항을 제품 계약 문구로 바꾸지 않습니다.

자동 `docs-check` 명령에는 유지되는 영어/한국어 대응 쌍의 로컬 문서 링크 일치,
제목 수준 구조 일치, 용어 기반 정확한 식별자 일치 점검이 포함됩니다. 하지만
전체 한영 의미 검토, 계약 담당 문서 검토, 기술 정확성 검토, 번역 판단, API
예시 정합성 검토, 제품 의미 검토를 수행하지 않습니다. 나머지 점검은 계속
사람이 하고 담당 문서로 경로를 잡습니다.

## 오래 유지될 테스트

문서나 구현 변경 때문에 새 자동 점검을 고려할 때는 현재 허용되는 오래 유지될
동작, 계약, 상태 전이, 사용자 가치, 안정적인 추상화 경계, 유지되는 검증 규칙을
검증하는 점검만 저장소에 둡니다.

파일 길이, 문서 길이, LOC 수는 오래 유지될 품질 점검이 아닙니다. 품질 기준
경계는 [제품 및 유지보수 헌장](product-maintenance-charter.md)을 사용하고, 담당
경로, 계약, 링크, 예시, 상태 전이, 독자 사용성을 검증하는 점검을 선호합니다.

구현 계층 배치와 테스트 작성 예시는 [테스트 전략](../architecture-guide/testing-strategy.md)을
사용합니다. 이 검증 정책은 그런 점검의 유지보수 점검, 검토, 보고 경계를
담당합니다.

닫힌 표면을 검사하는 테스트는 긍정적인 현재 형태를 검증합니다.

- CLI 도움말은 해당 명령의 현재 공개 옵션 허용 목록만 노출합니다.
- 유지되는 셸 예시는 지원되는 `volicord` 명령과 옵션을 사용합니다.
- 저장소 스키마 점검은 현재 기준 SQL, 테이블, 컬럼, 인덱스, 제약, 초기화,
  검증 동작을 확인합니다.
- MCP 사전 점검과 전송/스키마 점검은 현재 시작 동작, 공개 도구 노출, 공개 스키마
  형태를 검증합니다. 공개 MCP 스키마는 내부 envelope와 호출 필드를 숨기는
  안정적인 추상화 계약을 계속 지켜야 합니다.
- 용어 검증은 `connection_id`나 `project_id` 같은 식별자에 대해 넓은 산문 금지어
  검색을 추가하지 않고, 신원에 민감한 역할 메타데이터를 확인합니다.

오래 유지될 테스트 이름은 현재 계약을 기준으로 짓습니다. 예시는
`connect_help_exposes_only_public_connect_options`,
`documented_volicord_commands_match_public_cli_contract`,
`export_help_lists_authority_bundle`,
`mcp_public_schema_hides_internal_envelope_fields`,
`terminology_map_defines_identity_sensitive_roles`,
`storage_registry_contains_current_contract_columns`입니다.

## 유지보수성 보고서

검토자가 크거나 복잡한 저장소 표면을 빠르게 파악해야 할 때 유지보수성 보고서를
사용합니다.

```sh
cargo run -p xtask -- maintainability-report
```

이 보고서는 검토자 안내입니다. 가장 큰 Rust 파일, 테스트 파일, Markdown 파일,
명령 파싱/실행/렌더링 신호가 함께 보이는 휴리스틱 결과, 쉽게 추론할 수 있는
테스트 범위 힌트 같은 신호를 나열합니다. LOC 제한, LOC 예외 허용 목록, 긴 파일이
무효라는 상태, 또는 응집된 파일을 줄 수만으로 나누라는 요구를 정의하지 않습니다.
보고된 크기와 신호는 담당 범위, 가독성, 테스트 범위, 소스 구조에 관해 검토자가
질문할 계기로 다룹니다.

CI에서 이 보고서를 실행할 때 실패 종료 상태는 명령이 저장소를 검사하지 못했다는
뜻입니다. 큰 파일, 긴 문서, 혼합 신호, 테스트 범위 힌트 자체는 CI 실패가 아닙니다.

## 온보딩 사용성 검증

온보딩, 설치, Codex 설정, 문제 해결, 담당 경로가 실질적으로 바뀌면 대표 사용자를
대상으로 사용성 검증을 수행합니다. 이는 사람이 참여하는 검증이며 자동 검사나 에이전트의
문서 검토가 실제 참여자를 대신하지 않습니다.

처음 사용하는 운영자가 다음 작업을 완료할 수 있는지 확인합니다.

1. 문서에 나온 플랫폼과 저장소 토폴로지가 지원되는지 판단합니다.
2. 정확한 `volicord`와 Codex 실행 파일을 설치하거나 선택합니다.
3. `personal` 또는 `shared` 범위에서 `record` 프로필의 `codex` Agent Connection을
   프로비저닝합니다.
4. 연결을 검증하고 성공이 아닌 각 결과를 해석합니다.
5. `volicord inbox`로 대기 중인 `UserActionRequest`를 찾고
   `volicord inbox resolve`로 해결한 뒤 에이전트 작업을 재개합니다.
6. Volicord가 관리하는 구성만 복구하거나 제거합니다.
7. 정확한 계약의 집중 스키마 또는 저장소 담당 문서를 찾습니다.

결과는 유지 문서 밖에 기록합니다. 참여자 메모, 스크린샷, 녹화, 자격 증명, 대화
기록, Runtime Home, 꾸며 낸 완료 주장을 커밋하지 않습니다.

## 릴리스와 호스트 스모크 검증

Volicord 릴리스 검증은 일반적인 다섯 target 빌드, 패키지, checksum, binary smoke,
플랫폼, Docker, 게시 경로를 다룹니다. 운영 Codex 상호운용성 검증은 현재 관리 구성과
환경의 동작을 관찰하는 별도 검증입니다.

오래 유지되는 릴리스 패키징 저장소 점검은 다음과 같습니다.

```sh
cargo test --locked -p volicord-release-integrity-tests --all-targets --all-features
cargo run --locked -p volicord-release-smoke -- --bin <path-to-built-volicord>
```

이 테스트는 target 범위, 버전 일치, 기준 텍스트 바이트, archive 형태, 패키징한
binary identity, checksum 출력, workflow 의미를 보호합니다. Workflow 검증은 parsing한
action identity, matrix input, step 순서, 호출 수를 검사하며 완전한 shell 명령 하나를
비교하지 않습니다.

게시하지 않는 `tests/release-smoke` 패키지는 플랫폼 공통 실제 바이너리 하네스를
담당합니다. 폐기 가능한 Product Repository, Runtime Home, 안정적인 테스트 소유 Codex
fixture를 사용하며 한도가 있는 프로세스 실행과 정리는 `volicord-test-process`에
위임합니다. 로컬 composite action `.github/actions/volicord-release-smoke`가 하나뿐인
workflow 호출 경계입니다. 일반 CI는 빌드한 debug 바이너리를 정확히 한 번 전달합니다.
네이티브 릴리스 패키징 matrix의 각 항목도 artifact staging 전에 해당 target용으로 이미
빌드한 정확한 Linux, macOS, Windows 바이너리를 정확히 한 번 전달합니다. 스모크는 공개
`volicord mcp serve`를 사용하므로 session은 `manual_cli`로 남으며 managed-host 증거가
아닙니다.

실제 Codex 설치를 사용하는 선택적 smoke 실행은 관리형 구성, MCP 초기화, 필수 도구
검색, 안전한 도구 왕복, Guard 관찰을 확인할 수 있습니다. 결과는 해당 구성과 환경에
대한 운영 관찰로만 취급합니다. 보고된 Codex 버전은 진단 정보이며 버전 변경 시 운영
관찰을 다시 수행해야 합니다. smoke 인프라가 없으면 건너뜀 또는 사용할 수 없음으로
보고하며 일반 Volicord 릴리스 결과를 바꾸지 않습니다.

저장소의 운영 테스트는 제한 안의 임의 version 문자열을 연결 검증까지 전달하고,
initialize 및 도구 목록 milestone을 실행하며, 필수 도구와 안전 호출, Guard artifact와
필수 phase 관찰, session 소유권, revision 격리를 점검합니다.

실제 smoke는 폐기 가능한 Product Repository와 Runtime Home 경로에서만 실행합니다.
자격 증명, prompt, transcript, token, screenshot, 런타임 데이터는 저장소 밖에
둡니다. [Agent Connection](../reference/agent-connection.md)의 협력적 호스트 경계를
유지합니다. 성공한 행동은 사람 identity, process identity, 미래 호스트 행동,
관찰한 왕복 밖의 정책 준수를 증명하지 않습니다.

## Rust 구현 검증

Rust 소스, Cargo 매니페스트, 테스트, 픽스처, 빌드 설정을 바꾸지 않았다면 Rust
검증은 필요하지 않습니다.

Rust 구현을 편집한 뒤에는 워크스페이스나 변경된 크레이트에서 적용되는 Rust
검증을 실행합니다.

- `cargo fmt`
- `cargo clippy --all-targets --all-features`
- `cargo test --all-targets --all-features`

더 좁은 Cargo 명령은 저장소 구조나 작업 범위가 분명히 요구할 때만 사용하고 그
이유를 보고합니다.

## 생성 참조와 계약 드리프트 점검

생성되었거나 원본에서 파생되는 참조 표면은 안정적인 점검 명령을 사용합니다.

- `cargo run -p xtask -- docs-sync`는 영어와 한국어 관리 CLI 담당 문서에서 표시된
  문법 영역만 결정적으로 교체합니다. 명령 모델을 변경한 뒤 실행하고 생성 diff를
  검토합니다.
- `cargo run -p xtask -- docs-check`는 유지 문서 구조, 생성 또는 원본 파생 문서
  표면, 실행 가능한 `volicord` 명령 예시, 한영 링크·제목·정확한 식별자 일치,
  용어 메타데이터 담당 경로와 역할, 그리고
  `crates/volicord-store/src/schema/registry.sql` 및
  `crates/volicord-store/src/schema/project.sql`에 대한 기준 Storage DDL SQL
  블록을 점검합니다.
- `cargo test -p volicord-integration-tests --test public_contract_snapshots`는 API 요청
  스키마 투영과 MCP `workflow`/`read_only` 도구 투영의 생성 공개 계약 스냅샷이
  Rust 원본과 일치하는지 점검합니다.
- 의도적인 원본 변경 뒤 공개 계약 스냅샷을 다시 생성하려면
  `VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 cargo test -p volicord-integration-tests --test public_contract_snapshots`
  를 실행하고 `tests/integration/snapshots/` 아래의 생성 파일을 검토합니다.

공개 계약 스냅샷 파일은 `_generated`로 표시된 생성 테스트 산출물입니다. 손으로
편집하지 말고 먼저 스키마 또는 MCP 원본을 바꾼 뒤 다시 생성합니다. CLI 공개 명령
드리프트는 별도의 CLI JSON 스키마를 새로 만들지 않고 실행 가능한 문서 예시와
`volicord-cli`의 `binary_admin`, `mcp_transport` 같은 CLI 도움말/출력 테스트
대상으로 계속 다룹니다.

## 저장소 DDL 계약 점검

저장소 DDL, `volicord-store` 기준 SQL, 스키마 검증 코드를 편집했다면 담당
문서와 구현 사이의 정합성을 확인하는 집중 점검을 실행합니다.

```sh
cargo test -p volicord-store --test storage_ddl_contract
```

이 점검은 권위 있는 영어와 한국어 저장소 DDL SQL을 기준 registry/project SQL에서
초기화한 인메모리 SQLite 데이터베이스의 스키마와 비교합니다. Markdown 산문이나 SQL
서식을 비교하지 않고 테이블, 열, 기본값, 제약, 외래 키, 인덱스, 부분 인덱스, 유지되는
트리거 같은 스키마 의미를 확인합니다.

저장소 문서 점검은 영어와 한국어 저장소 DDL의 표시된 기준 SQL 블록이 기준
registry/project SQL 원본 파일과 일치하는지도 확인합니다.

이 점검은 저장소 유지보수와 구현 정합성 점검입니다. 일반 문서 구조 검증, 공개
런타임 적합성, 제품 수락, QA 완료, 닫기 준비 상태, 보안 증명, 잔여 위험 수락과
구분됩니다.

## 보고

검증 결과는 저장소 파일이 아니라 대화에 보고합니다. 변경 파일, 수행한 점검,
결과, 건너뛴 점검과 이유, 남은 문서 위험을 포함합니다.

`PASS`, `WARN`, `FAIL`, `SKIP`은 문서 유지보수 또는 구현 점검 결과로만
사용합니다. 통과한 검증 단계를 Volicord 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 보장, 잔여 위험 수락으로 설명하지 않습니다.
