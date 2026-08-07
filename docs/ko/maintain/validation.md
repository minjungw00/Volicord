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

현재 워크스페이스 패키지 그래프는 다음 명령으로 검증합니다.

```sh
cargo run -p xtask -- architecture-check
```

## 변경된 파일의 담당 경로 지정

넓은 범위를 탐색하기 전에 현재 Git 변경에 필요한 한정된 경로를 도출합니다.

```sh
cargo run -p xtask -- owner-route --changed
```

명시한 변경 series 기준 뒤의 commit된 변경과 staged, unstaged, untracked working
tree 경로를 함께 포함하려면 `--base <revision>`을 사용합니다. 다른 도구나
에이전트가 결과를 읽을 때는 `--json`을 추가합니다. 사람용 형식과 JSON 형식은
정렬된 같은 보고서에서 생성됩니다.

이 명령은 Git에서 변경 경로를, Cargo metadata에서 워크스페이스 패키지 identity를,
`docs/doc-index.yaml`에서 유지 문서 항목과 대응 언어 쌍을 읽습니다. 검증되는 지침,
직접 담당 문서, 검증 분류 연결은 `docs/owner-routing.yaml`에서 읽습니다. 적용되는
루트 및 범위별 `AGENTS.md`, 변경 패키지, 변경된 정확한 문서 항목과 대응 경로,
직접 담당 문서, 검증 분류, 유지 경로가 없는 경로를 반환합니다. 결과는 정렬되고
중복이 없습니다. 이 명령은 읽기 전용이며 임의 산문을 검색해 담당 범위를 추론하지
않습니다.

집중 검증은 담당 경로가 없는 모든 변경 경로를 담당 경로 사전 점검 실패로 처리하고
유지보수자에게 `docs/owner-routing.yaml`을 안내합니다. 알 수 없는 경로가 사실상 빈
성공 계획을 받을 수 없습니다. 루트 `Cargo.toml`이나 `Cargo.lock` 변경에는 항상
워크스페이스 아키텍처와 워크스페이스 컴파일 점검을 추가하지만, 집중 profile에
정확한 test aggregate를 추가하지는 않습니다.

## 검증 profile과 순차 series

순차 change series의 첫 commit 전에 그 상위 commit을 series 기준으로 기록합니다.
같은 series의 모든 profile 호출에 같은 명시적 revision을 사용합니다.

중간 작업에는 집중 profile을 실행합니다.

```sh
cargo run -p xtask -- validate focused --base <revision>
```

집중 profile은 `owner-route` 결과를 사용해 변경된 패키지, 직접 계약 및 생성 drift
점검, 문서와 아키텍처 점검, 저장소 위생을 선택합니다. `cargo test --workspace`나
다른 정확한 workspace aggregate를 실행하지 않습니다. 도구나 에이전트가 정확한
summary를 읽어야 하면 `--json`을 사용합니다.

계획한 모든 commit이 준비되고 working tree가 최종 gate를 실행할 상태가 되면 최종
검증 session 하나를 시작합니다.

```sh
cargo run -p xtask -- validate final --base <revision>
```

최종 profile을 미리 시험하거나 동시에 실행하거나 중간 commit마다 실행하지
않습니다. 이 session이 정확한 aggregate와 한도가 있는 진단을 포함한 현재의 완전한
저장소 정책을 담당합니다. 실패한 최종 session은 실패로 남습니다. 분해 결과를
완료로 설명하지 말고 보고한 뒤 수정된 series를 시작합니다.

`xtask::validation::current_plan`은 Linux 저장소 점검의 구성과 순서를 담당하는 현재의
typed 원본 하나입니다. 실행하지 않고 기계 판독 형태를 확인할 수 있습니다.

```sh
cargo run -p xtask -- validation-plan --json
```

최종 profile은 series 사전 점검 뒤에 이 계획을 사용합니다. 주 Linux CI job은 workflow
YAML에 명령을 반복하지 않고 `validate final --base HEAD`를 한 번 호출합니다. 플랫폼별
네이티브 운영 job은 별도로 유지합니다. 아래에 문서화된 직접 명령은 안정적인 집중 및
진단 점검으로 남고, profile이 series 수준 선택, 순서, 오래 남는 결과 수집, aggregate
처리, summary 상태를 담당합니다.

