use crate::adapter::*;
use crate::errors::{LocalHttpError, McpAdapterError};
use crate::http::*;
use crate::prelude::*;
use crate::routing::*;
use crate::util::*;

/// Local MCP adapter bound to a Core service and one Agent Connection.
pub(crate) fn start_stdio_local_web_consent_listener(
    runtime_home: &Path,
    context: &McpConnectionContext,
) -> Result<StartedLocalWebConsentListener, McpAdapterError> {
    if local_web_consent_disabled_by_env() {
        return Err(McpAdapterError::Environment(
            "disabled by VOLICORD_LOCAL_WEB_CONSENT".to_owned(),
        ));
    }
    let listen_addr: SocketAddr = "127.0.0.1:0".parse().map_err(|error| {
        McpAdapterError::Environment(format!("invalid listen address: {error}"))
    })?;
    let listener = TcpListener::bind(listen_addr).map_err(McpAdapterError::Io)?;
    let actual_addr = listener.local_addr().map_err(McpAdapterError::Io)?;
    if !actual_addr.ip().is_loopback() {
        return Err(McpAdapterError::Environment(format!(
            "local web consent listener did not bind to loopback ({actual_addr})"
        )));
    }

    let consent_context = LocalWebConsentContext {
        base_url: format!("http://{actual_addr}"),
    };
    let (readiness, listener_readiness) = LocalWebConsentReadiness::tracked();
    let adapter = McpAdapter::new(runtime_home, context.clone())
        .with_local_web_consent_readiness(consent_context.clone(), readiness.clone());
    thread::Builder::new()
        .name("volicord-local-web-consent".to_owned())
        .spawn(move || {
            let listener_readiness = listener_readiness;
            let mut server = LocalWebConsentServer::new(adapter);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        if let Err(error) = stream.set_read_timeout(Some(HTTP_READ_TIMEOUT)) {
                            eprintln!(
                                "warning: failed to set local web consent read timeout: {error}"
                            );
                        }
                        if let Err(error) = server.handle_stream(&mut stream) {
                            eprintln!("warning: local web consent request failed: {error}");
                        }
                    }
                    Err(error) => {
                        listener_readiness.mark_unavailable();
                        eprintln!("warning: local web consent listener stopped: {error}");
                        break;
                    }
                }
            }
        })
        .map_err(McpAdapterError::Io)?;
    Ok(StartedLocalWebConsentListener {
        context: consent_context,
        readiness,
    })
}

pub(crate) struct StartedLocalWebConsentListener {
    pub(crate) context: LocalWebConsentContext,
    pub(crate) readiness: LocalWebConsentReadiness,
}

pub(crate) fn local_web_consent_disabled_by_env() -> bool {
    std::env::var("VOLICORD_LOCAL_WEB_CONSENT")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "disabled"
            )
        })
}

pub(crate) struct LocalWebConsentServer {
    adapter: McpAdapter,
}

const LOCAL_WEB_CONSENT_CSP: &str = "default-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'; img-src 'none'; object-src 'none'; script-src 'none'; style-src 'unsafe-inline'";

impl LocalWebConsentServer {
    fn new(adapter: McpAdapter) -> Self {
        Self { adapter }
    }

    fn handle_stream(&mut self, stream: &mut TcpStream) -> Result<(), LocalHttpError> {
        let response = match read_http_request(stream) {
            Ok(request) => self.handle_request(request),
            Err(response) => response,
        };
        write_http_response(stream, response).map_err(LocalHttpError::Io)
    }

    fn handle_request(&mut self, request: HttpRequest) -> HttpResponse {
        if http_request_path(&request.target) == LOCAL_WEB_CONSENT_PATH {
            handle_local_web_consent_http_request(&self.adapter, request)
        } else {
            local_web_consent_error_page(
                404,
                "Not Found",
                "NOT_FOUND",
                "This local listener only serves Volicord consent pages.",
            )
        }
    }
}

