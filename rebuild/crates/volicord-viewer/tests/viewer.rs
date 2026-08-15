use tempfile::tempdir;
use volicord_context::{Clock, Principal, PrincipalKind, ProjectId, SystemClock, TimestampMicros};
use volicord_operations::{
    ConfirmationDecision, GuardedEffectCategory, GuardedEffectDraft, GuardedRisk, LocalOperations,
    RequestingProvenance, RuntimeLayout,
};
use volicord_viewer::{ExplanationLevel, ViewerAdapter, ViewerLocale, ViewerRequest};

fn setup() -> (tempfile::TempDir, ViewerAdapter, ProjectId) {
    let temporary = tempdir().expect("temporary directory");
    let runtime = temporary.path().join("runtime");
    let operations = LocalOperations::new(RuntimeLayout::new(runtime).expect("layout"));
    let initialized = operations
        .initialize_project("Viewer Project", Some(temporary.path()))
        .expect("initialize Project");
    let project = initialized.project.id;
    (temporary, ViewerAdapter::new(operations), project)
}

#[test]
fn reads_render_every_project_surface_without_mutating_canonical_state() {
    let (_temporary, viewer, project) = setup();
    let before = viewer
        .operations()
        .canonical_basis(project)
        .expect("basis before render");
    let repository_entries_before = repository_entries(&_temporary);
    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Deep,
                requested_language: "fr-CA".into(),
                guarded_request: None,
            },
            "test-request-authenticity",
        )
        .expect("render viewer");
    let after = viewer
        .operations()
        .canonical_basis(project)
        .expect("basis after render");
    let repository_entries_after = repository_entries(&_temporary);

    assert_eq!(before, after);
    assert_eq!(repository_entries_before, repository_entries_after);
    assert!(page.html.starts_with("<!doctype html><html lang=\"en\">"));
    for expected in [
        "Project overview",
        "Repository Map",
        "Decision trail / Context / Code",
        "Checkpoint timeline",
        "Candidate inspection",
        "Canonical context",
        "Privacy and provider",
        "Document preview / export",
        "Operation status",
        "fr-CA",
    ] {
        assert!(page.html.contains(expected), "missing {expected}");
    }
}

#[test]
fn korean_fixed_text_and_all_explanation_levels_are_available() {
    let (_temporary, viewer, project) = setup();
    for level in [
        ExplanationLevel::Overview,
        ExplanationLevel::Working,
        ExplanationLevel::Deep,
    ] {
        let page = viewer
            .render(
                &ViewerRequest {
                    project_id: project,
                    locale: ViewerLocale::Korean,
                    explanation_level: level,
                    requested_language: "한국어".into(),
                    guarded_request: None,
                },
                "test-request-authenticity",
            )
            .expect("render Korean viewer");
        assert!(page.html.starts_with("<!doctype html><html lang=\"ko\">"));
        assert!(page.html.contains("프로젝트 개요"));
        assert!(page.html.contains("저장소 지도"));
        assert!(page.html.contains("문서 미리보기 / 내보내기"));
    }
}

#[test]
fn guarded_fallback_preserves_exact_request_revision_and_source_linkage() {
    let (_temporary, viewer, project) = setup();
    let now = SystemClock.now().expect("clock");
    let request = viewer
        .operations()
        .create_guarded_request(GuardedEffectDraft {
            project_id: project,
            exact_action: "publish release".into(),
            target: "registry/example".into(),
            expected_effect: "public release".into(),
            risk: GuardedRisk {
                category: GuardedEffectCategory::ExternalDeploymentOrPublicPublication,
                concrete_consequence: "public artifact".into(),
            },
            scope: vec!["release:example".into()],
            expires_at: TimestampMicros::from_unix_micros(now.as_unix_micros() + 60_000_000),
            requesting_provenance: RequestingProvenance {
                actor: Principal {
                    kind: PrincipalKind::Agent,
                    identity: "test-agent".into(),
                },
                host: Some("codex".into()),
                session: Some("session-1".into()),
                basis: vec!["test".into()],
            },
        })
        .expect("create Guarded request");
    let page = viewer
        .render(
            &ViewerRequest {
                project_id: project,
                locale: ViewerLocale::English,
                explanation_level: ExplanationLevel::Working,
                requested_language: "en".into(),
                guarded_request: Some(request.confirmation_request_identity),
            },
            "test-request-authenticity",
        )
        .expect("render Guarded fallback");
    assert!(page
        .html
        .contains(&request.confirmation_request_identity.to_string()));
    assert!(page.html.contains(&request.effect_fingerprint));

    let response = viewer
        .confirm_guarded(
            request.confirmation_request_identity,
            request.request_revision,
            &request.effect_fingerprint,
            ConfirmationDecision::Confirmed,
            "viewer-session".into(),
            "I confirm this exact release".into(),
        )
        .expect("record viewer confirmation");
    assert_eq!(
        response.confirmation_request_identity,
        request.confirmation_request_identity
    );
    assert_eq!(response.request_revision, request.request_revision);
    assert_eq!(response.effect_fingerprint, request.effect_fingerprint);
    let canonical = viewer
        .operations()
        .canonical_basis(project)
        .expect("canonical basis");
    assert!(canonical
        .sources
        .iter()
        .any(|source| source.source.id == response.user_response_source_id));
}

fn repository_entries(temporary: &tempfile::TempDir) -> Vec<String> {
    let mut entries = std::fs::read_dir(temporary.path())
        .expect("read fixture repository")
        .map(|entry| {
            entry
                .expect("repository entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    entries.sort();
    entries
}