## 오래 남는 명령 결과

각 profile은 무시되는 `target/volicord-validation/<run-id>/summary.json`을
만듭니다. 실행한 각 명령은 같은 run directory 아래에 완전한 stdout log, stderr
log, 기계 판독 결과, 정확한 호출, working directory, 시작·종료 timestamp, exit
code를 남깁니다. 자식 stdout과 stderr는 실행 중에 그 파일로 직접 들어가므로
terminal buffer 연결 유지에 의존하지 않습니다.

Runner는 첫 명령 전에 초기 summary, `target/volicord-validation/active/` 아래의
활성 run locator, 참고용 `target/volicord-validation/latest-run.json` locator를
씁니다. 이어서 run ID와 summary 경로를 즉시 stderr에 보고하므로 `--json` stdout은
유효한 JSON 값 하나로 남습니다. 동시에 실행하는 run은 서로 다른 활성 record를
사용합니다. 정상적으로 끝난 run은 활성 record를 지우지만 중단된 run은 오래된
locator를 남길 수 있습니다.

Locator 파일은 찾기 정보일 뿐 검증 결과가 아닙니다. 상태와 exit code는 항상 locator가
가리키는 summary와 명령별 결과 record에서 확인합니다. Runner는 명령 상태가 바뀔
때마다 이 record를 전후로 checkpoint합니다. Runner가 계속 실행되는 동안 terminal이나
process handle을 잃었다면 이미 출력된 경로나 locator를 사용해 run을 복구합니다. 대기
중인 작업은 완료, 실패, 생략 명령과 구분됩니다. 검증 출력은 무시되는 빌드 출력이며
commit하지 않습니다.

## 정확한 aggregate와 한도 있는 분해

최종 profile만 다음 정확한 aggregate를 실행합니다.

```sh
cargo test --locked --workspace --all-targets --all-features
```

일반적으로 한 번 실행합니다. 출력에서 변경하지 않은 패키지의 실패 target 하나를
확인할 수 있으면 runner는 그 target과 전체 패키지를 실행할 수 있습니다. 둘 다
통과하면 정확한 aggregate를 한 번 재시도할 수 있습니다. 한 최종 session에서 정확한
aggregate는 두 번보다 많이 실행되지 않습니다.

두 번째 aggregate-only 실패 뒤에는 그 출력에서 첫 실패와 같은 변경하지 않은 단일
패키지의 재실행 target 하나를 정확히 확인한 경우에만 분해합니다. 이때 해당 패키지를
제외한 workspace와 전체 패키지를 따로 실행한 뒤 멈춥니다. 다른 패키지, 변경된 패키지,
여러 패키지나 target, 해석할 수 없는 출력이면 그 분해를 하지 않고 멈추며 정확한 진단
이유를 run summary에 기록합니다. 두 번째 실패를 해석하지 못했을 때 첫 실패 target을
대체값으로 다시 사용하지 않습니다.

영구적인 패키지 제외를 추가하지 않고 세 번째 정확한 시도를 하지 않습니다. 변경
패키지 실패에는 이 downgrade 경로를 적용하지 않습니다. 유지 담당 문서가 먼저
요구하지 않는 한 어떤 profile도 전역 `--test-threads=1`이나 `RUST_TEST_THREADS=1`을
설정하지 않습니다.

통과, 실패, 분해, 생략은 서로 독립적인 summary 분류입니다. 분해 명령은 통과하거나
실패할 수 있지만 성공해도 실패한 정확한 aggregate를 없애거나 전체 결과를 통과로
바꾸지 않습니다. 사람용 출력과 JSON 출력은 같은 명령 record와 분류 목록에서
생성됩니다.

## Commit 종류 범위

검증 사전 점검은 Conventional Commit subject를
`type[(scope)][!]: description` 형식으로 해석하고 명시적 기준과 `HEAD` 사이에서
기계로 확인 가능한 파일 범위를 검사합니다. 따라서 평범한 형식, scope가 있는 형식,
breaking 형식의 `test`와 `docs` subject에 같은 규칙을 적용합니다. `test` commit은
프로덕션 동작을 바꾸면 안 되고 `docs` commit은 프로덕션 코드나 런타임 계약을
바꾸면 안 됩니다. 파일 범위로 구분할 수 있는 프로덕션 구현 경로는 자동 점검이
거부하며 유지 문서 안의 계약 의미는 계속 의미 검토가 담당합니다.