/// MCP endpoint path used by the loopback-only local HTTP transport.
pub(crate) fn handle_local_web_consent_http_request(
    adapter: &McpAdapter,
    request: HttpRequest,
) -> HttpResponse {
    if !adapter.local_web_consent_listener_ready() {
        return local_web_consent_error_page(
            503,
            "Service Unavailable",
            "LOCAL_WEB_CONSENT_UNAVAILABLE",
            "Local web consent is not available for this Volicord process.",
        );
    }
    let Some(consent_context) = adapter.local_web_consent.as_ref() else {
        return local_web_consent_error_page(
            503,
            "Service Unavailable",
            "LOCAL_WEB_CONSENT_UNAVAILABLE",
            "Local web consent is not available for this Volicord process.",
        );
    };
    let origin = request.header("origin");
    let origin_matches = origin == Some(consent_context.base_url.as_str());
    let origin_is_required = request.method == "POST";
    if (origin_is_required || origin.is_some()) && !origin_matches {
        return local_web_consent_error_page(
            403,
            "Forbidden",
            "ORIGIN_NOT_ALLOWED",
            "This consent form only accepts same-origin submissions.",
        );
    }

    match request.method.as_str() {
        "GET" => handle_local_web_consent_get(adapter, request),
        "POST" => handle_local_web_consent_post(adapter, request),
        _ => local_web_consent_error_page(
            405,
            "Method Not Allowed",
            "METHOD_NOT_ALLOWED",
            "This consent endpoint supports only GET and POST.",
        )
        .with_header("Allow", "GET, POST"),
    }
}

pub(crate) fn handle_local_web_consent_get(
    adapter: &McpAdapter,
    request: HttpRequest,
) -> HttpResponse {
    let fields = match parse_urlencoded(http_request_query(&request.target)) {
        Ok(fields) => fields,
        Err(_) => {
            return local_web_consent_error_page(
                400,
                "Bad Request",
                "FORM_ENCODING_INVALID",
                "Consent link data must use valid percent encoding.",
            )
        }
    };
    let Some(project_id) = single_param(&fields, "project") else {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "INVALID_TOKEN",
            "The consent link is missing required token context.",
        );
    };
    let Some(token) = single_param(&fields, "token") else {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "INVALID_TOKEN",
            "The consent link is missing required token context.",
        );
    };
    let now = match local_web_consent_timestamp_for_validation(adapter, project_id) {
        Ok(now) => now,
        Err(McpAdapterError::Store(StoreError::NotFound { .. })) => {
            return local_web_consent_error_page(
                404,
                "Not Found",
                "INVALID_TOKEN",
                "This consent link is not valid.",
            )
        }
        Err(_) => {
            return local_web_consent_error_page(
                500,
                "Internal Server Error",
                "STORE_UNAVAILABLE",
                "Volicord could not check this consent token.",
            )
        }
    };
    match validate_local_web_consent(adapter, project_id, token, &now) {
        Ok(UserActionChannelTokenValidation::Valid(record)) => {
            match local_web_pending_user_action(adapter, &record, token, false) {
                Ok(action) => local_web_consent_page(adapter, &record, &action, token),
                Err(response) => response,
            }
        }
        Ok(UserActionChannelTokenValidation::Rejected(rejection)) => {
            local_web_consent_rejection_page(rejection)
        }
        Err(_) => local_web_consent_error_page(
            500,
            "Internal Server Error",
            "STORE_UNAVAILABLE",
            "Volicord could not check this consent token.",
        ),
    }
}

