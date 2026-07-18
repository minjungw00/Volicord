# 저장소 기록

이 문서는 최초 릴리스 단일 저장 계약의 의미와 기록 간 불변식을 담당합니다. 정확한
테이블, column, constraint, index, 정규 SQL은 [저장소 DDL](storage-ddl.md)이
담당합니다.

## 저장 위치

| 위치 | 목적 |
|---|---|
| `registry.sqlite` | Runtime Home identity, 설치 profile, 프로젝트, alias, Agent Connection, 명시적 프로젝트 membership, 관리 Codex binding/검증 metadata |
| 프로젝트 `state.sqlite` | 프로젝트 로컬 Core 상태, replay, authority event, UserAction, evidence, artifact, continuity, Guard 관찰, 조정 |
| artifact store | 영속 artifact row가 참조하는 bytes와 안전 notice |
| `diagnostics.sqlite` | 제한된 비권한 operability counter |

각 프로젝트 상태 데이터베이스는 등록된 정규 Product Repository 하나에 속합니다.
프로젝트를 가로지르는 row, ref, replay, 현재 pointer는 유효하지 않습니다.

### 로컬 Diagnostics 계약

`diagnostics.sqlite`는 현재 SQL의 table, column, index inventory에서 파생한 정확한
canonical schema digest와 `contract_id=volicord.sqlite.diagnostics`로 식별되는 별도
비권한 저장 계약 하나를 사용합니다. 매니페스트에는 singleton row가 정확히 하나 있어야
합니다. 새 diagnostics 저장소는 데이터베이스 경로가 없을 때만 만듭니다. 이미 존재하는
빈 데이터베이스, 빠지거나 추가된 매니페스트 row, 알 수 없는 contract identifier,
현재가 아닌 digest, 빠지거나 변경되거나 예상하지 않은 schema object는 migration,
복구, importer dispatch, 형식 추론 없이 거부합니다.

이 diagnostics 매니페스트는 권한 `StorageManifest`가 아니며 diagnostics 데이터베이스는
숫자 schema version을 compatibility identity로 사용하지 않습니다. 읽기는 이
데이터베이스를 만들지 않습니다. Diagnostics 실패는 Core 또는 User Channel 결과를 바꿀
수 없습니다.

## 기록 계열

Registry 기록에는 다음이 포함됩니다.

- Runtime Home identity 하나와 현재 `StorageManifest` carrier
- 설치와 실행 파일 선택
- 프로젝트 등록과 alias
- Agent Connection과 Connection Projects membership
- Agent Connection마다 최대 하나의 정규 연결 검증 보고서
- 정규 `ManagedHostBinding` identity, 생성 artifact identity, 현재 검증 receipt 좌표

프로젝트 상태 기록에는 다음이 포함됩니다.

- `project_state`, 프로젝트 workflow policy, Task, acceptance criterion, supplemental
  claim, Change Unit
- 쓰기 티켓, Run, 현재 close basis, blocker, authority event, immutable replay row
- evidence capture intent와 receipt, artifact와 link, evidence summary, observation,
  producer
- `UserActionRequest`, immutable `UserActionResolution`, project continuity
- 조정에 쓰는 expected write, Guard 관찰, prompt 관찰, unrecorded change

Prompt 관련 Guard 기록은 관찰일 뿐입니다. UserAction resolution, 사용자 답,
verification basis, 권한 출처가 아닙니다.

## Identity와 소유권

저장 식별자는 정확하고 비어 있지 않은 담당자 값입니다. Store는 표시 text에서 식별자를
trim, 추측, 재할당하거나 대체 식별자를 도출하지 않습니다. 모든 Task 범위 row는 소유
row와 같은 프로젝트와 Task를 이름 붙입니다. 모든 Change Unit, evidence target, Run,
artifact link, blocker, continuity ref는 소유 좌표에 맞춰 검증합니다.

현재 pointer는 같은 프로젝트의 현재 record를 참조해야 합니다. 현재 상태가 전진한 뒤에도
immutable history가 남을 수 있지만 timestamp 비교나 record 이름 순서로 현재가 되지는
않습니다.