프로덕션 패키지 manifest에서 `test` commit은 명시적 test target과 target별 항목을
포함한 개발 의존성만 바꿀 수 있습니다. Runtime target, 일반 또는 build 의존성,
feature, package metadata, workspace 구성, profile은 프로덕션 변경으로 남습니다.
같은 commit에서 `Cargo.lock`을 함께 바꾸려면 나머지 모든 변경 경로가 test 경로,
허용되는 test 전용 manifest 변경, 또는 비프로덕션 패키지 변경이어야 합니다.

테스트나 문서 작업 중 프로덕션 공백을 찾았다면 프로덕션 변경을 앞선 `feat:`,
`fix:`, `refactor:` commit에 둡니다. 그 공백을 `test:` 또는 `docs:` commit 안에
숨기지 않습니다.

## 구조 점검

문서 메타데이터, 경로, 링크, 용어 경로를 바꿨다면 저장소 루트에서
`cargo run -p xtask -- docs-check`를 실행합니다. 이 명령은 읽기 전용이며
기계로 확인할 수 있는 형태를 검증합니다.

- `docs/doc-index.yaml`이 YAML로 파싱되고 `version: 3`을 갖습니다.
- 필요한 최상위 섹션이 있으며 지원되지 않는 최상위 필드는 거부됩니다.
- 대응 문서는 중복 없는 의미 기반 `contracts`를 선언할 수 있습니다. `DocIndex`는
  일반 메서드 문서를 요청·응답 쌍 하나로 해석하고, 관례를 따르지 않거나 여러
  메서드를 담당하는 문서에는 완전하고 중복 없는 `method_contracts` 선언을
  요구합니다. 선언하거나 해석한 모든 계약은 기계 판독 가능한 담당 설명자
  하나에 존재해야 하며 정규화된 결합 순서는 결정적이어야 합니다.
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
  `journeys`, `canonical_for`, `depends_on`, `contracts`,
  `method_contracts`만 사용합니다.
- 공유 항목과 대응 항목에 필요한 필드가 있으며 `applies_to`와 대응 문서의
  `contracts`, `method_contracts`는 선택 필드입니다.
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
- `docs/en/architecture-guide/design/`과
  `docs/ko/architecture-guide/design/` 아래의 현재 개별 아키텍처 설계 문서는 문서
  정책이 언어별로 정의한 정확한 H2 순서를 사용하며 그 긍정적인 스키마 밖의 중첩
  제목 절을 두지 않습니다.
- 의미 기반 계약이 있는 각 대응 문서에 대해 모든 생성기와 검증기는 정규화된
  `DocIndex` 결합 집합 하나를 사용하고 그 정확한 담당 카탈로그만 만듭니다.
  공개 요청과 응답 설명자는 `volicord-types`에서, CLI 문법과 값은
  `volicord-command-model`에서, CLI inbox 출력은
  `volicord-user-action-presentation`에서, 진단 코드는 typed 진단
  레지스트리에서, MCP 식별자는 프로토콜 레지스트리에서 가져옵니다. 의도적으로
  인접한 계약 관계는 설명자에 명시하며 전체 공개 API 카탈로그로 확장하지
  않습니다.
- 점검기는 대응하는 파싱된 Markdown 의미 단위 안에서 카탈로그 항목을 비교합니다.
  의미 단위에는 제목 좌표, 문단, 중첩 목록 항목, 표 셀, 정의 항목, 콜아웃,
  각주, 펜스 예시가 포함됩니다. 구조 좌표는 번역된 제목 문구가 아니라 제목과
  블록 순번을 사용합니다. 따라서 식별자가 같은 제목 아래에 남아 있더라도 다른
  문단, 목록 항목, 표 셀로 이동하면 불일치입니다.
- 영어와 한국어 단위는 문서 범위에 대해 각각 검증하며, 양쪽에서 유효한 단위만
  일치 비교를 수행합니다. 다른 계약이 담당하는 정확한 식별자는 범위 밖입니다.
  현재 담당 원본 어디에도 없는 계약 유사 식별자는 두 언어에 모두 있어도
  유효하지 않습니다.