pub(crate) fn handle_local_web_consent_post(
    adapter: &McpAdapter,
    request: HttpRequest,
) -> HttpResponse {
    if !content_type_is_form(request.header("content-type")) {
        return local_web_consent_error_page(
            415,
            "Unsupported Media Type",
            "CONTENT_TYPE_UNSUPPORTED",
            "Consent form submissions must use application/x-www-form-urlencoded.",
        );
    }
    let body = match str::from_utf8(&request.body) {
        Ok(body) => body,
        Err(_) => {
            return local_web_consent_error_page(
                400,
                "Bad Request",
                "FORM_ENCODING_INVALID",
                "Consent form data must be UTF-8.",
            )
        }
    };
    let fields = match parse_urlencoded(body) {
        Ok(fields) => fields,
        Err(_) => {
            return local_web_consent_error_page(
                400,
                "Bad Request",
                "FORM_ENCODING_INVALID",
                "Consent form data must use valid percent encoding.",
            )
        }
    };
    let Some(project_id) = single_param(&fields, "project") else {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "INVALID_TOKEN",
            "The consent form is missing required token context.",
        );
    };
    let Some(token) = single_param(&fields, "token") else {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "INVALID_TOKEN",
            "The consent form is missing required token context.",
        );
    };
    let now = match local_web_consent_timestamp_for_validation(adapter, project_id) {
        Ok(now) => now,
        Err(McpAdapterError::Store(StoreError::NotFound { .. })) => {
            return local_web_consent_error_page(
                404,
                "Not Found",
                "INVALID_TOKEN",
                "This consent link is not valid.",
            )
        }
        Err(_) => {
            return local_web_consent_error_page(
                500,
                "Internal Server Error",
                "STORE_UNAVAILABLE",
                "Volicord could not check this consent token.",
            )
        }
    };
    let validation = match validate_local_web_consent(adapter, project_id, token, &now) {
        Ok(validation) => validation,
        Err(_) => {
            return local_web_consent_error_page(
                500,
                "Internal Server Error",
                "STORE_UNAVAILABLE",
                "Volicord could not check this consent token.",
            )
        }
    };
    let record = match validation {
        UserActionChannelTokenValidation::Valid(record)
        | UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Consumed(
            record,
        )) => record,
        UserActionChannelTokenValidation::Rejected(rejection) => {
            return local_web_consent_rejection_page(rejection)
        }
    };
    let action = match local_web_pending_user_action(adapter, &record, token, true) {
        Ok(action) => action,
        Err(response) => return response,
    };
    let resolution = match local_web_resolution_from_fields(&action, &fields) {
        Ok(resolution) => resolution,
        Err(message) => {
            return local_web_consent_error_page(400, "Bad Request", "INVALID_SELECTION", &message)
        }
    };

    match resolve_local_web_user_action(adapter, &action, resolution, token) {
        Ok(recorded)
            if recorded.response_value["base"]["response_kind"].as_str() == Some("result") =>
        {
            let consumed =
                local_web_consumed_record_after_recording(adapter, project_id, token, &now)
                    .unwrap_or(record);
            local_web_consent_success_page(&consumed, &action)
        }
        Ok(_) => local_web_post_recording_rejected(adapter, project_id, token, &now),
        Err(_) => local_web_post_recording_failed(adapter, project_id, token, &now),
    }
}

pub(crate) fn validate_local_web_consent(
    adapter: &McpAdapter,
    project_id: &str,
    token: &str,
    now: &str,
) -> Result<UserActionChannelTokenValidation, McpAdapterError> {
    validate_user_action_channel_token(
        &adapter.runtime_home,
        UserActionChannelTokenCheck {
            token: token.to_owned(),
            expected_project_id: project_id.to_owned(),
            expected_connection_internal_id: adapter.context.connection_internal_id.to_string(),
            now: now.to_owned(),
        },
    )
    .map_err(McpAdapterError::Store)
}

pub(crate) fn local_web_consent_timestamp_for_validation(
    adapter: &McpAdapter,
    project_id: &str,
) -> Result<String, McpAdapterError> {
    user_action_channel_current_timestamp(&adapter.runtime_home, project_id)
        .map_err(McpAdapterError::Store)
}

pub(crate) fn local_web_consumed_record_after_recording(
    adapter: &McpAdapter,
    project_id: &str,
    token: &str,
    now: &str,
) -> Option<UserActionChannelTokenRecord> {
    match validate_local_web_consent(adapter, project_id, token, now).ok()? {
        UserActionChannelTokenValidation::Rejected(UserActionChannelTokenRejection::Consumed(
            record,
        )) => Some(record),
        _ => None,
    }
}

pub(crate) fn local_web_post_recording_rejected(
    adapter: &McpAdapter,
    project_id: &str,
    token: &str,
    now: &str,
) -> HttpResponse {
    match validate_local_web_consent(adapter, project_id, token, now) {
        Ok(UserActionChannelTokenValidation::Rejected(rejection)) => {
            local_web_consent_rejection_page(rejection)
        }
        _ => local_web_consent_error_page(
            409,
            "Conflict",
            "USER_ACTION_RESOLUTION_REJECTED",
            "Volicord could not resolve this action because the pending request is no longer current.",
        ),
    }
}

pub(crate) fn local_web_post_recording_failed(
    adapter: &McpAdapter,
    project_id: &str,
    token: &str,
    now: &str,
) -> HttpResponse {
    match validate_local_web_consent(adapter, project_id, token, now) {
        Ok(UserActionChannelTokenValidation::Rejected(rejection)) => {
            local_web_consent_rejection_page(rejection)
        }
        _ => local_web_consent_error_page(
            500,
            "Internal Server Error",
            "USER_ACTION_RESOLUTION_FAILED",
            "Volicord could not resolve this action. The token remains usable only while the request is current and unexpired.",
        ),
    }
}

