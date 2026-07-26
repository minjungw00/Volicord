//! Bounded line-delimited stdio transport and framing.
//!
//! This module knows byte limits, UTF-8, newline framing, and output writes. It
//! delegates JSON syntax and all lifecycle/tool behavior to their owners.

use crate::adapter::McpAdapter;
use crate::diagnostics::{JsonRpcDiagnostic, McpDiagnostic};
use crate::errors::McpAdapterError;
use crate::json_rpc::{decode_json, invalid_request_response, json_rpc_error};
use crate::lifecycle::{
    close_session, handle_json_rpc_message, start_session, terminate_session, SessionStart,
    SessionState,
};
use crate::telemetry::{
    authoritative_observation_timestamp, record_current_session_finding_with_admission,
};
use serde_json::Value;
use std::io::{BufRead, Write};
use volicord_store::managed_launch_leases::ManagedMcpLaunchLeaseConsumption;
use volicord_types::integration_revision::McpRuntimeSessionSource;

pub(crate) const MAX_MCP_REQUEST_LINE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StdioRunOptions {
    pub(crate) session_source: McpRuntimeSessionSource,
    pub(crate) managed_lease: Option<ManagedMcpLaunchLeaseConsumption>,
    pub(crate) observed_host_executable_version: Option<String>,
}