- 인식 기준은 정확한 카탈로그 포함 여부입니다. 단순 소문자 값, `snake_case`
  필드, 하이픈 CLI 토큰, 점으로 구분한 진단 코드, 프로토콜 식별자도 포함하며
  담당 범주를 서로 구분합니다. 임의의 산문, 경로, 파일 이름, 환경 변수, 소스
  식별자를 API 필드로 취급하지 않습니다.
- 모든 참조 JSON/YAML 펜스는 `shape=`를 정확히 하나 선언합니다. 해석된
  `DocIndex` 결합 집합이 사용 가능한 의미 기반 계약을 정하며, 둘 이상의 사용
  가능한 계약이 그 형태를 제공하면 펜스는 문서에 이미 결합된
  `contract=<semantic_contract_id>`를 정확히 하나 더 선언합니다. 요청과 응답
  설명자는 서로 분리된 채로 유지합니다.
- 구조화 파서는 인스턴스를 만들기 전에 모든 중첩 깊이의 각 JSON 객체 또는 YAML
  매핑 안에서 키가 정확히 하나씩만 나타나는지 검사합니다. YAML 태그, 앵커,
  별칭, 병합 키, 문자열이 아닌 매핑 키는 거부합니다.
- 선택된 정확한 스키마는 고유 키로 구성된 JSON 호환 인스턴스를 검증합니다.
  스키마 컴파일 오류는 담당 원본 오류입니다. 인스턴스 검증은 필수 및 알 수
  없는 속성, 형식, 중첩 객체와 배열, enum과 const 값, 제약 조건, union, 참조,
  Null 허용 여부를 적용합니다. 정확한 `ToolError` 스키마는 정규 공개 오류
  코드/범주 관계도 적용하므로 두 언어에서 같은 불일치 쌍을 사용해도 유효하지
  않습니다. `schema` 펜스는 독자를 위한 형태 표기이며 인스턴스로 취급하지
  않습니다.
- 영어와 한국어 인스턴스는 각각 담당 계약과 형태를 해석하고 파싱한 뒤 스키마로
  검증합니다. 두 언어에서 모두 유효한 의미 단위에만 구조 및 정확한 식별자
  동등성 검사를 수행합니다.
- 셸 예시와 생성 관리 CLI 영역은 경로가 지정된 명령 모델 담당 원본을
  사용합니다. 퍼지 일치는 진단에서 가까운 현재 식별자를 제안할 때만 쓰며
  철자를 통과시키는 기준이 되지 않습니다.
- 식별자 진단은 결정적이며 문서 쌍, 구조적 의미 단위, 의미 기반 계약과 담당 경로,
  범위 밖이거나 누락되거나 유효하지 않은 식별자를 표시합니다.
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
  내부 명령을 제외합니다. 유지 담당 문서 경로는 `docs/doc-index.yaml`의
  `reference.admin-cli` 항목에서 가져옵니다.
- 신원에 민감한 용어의 역할 메타데이터는 허용된 역할 집합을 사용하고, 공개
  선택자, 저장소 내부, MCP 프로세스 바인딩, 진단에 필요한 역할을 포함합니다.
- `docs/terminology-map.yaml`의 `primary_owner`와 `related_references` 경로가
  존재하고 `doc-index.yaml`에 표현되어 있습니다.
- 한영 API 값 집합 담당 문서의 작업 범주 표는
  `volicord_types::values::OperationCategory`에서 생성한 JSON Schema와 일치합니다.
- 문서 정책이 표면 라벨을 요구하는 집중 참조 담당 문서는 표면 안정성 섹션을
  포함하고, 기준 어휘로 연결하며, `stable`, `beta`, `internal`,
  `diagnostic` 라벨만 사용합니다. 이 경로들은 집중 `doc_id` 항목에서
  가져옵니다.
- 한영 Storage DDL 담당 문서 경로는 `reference.storage-ddl`에서 가져오며, 표시된
  SQL 영역은 Store의 기준 SQL 원본과 일치합니다.
- 추적 파일은 `.gitignore`가 담당하는 저장소 아티팩트 제외 규칙과 일치하면
  안 됩니다.
