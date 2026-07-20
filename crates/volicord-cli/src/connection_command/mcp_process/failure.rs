use std::time::Duration;

use serde_json::{json, Value};

pub(super) const MAX_CAPTURED_STDERR_BYTES: usize = 2 * 1024;
pub(super) const MAX_PROTOCOL_DETAIL_BYTES: usize = 2 * 1024;
pub(super) const MAX_IO_DETAIL_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStage {
    Startup,
    Initialize,
    ToolsList,
    SafeToolCall,
    Shutdown,
}

impl McpStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Initialize => "initialize",
            Self::ToolsList => "tools_list",
            Self::SafeToolCall => "safe_tool_call",
            Self::Shutdown => "shutdown",
        }
    }

    const fn check_code(self) -> &'static str {
        match self {
            Self::Startup | Self::Shutdown => "mcp_server_process_failed",
            Self::Initialize => "mcp_server_initialize_failed",
            Self::ToolsList => "mcp_server_tools_list_failed",
            Self::SafeToolCall => "mcp_server_safe_call_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedText {
    pub(super) text: String,
    pub(super) truncated: bool,
    pub(super) omitted_bytes: usize,
}

impl BoundedText {
    pub(super) fn empty() -> Self {
        Self {
            text: String::new(),
            truncated: false,
            omitted_bytes: 0,
        }
    }

    pub(super) fn from_utf8(value: impl AsRef<str>, limit: usize, label: &str) -> Self {
        let value = value.as_ref();
        if value.len() <= limit {
            return Self {
                text: value.to_owned(),
                truncated: false,
                omitted_bytes: 0,
            };
        }
        let mut end = limit;
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        let omitted_bytes = value.len() - end;
        Self {
            text: format!(
                "{}\n...[{label} truncated; {omitted_bytes} bytes omitted]",
                &value[..end]
            ),
            truncated: true,
            omitted_bytes,
        }
    }

    pub(super) fn from_bytes(bytes: Vec<u8>, omitted_bytes: usize, label: &str) -> Self {
        let mut text = String::from_utf8_lossy(&bytes)
            .chars()
            .map(|character| {
                if character.is_control() && !matches!(character, '\n' | '\r' | '\t') {
                    '\u{fffd}'
                } else {
                    character
                }
            })
            .collect::<String>();
        if omitted_bytes > 0 {
            text.push_str(&format!(
                "\n...[{label} truncated; {omitted_bytes} bytes omitted]"
            ));
        }
        Self {
            text,
            truncated: omitted_bytes > 0,
            omitted_bytes,
        }
    }

