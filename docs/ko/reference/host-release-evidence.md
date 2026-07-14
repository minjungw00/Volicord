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
| 실제 호스트 행렬 결과 하나 | `volicord-host-release-cell-v1` |
| 정규 릴리스 게이트 결과 | `volicord-host-release-manifest-v1` |
| 별도 프로세스 재계산 결과 | `volicord-host-release-audit-v1` |
| 소스 아카이브 다이제스트 알고리즘 | `git_archive_tar_sha256_v1` |

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
| `build_environment` | 정확한 `runner_os`, `runner_os_version`, `runner_arch`, `git_version`, `rustc_version`, `cargo_version` 문자열. |
| `recorded_at` | 설명자와 모든 후보 다이제스트 계산을 완료한 시각. |

소스 checkout은 빌드 전에 깨끗해야 하며 후보 생성이 끝날 때까지
`source_revision`에 머물러야 합니다. 그 checkout에서 경로 접두사나 추가 attribute 없이
`git archive --format=tar <source_revision>`을 실행하고 명령의 원본 tar 표준 출력에
SHA-256을 계산합니다. 그 바이트 다이제스트가 `source_archive_sha256`입니다. 압축
아카이브, work tree, 디렉터리 목록, Git bundle을 해시하는 것은 같은 알고리즘이
아닙니다.

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
형식, 서로 다른 호스트 가용성 좌표를 섞은 입력은 구조적 명령 오류이며 manifest를 만들지
않습니다. 한 호스트 종류의 여섯 셀은 가용성 좌표 하나를 공유합니다. 모두 정확히 같은
`host_version`과 null이 아닌 실행 파일 다이제스트를 사용하거나, 모두 호스트 버전과 실행
파일 다이제스트에 명시적 `null`을 사용해야 합니다. 서로 다른 버전이나 가용성 좌표의
결과를 호스트 결과 하나로 합치면 안 됩니다. 새 호스트 버전에는 완전한 12개 셀
manifest를 새로 만들어야 합니다.

`volicord-host-release-cell-v1`에는 다음 필수 구성원만 들어갑니다.

| 구성원 | 계약 |
|---|---|
| `schema` | 정확한 값 `volicord-host-release-cell-v1`. |
| `candidate_id`, `binary_sha256`, `source_revision`, `target_triple`, `release_profile` | 후보 좌표의 정확한 복사본. |
| `host_kind`, `host_version` | 고정 호스트 종류 하나와 셀이 관찰한 정확한 설치 호스트 버전, 또는 해당 호스트를 사용할 수 없을 때 명시적 `null`. 구성원 자체는 항상 필수입니다. |
| `adapter_profile`, `adapter_version` | 정확한 관리 어댑터 좌표. |
| `feature` | 고정 기능 식별자 여섯 개 중 하나. |
| `implementation_disposition` | `implemented` 또는 `unsupported_by_host`. 담당자가 검토한 정적 입력이며 실제 실행 결과가 아닙니다. |
| `requested_verified` | 이 정확한 호스트 가용성·기능에 검증됨 주장을 요청했는지를 나타내는 boolean. 구현된 셀의 기본값은 `true`이고 명시적 `false`는 릴리스 제외 및 하향 조정입니다. 정적 미지원 셀은 `false`여야 합니다. |
| `claimed_status` | 생산자가 주장한 `HostFeatureSupportStatus`. 불일치 보고에만 보존하며 신뢰하지 않습니다. |
| `run_state` | `completed`, `running`, `ignored`, `not_applicable`. 정적 `unsupported_by_host`만 `not_applicable`을 사용할 수 있습니다. |
| `started_at`, `recorded_at` | 셀 시작 시각과 변경 불가능한 결과 기록 시각. |
| `environment` | 정확한 `runner_os`, `runner_os_version`, `runner_arch`, 필수-nullable `host_executable_sha256`와 `host_version`, 실행에 사용한 모든 호스트·어댑터 좌표. 최상위 및 environment의 호스트 정체성 필드 세 개는 모두 null이 아니거나 모두 null이어야 합니다. |
| `assertions` | 안정된 assertion ID, `passed` boolean, 선택적인 크기 제한 finding code를 담은 비어 있지 않은 크기 제한 배열. |
| `evidence_artifact_path`, `evidence_artifact_sha256` | 외부에 새로 만드는 크기 제한 증거 파일과 SHA-256. 사용할 수 없어 ignored인 셀을 포함한 구현된 셀에는 둘 다 필수이고, 정적 `unsupported_by_host`일 때만 둘 다 `null`. |

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