pub(crate) struct LocalWebPendingUserAction {
    request: UserActionRequest,
    form: UserActionInboxForm,
}

fn local_web_consent_form_mismatch_page() -> HttpResponse {
    local_web_consent_error_page(
        409,
        "Conflict",
        "TOKEN_FORM_MISMATCH",
        "This consent link does not match the current canonical user-action form.",
    )
}

pub(crate) fn local_web_pending_user_action(
    adapter: &McpAdapter,
    token_record: &UserActionChannelTokenRecord,
    token: &str,
    allow_resolved_replay: bool,
) -> Result<LocalWebPendingUserAction, HttpResponse> {
    match adapter.core.local_web_consent_user_action_projection(
        LocalWebConsentUserActionProjectionRequest {
            token: token.to_owned(),
            validated_token: token_record.clone(),
            allow_resolved_replay,
        },
    ) {
        Ok(LocalWebConsentUserActionProjectionOutcome::Projected(projection)) => {
            let projection = *projection;
            Ok(LocalWebPendingUserAction {
                request: projection.request,
                form: projection.form,
            })
        }
        Ok(LocalWebConsentUserActionProjectionOutcome::FormMismatch) => {
            Err(local_web_consent_form_mismatch_page())
        }
        Ok(LocalWebConsentUserActionProjectionOutcome::Invalid)
        | Err(CorePipelineError::Store(StoreError::NotFound { .. })) => {
            Err(local_web_consent_error_page(
                404,
                "Not Found",
                "INVALID_TOKEN",
                "This consent link is not valid for an available pending user action.",
            ))
        }
        Err(_) => Err(local_web_consent_error_page(
            500,
            "Internal Server Error",
            "STORE_UNAVAILABLE",
            "Volicord could not read this pending user action.",
        )),
    }
}

fn local_web_resolution_from_fields(
    pending: &LocalWebPendingUserAction,
    fields: &BTreeMap<String, Vec<String>>,
) -> Result<UserActionResolutionInput, String> {
    match &pending.form {
        UserActionInboxForm::Choice {
            choices,
            note_allowed,
            note_max_chars,
        } => {
            let allowed = if *note_allowed {
                &["project", "token", "selected_option_id", "note"][..]
            } else {
                &["project", "token", "selected_option_id"][..]
            };
            reject_unknown_field_names(fields.keys().map(String::as_str), allowed, "Consent form")?;
            let selected = single_param(fields, "selected_option_id").ok_or_else(|| {
                "Choose one stored user-action option before submitting.".to_owned()
            })?;
            let choice = choices
                .iter()
                .find(|choice| choice.choice_id.as_str() == selected)
                .ok_or_else(|| {
                    "The selected option is not valid for this pending user action.".to_owned()
                })?;
            let note = optional_param(fields, "note");
            if !*note_allowed && note.is_some() {
                return Err("This form does not accept a note.".to_owned());
            }
            if note
                .as_ref()
                .is_some_and(|note| note.chars().count() > *note_max_chars as usize)
            {
                return Err("The optional note exceeds its character limit.".to_owned());
            }
            Ok(UserActionResolutionInput::Choice {
                selected_option_id: choice.choice_id.clone(),
                note: note.into(),
            })
        }
        UserActionInboxForm::EvidenceObservation {
            target_candidates,
            artifact_candidates,
            relevance_options,
            summary_max_chars,
        } => {
            reject_unknown_field_names(
                fields.keys().map(String::as_str),
                &[
                    "project",
                    "token",
                    "selected_target",
                    "selected_artifact_ids",
                    "relevance_status",
                    "summary",
                ],
                "Consent form",
            )?;
            let target_selector = single_param(fields, "selected_target")
                .ok_or_else(|| "Choose one stored evidence target.".to_owned())?;
            let presentation = UserActionPresentationPlan::from_form(&pending.form)
                .map_err(|_| "The stored evidence form cannot be rendered.".to_owned())?;
            let UserActionPresentationForm::EvidenceObservation { targets, .. } =
                &presentation.form
            else {
                return Err("The stored evidence form is invalid.".to_owned());
            };
            let target_index = targets
                .iter()
                .position(|target| target.selector == target_selector)
                .ok_or_else(|| {
                    "The selected evidence target is not a stored candidate.".to_owned()
                })?;
            let target = target_candidates[target_index].clone();
            let selected_artifacts = fields
                .get("selected_artifact_ids")
                .ok_or_else(|| "Choose at least one stored artifact.".to_owned())?;
            if selected_artifacts.is_empty() {
                return Err("Choose at least one stored artifact.".to_owned());
            }
            let mut seen = BTreeSet::new();
            let mut artifact_ids = Vec::with_capacity(selected_artifacts.len());
            for id in selected_artifacts {
                let artifact = artifact_candidates
                    .iter()
                    .find(|artifact| artifact.artifact_id.as_str() == id)
                    .ok_or_else(|| "A selected artifact is not a stored candidate.".to_owned())?;
                if !seen.insert(id.clone()) {
                    return Err("Selected artifacts must not contain duplicates.".to_owned());
                }
                artifact_ids.push(artifact.artifact_id.clone());
            }
            let relevance_text = single_param(fields, "relevance_status")
                .ok_or_else(|| "Choose one relevance value.".to_owned())?;
            let relevance_status = serde_json::from_value(Value::String(relevance_text.to_owned()))
                .map_err(|_| "The selected relevance value is invalid.".to_owned())?;
            if !relevance_options.contains(&relevance_status) {
                return Err("The selected relevance value is not a stored option.".to_owned());
            }
            let summary = single_param(fields, "summary")
                .ok_or_else(|| "Enter an observation summary.".to_owned())?;
            if summary.trim().is_empty() || summary.chars().count() > *summary_max_chars as usize {
                return Err(
                    "The observation summary must be non-empty and within its character limit."
                        .to_owned(),
                );
            }
            Ok(UserActionResolutionInput::EvidenceObservation {
                target,
                artifact_ids,
                relevance_status,
                summary: summary.to_owned(),
            })
        }
    }
}