- `docs/owner-routing.yaml`의 모든 지침 경로, 직접 담당 `doc_id`, 워크스페이스
  패키지, 지원되는 검증 분류가 현재 지침 파일, 문서 색인, Cargo 워크스페이스와
  정확히 한 번씩 대응합니다.

`docs-check`는 Rust나 Markdown 줄에서 금지 단어나 문구를 검색하지 않습니다.
산문 품질, 브랜드 주장, 보안 표현, 호스트 지원 표현은 담당 문서와 사람 검토의
관심사입니다. 진단 신원은 문서 어휘 검색이 아니라 타입이 있는 진단 레지스트리와
렌더링 테스트가 다룹니다.

자동 구조 검증 뒤에는 남은 저장소 위생을 사람이 확인합니다.

- 생성된 기록, 런타임 홈, SQLite 파일, 생성 로그, 보관 사본, 변환 메모, 부수
  메모, 작업용 목록, 작업 로그가 유지 문서에 남아 있지 않습니다. Git 색인
  점검 밖에 있는 추적하지 않는 작업 파일도 확인합니다.

## 워크스페이스 아키텍처 검증

루트 `Cargo.toml`, 워크스페이스 멤버 매니페스트, 패키지 배치, 내부 의존성을
변경한 뒤에는 `cargo run -p xtask -- architecture-check`를 실행합니다. 이
명령은 Cargo 메타데이터에서 실제 워크스페이스 패키지 identity와 일반, 개발,
빌드 의존 간선을 읽습니다. 그런 다음 각 패키지의 의미 그룹, 한영 책임 설명,
분류, 프로덕션 여부, 경계, 종류별 내부 의존 허용 목록을 담당하는 단일 기계 판독
원본인 루트 `Cargo.toml`의 `workspace.metadata.architecture.packages`와
비교합니다.

검사는 선언되지 않았거나 Cargo에 없는 워크스페이스 패키지, 유효하지 않거나
중복된 책임 그룹, 해결되지 않는 의존 담당 패키지, 종류별로 허용되지 않은 간선,
프로덕션 패키지의 테스트 지원 패키지 대상 일반·빌드 의존성, Core 쪽에서
어댑터나 표현 패키지로 향하는 의존성, 필수 UserAction 서비스·Core·공유
타입·Store 경계 위반, 일반·빌드 의존 순환을 거부합니다. CI는 이 명령을 집중
워크스페이스 점검으로 실행합니다. 테스트는 검증기의 일반 동작에 중립적인 합성
그래프를 사용하고, 저장소 그래프 case에서는 현재 워크스페이스를 직접 읽습니다.

## 사람이 하는 의미 검토

한영 변경에서는 영어와 한국어를 의미 단위로 비교합니다. 독자 목적, 규범 강도,
담당 경로, 기준 범위와 지원 범위 밖 경계, 사용자 판단 경계, 부정 절, 비주장,
보장 강도, 제목, 표, 목록, 예시, 링크, 정확한 식별자를 보존합니다.
계약을 담는 의미 단위에서는 같은 파싱 구조 좌표 안에 식별자도 보존합니다. 현재
정책에는 담당 원본에서 도출한 식별자를 문단, 목록 항목, 표 셀 사이로 옮기도록
허용하는 예외가 없습니다.

계약과 가까운 편집에서는 정확한 API 동작, 스키마 의미, 오류 의미, 저장 효과,
보안 표현, 접근 경계, 닫기 준비 상태 의미, 값 집합 의미, Core 권한 의미가 집중
참조 담당 문서에 남아 있는지 확인합니다. 담당 문서가 아닌 곳은 요약하고
링크해야 하며 두 번째 계약 본문이 되면 안 됩니다.

용어 변경에서는 식별자 표현 정책, 선호 표현과 문맥별 형태, 자연스러운 한국어
지침, 담당 경로 무결성을 용어 지도에서 확인합니다. 정확한 계약 식별자는 문서의
의미 기반 계약과 그 현재 담당 설명자를 기준으로 확인합니다.

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
세부사항을 제품 계약 문구로 바꾸지 않습니다. 현재 아키텍처 설계 참조는 영어와
한국어 문서를 절별로 비교하고, 각 문서가 현재 구현만 설명하며, 모든 구현 경로와
집중 참조 담당 문서 링크가 정확한지 확인합니다.