정규 Product Repository 경로는
[런타임 경계](runtime-boundaries.md#product-repository-api-path-normalization)를 따릅니다.
Git object ID는 공유된 정확한 소문자 16진수 40자리 또는 64자리 계약을 사용합니다.
다른 길이와 16진수가 아닌 값은 쓰기에서 유효하지 않고 읽기에서 손상입니다.

## 저장 UserAction 엄격 검증

`user_action_requests`는 닫힌 요청 본문 하나, Core 파생 typed basis, `required_for`,
source method/idempotency identity, actor, expiry를 저장합니다.
`user_action_resolutions`는 최대 하나의 닫힌 종류 일치 resolution,
`channel_kind=cli`, 제한된 visible-ASCII submission identity,
`resolved_by_actor_source=local_user`, verification basis, assurance, Core capture
time을 저장합니다.

Store는 쓰기와 읽기 모두에서 완전한 typed 요청과 resolution을 검증합니다. 다음을
거부합니다.

- 알 수 없거나 섞인 union tag와 추가 필드
- 빠진 종류별 필드
- 요청 본문과 일치하지 않는 `action_kind`
- 저장 후보 밖의 option 또는 evidence 선택
- CLI가 아닌 channel 또는 local-user가 아닌 provenance
- 잘못된 제한, timestamp, ref, submission identity
- 요청, 프로젝트, Task, 현재 basis가 일치하지 않는 resolution

잘못된 저장 값은 영속 데이터 기계 판독 code를 가진 `Corrupt`입니다. 기본값을 넣거나
조용히 건너뛰거나 다른 column에서 복구하거나 부분적으로 유효한 객체로 반환하지 않습니다.
CLI inbox는 fail closed하며 MCP는 안전한 실패만 노출할 수 있고 row를 해결하지 않습니다.

<a id="exact-operation-result-storage"></a>
## Replay와 효과

커밋된 non-dry-run Core mutation 하나는 적격 응답을 method, project, actor, operation
category, idempotency identity, request hash, state version, 선택적 검증 workspace 좌표와
함께 정확히 저장합니다. 정확한 retry는 원래 bytes를 반환하고 같은 identity에 다른 정규
입력을 쓰면 conflict입니다.

User-only resolution replay는 Agent Connection이 접근할 수 없습니다.
Request-user-action resume은 원래 agent-safe 요청 결과와 별도로 새로 읽은 안전한 현재
projection만 읽을 수 있습니다.

## Guard와 조정 기록

Expected-write와 unrecorded-change 기록은 프로젝트 로컬입니다. Guard suppression은
제한된 정규 correlation 데이터만 읽고 정확한 `SuppressionOutcome`을 반환합니다. Store
읽기 실패, 손상된 기록, budget 소진, 유효하지 않은 correlation은 `Unavailable`이며
관찰 경로를 숨기지 않습니다.

Prompt 관찰은 제한된 관찰 schema 아래에서만 저장할 수 있습니다. 사용자 choice,
resolution 본문, 비공개 inbox form, credential을 담지 않습니다.

## 현재 close basis와 continuity

현재 close basis는 Task 소유이며 terminal close history와 구분됩니다. 없음은 생성된 빈
basis가 아니라 없음으로 표현합니다. Evidence와 acceptance ref는 담당 문서 아래에서
정확하고 현재 상태여야 합니다.

Project continuity record는 오래 유지되는 맥락이며 면제가 아닙니다. Typed cursor와
ordering은 status 메서드가 담당합니다. Carry-forward는 현재 scope, baseline, 쓰기
티켓, evidence, UserAction, close 검사를 우회하지 않습니다.

## 저장소 소유 JSON

권한과 관련된 모든 JSON 필드는 닫힌 typed schema, digest가 bytes에 의존할 때의 정규
encoding, 명시적 크기 제한을 사용합니다. 알 수 없거나 빠지거나 중복되거나 타입이
잘못되거나 noncanonical이거나 담당자 불변식과 맞지 않는 member는 유효하지 않은 입력이며
영속 데이터 손상입니다.

명시적으로 비권한으로 분류한 metadata는 계속 비권한입니다. 사용자 판단, evidence
assurance, acceptance, 쓰기 티켓 권한, 닫기 준비 상태를 만들 수 없습니다.

### Agent Connection 검증 보고서

`agent_connections.verification_report_json`은 유일한 영속 연결 검증 상태입니다.
Null이 아닌 값은 완전하고 엄격한 `ConnectionVerificationReport` 하나이며 파생 상태,
check, 사용자 action을 독립적으로 저장하거나 변경할 수 없습니다. SQL null은 완료된
보고서가 없다는 뜻입니다. 읽기 경로는 Registry 저장소를 바꾸지 않고 그 부재를 Agent
Connection 담당 문서의 합성 `verification_not_run` 보고서로 projection합니다.

Store는 쓰기 전과 읽은 뒤 공유 보고서 type을 검증합니다. 닫힌 값, 상한, 결정적 순서,
중복 거부, 파생 집계를 모두 확인합니다. 형식이 잘못되었거나 비정규인 보고서 JSON은
영속 담당 상태 손상입니다. 보고서 부재로 해석하거나 다른 column에서 복구하지 않습니다.

<a id="authority-bundle-export"></a>
## 권한 번들 내보내기

비변경 권한 번들은 담당 문서가 정의한 일관된 snapshot을 읽습니다. Diagnostics,
credential, 비공개 UserAction note, prompt, 대화 기록, runtime log, export 담당자가
선택하지 않은 artifact bytes는 포함하지 않습니다. 내보내기는 프로젝트 상태를 바꾸지
않습니다.

내보내는 record table 집합은 별도로 유지하는 table 목록이 아니라 정규 프로젝트 상태
`GeneratedSchemaMetadata`에서 투영합니다. `acceptance_criteria`, `authority_events`,
`evidence_claims`, `project_workflow_policies`를 포함한 모든 정규 table relation을
포함합니다. 정규 프로젝트 상태 스키마는 파생 호환 relation이 아니라 record table로
구성됩니다. Content redaction은 field 의미를 따릅니다. 예를 들어 `prompt_captures` row는
`prompt_text`를 내보내지 않고 user-only replay row는 response 본문을 내보내지 않습니다.

## 관련 담당 문서

- [저장소](storage.md)
- [저장소 DDL](storage-ddl.md)
- [저장 효과](storage-effects.md)
- [저장소 버전 관리](storage-versioning.md)
- [API UserAction 스키마](api/schema-user-action.md)
- [실패 모델](failure-model.md)
