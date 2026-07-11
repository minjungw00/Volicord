use crate::prelude::*;
#[derive(Debug)]
pub enum LocalHttpError {
    Config { code: &'static str, message: String },
    Adapter(McpAdapterError),
    Io(io::Error),
}

impl fmt::Display for LocalHttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config { code, message } => write!(formatter, "{code}: {message}"),
            Self::Adapter(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for LocalHttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Adapter(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Config { .. } => None,
        }
    }
}

#[derive(Debug)]
pub enum McpAdapterError {
    UnknownTool(String),
    InvalidParams {
        tool_name: String,
        issues: Vec<McpToolErrorIssue>,
        truncated: bool,
        source: Option<serde_json::Error>,
    },
    ToolExecution {
        tool_name: String,
        message: String,
    },
    Core(CorePipelineError),
    Store(StoreError),
    Io(io::Error),
    Json(serde_json::Error),
    Protocol(String),
    Environment(String),
}

impl fmt::Display for McpAdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(tool_name) => write!(formatter, "unknown MCP tool: {tool_name}"),
            Self::InvalidParams {
                tool_name,
                issues,
                truncated,
                source,
            } => {
                write!(formatter, "invalid params for {tool_name}")?;
                for issue in issues {
                    write!(
                        formatter,
                        "; {:?} at {}: {}",
                        issue.code, issue.path, issue.message
                    )?;
                }
                if *truncated {
                    formatter.write_str("; additional validation detail was truncated")?;
                }
                if let Some(source) = source {
                    write!(formatter, "; decoder source: {source}")?;
                }
                Ok(())
            }
            Self::ToolExecution { tool_name, message } => {
                write!(formatter, "{tool_name}: {message}")
            }
            Self::Core(error) => write!(formatter, "{error}"),
            Self::Store(error) => write!(formatter, "store error: {error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Protocol(message) | Self::Environment(message) => formatter.write_str(message),
        }
    }
}

impl Error for McpAdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidParams {
                source: Some(source),
                ..
            } => Some(source),
            Self::Core(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::UnknownTool(_)
            | Self::InvalidParams { source: None, .. }
            | Self::ToolExecution { .. }
            | Self::Protocol(_)
            | Self::Environment(_) => None,
        }
    }
}

impl From<RuntimeHomeResolutionError> for McpAdapterError {
    fn from(error: RuntimeHomeResolutionError) -> Self {
        Self::Environment(error.to_string())
    }
}

pub(crate) fn bound_mcp_tool_error_issue(
    mut issue: McpToolErrorIssue,
) -> (McpToolErrorIssue, bool) {
    let (path, path_truncated) = truncate_json_pointer(&issue.path, MAX_MCP_TOOL_ISSUE_PATH_BYTES);
    let (message, message_truncated) =
        truncate_utf8_with_suffix(&issue.message, MAX_MCP_TOOL_ISSUE_MESSAGE_BYTES);
    issue.path = path;
    issue.message = if message.is_empty() {
        "Validation failed.".to_owned()
    } else {
        message
    };
    (issue, path_truncated || message_truncated)
}

fn truncate_json_pointer(value: &str, max_bytes: usize) -> (String, bool) {
    let (mut truncated, was_truncated) = truncate_utf8_with_suffix(value, max_bytes);
    if was_truncated {
        let suffix_start = truncated.len().saturating_sub(3);
        if truncated[..suffix_start].ends_with('~') {
            truncated.remove(suffix_start - 1);
        }
    }
    (truncated, was_truncated)
}

fn truncate_utf8_with_suffix(value: &str, max_bytes: usize) -> (String, bool) {
    if value.len() <= max_bytes {
        return (value.to_owned(), false);
    }

    const SUFFIX: &str = "...";
    let mut end = max_bytes.saturating_sub(SUFFIX.len());
    while !value.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let mut truncated = value[..end].to_owned();
    truncated.push_str(SUFFIX);
    (truncated, true)
}