자동 `docs-check` 명령에는 유지되는 영어/한국어 대응 쌍의 로컬 문서 링크 일치,
제목 수준 구조 일치, 파싱된 구조적 의미 단위별 현재 담당 원본 기반 정확한 식별자
일치 점검이 포함됩니다. 인식된 식별자가 없는 산문이 같은 의미인지, 양쪽에 있는
같은 식별자가 같은 의미로 쓰였는지는 증명하지 않습니다. 전체 한영 의미 검토,
계약 담당 문서 검토, 기술 정확성 검토, 번역 판단, API 예시 정합성 검토, 제품
의미 검토도 수행하지 않습니다. 이런 책임은 사람의 의미 검토와 집중 담당 문서에
계속 남습니다.

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
- 진단 테스트는 임의의 소스 줄에서 진단 단어를 찾지 않고 타입이 있는 레지스트리
  항목을 구성한 뒤 그 항목에서 나온 사람용·구조화 렌더링을 비교합니다.

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

commit된 소스 배포본에는 다음 정규 생성 및 검증 명령을 사용합니다.

```sh
cargo run --locked -p xtask -- source-bundle --output /tmp/volicord-source.zip
cargo run --locked -p xtask -- source-bundle-validate --input /tmp/volicord-source.zip
```

생성 명령은 `HEAD`를 선택하고 추적 중인 index와 working tree가 변경되지 않았을 때만
진행합니다. 선택한 Git tree에서 entry와 blob을 읽고, 완성한 ZIP을 검증한 뒤
게시합니다. 두 명령 모두 `--commit <commit>`으로 다른 정확한 commit을 선택할 수
있습니다. ZIP은 상대 경로에 정방향 slash를 사용하고 중복되거나 안전하지 않은 경로를
거부합니다. 일반 파일은 `100644` 또는 `100755`, symlink는 target byte와 함께
`120777`로 저장하며, timestamp와 stored compression을 정규화합니다. 같은 선택
commit과 패키징 구현에서는 entry 순서, 내용, mode, link target, ZIP metadata가
바이트 단위로 결정적입니다.

검증 명령은 ZIP entry 전체 집합, 파일 형식, mode, 일반 파일 내용, symlink target을
Git tree와 대조합니다. 포함 대상은 Git tree entry에서만 오므로 `.git` metadata,
untracked 파일, 로컬 database, log, runtime data, 빌드 및 scratch 출력, 이전에 생성한
untracked archive는 소스 번들 입력이 아닙니다. 현재 Linux 검증 계획은 일반 CI에서
같은 생성 명령을 실행하고 태그 릴리스 게시는 게시 archive에 그 명령을 호출합니다.
Release-integrity 테스트는 두 경로를 모두 검증합니다.

게시하지 않는 `tests/release-smoke` 패키지는 플랫폼 공통 실제 바이너리 하네스를
담당합니다. 폐기 가능한 Product Repository, Runtime Home, 안정적인 테스트 소유 Codex
fixture를 사용하며 한도가 있는 프로세스 실행과 정리는 `volicord-test-process`에
위임합니다. 현재 Linux 검증 계획은 일반 CI에서 로컬 debug 바이너리를 빌드하고 그
바이너리로 패키지를 정확히 한 번 호출합니다. 로컬 composite action
`.github/actions/volicord-release-smoke`는 네이티브 릴리스 workflow 경계로 남습니다.
네이티브 패키징 matrix의 각 항목은 artifact staging 전에 해당 target용으로 이미 빌드한
정확한 Linux, macOS, Windows 바이너리를 정확히 한 번 전달합니다. 스모크는 공개
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

Rust 구현을 편집한 뒤에는 명시적 series 기준으로 `validate focused`를
사용합니다. 이 profile은 formatting, 변경 패키지 lint와 테스트, 경로에서 선택한
직접 점검, 위생을 계획합니다. 완전한 series 뒤에는 `validate final`을 한 번
사용합니다. 최종 profile은 workspace lint, 완전한 저장소 점검, 정확한 aggregate를
추가합니다. 실제 실행한 명령과 생략 작업은 오래 남는 summary가 담당합니다.

## 생성 참조와 계약 드리프트 점검

생성되었거나 원본에서 파생되는 참조 표면은 안정적인 점검 명령을 사용합니다.