impl Default for StdioRunOptions {
    fn default() -> Self {
        Self {
            session_source: McpRuntimeSessionSource::ManualCli,
            managed_lease: None,
            observed_host_executable_version: None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum BoundedJsonLine {
    Eof,
    Line(String),
    InvalidUtf8,
    TooLong,
    Incomplete,
}

pub(crate) fn run_stdio_transport<R, W>(
    adapter: McpAdapter,
    mut reader: R,
    mut writer: W,
    options: StdioRunOptions,
) -> Result<(), McpAdapterError>
where
    R: BufRead,
    W: Write,
{
    let mut state = Some(start_session(
        &adapter,
        SessionStart {
            session_source: options.session_source,
            managed_lease: options.managed_lease,
            observed_host_executable_version: options.observed_host_executable_version,
            process_started_at: authoritative_observation_timestamp(),
        },
    )?);

    let transport_result = (|| -> Result<(), McpAdapterError> {
        loop {
            let line = match read_bounded_json_line(&mut reader)? {
                BoundedJsonLine::Eof => break,
                BoundedJsonLine::Line(line) => line,
                BoundedJsonLine::InvalidUtf8 => {
                    record_current_session_finding_with_admission(
                        &adapter,
                        state.as_mut().expect("active session state").runtime_mut(),
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::ParseError),
                        Some(-32700),
                        Some("input was not valid UTF-8".to_owned()),
                        None,
                        Vec::new(),
                        false,
                    )?;
                    write_json_line(
                        &mut writer,
                        json_rpc_error(Value::Null, -32700, "Parse error", None),
                    )?;
                    continue;
                }
                BoundedJsonLine::TooLong => {
                    record_current_session_finding_with_admission(
                        &adapter,
                        state.as_mut().expect("active session state").runtime_mut(),
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::MessageSizeExceeded),
                        Some(-32600),
                        Some(format!(
                            "request exceeded the {MAX_MCP_REQUEST_LINE_BYTES}-byte limit"
                        )),
                        None,
                        Vec::new(),
                        false,
                    )?;
                    write_json_line(
                        &mut writer,
                        invalid_request_response(&Value::Null, "request message is too large"),
                    )?;
                    continue;
                }
                BoundedJsonLine::Incomplete => {
                    record_current_session_finding_with_admission(
                        &adapter,
                        state.as_mut().expect("active session state").runtime_mut(),
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::FramingFailure),
                        Some(-32600),
                        Some("request ended without newline framing".to_owned()),
                        None,
                        Vec::new(),
                        true,
                    )?;
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }

            let message = match decode_json(&line) {
                Ok(message) => message,
                Err(error) => {
                    record_current_session_finding_with_admission(
                        &adapter,
                        state.as_mut().expect("active session state").runtime_mut(),
                        McpDiagnostic::JsonRpc(JsonRpcDiagnostic::ParseError),
                        Some(-32700),
                        Some(error.detail),
                        None,
                        Vec::new(),
                        false,
                    )?;
                    write_json_line(
                        &mut writer,
                        json_rpc_error(Value::Null, -32700, "Parse error", None),
                    )?;
                    continue;
                }
            };

            let current_state = state.take().expect("active session state");
            match handle_json_rpc_message(&adapter, current_state, message) {
                Ok(transition) => {
                    state = Some(transition.state);
                    if let Some(response) = transition.output {
                        write_json_line(&mut writer, response)?;
                    }
                }
                Err(failure) => {
                    state = Some(*failure.state);
                    return Err(failure.error);
                }
            }
        }
        writer.flush().map_err(McpAdapterError::Io)
    })();

    let state = state.expect("transport retained session state");
    match transport_result {
        Ok(()) => close_session(&adapter, state)
            .map(|closed| {
                debug_assert!(matches!(closed, SessionState::Closed(_)));
            })
            .map_err(|failure| failure.error),
        Err(error) => {
            let _closed = terminate_session(&adapter, state, &error);
            Err(error)
        }
    }
}

fn read_bounded_json_line(reader: &mut impl BufRead) -> Result<BoundedJsonLine, McpAdapterError> {
    let mut bytes = Vec::with_capacity(8 * 1024);
    let mut exceeded = false;
    loop {
        let available = reader.fill_buf().map_err(McpAdapterError::Io)?;
        if available.is_empty() {
            return if bytes.is_empty() && !exceeded {
                Ok(BoundedJsonLine::Eof)
            } else {
                Ok(BoundedJsonLine::Incomplete)
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let consumed = newline.map_or(available.len(), |index| index + 1);
        let content_end = newline.unwrap_or(available.len());
        if !exceeded {
            let remaining = MAX_MCP_REQUEST_LINE_BYTES.saturating_sub(bytes.len());
            let retained = remaining.min(content_end);
            bytes.extend_from_slice(&available[..retained]);
            exceeded = retained < content_end;
        }
        reader.consume(consumed);
        if newline.is_some() {
            if exceeded {
                return Ok(BoundedJsonLine::TooLong);
            }
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
            return String::from_utf8(bytes)
                .map(BoundedJsonLine::Line)
                .or(Ok(BoundedJsonLine::InvalidUtf8));
        }
    }
}

pub(crate) fn write_json_line(
    writer: &mut impl Write,
    value: Value,
) -> Result<(), McpAdapterError> {
    serde_json::to_writer(&mut *writer, &value).map_err(McpAdapterError::Json)?;
    writer.write_all(b"\n").map_err(McpAdapterError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn framing_accepts_lf_and_crlf_without_exposing_delimiters() {
        let mut reader = BufReader::new(Cursor::new(b"{}\n[]\r\n".to_vec()));
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("first frame"),
            BoundedJsonLine::Line("{}".to_owned())
        );
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("second frame"),
            BoundedJsonLine::Line("[]".to_owned())
        );
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("EOF"),
            BoundedJsonLine::Eof
        );
    }

    #[test]
    fn framing_rejects_oversized_invalid_utf8_and_incomplete_lines() {
        let mut oversized = vec![b'x'; MAX_MCP_REQUEST_LINE_BYTES + 1];
        oversized.push(b'\n');
        let mut reader = BufReader::new(Cursor::new(oversized));
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("oversized frame"),
            BoundedJsonLine::TooLong
        );

        let mut reader = BufReader::new(Cursor::new(vec![0xff, b'\n']));
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("invalid UTF-8 frame"),
            BoundedJsonLine::InvalidUtf8
        );

        let mut reader = BufReader::new(Cursor::new(b"{}".to_vec()));
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("incomplete frame"),
            BoundedJsonLine::Incomplete
        );
    }

    #[test]
    fn framing_drains_an_oversized_line_before_reading_the_next_message() {
        let mut input = vec![b'x'; MAX_MCP_REQUEST_LINE_BYTES + 8];
        input.extend_from_slice(b"\n{}\n");
        let mut reader = BufReader::new(Cursor::new(input));
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("oversized frame"),
            BoundedJsonLine::TooLong
        );
        assert_eq!(
            read_bounded_json_line(&mut reader).expect("following frame"),
            BoundedJsonLine::Line("{}".to_owned())
        );
    }
}
