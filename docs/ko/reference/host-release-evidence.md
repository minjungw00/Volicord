# 호스트 릴리스 증거

이 문서는 관리되는 Codex 및 Claude Code 호스트 기능에 적용하는 정확한 최종 아티팩트
릴리스 검증 계약을 담당합니다. 릴리스 후보, 셀, manifest, 독립 audit 스키마와 고정 검증
행렬, 정규 상태 도출, 최신성, 게이트 판정, 개인정보를 보호하는 관리 호스트 세션 결속을
정의합니다.

공개 Core API 메서드, 운영 manifest 획득, 호스트 신뢰, 운영체제 격리, 런타임 저장소는
정의하지 않습니다. 관리 CLI 출력은 이 계약의 보조 상태 보기일 뿐입니다. 운영 local-web
자격은 계속 [Agent Connection](agent-connection.md)과
[MCP Transport](mcp-transport.md)가 담당합니다.

<a id="surface-stability"></a>
## 표면 안정성

네 스키마 식별자와 필수 필드, 고정 12개 셀 행렬,
`git_archive_tar_sha256_v1`, 정규 상태 및 판정 도출, 반개구간 최신성 규칙, 관리 호스트
세션 매핑은 안정된 릴리스 계약입니다. 호스트, 기능, 상태, 다이제스트 알고리즘, 필수 필드,
판정을 추가하려면 버전이 올라간 후속 스키마와 짝을 이루는 아키텍처 결정을 추가해야
합니다. 이 문서에서 명시하지 않은 파일 경로, 비공개 Rust 타입, 프로세스 배치,
사람이 읽는 렌더링은 구현 세부사항입니다.

## 계약 식별자

| 역할 | 정확한 식별자 |
|---|---|
| 정확한 후보 설명자 | `volicord-release-candidate-v1` |
| 실제 호스트 행렬 결과 하나 | `volicord-host-release-cell-v3` |
| 정규 릴리스 게이트 결과 | `volicord-host-release-manifest-v3` |
| 별도 프로세스 재계산 결과 | `volicord-host-release-audit-v3` |
| 소스 아카이브 다이제스트 알고리즘 | `git_archive_tar_sha256_v1` |
| 셀 입력 집합 다이제스트 도메인 | `volicord-host-release-cell-inputs-v3` |

네 아티팩트는 중복 키를 거부하는 정규 UTF-8 JSON 객체입니다. 알 수 없는 필드는
거부합니다. SHA-256 값은 소문자 64자리 16진수이고, 타임스탬프는 초 정밀도의 정규 UTC
RFC 3339이며, 스키마 식별자는 정확한 문자열입니다. 생산자는 각 목적지를 새 파일로
만들고 이미 존재하면 실패해야 합니다. 셀 JSON 파일은 최대 1 MiB, manifest 및 audit
JSON 파일은 최대 4 MiB, 셀이 참조하는 증거 아티팩트 하나는 최대 16 MiB입니다. 이
스키마가 이름 붙인 경로는 절대 정규화 경로여야 하며 소스 checkout, Cargo target
디렉터리, 유지보수 문서, 모든 Volicord Runtime Home 밖에 있어야 합니다.
제외하도록 설정한 루트는 겹침을 검사하기 전에 정규화합니다. 상대
`CARGO_TARGET_DIR`는 소스 checkout을 기준으로, 상대 `VOLICORD_HOME`은 호출 프로세스의
현재 디렉터리를 기준으로 해석합니다. 기존 symlink 접두사와 점 경로 요소로 제외 범위를
약화할 수 없습니다.

<a id="external-release-path-policy"></a>
### 정규 외부 릴리스 경로 정책

후보 설명자 생산자, 실제 셀 생산자, 게이트, 독립 audit은 자신이 소비하는 모든 릴리스
아티팩트 입력과 create-new 목적지에 하나의 정규 외부 경로 평가기를 적용합니다. 공통
제외 집합에는 정규 소스 checkout, 해석된 Cargo target 디렉터리, 유지되는 `docs/`
디렉터리, 프로세스에
명시된 Runtime Home, 사용자 홈에서 도출한 기본 Runtime Home, 호출자가 추가로 제공한
모든 Runtime Home, 현재 셀에 결속한 각 폐기 가능한 Runtime Home, 아티팩트 경로에서 발견한
registry 보유 상위 디렉터리가 들어갑니다. 후보 설명자, `candidate_path`, 셀 디렉터리와
출력, 증거 sidecar, manifest, audit 입력과 출력은 모두 이 평가기를 사용합니다.

받아들이는 모든 아티팩트 경로에는 정확한 UTF-8 표현이 있어야 합니다. 기존 입력이나
디렉터리는 정규 경로여야 하고 symlink 요소가 없어야 합니다. Create-new 출력은 존재하지
않아야 하며 이미 존재하는 정규 symlink 없는 부모를 가져야 합니다.
Manifest 또는 audit 목적지는 제공된 result root의 `cells/`와 `evidence/` 디렉터리 밖에
있어야 하며, 요약을 쓰는 동작이 자기 셀 또는 증거 입력 집합을 바꿀 수 없습니다.
생산자가 후보나 결과 경로를 처음 받아들인 뒤 셀의 폐기 가능한 Runtime Home을 결속하면,
같은 제외 context에 그 루트를 추가하고 result root lease 획득, 호스트 시작, 이후의 후보
읽기 또는 실행, 출력 스테이징, 최종 이름 게시 전에 보존 중인 모든 경로를 다시
검사합니다.

경로 정책 거부, 협력적 result-root lease를 획득하지 못한 경우, 또는 lease 아래 사전
검사에서 최종 목적지가 이미 존재하는 경우는 구조적 명령 오류입니다. 인증된 호스트를
시작하기 전, 정적 미지원 셀이라면 종단 게시 전에 발생하며 이 생산자는 최종 릴리스 이름을
게시하지 않습니다. 하향 조정, `ignored` 셀, audit exclusion으로 바꾸지 않습니다. 생산자만
사용하는 소스 검사, 병렬 경로 정책, 게이트나 audit까지 첫 거부를 미루는 동작은 계약에
맞지 않습니다.

<a id="append-only-live-cell-publication"></a>
### Append-only 실제 셀 게시

유지되는 행렬 셀은 새 외부 result root 하나를 사용합니다. Result root와 그 바로 아래의
정확한 `cells/`, `evidence/` 디렉터리는 미리 존재해야 하고, 정규 경로이며 symlink 요소가
없어야 하고, 공유 외부 경로 평가기를 충족해야 합니다. 셀 목적지는
`RESULT_ROOT/cells`의 바로 아래 항목이고, 구현된 셀에서 도출한 증거 목적지는
`RESULT_ROOT/evidence`의 바로 아래 항목입니다. 생산자는 두 디렉터리를 만들거나
교체하거나 이름을 바꾸거나 제거하지 않습니다.

폐기 가능한 Runtime Home을 결속하고 보존 중인 모든 경로를 다시 검사한 뒤, 생산자는
result root와 두 하위 디렉터리를 열어 고정하고 그 result root에 연결된 협력적 배타 lease
하나를 획득합니다. 호스트 실행과 종단 게시가 끝날 때까지 lease를 유지합니다. Lease를
유지한 상태에서 호스트 시작 전에 셀 최종 이름과, 구현된 셀이라면 증거 최종 이름이 모두
없는지 확인합니다. 정적인 `unsupported_by_host` 셀도 호스트 없이 종단 게시하기 전에 같은
lease 아래에서 셀 이름을 검사합니다. Lease는 계약을 따르는 생산자를 직렬화합니다.
계약을 따르지 않는 동시 작성자도 덮어쓰지 않도록 최종 이름 설치에는 원자적 no-replace
의미론을 별도로 적용합니다. Lease에 사용하는 안정된 비공개 조정 항목은 크기가 제한되고
동기화되는 게시 상태 하나도 보유합니다. 새 항목은 `clean`으로 시작합니다. Lease 아래의
일관성 검사를 마치고 호스트를 시작하기 전에 구현 셀 생산자는 상태를 `active`로 바꾸고
동기화합니다. 정적 미지원 생산자는 호스트 없는 게시 전에 같은 처리를 합니다. 최종 셀과
담당 디렉터리를 동기화한 뒤에만 그 생산자가 `active`를 완전하고 정확한 `clean` 레코드로
바꾸기 시작하고 그 동기화를 요청합니다. 후속 lease 아래에서 관찰한 완전하고 정확한
`clean` 레코드가 권위 있는 게시 커밋 표식입니다. 비어 있거나 일부만 있거나 잘못되었거나
`active`인 상태는 구조적 실패입니다. 이 조정 항목은 릴리스 아티팩트가 아니며 어떤 릴리스
digest의 입력도 아닙니다.

