# 에이전트 통합 참조

이 문서는 에이전트가 쓰는 접점의 등록, 현재 적용 접점 맥락, 역량 선언을 담당합니다. 담당 결과의 하네스 맥락을 에이전트 접점에 전달할 때의 경계도 이 문서가 정합니다.

API 스키마, 메서드 동작, 저장 효과, 보안 보장 의미, 상태 보기 표시 경계, 렌더링된 템플릿 문구는 이 문서가 정의하지 않습니다.

## 담당하는 것 / 담당하지 않는 것

이 문서가 담당합니다.

- 에이전트 통합에서 쓰는 접점 등록 입력과 선택자 의미
- `surface_id`, `surface_instance_id`, 요청 수준 `VerifiedSurfaceContext`, 권한 해결용 `VerifiedActorContext`를 포함한 현재 적용 접점과 행위자 맥락 경계
- `capability_profile`의 역량 선언 경계
- 담당 결과와 접점 사이의 에이전트 맥락 전달 규칙
- 선택된 접점이나 현재 적용 접점 맥락을 사용할 수 없거나, 맞지 않거나, 오래되었거나, 역량이 부족할 때의 대체 표시
- 에이전트 맥락에서 하나의 `doc_id`에 한 언어만 싣는 검색 지침

이 문서는 담당하지 않습니다.

- 접점별 사용 흐름: [접점별 사용 레시피](../guides/surface-recipes.md)
- API 요청 래퍼, 응답 분기, 스키마 형태, 메서드 접근 요구사항, 접근 등급 값 이름: [API 코어 스키마](api/schema-core.md), [API 메서드](api/methods.md), 메서드 담당 문서, [API 값 집합](api/schema-value-sets.md)
- `harness-mcp` 실행 파일 시작, 프로세스 환경, stdio 프레이밍, 시작 검증, 응답 래핑, 종료: [MCP 전송](mcp-transport.md)
- 저장소 배치, 아티팩트 생명주기, 스테이징 핸들 검증: [참조 색인](README.md)에서 고르는 저장소와 아티팩트 담당 문서
- 보안 보장 의미나 접근 경계 표현: [보안](security.md)
- 권한과 파생 표시의 구분 규칙: [상태 보기와 템플릿 표시 경계](projection-and-templates.md)
- 렌더링 본문 문구, 공개 표시 라벨, 템플릿 표현: [템플릿 본문](template-bodies.md)

## 통합 경계

에이전트가 쓰는 접점은 하네스 담당 결과와 에이전트 사이에서 맥락을 전달합니다. 접점 자체가 하네스 권한을 만들지는 않습니다.

조건:
- 에이전트는 담당 결과의 상태 또는 호환되는 현재 적용 접점 맥락을 통해서만 접점에 의존할 수 있습니다.
- 표시 문구, 대화 메시지, 생성 파일, 접점 설명, `Product Repository` 파일, 상태 보기, 에이전트 기억은 지원 맥락일 뿐입니다.

에이전트가 할 수 있는 것:
- 메서드 담당 문서가 요구할 때 등록된 접점 선택자를 전달할 수 있습니다.
- 담당 결과의 상태와 표시 라벨을 보여 줄 수 있습니다.
- 압축한 담당 결과 맥락을 에이전트에 전달할 수 있습니다.

에이전트가 하면 안 되는 것:
- 접점 산문, 복사된 식별자, 렌더링된 표시, 에이전트 기억을 권한으로 취급하면 안 됩니다.
- 표시 문구로 Core 상태, `Write Authorization`, 증거 충분성, 사용자 소유 판단, 닫기 준비 상태, 수락, 잔여 위험 수락, 아티팩트 권한, 보안 보장을 만들면 안 됩니다.

담당 문서 링크:
- [Core 모델](core-model.md)은 Core 권한, 사용자 소유 판단, 닫기 준비 상태, 수락, 잔여 위험 경계를 담당합니다.
- [런타임 경계](runtime-boundaries.md)는 `Product Repository`, `Harness Server` 소스/설치, 실행 파일 프로세스, `Harness Runtime Home`, 외부 MCP 호스트 설정의 분리를 담당합니다.
- [상태 보기와 템플릿 표시 경계](projection-and-templates.md)는 권한과 파생 표시의 구분 규칙을 담당합니다.

## 접점 등록

접점 등록은 사용자가 선택한 접점과, 메서드 담당 계약이 그 접점이 현재 요청을 지원할 수 있는지 판단할 때 필요한 사실을 이름 붙입니다.