pub(crate) fn resolve_local_web_user_action(
    adapter: &McpAdapter,
    pending: &LocalWebPendingUserAction,
    resolution: UserActionResolutionInput,
    token: &str,
) -> Result<PipelineResponse, McpAdapterError> {
    let completion_metadata = LocalWebConsentCompletionMetadata {
        selection_recording: Some("recorded".to_owned()),
        endpoint: Some(LOCAL_WEB_CONSENT_PATH.to_owned()),
    };
    let channel_submission_id = local_web_channel_submission_id(
        &pending.request.project_id,
        &pending.request.user_action_request_id,
        token,
        adapter.context.connection_internal_id.as_str(),
        &completion_metadata,
    )
    .map_err(McpAdapterError::Json)?;
    let request = ResolveUserActionRequest {
        envelope: ToolEnvelope {
            project_id: pending.request.project_id.clone(),
            task_id: Some(pending.request.task_id.clone()).into(),
            request_id: RequestId::new(format!(
                "req_{}",
                sanitize_metadata_component(&channel_submission_id)
            )),
            idempotency_key: Some(IdempotencyKey::new(channel_submission_id.clone())).into(),
            expected_state_version: RequiredNullable::null(),
            dry_run: false,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        },
        user_action_request_id: pending.request.user_action_request_id.clone(),
        channel_submission_id,
        resolution,
    };
    let invocation = InvocationContext::new(
        pending.request.project_id.clone(),
        ActorSource::LocalUser,
        OperationCategory::UserOnly,
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
    );
    adapter
        .core
        .resolve_local_web_consent_user_action(
            LocalWebConsentUserActionRequest {
                request,
                token: token.to_owned(),
                expected_connection_internal_id: adapter.context.connection_internal_id.to_string(),
                completion_metadata_json: serde_json::to_string(&completion_metadata)
                    .map_err(McpAdapterError::Json)?,
            },
            invocation,
        )
        .map_err(McpAdapterError::Core)
}