후속 생산자는 `clean`을 `active`로 바꾸기 전에 기존 cell 디렉터리 항목이 모두 크기가
제한된 엄격 유효 커밋 셀인지 확인합니다. Evidence 디렉터리의 각 항목은 그런 셀 하나가
정확히 한 번 이름 붙이고 바이트와 digest가 일치해야 합니다. 비공개 stage, 고아 evidence,
누락 evidence, 이미 완성된 셀 집합은 거부합니다. 이 일관성 검사는 잔여물을 허용하거나
복구하지 않으며 생산자 프로세스가 바뀌어도 fresh-root 복구를 강제합니다.

종단 바이트가 정해지면 구현 셀 생산자는 열어 고정한 각 담당 디렉터리에 비공개
create-new stage 파일을 최대 하나씩 만듭니다. 증거 stage를 완전히 쓰고 크기를 검사하고
동기화한 뒤 유지 중인 그 바이트에서 기록할 증거 다이제스트를 계산합니다. 이어서 그 최종
증거 경로와 다이제스트를 이름 붙이는 셀 stage를 완전히 쓰고 크기를 검사하고
동기화합니다. 증거 최종 이름을 먼저 no-replace 의미론으로 원자적으로 설치하고 evidence
디렉터리를 동기화합니다. 셀 최종 이름은 마지막에 같은 의미론으로 설치하고 cells
디렉터리를 동기화합니다. 엄격 검증을 통과하는 최종 셀만 그 셀·증거 쌍의 커밋
표식입니다. 정적인 `unsupported_by_host` 셀은 null 증거를 사용하며 최종 셀 하나만
스테이징하고 게시합니다. 생산자는 임시 `running` 셀을 게시하지 않습니다.

선택한 기능 실패 또는 선택한 호스트 자식 프로세스 실패를 생산자가 엄격하고 크기가 제한된
종단 바이트로 분류할 수 있고, 직접 자식 프로세스를 회수하고 유지되는 협력적 프로세스 격리
경계를 정지시킨 뒤 turn 이후 관리 기준선, 보존한 정체성, 후보 digest, 게시 영역
재검증까지 모두 끝냈다면 이는 게시 실패가 아닙니다. 유지되는 경계는 선택한 호스트를 시작할
때 부여한 전용 운영체제 프로세스 그룹이며, runner가 지원하는 경우 그 그룹을 벗어난 뒤에도
turn에서 상속한 ownership 표식을 유지하는 프로세스를 찾는 검사를 보조로 사용합니다. 정지는
직접 자식 프로세스를 회수하고, 프로세스 그룹에 살아 있는 구성원이 없으며, 발견 가능한
ownership 표식 유지 프로세스가 모두 종료되어야 성립합니다. 이는 협력적 격리이지 적대적
sandbox가 아닙니다. 부여된 그룹 밖으로 daemonize하면서 상속한 표식도 제거하는 host
adapter는 검증된 runner profile 밖에 있으므로 실제 host 검증됨으로 주장하면 안 됩니다.

제어 터미널에서 입력을 읽는 대화형 선택 호스트에는 별도의 job-control 불변조건이
적용됩니다. 처음에는 runner의 원래 프로세스 그룹이 그 터미널의 전경을 소유해야 합니다.
선택 자식 프로세스가 입력을 읽기 전에 제한 시간 안에서 동작하는 전경 controller가 전용
운영체제 프로세스 그룹에서 준비를 마치고, 제어 터미널의 전경을 정확히 그 그룹으로
이전하며, 복원 경로를 계속 유지해야 합니다. 생산자는 선택 호스트를 같은 그룹에서
시작하고 그 구성원임을 검증하며 turn ownership 표식을 유지합니다. 직접 자식 프로세스가
종료되어 회수된 뒤에는 그 controller가 전경을 runner의 원래 프로세스 그룹으로 복원하고
자신도 회수되어야 합니다. 전용 그룹과 ownership 표식 경계가 정지한 뒤 생산자는 전경 이전
전에 보관한 전체 터미널 속성을 다시 적용하고 정확히 검증해야 합니다. 정확한 전경 복원은
그룹 신호 전송의 선행 조건이며, 격리 경계 정지와 터미널 속성 복원은 모두 turn 이후 기준선
확보와 종단 게시보다 앞서 끝나야 합니다. controller 준비, 복원, 회수 대기에는 모두 제한
시간이 있습니다. 전용 그룹은 계속 격리 그룹이며, 전경 이전이 ownership 표식
보조 검사를 대신하지 않습니다. 이 불변조건은 pseudo-terminal(PTY)을 만들거나 그 존재를
보증하지 않습니다. runner에는 이미 제어 터미널이 있어야 합니다. 검증되는 runner
profile에서는 터미널의 `TOSTOP` local mode가 꺼져 있어야 하며, `Ctrl-Z`처럼 운영자가
선택 turn을 job-control로 중지하는 동작은 검증 범위 밖입니다. 중지했거나 다시 시작한
turn은 릴리스 증거가 아닙니다. controller의 준비·생존·회수 실패, 그룹 불일치,
활성화된 `TOSTOP`, 전경 이전·검증·복원 실패는 최종 이름 게시를 금지하고 result root를
사용할 수 없게 하며 비통과 셀로 표현할 수 없습니다. 전체 터미널 속성을 복원하거나 정확히
검증하지 못한 경우에도 같은 규칙을 적용합니다.

이후 생산자가
그 증거와 셀 바이트를 게시하고 정확한 `clean` 레코드를 커밋하면, 통과하지 않은 셀도
허용되는 행렬 입력이고 result root는 구조적으로 계속 사용할 수 있습니다. 같은 root에서
그 셀을 교체하거나 재시도하면 안 됩니다. 직접 자식 프로세스를 회수하지 못했거나 협력적
격리 경계의 정지를 확인하지 못했거나 turn 이후 기준선을 확정하지 못한 경우, 보존한
기준선이나 정체성이 교체된 경우, 정확한 `clean` 전의
생산자 종료, 게시 I/O 실패, 엄격한 종단 바이트를 만들지 못한 경우에는 최종 이름 게시를
금지하고 result root를 사용할 수 없게 하므로 아래의 새 root 복구가 필요합니다. 이런
무결성 실패를 `implemented_unverified` 셀로 바꾸면 안 됩니다. 유지 생산자가 전용 프로세스
그룹을 확정할 수 없는 runner 또는 협력적 격리 전제 위반이 알려진 검토 host profile에서는
셀을 게시하지 말고 선택한 실제 시도를 구조적으로 거부해야 합니다.

게시는 append-only입니다. 생산자는 게시된 최종 이름을 unlink, 교체, 다른 이름으로 이동,
rollback하지 않으며 result root나 그 `cells/`, `evidence/` 디렉터리를 제거하지 않습니다.
I/O 오류, 비정상 종료, 동시 이름 경쟁에서 패한 경우에도 검사 후 삭제하는 정리 동작을
수행하지 않습니다. 실패한 구현 셀 시도 하나는 크기가 제한된 비공개 stage를 최대 두 개
남기거나, 생산자 셀 없이 설치된 증거 최종 이름을 남기거나, 셀 이름 설치 뒤 게시 커밋을
확인하기 전에 실패하면 설치된 두 최종 이름을 모두 남길 수 있습니다. 실패한 정적 미지원
시도도 크기가 제한된 비공개 셀 stage 또는 설치된 최종 셀을 남길 수 있습니다. `active`,
빈 상태, 부분 상태, 잘못된 조정 상태 아래에 설치된 최종 셀은 허용되는 커밋 셀이 아닙니다.