조건:
- `surface_id`는 등록된 로컬 접점의 선택자입니다.
- `surface_instance_id`는 메서드 담당 문서가 반환하거나 요구할 때 등록된 인스턴스를 구분합니다.
- `surfaces.local_access_json`은 그 접점 인스턴스에 등록된 로컬 접근 허용의 기준 소스입니다.
- 선호되는 허용 필드는 `authorized_access_classes: string[]`입니다. 같은 접점 인스턴스에 대해 문서화된 접근 등급 여러 개를 담을 수 있습니다. `access_class: string`은 하위 호환을 위한 단일 값 대체 필드입니다.
- 기준 워크플로 등록 프로필은 `read_status`, `core_mutation`, `write_authorization`, `artifact_registration`, `run_recording`의 명시적 접근 등급 집합으로 확장될 수 있습니다.
- 전체 워크플로 프로필은 명시적으로 선택되어야 하며 암묵적 기본값이 되면 안 됩니다.
- `verification_basis: string`은 허용이 어떻게 성립했는지 설명하는 통제된 등록 또는 어댑터 바인딩 진단 메타데이터입니다. 접근을 부여하지 않습니다.
- `interaction_role: string`은 그 접점 인스턴스가 권한 해결에서 `agent`로 동작하는지 `user_interaction`으로 동작하는지를 식별합니다. 기준 등록에는 혼합 역할 접점 인스턴스가 없습니다.
- 등록 사실은 현재 요청에 대해 담당 결과가 반환한 확인을 통해서만 사용할 수 있습니다.

에이전트가 할 수 있는 것:
- 메서드 담당 문서가 요구할 때 `surface_id`와 `surface_instance_id`를 전달할 수 있습니다.
- 담당 결과가 반환한 사용 불가, 불일치, 오래됨, 역량 부족 접점 상태를 표시할 수 있습니다.

에이전트가 하면 안 되는 것:
- 호출자 산문, 복사된 식별자, 생성된 Markdown, 대화 텍스트, 상태 보기 텍스트, 에이전트 기억으로 로컬 도달 가능성, 접근 등급, `verified=true`, 아티팩트 출처를 추론하면 안 됩니다.
- `surface_id`, `surface_instance_id`, 접점 이름을 권한 증거로 취급하면 안 됩니다.
- `capability_profile`, 요청된 호출 접근, `verification_basis`를 접근 허용으로 취급하면 안 됩니다.
- 환경 변수, 공개 요청 필드, 호출자가 제공한 라벨을 신뢰된 검증 근거 문구나 감사 사실로 취급하면 안 됩니다.

담당 문서 링크:
- [API 메서드](api/methods.md)와 메서드 담당 문서는 메서드 요청 조건을 정의합니다.
- [API 값 집합](api/schema-value-sets.md)은 접근 등급 값 이름을 담당합니다.
- [보안](security.md)은 접근 경계와 보장 표현을 담당합니다.

<a id="current-surface-context"></a>
## 현재 적용 접점 맥락

`VerifiedSurfaceContext`는 한 번의 호출에 대해 내부에서 파생되는 맥락입니다. `harness-mcp` 로컬 어댑터 프로세스 같은 `Harness Server` 실행 파일 역할은 등록된 접점 기록, 어댑터가 파생한 호출 맥락, 요청된 호출 접근에서 이를 파생하고, 그 뒤 메서드 담당 문서가 파생된 맥락이 요청과 호환되는지 판단합니다. 이는 공개 요청 페이로드가 아닙니다.

MCP 세션은 어댑터 시작 시 정확히 하나의 `project_id`, 하나의 `surface_id`, 하나의 `surface_instance_id`에 묶입니다. 이 바인딩 값은 세션 수명 동안 고정됩니다. 공개 `ToolEnvelope.project_id`와 `ToolEnvelope.surface_id` 필드는 프로토콜 일관성을 위해 남아 있지만, 각 요청 값은 세션 바인딩과 정확히 일치해야 합니다. 요청은 프로젝트, 접점, 접점 인스턴스를 바꿀 수 없습니다.

`surface_instance_id`가 명시적으로 설정되지 않았을 때 어댑터 시작은 묶인 프로젝트와 접점에 등록된 기본값 또는 하나뿐인 명확한 등록 후보를 선택할 수 있습니다. 없거나 모호하면 시작 실패입니다. 서로 다른 프로젝트에 같은 `surface_instance_id` 값이 있어도 프로젝트 바인딩은 약해지지 않습니다. 묶인 `project_id`는 세션 식별성의 일부로 남으며 모든 요청은 이에 일치해야 합니다.