pub(crate) fn local_web_consent_page(
    adapter: &McpAdapter,
    token_record: &UserActionChannelTokenRecord,
    pending: &LocalWebPendingUserAction,
    token: &str,
) -> HttpResponse {
    let Some(consent_context) = adapter.local_web_consent.as_ref() else {
        return local_web_consent_error_page(
            503,
            "Service Unavailable",
            "LOCAL_WEB_CONSENT_UNAVAILABLE",
            "Local web consent is not available for this Volicord process.",
        );
    };
    let action = format!("{}{}", consent_context.base_url, LOCAL_WEB_CONSENT_PATH);
    let project = local_web_consent_project_display(adapter, token_record.project_id.as_str());
    let repository_path = project
        .repo_root
        .as_ref()
        .map(|repo_root| {
            format!(
                "<dt>Repository path</dt><dd><code>{}</code></dd>",
                html_escape(repo_root)
            )
        })
        .unwrap_or_default();
    let cli_command = format!(
        "volicord inbox resolve {}",
        pending.request.user_action_request_id.as_str()
    );
    let presentation = match UserActionPresentationPlan::from_form(&pending.form) {
        Ok(presentation) => presentation,
        Err(_) => {
            return local_web_consent_error_page(
                500,
                "Internal Server Error",
                "FORM_UNAVAILABLE",
                "Volicord could not render the complete closed user-action form.",
            )
        }
    };
    let form_fields = match &presentation.form {
        UserActionPresentationForm::Choice {
            choices,
            note_allowed,
            note_max_chars,
        } => {
            let options = choices.iter().map(|choice| format!(
                "<label class=\"option\"><input type=\"radio\" name=\"selected_option_id\" value=\"{}\"{}><span><strong>{}</strong><br><small>Choice ID: <code>{}</code></small><br>{}<br><small>Consequence: {}</small></span></label>",
                html_escape(choice.choice_id.as_str()), if choice.is_default { " checked" } else { "" }, html_escape(&choice.label), html_escape(choice.choice_id.as_str()), html_escape(&choice.description), html_escape(&choice.consequence)
            )).collect::<Vec<_>>().join("\n");
            let note = if *note_allowed {
                format!("<label>Optional note<textarea name=\"note\" maxlength=\"{}\" rows=\"4\"></textarea></label>", note_max_chars)
            } else {
                String::new()
            };
            format!("<fieldset><legend>Available choices</legend>{options}</fieldset>{note}")
        }
        UserActionPresentationForm::EvidenceObservation {
            targets,
            artifacts,
            relevance_options,
            summary_max_chars,
        } => {
            let targets = targets.iter().enumerate().map(|(index, target)| format!("<label class=\"option\"><input type=\"radio\" name=\"selected_target\" value=\"{}\"{}><span><strong>{}</strong><br><code>{}</code><br><small>{}</small></span></label>", html_escape(&target.selector), if index == 0 { " checked" } else { "" }, html_escape(&target.display_name), html_escape(&target.selector), html_escape(&target.metadata_json))).collect::<Vec<_>>().join("\n");
            let artifacts = artifacts.iter().map(|artifact| format!("<label class=\"option\"><input type=\"checkbox\" name=\"selected_artifact_ids\" value=\"{}\"><span><strong>{}</strong><br><code>{}</code><br><small>{}</small></span></label>", html_escape(&artifact.artifact_id), html_escape(&artifact.display_name), html_escape(&artifact.artifact_id), html_escape(&artifact.metadata_json))).collect::<Vec<_>>().join("\n");
            let relevance = relevance_options.iter().enumerate().map(|(index, status)| format!("<label class=\"option\"><input type=\"radio\" name=\"relevance_status\" value=\"{}\"{}><span>{}</span></label>", html_escape(status), if index == 0 { " checked" } else { "" }, html_escape(status))).collect::<Vec<_>>().join("\n");
            format!("<fieldset><legend>Evidence target</legend>{targets}</fieldset><fieldset><legend>Observed artifacts</legend>{artifacts}</fieldset><fieldset><legend>Relevance</legend>{relevance}</fieldset><label>Observation summary<textarea name=\"summary\" maxlength=\"{}\" rows=\"6\" required></textarea></label>", summary_max_chars)
        }
    };
    let body = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Volicord User Action</title>{}</head><body><main><h1>Resolve user action</h1><section class="notice"><p>This page records one user-owned action through the local User Channel. The agent cannot resolve it on your behalf.</p><p>The resolution does not prove correctness, test sufficiency, deployment success, review completion, security enforcement, or close readiness.</p></section><section><h2>Question</h2><p>{}</p><h2>Context</h2><p>{}</p></section><section><h2>Action identity</h2><dl><dt>Project name</dt><dd>{}</dd><dt>Project identifier</dt><dd><code>{}</code></dd>{}<dt>Connection identifier</dt><dd><code>{}</code></dd><dt>User-action request id</dt><dd><code>{}</code></dd><dt>Token expires</dt><dd>{}</dd><dt>Fallback CLI command</dt><dd><code>{}</code></dd></dl></section><form method="post" action="{}"><input type="hidden" name="project" value="{}"><input type="hidden" name="token" value="{}">{}<button type="submit">Record user action</button></form></main></body></html>"#,
        local_web_consent_css(),
        html_escape(pending.request.body.question()),
        html_escape(pending.request.body.context_summary()),
        html_escape(&project.project_name),
        html_escape(&project.project_id),
        repository_path,
        html_escape(token_record.connection_internal_id.as_str()),
        html_escape(token_record.user_action_request_id.as_str()),
        html_escape(token_record.expires_at.as_str()),
        html_escape(&cli_command),
        html_escape(&action),
        html_escape(token_record.project_id.as_str()),
        html_escape(token),
        form_fields
    );
    local_web_consent_html_response(200, "OK", body)
}

