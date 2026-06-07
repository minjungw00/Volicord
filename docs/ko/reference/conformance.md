# 적합성 참조

## 1. 현재 상태

이 저장소는 문서 전용이며 아직 문서 검토 단계입니다. 여기에는 Harness Server 런타임, 적합성 실행기, 실행 가능한 fixture 파일, 생성된 적합성 보고서, 생성된 런타임 산출물, 현재 런타임 적합성 결과가 없습니다.

이 문서는 실행 가능한 적합성 테스트 모음이 아닙니다. 현재는 적합성의 의미, 향후 fixture 형식, 주장 권한, 간결한 대표 예시를 다루는 계획 담당 문서입니다. 현재 단계와 인계 상태는 [MVP 계획](../build/mvp-plan.md#문서-수락-상태)이 담당합니다.

## 2. 적합성이 뜻하는 것

적합성은 Harness Server와 실행기가 생긴 뒤, 향후 실행 점검이 담당 문서가 정의한 특정 동작을 담당 문서의 권한 기록과 비교할 수 있다는 뜻입니다. 향후 점검은 Core, API, operator 동작 하나를 실행하고, 응답에 담긴 사실과 담당 문서가 소유하는 상태 변경 효과를 수집한 뒤 구조화된 기대값과 비교합니다. 금지된 부작용이 없어야 한다는 주장도 여기에 포함됩니다.

문서 점검은 별도입니다. Markdown 문서 점검은 링크, 용어, 담당 문서 경계, active/later 문구, 보안 표현, 한영 문서 의미 일치를 확인합니다. 이는 현재 문서 유지보수 보조 도구일 뿐이며 런타임 적합성이 아닙니다.

적합성은 생성된 글, 에이전트 요약, 렌더링된 보고서, 상태 문구를 판단하지 않습니다. 담당 문서가 권한 있는 사실로 정한 것만 판단합니다.

## 3. 아직 없는 것

아래 항목은 향후 구현 작업이며 현재 저장소 내용이 아닙니다.

- Harness Server 런타임 또는 Harness Runtime Home 데이터
- 실행 가능한 fixture 파일 또는 fixture 디렉터리
- 적합성 실행기 또는 `harness conformance run` 구현
- 생성된 적합성 보고서, 생성된 런타임 산출물, Projection, 운영 파일, 런타임 상태
- 현재 MVP 동작이나 later 후보에 대한 현재 런타임 결과
- 예방적 차단, OS 권한 제어, 임의 도구 샌드박스, 변조 방지 저장소, 보안 격리, profile-gated `preventive` / `isolated` 보장에 대한 현재 증명

이 문서의 예시는 계획을 도울 수 있습니다. 하지만 런타임 상태, 수락 증거, 닫기 준비 상태, 잔여 위험 수락, 생성된 보고서, 구현 준비 상태를 만들지 않습니다.

## 4. fixture 형식

fixture 형식은 향후 구조를 설명할 뿐 현재 파일을 만들지 않습니다. Harness Server와 실행기가 생긴 뒤 승격된 fixture는 아래 부분을 담은 작은 구조화 기록이어야 합니다.

| 부분 | 목적 |
|---|---|
| `scenario_id` | 검토할 동작의 안정적인 식별자입니다. |
| 권한 맥락 | 동작 전에 필요한 Task, Change Unit, 상태 버전, 접점, 담당 문서 참조, Core 상태, 저장소 row, `ArtifactRef`, 접점 기능 사실입니다. |
| 동작 | 담당 요청 스키마를 사용하는 공개 Core, API, operator 요청 하나입니다. |
| 기대 주장 | 구조화된 응답에 담긴 사실, 담당 문서가 소유하는 상태 변경 효과, 저장소 또는 아티팩트 사실, 차단 사유 사실, 오류 사실, 보장 수준 사실, 금지된 부작용의 필수 부재입니다. |
| 담당 문서 링크 | 정확한 값과 의미를 정의하는 API, Core, Storage, Security, Agent Integration, ArtifactRef, policy 담당 문서입니다. |

구체화된 fixture는 공개 담당 스키마를 사용해야 합니다. fixture 전용 enum 값, 가짜 필드, 상태로 쓰는 지역화 표시 라벨, 글로만 된 기대값, later 후보 전용 값을 만들면 안 됩니다.

## 5. 주장 권한

주장 권한은 향후 fixture가 판단할 수 있는 사실의 좁은 범위입니다. 권한은 시나리오 설명이나 생성된 요약이 아니라 담당 문서가 정의한 사실에서 옵니다.

향후 권한 있는 주장은 다음을 사용할 수 있습니다.

- 공개 담당 API가 반환한 응답에 담긴 사실
- Core가 소유하는 Task, Change Unit, 사용자 판단, Write Authorization, Run 또는 증거 요약, 차단 사유, 닫기, 잔여 위험 상태
- Storage가 소유하는 row 변경 효과, idempotency/replay 사실, 상태 버전 사실, 아티팩트가 범위에 있을 때의 아티팩트 무결성 사실
- Core 담당 문서가 이벤트 이름을 승격한 뒤의 안정적인 `task_events`
- API, Core, Security, Agent Integration 담당 문서와 맞는 주 `ErrorCode`, 구조화된 차단 사유 필드, 보장 수준 사실
- 지속되는 승인 없음, Run row 없음, 아티팩트 변경 없음, 닫기 상태 변경 없음 같은 금지된 부작용의 부재 주장

현재 활성 예시는 `cooperative`와 지원되는 `detective` 사실만 주장할 수 있습니다. `preventive` 또는 `isolated` 주장은 승격된 profile과 담당 문서가 정의한 증명이 있을 때만 유효합니다. 적합성 계획 문구만으로 이런 보장이 현재 실행 가능하거나 증명된 것이 되지 않습니다.

권한이 없는 자료에는 시나리오 설명, 주석, 작성자 메모, 렌더링된 Markdown, 생성된 보고서, 상태 문구, 에이전트 요약, 문서 점검 라벨, Projection이 포함됩니다. Projection 지원이 명시 범위에 있을 때만 최신성 또는 가용성 주장이 예외로 가능할 수 있습니다.

## 6. 현재 MVP 대표 예시

아래 항목은 간결한 동작 참조일 뿐입니다. fixture 파일, 전체 YAML 본문, 현재 런타임 결과가 아닙니다.

| 예시 | 동작 | 향후 주장 초점 |
|---|---|---|
| `MVP-ACTIVE-prepare-write-blocked-or-dry-run-no-durable-authorization` | 차단되었거나 dry-run인 `prepare_write`는 지속되는 승인을 만들지 않습니다. | 응답에는 소비 가능한 Write Authorization이 없습니다. `write_authorizations`에는 활성 row가 추가되지 않습니다. Run, 아티팩트, 증거, 닫기, 최종 수락, 잔여 위험 상태가 바뀌지 않습니다. |
| `MVP-ACTIVE-prepare-write-committed-scoped-authorization` | 허용되어 committed 처리된 `prepare_write`는 범위가 정해진 1회용 Write Authorization을 기록합니다. | 응답의 승인 범위, Core 상태, `write_authorizations.attempt_scope_json`이 Task, Change Unit, 상태 버전, 접점, 허용된 경로/도구/명령/네트워크, 비밀값과 민감 범주, 기준 참조, 관련 판단, 보장 수준에서 같은 의미를 가집니다. |
| `MVP-ACTIVE-close-task-blocks-missing-acceptance-or-risk-condition` | 필요한 최종 수락이 없거나, 닫기에 영향을 주는 잔여 위험 조건이 필요한 수준으로 보이지 않거나 활성 닫기 경로가 요구하는 수락이 없으면 `close_task`가 차단됩니다. | 응답 차단 사유는 담당 문서의 category와 필요한 경우 `required_judgment_kind`를 사용합니다. Task는 completed가 되지 않습니다. 닫기 상태가 누락된 수락이나 위험 수락을 대신하지 않습니다. 증거, 최종 수락, 잔여 위험 상태는 분리되어 남습니다. |

## 7. 향후 항목을 목록으로만 유지하는 경계

향후 fixture 계열은 [Later 후보 색인: Future fixture families](../later/index.md#future-fixture-families)에 둡니다. 그 색인은 later 후보 이름만 보존하며, 이 문서는 그 목록을 반복하지 않습니다.

향후 계열 이름은 시나리오 스크립트, fixture 본문, active API payload 예시, 실행기 또는 보고 요구사항, 현재 MVP 범위, 구현 작업, 현재 결과, 증명이 아닙니다. 향후 담당 문서가 좁은 동작을 범위, 대체 동작, 정확한 계약, 증명 기대치와 함께 승격해야 실행 가능한 fixture 자료가 생깁니다.

## 8. 지표 경계

현재 문서 세트에서 지표는 적합성 권한이 아닙니다. 향후 로컬 지표는 진단이나 계획에 유용할 수 있지만, 담당 문서가 승격하기 전에는 읽기 전용 파생 표시로 남습니다.

지표는 Core 상태를 만들거나, 증거를 충족하거나, QA 또는 검증을 통과시키거나, 쓰기를 승인하거나, 최종 결과를 수락하거나, 잔여 위험을 수락하거나, 작업을 닫거나, 구현 준비 상태를 증명하거나, 런타임 적합성을 대신하면 안 됩니다. 향후 지표가 승격되면 담당 문서가 원천 기록, 최신성 경계, 표시 문구, 대체 불가 규칙을 정의해야 합니다.