완전하고 정확한 `clean` 레코드가 관찰 가능해지기 전에 발생한 오류나 종료는 `active`, 빈
상태, 잘못된 상태를 남기며 해당 result root를 사용할 수 없게 합니다. 쓰기나 동기화 확인은
불확정일 수 있습니다. 완전한 `clean` 바이트가 관찰 가능해진 뒤의 오류나 종료는 생산자가
성공 반환을 관찰하지 못했더라도 커밋된 clean root를 남길 수 있습니다. 후속 프로세스는 그
반환 여부를 추론할 수 없습니다. 정확한 `clean` 레코드만 상태 커밋으로 취급하고 전체
셀·증거 집합을 별도로 다시 검사합니다. 이는 복구나 잔여물 채택이 아닙니다. 생산자는
관찰한 모든 실패를 보고하고 재시도하지 않으며, 유지되는 운영 절차는 게시 오류가 보고되거나
비정상 종료가 발생하면 보수적으로 해당 root를 포기합니다. 복구할 때는 `cells/`와
`evidence/`를 새로 미리 만든 새 외부 result root를 사용하고 완전한 12개 셀 행렬을 다시
실행합니다. 포기한 root의 이전 셀, 비공개 stage, 고아 증거, 설치된 최종 이름을 복사하거나
채택하거나 합성하지 않습니다.

게이트와 audit은 result root를 복구하거나 정리하지 않습니다. 셀을 다시 열기 전에 각각 그
result root의 협력적 공유 lease를 획득하여 유지합니다. 생산자가 실행 중이거나 조정 항목이
남아 있는 `active`, 빈 상태, 부분 상태, 잘못된 상태이면 구조적 명령 실패입니다. 지정한 셀
디렉터리에서 최종 `.json` 항목 정확히
12개만 받아들이고, 엄격 검증을 통과한 그 셀이 이름 붙인 증거 경로만 따라갑니다. 최종
셀이 없거나, 비공개 stage를 포함한 추가 셀 디렉터리 항목이 있거나, 참조한 증거가
잘못되었거나 일치하지 않으면 구조적 명령 실패입니다. 참조되지 않은 비공개 증거 stage나
고아 증거 파일은 입력 집합 밖에 있으며 누락 셀을 충족할 수 없습니다. 이런 잔여물은 하향
조정도 audit exclusion도 아닙니다.

v3 게이트와 audit은 과거 v1 및 v2의 셀, manifest, audit, 셀 입력 다이제스트 도메인
식별자를 거부합니다. 그 아티팩트를 v3 규칙으로 import, migration, 재해석하지 않습니다.
후보와 아카이브 preimage 의미는 바뀌지 않았으므로
후보는 계속 `volicord-release-candidate-v1`, 소스 아카이브 알고리즘은 계속
`git_archive_tar_sha256_v1`입니다.

## 정확한 릴리스 후보

`volicord-release-candidate-v1`에는 다음 필수 구성원만 들어갑니다.

| 구성원 | 계약 |
|---|---|
| `schema` | 정확한 값 `volicord-release-candidate-v1`. |
| `candidate_id` | 해당 릴리스 실행 안에서 고유한 비어 있지 않은 불투명 식별자. |
| `candidate_path` | 모든 셀이 시험하는 최종 실행 파일 하나의 외부 절대 경로. |
| `source_revision` | 소문자 40자리 또는 64자리 16진수 commit 객체 ID. |
| `source_clean` | 반드시 `true`이며 dirty 또는 추적되지 않은 소스 트리는 부적격입니다. |
| `source_archive_algorithm` | 정확한 값 `git_archive_tar_sha256_v1`. |
| `source_archive_sha256` | 아래의 결정적 소스 아카이브 SHA-256. |
| `target_triple` | 후보를 빌드할 때 사용한 정확한 Cargo target triple. |
| `release_profile` | 정확한 유지 Cargo profile 이름 `release`. 근사 profile class나 다른 profile은 유효하지 않습니다. |
| `binary_sha256` | `candidate_path` 바이트의 SHA-256. |
| `build_environment` | 정확한 `runner_os`, `runner_os_version`, `runner_arch`, `git_version`, `rustc_version`, `cargo_version` 문자열. 후보 v1은 각 문자열이 제어 문자가 없는 UTF-8 1바이트 이상 512바이트 이하이면 허용합니다. |
| `recorded_at` | 설명자와 모든 후보 다이제스트 계산을 완료한 시각. |

소스 checkout은 빌드 전에 깨끗해야 하며 후보 생성이 끝날 때까지
`source_revision`에 머물러야 합니다. 그 checkout에서 경로 접두사나 추가 attribute 없이
`git archive --format=tar <source_revision>`을 실행하고 명령의 원본 tar 표준 출력에
SHA-256을 계산합니다. 그 바이트 다이제스트가 `source_archive_sha256`입니다. 압축
아카이브, work tree, 디렉터리 목록, Git bundle을 해시하는 것은 같은 알고리즘이
아닙니다.

유지되는 설명자 생산자는 이미 외부에 배치한 최종 실행 파일과 아직 존재하지 않는 외부
설명자 경로를 입력으로 받습니다.

```sh
cargo run --locked -p volicord-release-validation-tests --bin host-release-candidate -- --candidate-id CANDIDATE_ID --candidate-path CANDIDATE_BINARY --candidate-out CANDIDATE.json
```

후보가 제어하는 바이트를 실행하기 전에 정규 외부 경로 평가기로 두 경로를 검증합니다.
그런 다음 깨끗한 HEAD와 원본 소스 아카이브 다이제스트를 도출하고, 정확한 실행 파일을
해시하여 비공개 복사본으로 검사하며, 실행 환경과 도구 체인의 좌표를 크기 제한에 맞춰
기록합니다. 생산자는 명령 출력의 앞뒤 공백을 제거하고 각 좌표를 제어 문자가 없고
공백만으로 구성되지 않은 UTF-8 1바이트 이상 512바이트 이하로 기록합니다. 마지막으로
실행 파일과 소스 안정성을 다시 검사하고 모든 검사가 통과한 뒤에만
`CANDIDATE.json`을 새로 만듭니다. 출력 경로는 미리 존재하면 안 되며 절대
덮어쓰지 않습니다. 이 명령은 후보를 빌드하거나 외부 최종 실행 파일을 배치, 교체,
변경하지 않습니다. 필요한 게시자 측 배치나 후처리가 있다면 명령을 실행하기 전에 끝내야
합니다. 이 명령이 만드는 유일한 복사본은 아래에서 설명하는 일시적인 비공개 검증
복사본이며 `candidate_path`나 릴리스 아티팩트가 아닙니다.

설명자 생산자는 후보를 빌드한 때와 동일하고 변경되지 않은 실행 환경 및 Git, Rust,
Cargo 도구 체인 환경에서 실행합니다. `build_environment` 문자열은 이 전제 아래에서
설명자 생산자 프로세스가 측정한 값이며, 아래에서 설명하는 비적대적이고 독립적으로
attestation되지 않은 좌표로 남습니다. 다른 빌드 환경에서 옮겨 온 후보나 빌드와 설명자
생성 사이에 도구 체인이 바뀐 후보는 부적격입니다.