정직하게 실행하지 못한 구현 셀은 호스트 정체성이 null이고 `run_state=ignored`이며 필수
assertion이 실패하고 크기가 제한된 증거 아티팩트를 가진 실제 셀로 표현합니다. 이 셀은
`implemented_unverified`로 도출됩니다. 따라서 `requested_verified=true`이면 게이트가
실패하고, 명시적 `requested_verified=false`이면 `pass_with_downgrades`만 허용합니다. null인
정적 미지원 셀은 `run_state=not_applicable`, null 증거, `requested_verified=false`를
사용합니다. 주장 및 하향 조정 키의 null 버전 구간에는 리터럴 `unavailable`을 사용합니다.
셀이나 증거 파일이 없거나 형식이 잘못된 것은 이런 정직한 하향 조정 표현이 아니라 구조적
명령 오류이므로 manifest를 만들지 않습니다. 구현된 셀이 완료 상태로 통과하려면 모든
필수 assertion이 통과하고 증거 아티팩트가 존재하며 크기 제한 안에 있고 기록한
다이제스트와 일치해야 합니다.

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
2. `implemented` 셀은 존재하고, `completed`이며, 최신이고, 좌표와 다이제스트가 정확히
   일치하고, 모든 assertion이 통과할 때만 `verified`로 도출합니다.
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

`volicord-host-release-manifest-v1`에는 다음 구성원만 들어갑니다.

- 정확한 값 `volicord-host-release-manifest-v1`인 `schema`
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

`volicord-host-release-audit-v1`에는 다음 구성원만 들어갑니다.

- 정확한 값 `volicord-host-release-audit-v1`인 `schema`
- `manifest_path`, `manifest_sha256`, `cell_directory`, `cell_inputs_sha256`,
  `candidate_path`, `candidate_sha256`
- 별도 audit 프로세스의 `started_at`, `evaluated_at`
- 안정된 invariant ID와 통과 여부를 담은 `invariant_results`
- 필수-nullable `host_version`, 12개 셀의 모든 재계산 상태와 finding code를 담은
  `recalculated_cells`
- 불일치 또는 잘못된 입력을 정렬한 크기 제한 목록 `findings`
- 의도적으로 하지 않은 검사를 정렬한 크기 제한 목록 `exclusions`와 각 항목의 비어 있지
  않은 이유
- `recalculated_verdict`와 `pass` 또는 `fail`인 `audit_verdict`

`cell_directory`는 입력 디렉터리의 정확한 외부 절대 경로 문자열입니다.
`cell_inputs_sha256`은 다음 preimage의 SHA-256입니다. ASCII domain
`volicord-host-release-cell-inputs-v1` 뒤에 NUL을 붙이고, 정확한 UTF-8 절대 셀 경로
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
관리 marker는 영속 진단이나 프로토콜 상태를 만들기 전에 실패합니다.

원본 native session identifier와 원본 native event, tool-call, capture, turn, invocation
identifier는 검증·해시하거나 domain 분리 불투명 값으로 바꾸는 동안에만 존재합니다. 영속
저장, 로그, 진단 렌더링, 증거 첨부, 릴리스 아티팩트 저장을 하지 않습니다. Native
식별자가 없거나 값이 잘못됐거나, 매핑 좌표가 다르거나, 관리 MCP 관찰과 훅 관찰이
일치하지 않으면 Strong Evidence를 만들 수 없으며 명시적인 누락 또는 불일치 finding으로
남겨야 합니다. 구현은 대체 세션 ID를 조용히 만들거나 호스트 종류 또는 등록된 연결
경계를 넘어 상관관계를 만들면 안 됩니다.

## 명령 경로

구현 패키지는 `tests/release-validation`이며 Cargo 패키지 이름은
`volicord-release-validation-tests`입니다. 정확한 검증 경로는 다음과 같습니다.

```sh
cargo test -p volicord-release-validation-tests
cargo run --locked -p volicord-release-validation-tests --bin host-release-gate -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest-out MANIFEST.json
cargo run --locked -p volicord-release-validation-tests --bin host-release-audit -- --candidate CANDIDATE.json --cell-dir CELL_DIR --manifest MANIFEST.json --audit-out AUDIT.json
```

모든 아티팩트 및 디렉터리 인자는 이 계약을 따르는 외부 절대 경로입니다. Audit 명령은
게이트 프로세스가 끝난 뒤 별도 프로세스로 실행합니다. 관리 CLI fallback과 진단은 이
아티팩트를 요약할 수 있지만 보조 수단일 뿐 두 명령을 대신할 수 없습니다.

## 관련 담당 문서

- [시스템 요구사항](system-requirements.md)은 환경 적용 가능성을 담당합니다.
- [Agent Connection](agent-connection.md)은 런타임 지원 상태와 fallback을 담당합니다.
- [MCP Transport](mcp-transport.md)는 관리 stdio transport 동작을 담당합니다.
- [보안](security.md)은 신뢰 및 비권한 경계를 담당합니다.
- [관리 CLI](admin-cli.md)는 운영자용 보조 상태 보기를 담당합니다.
- [검증](../maintain/validation.md)은 유지관리자 실행과 보고를 담당합니다.
- [호스트 릴리스 증거 게이트 결정](../architecture-guide/decisions/host-release-evidence-gate.md)은
  이 계약을 외부에 두고 독립적으로 재계산하는 이유를 기록합니다.