    pub(super) fn to_json(&self) -> Value {
        json!({
            "text": self.text,
            "truncated": self.truncated,
            "omitted_bytes": self.omitted_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpProcessFailure {
    Spawn {
        stage: McpStage,
        io_detail: BoundedText,
    },
    ExitedBeforeResponse {
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: BoundedText,
    },
    Timeout {
        stage: McpStage,
        timeout: Duration,
        stderr: BoundedText,
    },
    Read {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Write {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Protocol {
        stage: McpStage,
        protocol_detail: BoundedText,
        missing_tools: Vec<String>,
        stderr: BoundedText,
    },
    Wait {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Cleanup {
        stage: McpStage,
        io_detail: BoundedText,
        stderr: BoundedText,
    },
    Shutdown {
        stage: McpStage,
        exit_code: Option<i32>,
        stderr: BoundedText,
    },
}

impl McpProcessFailure {
    pub fn protocol(stage: McpStage, detail: impl Into<String>) -> Self {
        Self::Protocol {
            stage,
            protocol_detail: bounded_protocol_detail(detail.into()),
            missing_tools: Vec::new(),
            stderr: BoundedText::empty(),
        }
    }

    pub const fn stage(&self) -> McpStage {
        match self {
            Self::Spawn { stage, .. }
            | Self::ExitedBeforeResponse { stage, .. }
            | Self::Timeout { stage, .. }
            | Self::Read { stage, .. }
            | Self::Write { stage, .. }
            | Self::Protocol { stage, .. }
            | Self::Wait { stage, .. }
            | Self::Cleanup { stage, .. }
            | Self::Shutdown { stage, .. } => *stage,
        }
    }

    pub const fn check_code(&self) -> &'static str {
        self.stage().check_code()
    }

    pub(super) fn with_stderr(mut self, stderr: BoundedText) -> Self {
        match &mut self {
            Self::Spawn { .. } => {}
            Self::ExitedBeforeResponse {
                stderr: captured, ..
            }
            | Self::Timeout {
                stderr: captured, ..
            }
            | Self::Read {
                stderr: captured, ..
            }
            | Self::Write {
                stderr: captured, ..
            }
            | Self::Protocol {
                stderr: captured, ..
            }
            | Self::Wait {
                stderr: captured, ..
            }
            | Self::Cleanup {
                stderr: captured, ..
            }
            | Self::Shutdown {
                stderr: captured, ..
            } => *captured = stderr,
        }
        self
    }

    pub(super) fn summary(&self) -> String {
        match self {
            Self::Spawn { io_detail, .. } => {
                format!("MCP process spawn failed: {}", io_detail.text)
            }
            Self::ExitedBeforeResponse {
                stage, exit_code, ..
            } => format!(
                "MCP process exited before a response during {} with exit code {}",
                stage.as_str(),
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unavailable".to_owned())
            ),
            Self::Timeout { stage, .. } => {
                format!("MCP process timed out during {}", stage.as_str())
            }
            Self::Read {
                stage, io_detail, ..
            } => format!(
                "MCP process read failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Write {
                stage, io_detail, ..
            } => format!(
                "MCP process write failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Protocol {
                stage,
                protocol_detail,
                ..
            } => format!(
                "MCP protocol failed during {}: {}",
                stage.as_str(),
                protocol_detail.text
            ),
            Self::Wait {
                stage, io_detail, ..
            } => format!(
                "MCP process wait failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Cleanup {
                stage, io_detail, ..
            } => format!(
                "MCP process cleanup failed during {}: {}",
                stage.as_str(),
                io_detail.text
            ),
            Self::Shutdown { exit_code, .. } => format!(
                "MCP process shutdown failed with exit code {}",
                exit_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unavailable".to_owned())
            ),
        }
    }

    pub(in crate::connection_command) fn to_json(&self) -> Value {
        let mut failure = serde_json::Map::new();
        failure.insert("kind".to_owned(), Value::String(self.kind().to_owned()));
        failure.insert(
            "stage".to_owned(),
            Value::String(self.stage().as_str().to_owned()),
        );
        match self {
            Self::Spawn { io_detail, .. } => {
                failure.insert("io_detail".to_owned(), io_detail.to_json());
            }
            Self::ExitedBeforeResponse {
                exit_code, stderr, ..
            }
            | Self::Shutdown {
                exit_code, stderr, ..
            } => {
                failure.insert(
                    "exit_code".to_owned(),
                    exit_code.map_or(Value::Null, |code| Value::from(i64::from(code))),
                );
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
            Self::Timeout {
                timeout, stderr, ..
            } => {
                failure.insert(
                    "timeout_ms".to_owned(),
                    Value::from(timeout.as_millis() as u64),
                );
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
            Self::Read {
                io_detail, stderr, ..
            }
            | Self::Write {
                io_detail, stderr, ..
            }
            | Self::Wait {
                io_detail, stderr, ..
            }
            | Self::Cleanup {
                io_detail, stderr, ..
            } => {
                failure.insert("io_detail".to_owned(), io_detail.to_json());
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
            Self::Protocol {
                protocol_detail,
                missing_tools,
                stderr,
                ..
            } => {
                failure.insert("protocol_detail".to_owned(), protocol_detail.to_json());
                if !missing_tools.is_empty() {
                    failure.insert("missing_tools".to_owned(), json!(missing_tools));
                }
                failure.insert("stderr".to_owned(), stderr.to_json());
            }
        }
        Value::Object(failure)
    }

    const fn kind(&self) -> &'static str {
        match self {
            Self::Spawn { .. } => "spawn",
            Self::ExitedBeforeResponse { .. } => "exited_before_response",
            Self::Timeout { .. } => "timeout",
            Self::Read { .. } => "read",
            Self::Write { .. } => "write",
            Self::Protocol { .. } => "protocol",
            Self::Wait { .. } => "wait",
            Self::Cleanup { .. } => "cleanup",
            Self::Shutdown { .. } => "shutdown",
        }
    }
}

pub(super) fn bounded_protocol_detail(detail: impl AsRef<str>) -> BoundedText {
    BoundedText::from_utf8(detail, MAX_PROTOCOL_DETAIL_BYTES, "protocol detail")
}

pub(super) fn bounded_io_detail(error: impl std::fmt::Display) -> BoundedText {
    bounded_io_text(error.to_string())
}

pub(super) fn bounded_io_text(detail: impl AsRef<str>) -> BoundedText {
    BoundedText::from_utf8(detail, MAX_IO_DETAIL_BYTES, "I/O detail")
}