게이트는 후보가 제어하는 바이트를 실행하기 전에 `candidate_path`의 일반 파일을 열어 그
파일 핸들을 유지한 채 해시하고 설명자 다이제스트와 정확히 일치하는지 확인합니다. 유지한
바이트를 비공개 create-new 실행 파일로 복사하고 복사본 다이제스트를 확인한 뒤 쓰기
핸들을 닫습니다. 주변 환경을 모두 지운 상태에서 이 비공개 복사본만 실행합니다. 실행
뒤에는 유지한 핸들의 다이제스트, 최종 경로의 다이제스트와 파일 정체성이 모두 변하지
않았는지 `candidate_binary_final_stable` 불변조건으로 확인합니다. 이 실행 후 안정성
불일치는 실패 manifest를 만듭니다. 반면 설명자 또는 비공개 복사본 다이제스트가 실행 전에
불일치하면 명령 오류가 되어 manifest를 만들지 않으며, 그 뒤 후보가 제어하는 바이트를
실행하지 않습니다.

내장 `--version` 빌드 메타데이터와 소스 아카이브 검사는 비적대적 provenance 및 좌표
무결성 검사입니다. 게이트는 후보를 다시 빌드하거나 재현 가능한 빌드를 증명하지 않으며,
이름 붙인 소스 revision이 임의의 후보 바이트를 만들었다고 attestation하지 않습니다.

비공개 후보의 `--version` 출력은 다음 빌드 불변조건으로 정확히 parse되어야 합니다.
Package version은 게이트 패키지가 상속한 workspace SemVer와 같고,
`git_commit=source_revision`, `tree=clean`, `metadata_source=environment`,
`target=target_triple`, `profile=release`, `profile_class=release`,
`profile_exact=true`여야 합니다. 하나라도 실패하면 게이트 불변조건 실패입니다. 설명자의
빌드 환경 문자열은 비적대적 좌표로 기록되지만 이 비교가 독립적으로 attestation하지는
않습니다.

정확한 최종 실행 파일은 한 번만 빌드하여 `candidate_path`로 복사하며 게이트가 실행되는
동안 다시 빌드하거나 patch, strip, 제자리 서명, 교체하지 않습니다. 후처리가 필요하면
후보 설명자 계산 전에 끝내야 합니다. 게이트와 audit 명령은 `source_revision`에 있는 같은
checkout에서 실행하고, 선언된 아카이브 및 소스 좌표를 다시 계산하는 동안 깨끗한 상태를
유지해야 합니다.

## 고정 12개 셀 행렬

Manifest 하나에는 아래 두 집합의 모든 곱에 해당하는 셀이 정확히 하나씩 들어갑니다.

- `host_kind`: `codex`, `claude_code`
- `feature`: `native_user_action`, `local_web_user_channel`,
  `verified_tool_producer`, `registered_connection_observation`,
  `record_final_output`, `detective_final_output`

결과는 실제로 존재하는 JSON 셀 파일 12개입니다. 중복, 누락, 추가, 다른 이름, 잘못된
형식, 필수 필드 집합 일부만 null인 입력은 구조적 명령 오류이며 manifest를 만들지
않습니다. 한 호스트 종류의 여섯 셀은 호스트 가용성 좌표 하나를 공유합니다. 모두 정확히
같은 `host_version`과 null이 아닌 실행 파일 다이제스트를 사용하거나, 모두 호스트 버전과
실행 파일 다이제스트에 명시적 `null`을 사용해야 합니다. null이 아닌 클라이언트 정체성을
가진 셀 사이에서는 여섯 셀이 정확한 `client_name`과 `client_version` 쌍을 최대 하나만
사용합니다. 정적 disposition이 MCP initialize 전에 단락될 수 있으므로 정적으로
`unsupported_by_host`인 셀은 호스트 가용성 좌표가 null이 아니어도 null 클라이언트
정체성을 사용할 수 있습니다. null이 아닌 호스트 버전, 실행 파일, 클라이언트 정체성이
서로 다른 결과를 호스트 결과 하나로 합치면 안 됩니다. 새 호스트 버전이나 클라이언트
정체성에는 완전한 12개 셀 manifest를 새로 만들어야 합니다.

`volicord-host-release-cell-v3`에는 다음 필수 구성원만 들어갑니다.

| 구성원 | 계약 |
|---|---|
| `schema` | 정확한 값 `volicord-host-release-cell-v3`. |
| `candidate_id`, `binary_sha256`, `source_revision`, `target_triple`, `release_profile` | 후보 좌표의 정확한 복사본. |
| `host_kind`, `host_version` | 고정 호스트 종류 하나와 셀이 관찰한 정확한 설치 호스트 버전, 또는 해당 호스트를 사용할 수 없을 때 명시적 `null`. 구성원 자체는 항상 필수입니다. |
| `client_name`, `client_version` | 이 셀의 성공한 관리 MCP `initialize`에서 관찰한 정확한 `clientInfo.name`과 `clientInfo.version`인 필수-nullable 구성원. null이 아닌 각 값은 1바이트 이상 256 UTF-8 바이트 이하이고 공백이 아닌 문자를 하나 이상 포함하며 제어 문자가 없어야 하고, 그 밖에는 정확한 문자열을 그대로 보존합니다. 정적 미지원 셀은 MCP를 initialize하지 않고 명시적 `null`을 사용할 수 있습니다. |
| `adapter_profile`, `adapter_version` | 정확한 관리 어댑터 좌표. |
| `feature` | 고정 기능 식별자 여섯 개 중 하나. |
| `implementation_disposition` | `implemented` 또는 `unsupported_by_host`. 담당자가 검토한 정적 입력이며 실제 실행 결과가 아닙니다. |
| `requested_verified` | 이 정확한 호스트 가용성·기능에 검증됨 주장을 요청했는지를 나타내는 boolean. 구현된 셀의 기본값은 `true`이고 명시적 `false`는 릴리스 제외 및 하향 조정입니다. 정적 미지원 셀은 `false`여야 합니다. |
| `claimed_status` | 생산자가 주장한 `HostFeatureSupportStatus`. 불일치 보고에만 보존하며 신뢰하지 않습니다. |
| `run_state` | `completed`, `running`, `ignored`, `not_applicable`. 정적 `unsupported_by_host`만 `not_applicable`을 사용할 수 있습니다. |
| `started_at`, `recorded_at` | 셀 시작 시각과 변경 불가능한 결과 기록 시각. |
| `environment` | 정확한 `runner_os`, `runner_os_version`, `runner_arch`, 필수-nullable `host_executable_sha256`, `host_version`, `client_name`, `client_version`, 실행에 사용한 모든 호스트·어댑터 좌표. 중복된 정체성 값은 최상위 값과 정확히 일치합니다. |
| `assertions` | 안정된 assertion ID, `passed` boolean, 선택적인 크기 제한 finding code를 담은 비어 있지 않은 크기 제한 배열. |
| `evidence_artifact_path`, `evidence_artifact_sha256` | 외부에 새로 만드는 크기 제한 증거 파일과 SHA-256. 사용할 수 없어 ignored인 셀을 포함한 구현된 셀에는 둘 다 필수이고, 정적 `unsupported_by_host`일 때만 둘 다 `null`. |

기존 호스트 가용성 집합인 최상위 `host_version`, `environment.host_version`,
`environment.host_executable_sha256`은 모두 문자열이거나 모두 명시적 `null`입니다. 별도로
최상위 및 `environment`의 `client_name`과 `client_version` 복사본 네 개는 모두 문자열이거나
모두 명시적 `null`입니다. null이 아닌 클라이언트 구성원에는 null이 아닌 호스트 가용성
집합이 필요합니다. 값이 있으면 각 `environment` 클라이언트 값은 최상위 복사본과 정확히
같아야 합니다. 필수-nullable 구성원을 생략하거나 한 집합의 일부만 null이거나 타입이
잘못되었거나 호스트 가용성이 null인데 클라이언트 정체성이 null이 아니면 하향 조정이 아니라
구조적 오류입니다. 중복 값 불일치는 좌표 불일치이며, 복사하거나 추론한 정체성은 관찰한
좌표가 아닙니다.

