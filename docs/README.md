# Harness documentation / 하네스 문서

This directory is the active bilingual documentation router for Harness planning. Use it to choose a language and a first reading path. Contract details stay in the owner documents.

이 디렉터리는 하네스 계획 문서의 현재 한영 경로 안내입니다. 언어와 첫 읽기 경로를 고르는 데 사용합니다. 계약 세부사항은 담당 문서에서 확인합니다.

## Choose a language / 언어 선택

English and Korean are both active documentation languages. Choose the language you want to read first, then stay in that language unless you are checking translation parity.

영어와 한국어 문서는 모두 활성 문서입니다. 먼저 읽을 언어를 고르고, 번역 의미 일치를 확인하는 경우가 아니라면 같은 언어 안에서 읽습니다.

- English: [`docs/en/start.md`](en/start.md)
- 한국어: [`docs/ko/start.md`](ko/start.md)

## English reading paths

Use these routes for first-hop navigation. Follow the linked pages to the exact owner when you need more detail.

- New user: [`docs/en/start.md`](en/start.md) -> [`docs/en/use/user-guide.md`](en/use/user-guide.md)
- Working user: [`docs/en/use/user-guide.md`](en/use/user-guide.md) -> [`docs/en/reference/scope.md`](en/reference/scope.md)
- Agent behavior: [`docs/en/use/agent-guide.md`](en/use/agent-guide.md) -> [`docs/doc-index.yaml`](doc-index.yaml)
- Technical contract: [`docs/en/reference/README.md`](en/reference/README.md) -> [`docs/doc-index.yaml`](doc-index.yaml)
- Maintenance: [`docs/en/maintain/authoring-guide.md`](en/maintain/authoring-guide.md) -> [`docs/en/maintain/translation-guide.md`](en/maintain/translation-guide.md) -> [`docs/terminology-map.yaml`](terminology-map.yaml)
- Current scope: [`docs/en/reference/scope.md`](en/reference/scope.md)

## 한국어 읽기 경로

아래 경로는 처음 이동할 문서를 고르는 안내입니다. 더 자세한 기준이 필요하면 연결된 문서에서 담당 문서로 이동합니다.

- 처음 읽는 사용자: [`docs/ko/start.md`](ko/start.md) -> [`docs/ko/use/user-guide.md`](ko/use/user-guide.md)
- 작업 중인 사용자: [`docs/ko/use/user-guide.md`](ko/use/user-guide.md) -> [`docs/ko/reference/scope.md`](ko/reference/scope.md)
- 에이전트 동작: [`docs/ko/use/agent-guide.md`](ko/use/agent-guide.md) -> [`docs/doc-index.yaml`](doc-index.yaml)
- 기술 계약: [`docs/ko/reference/README.md`](ko/reference/README.md) -> [`docs/doc-index.yaml`](doc-index.yaml)
- 유지보수: [`docs/ko/maintain/authoring-guide.md`](ko/maintain/authoring-guide.md) -> [`docs/ko/maintain/translation-guide.md`](ko/maintain/translation-guide.md) -> [`docs/terminology-map.yaml`](terminology-map.yaml)
- 현재 범위: [`docs/ko/reference/scope.md`](ko/reference/scope.md)

## Reference owner routing / 참조 담당 문서 찾기

Use the reference README to find owners for API, schema, storage, security, scope, and other contract areas. Use `docs/doc-index.yaml` when an agent or maintainer needs stable `doc_id` routing. This README intentionally does not repeat API schemas, storage effects, active MVP details, or security contracts.

API, 스키마, 저장소, 보안, 범위, 그 밖의 계약 영역은 참조 README에서 담당 문서를 찾아 읽습니다. 에이전트나 유지보수자가 안정적인 `doc_id` 경로가 필요할 때는 `docs/doc-index.yaml`을 사용합니다. 이 README는 API 스키마, 저장 효과, 현재 MVP 세부사항, 보안 계약을 반복하지 않습니다.

- English reference index: [`docs/en/reference/README.md`](en/reference/README.md)
- Korean reference index: [`docs/ko/reference/README.md`](ko/reference/README.md)
- Current MVP scope route: [`docs/en/reference/scope.md`](en/reference/scope.md), [`docs/ko/reference/scope.md`](ko/reference/scope.md)
- Owner routing metadata: [`docs/doc-index.yaml`](doc-index.yaml)
- Bilingual terminology controls: [`docs/terminology-map.yaml`](terminology-map.yaml)

## Maintenance documents / 유지보수 문서

Use maintain documents for documentation editing rules, bilingual practice, and checks. They guide documentation work; they do not own runtime or technical contracts.

유지보수 문서는 문서 편집 규칙, 한영 문서 동시 유지 방식, 점검 절차를 다룹니다. 문서 작업을 안내할 뿐, 런타임이나 기술 계약을 담당하지 않습니다.

- English authoring: [`docs/en/maintain/authoring-guide.md`](en/maintain/authoring-guide.md)
- Korean authoring: [`docs/ko/maintain/authoring-guide.md`](ko/maintain/authoring-guide.md)
- English translation guide: [`docs/en/maintain/translation-guide.md`](en/maintain/translation-guide.md)
- Korean translation guide: [`docs/ko/maintain/translation-guide.md`](ko/maintain/translation-guide.md)
- Documentation checks: [`docs/en/maintain/checks.md`](en/maintain/checks.md), [`docs/ko/maintain/checks.md`](ko/maintain/checks.md)
- Terminology map: [`docs/terminology-map.yaml`](terminology-map.yaml)

## Agent retrieval rule / 에이전트 검색 규칙

Agents should use [`docs/doc-index.yaml`](doc-index.yaml) before loading reference content. Read only one language version of the same `doc_id` unless checking translation parity.

에이전트는 참조 내용을 불러오기 전에 [`docs/doc-index.yaml`](doc-index.yaml)에서 담당 경로를 먼저 고릅니다. 번역 의미 일치를 확인하는 경우가 아니라면 같은 `doc_id`의 한 언어 버전만 읽습니다.