MCP 세션은 프로세스 전체에 고정된 접근 등급 하나에 묶이지 않습니다. MCP 어댑터는 현재 호출의 공개 메서드 이름과 타입이 지정된 params에서 요청된 호출 접근을 파생합니다. 공개 요청 params에는 호출 접근 등급, 호출 `surface_instance_id`, 역량 프로필, 검증 근거, `VerifiedSurfaceContext`가 들어가지 않습니다. Core는 `VerifiedSurfaceContext`를 파생하기 전에 세션 바인딩과, 메서드에서 파생된 요청 접근이 `surfaces.local_access_json`의 등록된 허용에 포함되는지를 독립적으로 확인합니다.

메서드에서 파생되는 요청 접근:

| 공개 메서드와 타입이 지정된 params | 요청 접근 |
|---|---|
| `harness.status` | `read_status` |
| `harness.intake` | `core_mutation` |
| `harness.update_scope` | `core_mutation` |
| `harness.prepare_write` | `write_authorization` |
| `harness.stage_artifact` | `artifact_registration` |
| `harness.record_run` | `run_recording` |
| `harness.request_user_judgment` | `core_mutation` |
| `harness.record_user_judgment` | `core_mutation` |
| `harness.close_task` with `intent=check` | `read_status` |
| Other `harness.close_task` intents | `core_mutation` |

`InvocationContext.access_class` 또는 동등한 구현 개념은 현재 호출이 요청한 접근 등급입니다. 이는 권한이 아니며 접근 등급을 부여할 수 없습니다. `VerifiedSurfaceContext`는 요청된 호출 접근이 `surfaces.local_access_json`의 등록된 허용 목록에 포함될 때만 파생될 수 있습니다.

새로 파생되는 맥락의 검증 근거는 통제된 등록 값과 어댑터 바인딩 값으로만 구성됩니다. 환경 변수와 공개 요청 필드는 임의의 검증 근거 문구를 제공할 수 없습니다. 통제된 예시는 `local_admin_registration`, `mcp_stdio_surface_binding`, `cli_direct_surface_binding`, `test_fixture_binding`입니다. 기존에 저장된 임의 근거 문자열은 이력 데이터로 남을 수 있지만, 새로 쓰는 값은 통제된 어휘를 사용합니다. 검증 근거는 진단 메타데이터이며 접근을 부여하지 않습니다.

내부 접점 형태이며 공개 API 스키마가 아닙니다.

```yaml
VerifiedSurfaceContext:
  project_id: string
  surface_id: string
  surface_instance_id: string
  access_class: string
  capability_profile: object
  verification_basis: string
```

`VerifiedActorContext`는 메서드가 권한을 지니는 사용자 판단을 해결할 때 사용하는 내부 파생 행위자 출처 맥락입니다. 묶인 접점 인스턴스, 등록 역할, 어댑터 호출 맥락, 공개 `ToolEnvelope.actor_kind` 귀속값에서 파생됩니다. 공개 요청 페이로드가 아닙니다.

내부 행위자 형태이며 공개 API 스키마가 아닙니다.

```yaml
VerifiedActorContext:
  role: agent | user_interaction
  surface_id: string
  surface_instance_id: string
  verification_basis: string
  assurance_level: string
```

기준 `assurance_level`은 협력적 등록 접점 출처를 뜻하며 암호학적 인간 신원 증명이 아닙니다. 권한을 지니는 해결에는 `VerifiedActorContext.role=user_interaction`, 묶인 `surface_id`와 `surface_instance_id`의 일치, 공개 `actor_kind=user`가 필요합니다. `ToolEnvelope.actor_kind`는 귀속일 뿐입니다. `agent` 역할 접점은 `actor_kind=user`를 제출해도 사용자 권한을 얻을 수 없습니다.