pub(crate) fn local_web_consent_success_page(
    token_record: &UserActionChannelTokenRecord,
    pending: &LocalWebPendingUserAction,
) -> HttpResponse {
    let body = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Volicord User Action Recorded</title>{}</head><body><main><h1>Resolution recorded</h1><p>Volicord resolved user action <code>{}</code> through the local User Channel.</p><p>This record does not prove correctness, test sufficiency, deployment success, review completion, security enforcement, or close readiness.</p><dl><dt>Project identifier</dt><dd><code>{}</code></dd><dt>Connection identifier</dt><dd><code>{}</code></dd><dt>Completed</dt><dd>{}</dd></dl></main></body></html>"#,
        local_web_consent_css(),
        html_escape(pending.request.user_action_request_id.as_str()),
        html_escape(token_record.project_id.as_str()),
        html_escape(token_record.connection_internal_id.as_str()),
        html_escape(
            token_record
                .completed_at
                .as_deref()
                .unwrap_or(token_record.consumed_at.as_deref().unwrap_or(""))
        )
    );
    local_web_consent_html_response(200, "OK", body)
}

pub(crate) fn local_web_consent_rejection_page(
    rejection: UserActionChannelTokenRejection,
) -> HttpResponse {
    match rejection {
        UserActionChannelTokenRejection::Invalid => local_web_consent_error_page(
            404,
            "Not Found",
            "INVALID_TOKEN",
            "This consent link is not valid.",
        ),
        UserActionChannelTokenRejection::Expired(_) => local_web_consent_error_page(
            410,
            "Gone",
            "TOKEN_EXPIRED",
            "This consent link has expired.",
        ),
        UserActionChannelTokenRejection::Consumed(_) => local_web_consent_error_page(
            409,
            "Conflict",
            "TOKEN_CONSUMED",
            "This consent link has already been used.",
        ),
        UserActionChannelTokenRejection::WrongConnection { .. } => local_web_consent_error_page(
            403,
            "Forbidden",
            "WRONG_CONNECTION",
            "This consent link does not match this Volicord connection.",
        ),
    }
}

pub(crate) fn local_web_consent_error_page(
    status: u16,
    reason: &'static str,
    code: &'static str,
    message: &str,
) -> HttpResponse {
    let body = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Volicord Consent Error</title>{}</head><body><main><h1>Consent unavailable</h1><p>{}</p><p><code>{}</code></p></main></body></html>",
        local_web_consent_css(),
        html_escape(message),
        html_escape(code)
    );
    local_web_consent_html_response(status, reason, body)
}