- `cargo run -p xtask -- docs-sync`는 표시된 CLI 문법 영역, 영어·한국어 API
  메서드 담당 문서의 스키마 생성 요청 및 응답 구조 영역, API 코어 스키마의 정식
  공유 응답 구조, 아키텍처의 한영 패키지 책임 및 의존 방향 영역을 결정적으로
  교체합니다. 명령, 요청, 응답, 결과, 공유 응답 설명자 또는 워크스페이스
  아키텍처 메타데이터를 변경한 뒤 실행하고 생성 diff를 검토합니다. 두 번째로
  실행했을 때 갱신 파일이 없어야 합니다.
- `cargo run -p xtask -- docs-check`는 유지 문서 구조, 해석된 정확한 요청 및
  응답 영역 결합과 스키마 드리프트, 생성 또는 원본 파생 문서 표면, 실행 가능한
  `volicord` 명령 예시, 한영 링크·제목·정확한 식별자 일치,
  용어 메타데이터 담당 경로와 역할, 그리고
  `crates/volicord-store/src/schema/registry.sql` 및
  `crates/volicord-store/src/schema/project.sql`에 대한 기준 Storage DDL SQL
  블록을 점검합니다.
- `cargo test -p volicord-integration-tests --test public_contract_snapshots`는 API 요청
  스키마 투영과 MCP `workflow`/`read_only` 도구 투영의 생성 공개 계약 스냅샷이
  Rust 원본과 일치하는지 점검합니다.
- `cargo test -p volicord-cli --test diagnostic_registry_contract`는 생성된 기계 판독
  진단 코드 산출물이 현재 typed 레지스트리와 일치하는지 점검합니다. 레지스트리를
  의도적으로 바꾼 뒤에는
  `VOLICORD_UPDATE_DIAGNOSTIC_REGISTRY=1 cargo test -p volicord-cli --test diagnostic_registry_contract`
  로 다시 생성하고
  `crates/volicord-cli/tests/fixtures/diagnostic-registry.json`을 검토합니다.
- `cargo test -p volicord-user-action-presentation --test cli_output_contracts`는 간결한
  CLI inbox 출력 설명자 산출물이 typed 표현 담당 원본과 일치하는지 점검합니다.
  의도적인 변경은
  `VOLICORD_UPDATE_CLI_OUTPUT_CONTRACTS=1 cargo test -p volicord-user-action-presentation --test cli_output_contracts`
  로 다시 생성하고 생성 픽스처를 검토합니다.
- 의도적인 원본 변경 뒤 공개 계약 스냅샷을 다시 생성하려면
  `VOLICORD_UPDATE_CONTRACT_SNAPSHOTS=1 cargo test -p volicord-integration-tests --test public_contract_snapshots`
  를 실행하고 `tests/integration/snapshots/` 아래의 생성 파일을 검토합니다.

공개 계약 스냅샷과 진단 레지스트리 파일은 `_generated`로 표시된 생성 테스트
산출물입니다. 손으로 편집하지 말고 먼저 typed 담당 원본을 바꾼 뒤 다시
생성합니다. CLI 공개 명령 드리프트는 실행 가능한 문서 예시와
`volicord-cli`의 `binary_admin`, `mcp_transport` 같은 CLI 도움말/출력 테스트
대상으로 계속 다룹니다. 별도의 typed UserAction inbox JSON schema는
`volicord-user-action-presentation`이 담당하고 테스트하며, CLI 테스트는 실제
`--json` 출력도 이 model로 deserialize합니다.

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

검증 결과는 저장소 파일이 아니라 대화에 보고합니다. 변경 파일, run ID, summary
경로, 별도의 통과·실패·분해·생략 목록을 포함합니다. 생략 이유와 남은 문서 위험도
포함합니다. 정확한 aggregate 시도 중 하나라도 실패했거나 정확한 aggregate를
실행하지 않았다면 검증이 통과했다고 말하지 않습니다.

`PASS`, `WARN`, `FAIL`, `SKIP`은 문서 유지보수 또는 구현 점검 결과로만
사용합니다. 통과한 검증 단계를 Volicord 런타임 적합성, 제품 수락, QA 완료, 닫기
준비 상태, 보안 보장, 잔여 위험 수락으로 설명하지 않습니다.