v3 평가기는 생산자가 적은 `implementation_disposition`을 독립된 사실로 받아들이지 않고
정확한 호스트 종류별 담당 표와 대조합니다. Codex와 Claude Code는 여섯 기능 표면을 모두
구현하며 `host_version`은 그 disposition을 선택하거나 바꾸지 않습니다. 검토된 Codex의
정확한 probe 출력은 계속 `codex-cli 0.144.4`이고 셀은 probe 외피가 아니라 해석된 정규
bare 좌표 `0.144.4`만 저장합니다. null이 아닌 모든 Codex `host_version`은 공유 정규 bare
버전 parser를 통과해야 합니다. `host_version`에 `codex-cli 0.144.4` 같은 원문 probe 외피를
넣으면 구조적 오류입니다. 정확한 셀 Evidence가 없으면 구현된 셀은
`implemented_unverified`로 남으며 정적 `unsupported_by_host`가 되지 않습니다.

정확한 버전은 릴리스 Evidence와 검증 좌표일 뿐 기능 disposition이나 런타임 gate가
아닙니다.
런타임 지원은 [Agent Connection](agent-connection.md#host-feature-support-state)에 따라
capability probe를 우선합니다. 다른 또는 더 새로운 유효 설치 버전은 구현된 표면에 대해
probe와 최신 Evidence가 다른 상태를 확정할 때까지 `implemented_unverified`이며, 이 표에
행이 없다는 이유만으로 `unsupported_by_host`가 되지 않습니다. 그 버전의 릴리스 주장은
여전히 담당 문서 변경과 자체 완전한 12개 셀 manifest를 요구합니다.

null이 아닌 클라이언트 정체성으로 허용되는 유일한 값은 해당 셀에 사용한 성공한 관리 MCP
`initialize`에서 실제로 관찰한 정확한 쌍입니다. `host_kind`, 호스트 실행 파일 이름, 버전
probe 출력, 환경이나 설정, 프로토콜 버전, 알려진 상수, 이후 도구 메타데이터, 다른 셀에서
이를 추론하면 안 됩니다. 기록기는 해당 관리 세션의
`session_watch_baselines.metadata_json`에 보존된 크기가 제한된 최상위 `client_name`과
`client_version`을 읽을 수 있습니다. 원본 initialize 메시지나 원본 프로토콜, 세션,
thread, turn, tool-call payload를 릴리스 증거로 보존하거나 사용하면 안 됩니다.

인증된 셀 호스트 프로세스를 하나라도 시작하기 전에 기록기는 해당 셀의 초기화 결과에서
얻은 정확한 Agent Connection ID를 단조롭게 결속해야 합니다. 같은 정확한 ID를 다시
결속하는 것은 멱등입니다. 결속이 없거나 형식이 잘못되었거나 기존 값과 충돌하면 종단
구조 기록기 실패입니다. 생산자는 호스트 프로세스를 시작하거나 어느 최종 이름도
게시하면 안 됩니다. 호스트를 시작하지 않는 정적 미지원 경로나 호스트 없음 경로는 결속
없이 끝날 수 있습니다.

기록기는 인증된 셀 호스트 turn 전에 셀에 결속된 깨끗하고 폐기 가능한 Runtime Home의
정확한 관리 기준선을 크기가 제한된 형태로 먼저 관찰하고, 최종 셀을 기록하기 전에 같은
범위를 다시 관찰합니다. 불투명 관리 세션, 연결, 프로젝트, 호스트 좌표가 그 인증된 turn과
일치하고, 그 turn 동안 새로 만들어졌거나 그 turn의 성공한 관리 `initialize`를 기록하여 두
관찰 사이에 `metadata_json`이 바뀐 기준선 행에서만 클라이언트 정체성을 받아들일 수
있습니다. 두 관찰에 모두 존재하고 메타데이터가 바뀌지 않은 행은 같은 연결에 속하고 예상
쌍을 포함하더라도 과거 행이므로 해당 셀의 증거가 아닙니다. 기록기는 연결 전체 이력을
검색해 유일하거나 가장 최신인 정체성을 대신 사용할 수 없습니다. 조건을 만족하는 모든
행은 정확히 같은 쌍 하나를 제공해야 합니다. 조건을 만족하는 행이 없으면 클라이언트 집합을
null로 두고, 일부만 있거나 형식이 잘못되었거나 서로 다른 결과가 있으면 값을 채우거나
교체하거나 추론하지 않고 셀 기록을 중단합니다.

조건을 만족하는 각 행의 after 관찰은 정확한
`{project_id, watch_baseline_id}` 키와 정확한 `metadata_json` 바이트의 SHA-256을 그 행의
예상 after-turn 다이제스트로 보존합니다. 이 키에 세션이나 연결 필드를 중복해서 넣지
않습니다. 정규 기준선 ID가 검증된 불투명 세션을 결속하고, 다시 연 행과 메타데이터
다이제스트가 정확한 연결, 프로젝트, 호스트 좌표를 결속합니다. 같은 셀에서 인증된 turn을
추가로 실행할 때는
그 turn의 before 관찰에 기존 예상 다이제스트가 정확히 있어야 이미 보존한 키를 전진시킬
수 있으며, 이때 after 관찰의 다이제스트가 새 예상값이 됩니다. 변경이 없는 재생은 기존
예상 다이제스트를 그대로 유지합니다. 게시 단계에 들어가기 전에 기록기는 보존한 모든
키를 다시 열고, 행이 정확한 관리 세션, 연결, 프로젝트, 호스트, 정규 기준선 ID, 예상
메타데이터 다이제스트와 함께 존재하는지 확인합니다. 같은 키의 행을 삭제하거나 교체하거나,
이후에 캡처한 turn으로 설명되지 않는 메타데이터 변경이 있으면 이 생산자가 어느 최종
이름도 게시하기 전에 기록을 중단합니다. 동시에 존재하는 이름을 제거하거나 교체하지
않으며, 실패를 null 클라이언트 집합이나 `implemented_unverified` 셀로 바꾸지 않습니다.

한 호스트 종류에서 클라이언트 정체성이 null이 아닌 모든 셀은 정확한 클라이언트 쌍 하나를
사용합니다. 구현된 모든 exact-live 셀은 `client_version == host_version`일 때만
`verified`로 도출될 수 있습니다.
검토한 Codex `host_version=0.144.4` 좌표는 추가로 `client_name=codex-mcp-client`와
`client_version=0.144.4`를 요구합니다.

canonical `adapter_profile`은 `record_final_output`에서만 `record`이고,
`detective_final_output`을 포함한 나머지 다섯 기능에서는 `detective`입니다.
`adapter_version`은 검증된 비공개 후보의 `--version` 출력에서 파싱한 정확한
`build_id`입니다. 정적 `unsupported_by_host` 셀을 포함한 모든 셀에서 최상위 및
`environment`의 두 좌표 복사본은 이 canonical 값과 같아야 합니다. 임의 문자열을 두
곳에 똑같이 복사한 것은 좌표 일치가 아닙니다.

`assertions` 배열은 `assertion_id`의 바이트 순서로 정렬하며 disposition과 기능에 따라
아래의 정확한 집합만 포함합니다.

| Disposition 또는 기능 | 정확한 assertion ID |
|---|---|
| `unsupported_by_host` | `static_unsupported_by_host` |
| `native_user_action` | `actual_host_session`, `authority_receipt_observed`, `native_user_selector_observed`, `operator_choice_confirmed`, `same_connection_resume` |
| `local_web_user_channel` | `actual_host_session`, `browser_submission_observed`, `host_owned_surface_observed`, `model_visible_payload_absence_observed`, `same_connection_resume`, `strong_evidence_close_chain`, `trusted_capability_current` |
| `verified_tool_producer` | `actual_host_tool_event`, `capture_receipt_bound`, `criterion_coverage_projected`, `exact_session_connection_actor_scope_baseline`, `intent_precedes_source`, `negative_rejections_zero_effect`, `strong_producer_chain` |
| `registered_connection_observation` | `actual_host_connection_event`, `capture_receipt_bound`, `criterion_coverage_projected`, `exact_session_connection_actor_scope_baseline`, `intent_precedes_source`, `negative_rejections_zero_effect`, `strong_producer_chain` |
| `record_final_output` | `actual_host_session`, `authenticated_exact_replay_observed`, `authority_display_observed` |
| `detective_final_output` | `actual_host_session`, `authenticated_exact_replay_observed`, `authority_display_observed`, `block_finalization_observed` |

`verified_tool_producer`에서 실제 하네스의 source 관찰 장벽은 불변 intent 및 intent 이후의
정확한 `pre_tool` 이벤트와 짝을 이루는 완전한 `post_tool` 이벤트가 영속 저장되는
시점입니다. 이때 짝이 되는 `pre_tool` 결정은 `deny`가 아니어야 합니다. Stop 이벤트나
결정, 닫기 준비 결과, 모델 응답 완료, 호스트 turn 완료, 호스트 프로세스 종료는 이 장벽의
일부가 아닙니다. Stop으로 선택한 `registered_connection_observation`의 source 관찰 장벽은
intent 이후의 정확한 Stop 이벤트가 영속 저장되는 시점입니다. 완료 주장 flag와 항상
allow인 종료 결과는 캡처하는 source 결과이며 프로세스 종료 전제 조건이 아닙니다.
`guard_events` 행은
append-only이므로 단독 `post_tool` 또는 불완전한 `post_tool`을 관찰하면 종단 source-shape
실패입니다. 나중에 행을 추가해도 어느 상태도 정확한 쌍이 될 수 없습니다. 후보가 없거나
deny가 아닌 정확한 `pre_tool` 하나만 있는 경우에만 쌍을 기다리는 상태로 남습니다. 제한
시간 경계에서는 영속 상태를 마지막으로 한 번 더 검사하여 막 커밋된 정확한 쌍을 timeout으로
바꾸지 않습니다. 유지되는 하네스는 해당 장벽
직후 intent 만료 전에 불일치 거부와 정확한 receipt capture를 수행하며, 호스트 turn이나
프로세스 종료를 기다리지 않습니다. Receipt capture는 별도의 source fulfillment
트랜잭션입니다. 생명주기 신호는 intent 기간을 연장하지 않으며 누락된 source 이벤트를
대신하지 않습니다.

이 두 producer 셀에서 `negative_rejections_zero_effect`의 v3 의미는 좁습니다. Tool
producer에서는 pre/post 참조를 뒤집은 경우, connection producer에서는 보존한 intent 이전
다른 세션 Stop 참조를 사용한 경우라는 실제 host 불일치 탐침 하나에 대해, 거부된 명령 전후로
표본화한 capture 소유 Core 테이블, 선택한 불변 intent 및 source 행, Project clock과 version,
범위가 제한된 전체 artifact-store 파일 집합이 같음을 증명합니다. 동시에 변할 수 있는 host
생명주기 session 및 watch 행은 이 snapshot에서 제외하고 turn 뒤에 별도로 다시 검증합니다.
Invocation identity, actor, scope revision, baseline, connection,
session, freshness의 누락이나 불일치를 각각 독립적인 실제 host 사례로 실행했다는 뜻은
아닙니다. Fixture-only 표는 공유 predicate를 보호할 수 있지만 해당 실제 host 관찰로 보고할
수 없습니다. v3 sidecar에는 사례별 provenance 필드가 없으므로 셀이나 gate 판정을 더 넓은
negative 행렬의 증거로 인용하면 안 됩니다.

`native_user_action`의 `authority_receipt_observed`는 실제 셀이 정확히 인증된 세션, 같은
연결, Task에 결속된 Stop 이벤트가 저장한 완전하고 최신인 영수증을 관찰했다는 뜻입니다.
영수증은 선택한 Project, Task, 현재 `state_version`, 선택지를 소비한 정확한 Run에
결속되어야 합니다. 저장된 Stop 결정, 완료 주장 flag, 이유, `close_state`, 완전한
`close_blockers` 집합도 영수증과 내부적으로 일관되어야 합니다. Stop 종료는 항상
`allow`입니다. 유지하는 깨끗한 픽스처는 정확히 두 완료 결과만 허용합니다. 경고나 차단
사유 없이 완전한 `mcp_start` 관찰 범위를 가진 `ready`와
`completion_claim_allowed=true`, 또는 일부 관찰 경고가 있는 활성
`first_project_selection`이나 `method_boundary` 관찰 범위에서
`close_readiness_blocked` 이유 하나와 정확한 `session_watch_unavailable` 차단 사유를 가진
`completion_claim_allowed=false`입니다. 어느 결과도 호스트에 Stop retry를 요구하지
않습니다. 영수증이 정직하더라도 다른
결과이면 이 셀은 실패합니다. 새 LocalUser 상태가 `ready`이고 차단 사유가 없는 것은
깨끗한 픽스처의 별도 정상성 전제이며, 이 검증 단언을 충족하는 영수증은 아닙니다.
LocalUser CLI 상태 영수증과 Agent Connection Stop 영수증은 권한 좌표를 공유하지만 호출
맥락별 상태 보기입니다. 따라서 `close_state`와 `close_blockers`가 같을 필요가 없으며 두
호출 맥락의 영수증 전체가 같다는 것은 검증 단언이 아닙니다.

기존 이름 `block_finalization_observed`는 종료를 허용하면서 호스트가 완료 주장 억제를
별도로 표시했다는 뜻이며 Stop deny나 retry를 뜻하지 않습니다. 이 호스트 고유 셀 관찰은 `authority_display_observed`,
`authenticated_exact_replay_observed`, `block_finalization_observed`를 충족하지 않으며
어느 최종 출력 기능도 승격하지 않습니다. 해당 검증 단언은 계속 각 최종 출력
셀만 담당합니다.

정직하게 실행하지 않은 구현 셀은 `run_state=ignored`이고 필수 assertion이 실패하며 크기가
제한된 증거 아티팩트를 가진 실제 셀로 표현합니다. 호스트를 사용할 수 없으면 호스트 가용성
집합과 클라이언트 집합을 모두 null로 사용합니다. 호스트를 사용할 수 있지만 성공한 관리
initialize 정체성을 관찰하지 못했으면 null이 아닌 호스트 가용성 집합과 null 클라이언트
집합을 사용합니다. 어느 셀이든 `implemented_unverified`로 도출됩니다. 따라서
`requested_verified=true`이면 게이트가
실패하고, 명시적 `requested_verified=false`이면 `pass_with_downgrades`만 허용합니다. null인
정적 미지원 셀은 `run_state=not_applicable`, null 증거, `requested_verified=false`를
사용합니다. 이 셀은 호스트 가용성 집합이 null이든 null이 아니든 클라이언트 집합을 null로
둘 수 있습니다. 주장 및 하향 조정 키의 null 버전 구간에는 리터럴 `unavailable`을 사용합니다.
셀이나 증거 파일이 없거나 형식이 잘못된 것은 이런 정직한 하향 조정 표현이 아니라 구조적
명령 오류이므로 manifest를 만들지 않습니다. 구현된 셀이 완료 상태로 통과하려면 모든
필수 assertion이 통과하고 증거 아티팩트가 존재하며 크기 제한 안에 있고 기록한
다이제스트와 일치해야 합니다.

클라이언트 집합이 null인 구현 셀은 도출 finding code에 `client_identity_missing`을
포함합니다. null이 아닌 클라이언트 집합의 중복 복사본이 서로 다르거나,
`client_version`이 `host_version`과 다르거나, 검토한 Codex 쌍이
`codex-mcp-client`/`0.144.4`와 다르면 `client_identity_mismatch`를 포함합니다. 어느
finding도 `implemented_unverified`를 강제하며 다른 셀이나 추론한 값으로 복구하지
않습니다. 중복 복사본 불일치는 `all_cell_environment_coordinates_exact` 불변조건도
실패시킵니다. 정적 미지원 셀은 호스트 가용성 집합이 null이든 null이 아니든 null 클라이언트
집합을 두 finding 없이 유지하면서 `unsupported_by_host`로 도출될 수 있습니다. 한
호스트의 null이 아닌 정체성이 서로 다르면
`single_host_client_identity_per_host` 불변조건이 실패합니다.

구현된 셀에서 `run_state=completed`는 선택한 시도가 종단 분류에 도달하고 기록기가 변경할
수 없는 종단 바이트를 만들었다는 뜻이며 기능이 통과했다는 뜻이 아닙니다. 설치된 호스트를
결속한 뒤 분류 가능한 기능, source 관찰, capture 또는 producer chain 시도가 모든 필수
assertion의 판정을 확정하기 전에 실패했고 게시 무결성 재검증은 모두 성공했다면, 생산자는
크기가 제한된 증거를 기록하고 입증하지 못한 모든 필수 assertion을 크기가 제한된 finding
code와 함께 `passed=false`로 두며 `implemented_unverified`보다 강한 상태를 주장하지
않습니다. 자식 프로세스 회수, turn 이후 기준선, 보존한 정체성, 후보 무결성, 게시 자체의
실패는 최종 셀이 없는 구조적 실패로 유지합니다. `ignored`는 구현된 호스트 경로를 실행하지
않은 경우에만 사용합니다. `running`은 종단 셀로 게시하지 않습니다. 필수 assertion 하나
이상이 실패한 엄격한 커밋 `completed` 셀이 선택한 시도 실패의 정규 표현이며
`implemented_unverified`로 도출됩니다.

## 정규 평가와 최신성

정규 평가기는 후보 하나, 원본 셀 정확히 12개, `evaluated_at` 하나를 입력받습니다. 구조를
검사하고 도달할 수 있는 모든 파일 다이제스트를 다시 계산한 다음 각 상태를 도출합니다.
도출 상태의 입력으로 `claimed_status`를 받아들이지 않습니다.

실제 셀은 다음 조건을 만족할 때만 최신입니다.

```text
started_at <= recorded_at <= evaluated_at < started_at + 24h
```

후보가 셀보다 먼저 완성되어야 하므로
`candidate.recorded_at <= cell.started_at`도 만족해야 합니다. 후보 설명자를 완성하기 전에
시작한 셀은 정확한 실제 증거가 아니며 하향 조정됩니다.

구간은 반개구간입니다. 24시간 경계와 같으면 오래된 셀입니다. 미래, 역순, 잘못된 형식,
비정규 고정밀도 타임스탬프는 유효하지 않습니다. 나머지 좌표가 모두 일치해도 다른 호스트
버전의 셀을 호스트 결과 하나로 평가하면 유효하지 않습니다.

상태는 다음과 같이 결정적으로 도출합니다.

1. 정적 검토를 거친 `unsupported_by_host` disposition은
   `unsupported_by_host`로 도출합니다. 실제 실행 부재나 실패가 이를 승격하거나 다른
   상태로 바꿀 수 없습니다.
2. `implemented` 셀은 존재하고, `completed`이며, 최신이고, 좌표·클라이언트 정체성·
   다이제스트가 정확히 일치하고, 모든 assertion이 통과할 때만 `verified`로 도출합니다.
3. 실제로 존재하는 구현 셀이 `ignored`, `running`, 오래됨, 실패, 불일치이면
   `implemented_unverified`로 도출합니다. 구조적 입력이 없거나 형식이 잘못되면 상태를
   합성하지 않고 manifest 생성을 막습니다.
4. 설정과 현재 런타임 선행 조건은 별개입니다. 기존 런타임 평가기를 통해서만
   `temporarily_unavailable`이 될 수 있으며 릴리스 게이트가 이 상태를 만들어 내지 않습니다.
5. `claimed_status`와 도출 상태가 다르면 finding을 남기고 도출 상태를 사용합니다.

`requested_verified=true`인 셀 중 하나라도 `verified`로 도출되지 않거나 후보 또는
manifest 불변조건이 실패하면 게이트 판정은 `fail`입니다. 요청한 검증됨 주장을 모두
충족했지만 구현된 기능 하나 이상이 검증되지 않았거나
`requested_verified=false`로 명시적으로 제외되면 판정은
`pass_with_downgrades`입니다. 제외된 셀의 증거가 독립적으로 `verified`로 도출되어도
제외는 하향 조정으로 남습니다. 모든 불변조건이 깨끗하고 구현된 모든 셀이 검증되며
제외되지 않은 행렬만 `pass`입니다.

## 릴리스 Manifest

`volicord-host-release-manifest-v3`에는 다음 구성원만 들어갑니다.

- 정확한 값 `volicord-host-release-manifest-v3`인 `schema`
- 검증을 마친 완전한 후보 객체인 `candidate`
- `evaluated_at`
- 각 원본 셀, `derived_status`, 안정된 `finding_codes`를 담은 정확히 12개의 `cells`
- `requested_verified=true`인 정확한 호스트·버전·기능 키를 정렬한
  `requested_verified_claims`
- `verified`로 도출되지 않았거나 `requested_verified=false`로 명시적으로 제외된 구현 셀을
  정렬한 `downgrades`
- 정렬된 크기 제한 목록 `invariant_findings`
- `pass`, `pass_with_downgrades`, `fail` 중 도출된 `verdict`

게이트는 모든 셀을 평가한 뒤에만 manifest를 새로 만듭니다. 호출자는 상태, finding,
하향 조정, 판정을 덮어쓸 수 없습니다. JSON 키 순서에는 의미가 없습니다. 위에서
정렬한다고 명시한 배열의 순서는 의미가 있으며 UTF-8 바이트 오름차순을 사용합니다.

## 독립 Audit

Audit은 후보, 셀 파일, manifest를 만들지 않은 새 프로세스에서 실행합니다. Manifest에
내장된 원본 셀을 신뢰하지 않고, 지정한 셀 디렉터리에서 원본 `.json` 파일 정확히 12개를
독립적으로 엄격하게 읽습니다. 외부 경로의 변경 불가능한 입력을 열어 manifest SHA-256,
후보 SHA-256, 소스 아카이브 SHA-256, 셀 입력 및 셀 증거 SHA-256, 모든 구조 불변조건, 각
도출 상태, 게이트 판정을 다시 계산합니다. Manifest의 주장 상태나 판정만 읽는 모드를
호출해서는 안 됩니다.

`volicord-host-release-audit-v3`에는 다음 구성원만 들어갑니다.

- 정확한 값 `volicord-host-release-audit-v3`인 `schema`
- `manifest_path`, `manifest_sha256`, `cell_directory`, `cell_inputs_sha256`,
  `candidate_path`, `candidate_sha256`
- 별도 audit 프로세스의 `started_at`, `evaluated_at`
- 안정된 invariant ID와 통과 여부를 담은 `invariant_results`
- 필수-nullable `host_version`, `client_name`, `client_version`, 12개 셀의 모든 재계산
  상태와 finding code를 담은 `recalculated_cells`
- 불일치 또는 잘못된 입력을 정렬한 크기 제한 목록 `findings`
- 의도적으로 하지 않은 검사를 정렬한 크기 제한 목록 `exclusions`와 각 항목의 비어 있지
  않은 이유
- `recalculated_verdict`와 `pass` 또는 `fail`인 `audit_verdict`

`cell_directory`는 입력 디렉터리의 정확한 외부 절대 경로 문자열입니다.
`cell_inputs_sha256`은 다음 preimage의 SHA-256입니다. ASCII domain
`volicord-host-release-cell-inputs-v3` 뒤에 NUL을 붙이고, 정확한 UTF-8 절대 셀 경로
바이트를 바이트 오름차순으로 정렬한 뒤 12개 셀 각각에 대해 경로 바이트 길이를 unsigned
64-bit big-endian으로, 정확한 경로 바이트를, 셀 파일의 정확한 바이트에 대한 SHA-256
32바이트 원본 값을 차례로 붙입니다. Audit은 다시 연 셀 입력이 manifest의 원본 셀과
같아야 한다고 요구합니다. 원본 셀 입력 없이 manifest만 일관되게 다시 쓴 경우에도
`cell_inputs_match_manifest`가 실패합니다. `candidate_sha256`으로 기록하는 최종 경로
다이제스트도 설명자 다이제스트와 같아야 합니다. 그렇지 않으면
`audit_candidate_binary_digest_exact`가 실패하고 audit 판정은 `fail`입니다.

`audit_verdict=pass`가 되려면 finding이 없고, 요청한 검증됨 주장 또는 불변조건에 영향을
주는 exclusion이 없고, manifest와 정확히 일치하며, 재계산 manifest 판정이 `fail`이
아니어야 합니다. Audit 목적지도 외부에 새로 만듭니다. Manifest와 audit은 릴리스
증거이며 Volicord 런타임 신뢰 입력, Core 증거, User Channel 권한, host attestation,
게시 허가가 아닙니다.

## 관리 호스트 세션 결속

Codex 및 Claude Code의 native session identifier는 관리 어댑터 경로에서만 받습니다. 원본
값은 유효한 UTF-8이고 1바이트 이상 256바이트 이하여야 하며
`[A-Za-z0-9._:-]+`와 일치해야 합니다. 공백, 제어 문자, 빈 값, 그 밖의 모든 바이트는
거부합니다.

검토한 Codex 좌표에서 설치 호스트 probe envelope는 정확히 `codex-cli 0.144.4`이고 MCP
initialize는 정확히 `clientInfo.name=codex-mcp-client`와
`clientInfo.version=0.144.4`를 보고해야 합니다. 관리 Codex provenance가 있는 시작은
세션 미결속 상태입니다. 그 밖의 조건을 만족하는 알려진 tool의 유효한 call만 다음의
정확한 요청 메타데이터로 결속을 제공할 수 있습니다.

- `_meta.threadId`는 유효한 native identifier입니다.
- `_meta["x-codex-turn-metadata"]`는 `session_id`, `thread_id`, `turn_id`가 모두 유효한
  native identifier인 객체입니다.
- `_meta.threadId`는 중첩 `thread_id`와 같습니다.

아래 매핑에서 native session 값으로 사용하는 것은 중첩 `session_id`입니다. Subagent를
포함하여 구체적인 `thread_id`는 `session_id`와 다를 수 있지만 평면 및 중첩 복사본은
일치해야 합니다. Volicord는 이 thread 값을 별도의 domain 분리 프로세스 로컬 다이제스트로
줄입니다. 첫 유효 call은 관리 stdio 프로세스를 매핑된 root 세션과 이 thread 다이제스트
모두에 정확히 한 번 결속합니다. 이후 모든 call은 두 값 모두에 맞는 유효한 메타데이터를
가져야 하며 이후 turn은 다른 유효한 `turn_id`를 사용할 수 있습니다. 메타데이터가 없거나
잘못됐거나 일치하지 않으면 다시 결속하지 않고 tool dispatch 전에 거부합니다. 주변 또는
설정의 `CODEX_THREAD_ID`, 시각, 도착 순서, 가장 가깝거나 최근인 세션은 결속 입력이
아닙니다.

검증한 값에 대해 Volicord는 다음을 계산합니다.

```text
digest = SHA-256(
  b"volicord-managed-host-session-v1\0" ||
  host_kind_utf8 || b"\0" || connection_internal_id_utf8 || b"\0" ||
  native_session_id_utf8
)
managed_host_session_id = "mhs_" || lowercase_hex(digest)
```

관리 MCP 관찰과 호스트 훅 관찰은 같은 `managed_host_session_id`를 사용합니다. `mhs_`
namespace는 이 관리 매핑에만 예약됩니다. 등록된 연결 및 호스트 종류 좌표는 바꿀 수
없으며 generic 또는 manual 경로에서 미리 심거나 재사용할 수 없습니다. 유효하지 않은
관리 marker는 영속 진단이나 프로토콜 상태를 만들기 전에 실패합니다. Codex 시작이 세션
미결속 상태인 동안 성공한 startup, initialize, tools-list 사실은 크기가 제한된 프로세스
로컬 상태로만 보유할 수 있습니다. 영속 관리 세션, 생명주기, 진단, tool, capture, token,
watch 효과를 만들지 않습니다. 첫 유효 결속은 허용되는 보유 생명주기 사실을 정규 순서로
한 번만 구체화한 뒤 허용한 call을 기록합니다. 세션 watch 관찰 범위는 결속 시점에
시작하고 명시적으로 부분 범위에 머뭅니다. 지연된 생명주기 사실이 저장소 관찰 시점을
과거로 소급하지 않습니다. 거부된 결속 시도는 이런 효과를 만들지 않으며 이후 유효한
call이 다시 시도할 수 있습니다.

원본 native session identifier와 원본 native event, tool-call, capture, turn, invocation
identifier는 검증·해시하거나 domain 분리 불투명 값으로 바꾸는 동안에만 존재합니다. 영속
저장, 로그, 진단 렌더링, 증거 첨부, 릴리스 아티팩트 저장을 하지 않습니다. Native
식별자가 없거나 값이 잘못됐거나, 매핑 좌표가 다르거나, 관리 MCP 관찰과 훅 관찰이
일치하지 않으면 Strong Evidence를 만들 수 없으며 명시적인 누락 또는 불일치 finding으로
남겨야 합니다. 구현은 대체 세션 ID를 조용히 만들거나 호스트 종류 또는 등록된 연결
경계를 넘어 상관관계를 만들면 안 됩니다. Call별 결속은 기존 기능 assertion 집합으로
검사하며 릴리스 assertion 식별자를 추가하지 않습니다.

## 명령 경로

구현 패키지는 `tests/release-validation`이며 Cargo 패키지 이름은
`volicord-release-validation-tests`입니다. 정확한 검증 경로는 다음과 같습니다.

```sh
cargo test -p volicord-release-validation-tests
cargo run --locked -p volicord-release-validation-tests --bin host-release-candidate -- --candidate-id CANDIDATE_ID --candidate-path CANDIDATE_BINARY --candidate-out CANDIDATE.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-gate -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest-out MANIFEST.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-audit -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest MANIFEST.json --audit-out AUDIT.json
```

모든 아티팩트 및 디렉터리 인자는 이 계약을 따르는 외부 절대 경로입니다. 후보 명령은
최종 실행 파일을 외부에 배치한 뒤 첫 셀을 시작하기 전에 실행합니다. Audit 명령은 게이트
프로세스가 끝난 뒤 별도 프로세스로 실행합니다. 관리 CLI fallback과 진단은 이 아티팩트를
요약할 수 있지만 보조 수단일 뿐 후보, 게이트, audit 명령을 대신할 수 없습니다.

## 관련 담당 문서

- [시스템 요구사항](system-requirements.md)은 환경 적용 가능성을 담당합니다.
- [Agent Connection](agent-connection.md)은 런타임 지원 상태와 fallback을 담당합니다.
- [MCP Transport](mcp-transport.md)는 관리 stdio transport 동작을 담당합니다.
- [API 상태 스키마](api/schema-state.md)는 `AuthorityReceipt`와 닫기 준비 상태 보기의
  형태를 담당합니다.
- [`close_task`](api/method-close-task.md)는 닫기 준비 차단 사유의 코드, 범주, 해결 의미를
  담당합니다.
- [저장소 레코드](storage-records.md)는
  `session_watch_baselines.metadata_json`의 크기가 제한된 관리 initialize 정체성 배치를
  담당합니다.
- [보안](security.md)은 신뢰 및 비권한 경계를 담당합니다.
- [관리 CLI](admin-cli.md#guard-hook-commands)는 숨겨진 Stop 이벤트 동작과 운영자용 보조
  상태 보기를 담당합니다.
- [검증](../maintain/validation.md)은 유지관리자 실행과 보고를 담당합니다.
- [호스트 릴리스 증거 게이트 결정](../architecture-guide/decisions/host-release-evidence-gate.md)은
  이 계약을 외부에 두고 독립적으로 재계산하는 이유를 기록합니다.
- [관리 호스트 세션·thread 결속과 호출별 turn 검증 결정](../architecture-guide/decisions/managed-host-session-turn-binding.md)은
  Codex 결속이 call 범위이고 닫힌 상태로 실패하는 이유를 기록합니다.