조건:
- 공개 API 요청 하나에는 요청 수준 `VerifiedSurfaceContext.access_class`가 정확히 하나 있습니다.
- 공개 API 요청 하나에는 권한과 관련된 `VerifiedActorContext`가 최대 하나 있으며, 권한 해결 메서드 담당 문서만 이를 소비합니다.
- 공개 `ToolEnvelope.project_id`와 `ToolEnvelope.surface_id`는 고정된 세션 바인딩을 요청에서 되비추는 값입니다. 호출자가 선택하는 권한이 아니며 세션을 바꿀 수 없습니다.
- `surface_instance_id`는 어댑터가 파생한 호출 맥락으로 남습니다. `ToolEnvelope`에는 `surface_instance_id`가 추가되지 않습니다. 공통 요청 래퍼는 [API 코어 스키마](api/schema-core.md#tool-envelope)에 둡니다.
- `ArtifactInput`이나 `StagedArtifactHandle` 같은 중첩 페이로드는 두 번째 요청 수준 접근 등급을 추가하지 않습니다.
- `created_by_surface_id`, `created_by_surface_instance_id` 같은 스테이징된 아티팩트 출처 필드는 호출자 텍스트나 중첩 아티팩트 입력이 아니라 스테이징 시점의 파생된 `VerifiedSurfaceContext`에서 옵니다.
- 해결된 권한 판단의 권한 출처 필드는 호출자 텍스트, 라벨, 답변 본문, 복사된 참조가 아니라 `VerifiedActorContext.surface_id`와 `VerifiedActorContext.surface_instance_id`에서 옵니다.
- 보호된 읽기, 상태 변경, 아티팩트 동작은 메서드 담당 문서가 파생된 확인 맥락을 받아들일 때만 접점에 의존할 수 있습니다.
- `capability_profile`은 지원 역량을 설명할 수 있지만 `VerifiedSurfaceContext.access_class`를 부여하거나 높일 수 없습니다.

에이전트가 할 수 있는 것:
- 맥락을 표시하거나 전달할 때 요청 수준 `VerifiedSurfaceContext.access_class`를 보존할 수 있습니다.
- 맥락이 없거나 호환되지 않으면 사용 불가, 불일치, 오래됨, 역량 부족 접점 상태로 표시할 수 있습니다.

에이전트가 하면 안 되는 것:
- `VerifiedSurfaceContext`를 요청 페이로드로 제출하면 안 됩니다.
- `VerifiedActorContext`를 요청 페이로드로 제출하면 안 됩니다.
- `verified=true`를 스스로 주장하면 안 됩니다.
- `surface_instance_id`를 확인 권한 근거로 제출하면 안 됩니다.
- `agent` 역할 접점에서 `actor_kind=user`를 제출해 사용자 권한을 만족시키면 안 됩니다.
- 접근 등급, 역량 프로필, 검증 근거를 공개 요청 권한으로 제출하면 안 됩니다.
- 스테이징된 아티팩트 출처를 꾸며 내면 안 됩니다.
- 복사된 식별자, 생성된 Markdown, 대화 텍스트, 상태 보기 텍스트, 에이전트 기억을 확인된 맥락의 대체물로 쓰면 안 됩니다.
- `capability_profile`이나 요청된 호출 접근을 등록된 허용의 대체물로 쓰면 안 됩니다.

담당 문서 링크:
- 정확한 요청 래퍼와 응답 형태는 [API 코어 스키마](api/schema-core.md), [API 메서드](api/methods.md), 메서드 담당 문서가 담당합니다.
- 접근 등급 값은 [API 값 집합](api/schema-value-sets.md)이 담당합니다.
- `harness-mcp` 시작, 환경 변수, stdio 프레이밍, 시작 검증, 응답 래핑, 종료는 [MCP 전송](mcp-transport.md)이 담당합니다.

## 역량 선언

`capability_profile`은 등록된 접점이 무엇을 지원할 수 있는지 설명하는 통합 선언입니다. 그 자체로 권한은 아닙니다.

조건:
- 어떤 역량은 [범위 참조](scope.md)와 영향받는 담당 문서가 그 역량을 기준 범위 또는 프로필 조건부 지원 동작으로 정의할 때만 지원된다고 선언할 수 있습니다.
- 보호된 읽기, 상태 변경, 아티팩트 동작, 보장 표시는 메서드 담당 문서의 지원을 받으며 호환되는 접점 맥락이 현재 적용될 때만 역량 선언을 사용할 수 있습니다.
- 역량 선언은 권한이 아니며 `surfaces.local_access_json`에 허용을 추가할 수 없습니다.

에이전트가 할 수 있는 것:
- 지원되는 접근 등급을 설명할 수 있습니다.
- 로컬 도달 가능성을 설명할 수 있습니다.
- 아티팩트 스테이징 또는 본문 읽기 지원을 설명할 수 있습니다.
- 표시 한계를 설명할 수 있습니다.
- 빠진 지원을 사용 불가 또는 역량 제한으로 보여 줄 수 있습니다.

에이전트가 하면 안 되는 것:
- `capability_profile`로 지원 범위 밖 기능을 켜면 안 됩니다.
- `capability_profile`로 접근 등급을 부여하거나 높이면 안 됩니다.
- 오래되었거나 복사되었거나 생성되었거나 사용자가 말로 제공한 역량 문구로 더 강한 보안 보장을 정당화하면 안 됩니다.
- 메서드 담당 문서의 접근 조건이나 보안 담당 문서의 보장 표현을 역량 선언으로 대체하면 안 됩니다.

담당 문서 링크:
- [범위 참조](scope.md)는 기준 범위와 프로필 조건부 범위 경계를 담당합니다.
- [보안](security.md)은 보장 어휘와 보장 강도 비주장을 담당합니다.
- [API 값 집합](api/schema-value-sets.md)은 접근 등급 값 이름을 담당합니다.

## 에이전트 맥락 전달

에이전트 맥락 전달은 다음 행동에 필요한 담당 맥락만 에이전트에 제공하되, 그 패킷을 권한 기록으로 만들지 않는 규칙입니다.

조건:
- 에이전트 맥락에는 다음 행동에 필요한 담당 결과와 그 행동에 영향을 주는 현재 적용 접점 맥락의 한계만 담아야 합니다.
- 맥락 패킷은 지원 맥락일 뿐 Core 상태, 저장소 상태, 증거, 수락, 잔여 위험 수락, 닫기 출력이 아닙니다.

에이전트가 할 수 있는 것:
- 현재 `Task` 요약, 현재 적용 범위, `state_version`, 대기 중인 사용자 소유 판단, 차단 사유, 다음 안전한 행동, 증거와 아티팩트 요약, 닫기 준비 상태와 잔여 위험 요약, 담당 문서가 뒷받침하는 보장 표시, 출처 또는 제한 메모를 담은 압축 맥락을 전달할 수 있습니다.
- 다음 행동에 필요할 때만 정확한 담당 문서 섹션을 가져올 수 있습니다.
- 한영 문서 유지보수에서 의미 일치 검토가 필요할 때만 같은 `doc_id`의 두 언어 문서를 함께 가져올 수 있습니다.

에이전트가 하면 안 되는 것:
- 전체 스키마, DDL, 과거 로그, 아티팩트 본문, 관련 없는 계약 자료, 지원 범위 밖 기능 목록, 정확한 템플릿 본문, 같은 `doc_id`의 두 언어 문서를 기본으로 주입하면 안 됩니다.
- 오래되었거나 복사된 맥락 패킷을 담당 결과나 기반 기록보다 최신 권한처럼 취급하면 안 됩니다.

담당 문서 링크:
- [템플릿 본문](template-bodies.md)은 에이전트 맥락 패킷 문구를 담당합니다.
- [참조 색인](README.md)은 정확한 담당 문서 섹션 경로를 안내합니다.
- [번역 정책](../maintain/translation-policy.md)은 한영 의미 일치 검토 지침을 담당합니다.

## 대체 경계

현재 적용 접점 맥락이나 필요한 통합 역량을 사용할 수 없거나, 맞지 않거나, 오래되었거나, 충분하지 않을 때 대체 표시를 사용합니다.

에이전트가 할 수 있는 것:
- 역량 있는 접점으로 옮길 수 있습니다.
- 동작을 좁힐 수 있습니다.
- 빠진 사용자 소유 판단을 요청할 수 있습니다.
- 사용자가 그 방식을 명시적으로 선택한 경우에만 하네스 밖에서 계속할 수 있습니다.

에이전트가 해야 하는 것:
- 제한을 지원 문구나 표시 문구에 드러내야 합니다.
- 기계 판독용 실패 의미는 [API 오류 코드](api/error-codes.md)와 [API 오류 세부사항](api/error-details.md)으로 보내야 합니다.
- 사용자에게 보이는 문구는 [템플릿 본문](template-bodies.md) 또는 [접점별 사용 레시피](../guides/surface-recipes.md)로 보내야 합니다.

에이전트가 하면 안 되는 것:
- 권한을 지어내면 안 됩니다.
- 사용 불가, 불일치, 오래됨, 역량 부족 상태를 일반 성공 문구 속에 숨기면 안 됩니다.
- 사용자의 명시적 선택 없이 하네스 밖에서 계속하면 안 됩니다.
