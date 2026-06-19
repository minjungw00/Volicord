# 관리 CLI 참조

이 문서는 로컬 `harness` 관리/부트스트랩 CLI 계약을 담당합니다. 이 CLI는 `Harness Runtime Home`을 초기화하고, 로컬 프로젝트와 로컬 접점을 등록합니다. 이 명령들은 공개 하네스 API 메서드가 아닙니다.

이 문서는 공개 API 메서드 동작, API 스키마, 접근 등급 값 의미, 저장소 기록 배치, 보안 보장, Core 권한 의미, MCP stdio 전송 동작을 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- `harness` 명령 이름, 명령줄 인자, 기본값, stdout/stderr 처리, 프로세스 종료 코드
- `harness` 관리 명령의 Runtime Home 경로 선택
- 관리용 프로젝트와 접점 등록 기본값
- `baseline-workflow` 로컬 등록 프로필 확장
- 관리 명령과 공개 하네스 API 메서드 사이의 경계

이 문서는 담당하지 않습니다.

- 공개 하네스 API 메서드: [API 메서드](api/methods.md)
- `access_class` 값의 API 값 의미: [API 값 집합](api/schema-value-sets.md#access-class-values)
- 접점 등록 의미, 확인된 접점 맥락, 행위자 출처, 역량 선언 경계: [에이전트 통합](agent-integration.md)
- 런타임 데이터 경계 의미: [런타임 경계](runtime-boundaries.md)
- MCP 프로세스 시작, stdio 프레이밍, 응답 래핑, 종료: [MCP 전송](mcp-transport.md)

## 명령 모델

`harness`는 로컬 관리/부트스트랩 실행 파일입니다. 장기 실행 서버가 아니며 공개 하네스 API를 직접 노출하지 않습니다.

지원되는 기준 명령은 아래와 같습니다.

```text
harness --help
harness --version
harness init [--runtime-home-id ID]
harness project register --project-id ID --repo-root PATH [--status active]
harness project list
harness surface register --project-id ID --surface-id ID [--surface-instance-id ID] [--kind KIND] [--name NAME] [--interaction-role agent|user_interaction] [--access-class ACCESS_CLASS ...] [--profile baseline-workflow] [--capability-profile JSON]
harness surface list --project-id ID
```

종료 코드와 스트림 동작:

- 성공한 명령은 성공 출력을 stdout에 쓰고 종료 코드 `0`으로 끝납니다.
- `harness --version`은 stdout에 `harness <version>`을 쓰며 Runtime Home 해석을 요구하지 않습니다.
- 사용법 오류는 진단을 stderr에 쓰고 종료 코드 `2`로 끝납니다.
- 런타임, 환경, 저장소 오류는 진단을 stderr에 쓰고 종료 코드 `1`로 끝납니다.

지원하지 않는 것:

- CLI에는 `serve`, `server`, `connect` 명령이 없습니다.
- 관리 명령은 공개 하네스 API 메서드가 아니며 공개 메서드 목록에 추가되면 안 됩니다.

<a id="runtime-home-selection"></a>
## Runtime Home 선택

`harness` 관리 CLI는 아래 Runtime Home 경로 해석 규칙을 사용합니다. `harness-mcp` 프로세스 환경과 현재 MCP Runtime Home 경로 해석은 [MCP 전송](mcp-transport.md#process-environment)이 담당합니다.

해석 순서:

1. `HARNESS_HOME`이 존재하지만 비어 있으면 오류입니다.
2. 절대 경로 `HARNESS_HOME`은 제공된 그대로 사용합니다.
3. 상대 경로 `HARNESS_HOME`은 그 경로가 존재하지 않아도 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
4. `HARNESS_HOME`이 없으면 `HOME`, `USERPROFILE`, `HOMEDRIVE`와 `HOMEPATH` 결합 순서로 첫 번째 비어 있지 않은 홈 소스를 사용합니다.
5. 선택한 사용자 홈에 `.harness`를 붙입니다.
6. 선택한 홈이 상대 경로이면 프로세스의 현재 작업 디렉터리를 기준으로 해석합니다.
7. `harness init` 전에 정규화를 요구하지 않습니다.

`harness init`은 선택된 Runtime Home registry를 만들거나 검증할 수 있습니다. 다른 관리 명령은 선택된 Runtime Home에 요청 작업에 필요한 기록이 있어야 합니다.

## 프로젝트 등록

`harness project register --project-id ID --repo-root PATH [--status active]`는 로컬 `Product Repository`를 선택된 Runtime Home에 등록합니다.

규칙:

- `--project-id`는 필수입니다.
- `--repo-root`는 필수입니다.
- `--status`의 기본값은 `active`입니다.
- 기준 등록은 `status=active`를 받습니다.
- `--repo-root`는 프로젝트 등록에 쓰는 로컬 저장소 루트를 식별합니다.

`harness project list`는 선택된 Runtime Home의 등록된 프로젝트를 나열합니다.

`Product Repository`와 `Harness Runtime Home`의 구분을 포함한 런타임 위치 경계는 [런타임 경계](runtime-boundaries.md)가 담당합니다.

## 접점 등록

`harness surface register`는 등록된 프로젝트에 로컬 접점 인스턴스 하나를 기록합니다.

기본값:

- `surface_kind` 기본값은 `cli`입니다.
- `interaction_role` 기본값은 `agent`입니다.
- 기본 접근은 `read_status`뿐입니다.
- 생성되는 Runtime Home ID와 생성되는 `surface_instance_id` 값은 구현이 생성하는 불투명 값입니다.

등록 프로필:

- `--profile baseline-workflow`는 명시적으로 선택해야 합니다.
- `baseline-workflow`는 `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, `run_recording`으로 확장됩니다.
- 명시 접근 등급과 프로필에서 파생된 접근 등급은 결정적이고 중복 제거된 합집합을 이룹니다.
- `baseline-workflow` 프로필은 `artifact_read`를 포함하지 않습니다.

`user_interaction` 제약:

- `user_interaction`에는 `core_mutation`이 필요합니다.
- `user_interaction`은 `read_status`와 `core_mutation`만 가질 수 있습니다.
- 따라서 `baseline-workflow`는 `user_interaction` 접점에 유효하지 않습니다.

MCP 등록 지침:

- MCP 프로세스 등록에는 명시적인 `--kind mcp`를 사용합니다.
- 등록된 접점 인스턴스를 `HARNESS_SURFACE_INSTANCE_ID`로 참조할 때는 명시적인 `--surface-instance-id`를 사용합니다.

접근 등급 값 이름과 의미는 [API 값 집합](api/schema-value-sets.md#access-class-values)이 담당합니다. 접점 등록 의미와 확인된 맥락 경계는 [에이전트 통합](agent-integration.md)이 담당합니다.

## 접점 목록

`harness surface list --project-id ID`는 선택된 Runtime Home에서 한 프로젝트의 등록된 접점을 나열합니다.

규칙:

- `--project-id`는 필수입니다.
- 목록 출력은 진단용 등록 정보입니다.
- 목록 출력은 권한을 부여하거나, 로컬 도달 가능성을 증명하거나, 담당 결과가 반환한 확인된 접점 맥락을 대신하지 않습니다.

## 관리 경계

관리 CLI는 로컬 리소스를 초기화하고 등록할 수 있습니다. 그 자체로 공개 하네스 API 메서드를 만들지 않으며 Core 권한, `Write Authorization`, 증거 충분성, 닫기 준비 상태, 사용자 소유 판단, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을 만들지 않습니다.

담당 문서 경로:

- 공개 메서드 목록과 메서드 경로: [API 메서드](api/methods.md).
- 공통 요청/응답 스키마: [API 코어 스키마](api/schema-core.md).
- 접근 등급 값: [API 값 집합](api/schema-value-sets.md#access-class-values).
- 접점과 행위자 맥락 의미: [에이전트 통합](agent-integration.md).
- 런타임 위치 경계: [런타임 경계](runtime-boundaries.md).