pub(crate) fn local_web_consent_css() -> &'static str {
    "<style>body{font-family:system-ui,-apple-system,BlinkMacSystemFont,\"Segoe UI\",sans-serif;margin:0;background:#f7f7f4;color:#1e2528}main{max-width:820px;margin:0 auto;padding:32px 20px}h1{font-size:1.7rem;margin:0 0 20px}h2{font-size:1rem;margin:22px 0 8px}p,dd,li{line-height:1.45}.notice{border-left:4px solid #0f5f6b;background:#fff;padding:10px 14px}dl{display:grid;grid-template-columns:max-content 1fr;gap:8px 16px}dt{font-weight:700}dd{margin:0;overflow-wrap:anywhere}fieldset{border:1px solid #c8d0d4;padding:12px;margin:18px 0}legend{font-weight:700}.option{display:grid;grid-template-columns:24px 1fr;gap:8px;margin:10px 0;padding:10px;border:1px solid #d6dcdf;background:#fff}textarea{display:block;width:100%;box-sizing:border-box;margin-top:8px}button{margin-top:16px;padding:10px 14px;font:inherit;background:#0f5f6b;color:#fff;border:0}code{background:#e9eeee;padding:2px 4px}</style>"
}

pub(crate) fn local_web_consent_html_response(
    status: u16,
    reason: &'static str,
    body: String,
) -> HttpResponse {
    HttpResponse::html(status, reason, body)
        .with_header("Cache-Control", "no-store")
        .with_header("Referrer-Policy", "no-referrer")
        .with_header("X-Content-Type-Options", "nosniff")
        .with_header("Content-Security-Policy", LOCAL_WEB_CONSENT_CSP)
}

struct LocalWebConsentProjectDisplay {
    project_name: String,
    project_id: String,
    repo_root: Option<String>,
}

fn local_web_consent_project_display(
    adapter: &McpAdapter,
    project_id: &str,
) -> LocalWebConsentProjectDisplay {
    let project_id = project_id.to_owned();
    match CoreProjectStore::open(&adapter.runtime_home, &ProjectId::new(project_id.clone())) {
        Ok(store) => {
            let project = store.project_record();
            LocalWebConsentProjectDisplay {
                project_name: if project.project_name.trim().is_empty() {
                    project.project_id.clone()
                } else {
                    project.project_name.clone()
                },
                project_id: project.project_id.clone(),
                repo_root: Some(project.repo_root.display().to_string()),
            }
        }
        Err(_) => LocalWebConsentProjectDisplay {
            project_name: project_id.clone(),
            project_id,
            repo_root: None,
        },
    }
}

pub(crate) fn html_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

pub(crate) fn http_request_path(target: &str) -> &str {
    target
        .split_once('?')
        .map(|(path, _)| path)
        .unwrap_or(target)
}

pub(crate) fn http_request_query(target: &str) -> &str {
    target.split_once('?').map(|(_, query)| query).unwrap_or("")
}

pub(crate) fn parse_urlencoded(input: &str) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut fields = BTreeMap::<String, Vec<String>>::new();
    for pair in input.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let name = percent_decode_form(name)
            .ok_or_else(|| "form field name uses invalid percent encoding".to_owned())?;
        let value = percent_decode_form(value)
            .ok_or_else(|| "form field value uses invalid percent encoding".to_owned())?;
        fields.entry(name).or_default().push(value);
    }
    Ok(fields)
}

pub(crate) fn single_param<'a>(
    fields: &'a BTreeMap<String, Vec<String>>,
    name: &str,
) -> Option<&'a str> {
    let values = fields.get(name)?;
    (values.len() == 1 && !values[0].trim().is_empty()).then_some(values[0].as_str())
}

pub(crate) fn optional_param(fields: &BTreeMap<String, Vec<String>>, name: &str) -> Option<String> {
    let values = fields.get(name)?;
    if values.len() != 1 || values[0].is_empty() {
        None
    } else {
        Some(values[0].clone())
    }
}

pub(crate) fn percent_decode_form(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let high = hex_value(bytes[index + 1])?;
                let low = hex_value(bytes[index + 2])?;
                output.push((high << 4) | low);
                index += 3;
            }
            b'%' => return None,
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(output).ok()
}

pub(crate) fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn content_type_is_form(header: Option<&str>) -> bool {
    let Some(header) = header else {
        return false;
    };
    header
        .split_once(';')
        .map(|(media_type, _)| media_type.trim())
        .unwrap_or_else(|| header.trim())
        == "application/x-www-form-urlencoded"
}

#[cfg(test)]
mod readiness_tests {
    use super::*;

    #[test]
    fn listener_guard_marks_shared_readiness_unavailable_on_exit() {
        let (readiness, guard) = LocalWebConsentReadiness::tracked();
        {
            let _guard = guard;
            assert!(readiness.is_ready());
        }
        assert!(!readiness.is_ready());
    }
}
