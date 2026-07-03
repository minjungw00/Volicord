use crate::adapter::*;
use crate::errors::{LocalHttpError, McpAdapterError};
use crate::http::*;
use crate::prelude::*;
use crate::routing::*;
use crate::stdio::*;
use crate::util::*;

/// Local MCP adapter bound to a Core service and one Agent Connection.
pub(crate) fn start_stdio_local_web_consent_listener(
    runtime_home: &Path,
    context: &McpConnectionContext,
) -> Result<LocalWebConsentContext, McpAdapterError> {
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
    let adapter = McpAdapter::new(runtime_home, context.clone())
        .with_local_web_consent(consent_context.clone());
    thread::Builder::new()
        .name("volicord-local-web-consent".to_owned())
        .spawn(move || {
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
                        eprintln!("warning: local web consent listener stopped: {error}");
                        break;
                    }
                }
            }
        })
        .map_err(McpAdapterError::Io)?;
    Ok(consent_context)
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
        let origin = request.header("origin").map(str::to_owned);
        if http_request_path(&request.target) == LOCAL_WEB_CONSENT_PATH {
            handle_local_web_consent_http_request(&self.adapter, request, origin.as_deref())
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
    origin: Option<&str>,
) -> HttpResponse {
    let Some(consent_context) = adapter.local_web_consent.as_ref() else {
        return local_web_consent_error_page(
            503,
            "Service Unavailable",
            "LOCAL_WEB_CONSENT_UNAVAILABLE",
            "Local web consent is not available for this Volicord process.",
        );
    };
    if let Some(origin) = origin {
        if origin != consent_context.base_url {
            return local_web_consent_error_page(
                403,
                "Forbidden",
                "ORIGIN_NOT_ALLOWED",
                "This consent form only accepts same-origin submissions.",
            );
        }
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
    let fields = parse_urlencoded(http_request_query(&request.target));
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
        Ok(LocalWebConsentTokenValidation::Valid(record)) => {
            match local_web_pending_judgment(adapter, &record) {
                Ok(judgment) => local_web_consent_page(adapter, &record, &judgment, token),
                Err(response) => response,
            }
        }
        Ok(LocalWebConsentTokenValidation::Rejected(rejection)) => {
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
    let fields = parse_urlencoded(body);
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
    let Some(selected_option_id) = single_param(&fields, "selected_option_id") else {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "INVALID_SELECTION",
            "Choose one judgment option before submitting.",
        );
    };
    let note = optional_param(&fields, "note");
    if note.as_ref().is_some_and(|value| value.len() > 1000) {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "NOTE_TOO_LONG",
            "The optional note must be at most 1000 characters.",
        );
    }

    let now = match local_web_consent_timestamp_for_validation(adapter, project_id) {
        Ok(now) => now,
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
        LocalWebConsentTokenValidation::Valid(record) => record,
        LocalWebConsentTokenValidation::Rejected(rejection) => {
            return local_web_consent_rejection_page(rejection)
        }
    };
    let judgment = match local_web_pending_judgment(adapter, &record) {
        Ok(judgment) => judgment,
        Err(response) => return response,
    };
    let Some(selected_option) = judgment
        .options
        .iter()
        .find(|option| option.option_id.as_str() == selected_option_id)
        .cloned()
    else {
        return local_web_consent_error_page(
            400,
            "Bad Request",
            "INVALID_SELECTION",
            "The selected option is not valid for this pending judgment.",
        );
    };

    match record_local_web_judgment(adapter, &judgment, &selected_option, token, note) {
        Ok(recorded)
            if recorded.response_value["base"]["response_kind"].as_str() == Some("result") =>
        {
            let consumed =
                local_web_consumed_record_after_recording(adapter, project_id, token, &now)
                    .unwrap_or(record);
            local_web_consent_success_page(&consumed, &judgment, &selected_option)
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
) -> Result<LocalWebConsentTokenValidation, McpAdapterError> {
    validate_local_web_consent_token(
        &adapter.runtime_home,
        LocalWebConsentTokenCheck {
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
    match local_web_consent_current_timestamp(&adapter.runtime_home, project_id) {
        Ok(now) => Ok(now),
        Err(StoreError::NotFound { entity, id }) if entity == "project" => {
            let projects = match adapter.allowed_project_availabilities("local web consent") {
                Ok(projects) => projects,
                Err(_) => return Err(McpAdapterError::Store(StoreError::NotFound { entity, id })),
            };
            for project in projects {
                if project.available {
                    return local_web_consent_current_timestamp(
                        &adapter.runtime_home,
                        &project.project_id,
                    )
                    .map_err(McpAdapterError::Store);
                }
            }
            Err(McpAdapterError::Store(StoreError::NotFound { entity, id }))
        }
        Err(error) => Err(McpAdapterError::Store(error)),
    }
}

pub(crate) fn local_web_consumed_record_after_recording(
    adapter: &McpAdapter,
    project_id: &str,
    token: &str,
    now: &str,
) -> Option<LocalWebConsentTokenRecord> {
    match validate_local_web_consent(adapter, project_id, token, now).ok()? {
        LocalWebConsentTokenValidation::Rejected(LocalWebConsentTokenRejection::Consumed(
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
        Ok(LocalWebConsentTokenValidation::Rejected(rejection)) => {
            local_web_consent_rejection_page(rejection)
        }
        _ => local_web_consent_error_page(
            409,
            "Conflict",
            "JUDGMENT_RECORDING_REJECTED",
            "Volicord could not record this answer because the pending judgment is no longer current. The token remains usable until it expires.",
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
        Ok(LocalWebConsentTokenValidation::Rejected(rejection)) => {
            local_web_consent_rejection_page(rejection)
        }
        _ => local_web_consent_error_page(
            500,
            "Internal Server Error",
            "JUDGMENT_RECORDING_FAILED",
            "Volicord could not record this answer. The token remains usable until it expires if the pending judgment is still current.",
        ),
    }
}

pub(crate) fn local_web_pending_judgment(
    adapter: &McpAdapter,
    token_record: &LocalWebConsentTokenRecord,
) -> Result<UserJudgment, HttpResponse> {
    let project_id = ProjectId::new(token_record.project_id.clone());
    let store = CoreProjectStore::open(&adapter.runtime_home, &project_id).map_err(|_| {
        local_web_consent_error_page(
            404,
            "Not Found",
            "WRONG_PROJECT",
            "This consent token does not match an available project.",
        )
    })?;
    let record = store
        .user_judgment_record(&token_record.judgment_id)
        .map_err(|_| {
            local_web_consent_error_page(
                500,
                "Internal Server Error",
                "STORE_UNAVAILABLE",
                "Volicord could not read this pending judgment.",
            )
        })?
        .ok_or_else(|| {
            local_web_consent_error_page(
                404,
                "Not Found",
                "INVALID_TOKEN",
                "This consent token does not identify an available pending judgment.",
            )
        })?;
    if record.status != "pending" {
        return Err(local_web_consent_error_page(
            409,
            "Conflict",
            "TOKEN_CONSUMED",
            "This pending judgment has already been answered or is no longer available.",
        ));
    }
    user_judgment_from_record(&record).map_err(|_| {
        local_web_consent_error_page(
            500,
            "Internal Server Error",
            "STORE_UNAVAILABLE",
            "Volicord could not render this pending judgment.",
        )
    })
}

pub(crate) fn user_judgment_from_record(
    record: &UserJudgmentRecord,
) -> Result<UserJudgment, McpAdapterError> {
    let request_json: Value =
        serde_json::from_str(&record.request_json).map_err(McpAdapterError::Json)?;
    let context: UserJudgmentContext =
        serde_json::from_str(&record.context_json).map_err(McpAdapterError::Json)?;
    let affected_refs: Vec<StateRecordRef> =
        serde_json::from_str(&record.affected_refs_json).map_err(McpAdapterError::Json)?;
    let options = serde_json::from_str::<PersistedUserJudgmentOptions>(&record.options_json)
        .map_err(McpAdapterError::Json)?
        .into_current_options()
        .map_err(|error| McpAdapterError::ToolExecution {
            tool_name: LOCAL_WEB_CONSENT_PATH.to_owned(),
            message: error.to_string(),
        })?;
    let basis: PersistedJudgmentBasis =
        serde_json::from_str(&record.basis_json).map_err(McpAdapterError::Json)?;
    let judgment_kind =
        serde_json::from_value::<JudgmentKind>(Value::String(record.judgment_kind.clone()))
            .map_err(McpAdapterError::Json)?;
    let status = serde_json::from_value::<UserJudgmentStatus>(Value::String(record.status.clone()))
        .map_err(McpAdapterError::Json)?;
    let presentation = serde_json::from_value(
        request_json
            .get("presentation")
            .cloned()
            .unwrap_or(Value::String("short".to_owned())),
    )
    .map_err(McpAdapterError::Json)?;
    let question = request_json
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let required_for = serde_json::from_value(
        request_json
            .get("required_for")
            .cloned()
            .unwrap_or_else(|| json!([])),
    )
    .map_err(McpAdapterError::Json)?;
    let expires_at = serde_json::from_value(
        request_json
            .get("expires_at")
            .cloned()
            .unwrap_or(Value::Null),
    )
    .map_err(McpAdapterError::Json)?;
    let created_at = serde_json::from_value(Value::String(record.requested_at.clone()))
        .map_err(McpAdapterError::Json)?;
    Ok(UserJudgment {
        judgment_id: record.judgment_id.clone().into(),
        project_id: record.project_id.clone().into(),
        task_id: record.task_id.clone().into(),
        change_unit_id: record.change_unit_id.clone().map(Into::into),
        judgment_kind,
        status,
        presentation,
        question,
        options,
        context,
        affected_refs,
        basis,
        required_for,
        resolution: None,
        expires_at,
        created_at,
        resolved_at: None,
    })
}

pub(crate) fn record_local_web_judgment(
    adapter: &McpAdapter,
    judgment: &UserJudgment,
    selected_option: &UserJudgmentOption,
    token: &str,
    note: Option<String>,
) -> Result<PipelineResponse, McpAdapterError> {
    let state_version = judgment.basis.created_at_state_version + 1;
    let request = RecordUserJudgmentRequest {
        envelope: ToolEnvelope {
            project_id: judgment.project_id.clone(),
            task_id: Some(judgment.task_id.clone()).into(),
            request_id: RequestId::new(generated_metadata_id(
                "req_local_web_record",
                adapter.context.connection_internal_id.as_str(),
                "volicord.record_user_judgment",
            )),
            idempotency_key: Some(IdempotencyKey::new(generated_metadata_id(
                "idem_local_web_record",
                adapter.context.connection_internal_id.as_str(),
                "volicord.record_user_judgment",
            )))
            .into(),
            expected_state_version: Some(state_version).into(),
            dry_run: false,
            locale: Some(DEFAULT_LOCALE.to_owned()).into(),
        },
        user_judgment_id: judgment.judgment_id.clone(),
        judgment_kind: judgment.judgment_kind,
        selected_option_id: selected_option.option_id.clone(),
        answer: answer_payload_for_judgment(judgment, selected_option)?,
        rationale: rationale_for_selected_option(
            judgment.judgment_kind,
            selected_option,
            "local web consent",
        ),
        note: note.into(),
        accepted_risks: accepted_risks_for_judgment(judgment, selected_option),
    };
    let invocation = InvocationContext::new(
        judgment.project_id.clone(),
        ActorSource::LocalUser,
        OperationCategory::UserOnly,
        VERIFICATION_BASIS_LOCAL_USER_LOCAL_WEB,
    );
    adapter
        .core
        .record_local_web_consent_judgment(
            LocalWebConsentJudgmentRequest {
                request,
                token: token.to_owned(),
                expected_connection_internal_id: adapter.context.connection_internal_id.to_string(),
                completion_metadata_json: json!({
                    "selection_recording": "recorded",
                    "endpoint": LOCAL_WEB_CONSENT_PATH
                })
                .to_string(),
            },
            invocation,
        )
        .map_err(McpAdapterError::Core)
}

pub(crate) fn local_web_consent_page(
    adapter: &McpAdapter,
    token_record: &LocalWebConsentTokenRecord,
    judgment: &UserJudgment,
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
        "volicord inbox answer {} --choice <choice>",
        judgment.judgment_id.as_str()
    );
    let options = judgment
        .options
        .iter()
        .map(|option| {
            format!(
                "<label class=\"option\"><input type=\"radio\" name=\"selected_option_id\" value=\"{}\"{}><span><strong>{}</strong><br><small>Option ID: <code>{}</code></small><br>{}<br><small>Meaning: {}</small></span></label>",
                html_escape(option.option_id.as_str()),
                if option.is_default { " checked" } else { "" },
                html_escape(&option.label),
                html_escape(option.option_id.as_str()),
                html_escape(&option.description),
                html_escape(&option.consequence)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let constraints = if judgment.context.constraints.is_empty() {
        String::new()
    } else {
        format!(
            "<h2>Constraints</h2><ul>{}</ul>",
            judgment
                .context
                .constraints
                .iter()
                .map(|constraint| format!("<li>{}</li>", html_escape(constraint)))
                .collect::<Vec<_>>()
                .join("")
        )
    };
    let body = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Volicord User Judgment</title>{}</head><body><main><h1>Record user-owned judgment</h1><section class="notice"><p>This page records one user-owned judgment through the local User Channel. The agent cannot record this judgment on your behalf.</p><p>This judgment records only the selected option for the pending judgment shown here. It does not prove correctness, test sufficiency, deployment success, review completion, security enforcement, or close readiness.</p></section><section><h2>Question</h2><p>{}</p><h2>Context</h2><p>{}</p>{}</section><section><h2>Judgment identity</h2><dl><dt>Project name</dt><dd>{}</dd><dt>Project identifier</dt><dd><code>{}</code></dd>{}<dt>Connection identifier</dt><dd><code>{}</code></dd><dt>Judgment id</dt><dd><code>{}</code></dd><dt>Token expires</dt><dd>{}</dd><dt>Fallback CLI command</dt><dd><code>{}</code></dd></dl></section><form method="post" action="{}"><input type="hidden" name="project" value="{}"><input type="hidden" name="token" value="{}"><fieldset><legend>Available choices</legend>{}</fieldset><label>Optional note<textarea name="note" maxlength="1000" rows="4"></textarea></label><button type="submit">Record selected judgment</button></form></main></body></html>"#,
        local_web_consent_css(),
        html_escape(&judgment.question),
        html_escape(&judgment.context.summary),
        constraints,
        html_escape(&project.project_name),
        html_escape(&project.project_id),
        repository_path,
        html_escape(token_record.connection_internal_id.as_str()),
        html_escape(token_record.judgment_id.as_str()),
        html_escape(token_record.expires_at.as_str()),
        html_escape(&cli_command),
        html_escape(&action),
        html_escape(token_record.project_id.as_str()),
        html_escape(token),
        options
    );
    local_web_consent_html_response(200, "OK", body)
}

pub(crate) fn local_web_consent_success_page(
    token_record: &LocalWebConsentTokenRecord,
    judgment: &UserJudgment,
    selected_option: &UserJudgmentOption,
) -> HttpResponse {
    let body = format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Volicord Consent Recorded</title>{}</head><body><main><h1>Answer recorded</h1><p>Volicord recorded user-owned judgment <code>{}</code> with option <code>{}</code> through the local User Channel.</p><p>This record does not prove correctness, test sufficiency, deployment success, review completion, security enforcement, or close readiness.</p><dl><dt>Project identifier</dt><dd><code>{}</code></dd><dt>Connection identifier</dt><dd><code>{}</code></dd><dt>Completed</dt><dd>{}</dd></dl></main></body></html>"#,
        local_web_consent_css(),
        html_escape(judgment.judgment_id.as_str()),
        html_escape(selected_option.option_id.as_str()),
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
    rejection: LocalWebConsentTokenRejection,
) -> HttpResponse {
    match rejection {
        LocalWebConsentTokenRejection::Invalid => local_web_consent_error_page(
            404,
            "Not Found",
            "INVALID_TOKEN",
            "This consent link is not valid.",
        ),
        LocalWebConsentTokenRejection::Expired(_) => local_web_consent_error_page(
            410,
            "Gone",
            "TOKEN_EXPIRED",
            "This consent link has expired.",
        ),
        LocalWebConsentTokenRejection::Consumed(_) => local_web_consent_error_page(
            409,
            "Conflict",
            "TOKEN_CONSUMED",
            "This consent link has already been used.",
        ),
        LocalWebConsentTokenRejection::WrongProject { .. } => local_web_consent_error_page(
            403,
            "Forbidden",
            "WRONG_PROJECT",
            "This consent link does not match the requested project.",
        ),
        LocalWebConsentTokenRejection::WrongConnection { .. } => local_web_consent_error_page(
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

pub(crate) fn parse_urlencoded(input: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::<String, Vec<String>>::new();
    for pair in input.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        let Some(name) = percent_decode_form(name) else {
            continue;
        };
        let Some(value) = percent_decode_form(value) else {
            continue;
        };
        fields.entry(name).or_default().push(value);
    }
    fields
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
