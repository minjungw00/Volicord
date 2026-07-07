use std::{error::Error, process::Output};

use serde_json::Value;

use super::binary_fixture::CapturedChildOutput;

pub(crate) fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        stdout(output),
        stderr(output)
    );
}

pub(crate) fn assert_success_captured(output: &CapturedChildOutput) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        captured_stdout(output),
        captured_stderr(output)
    );
}

pub(crate) fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub(crate) fn captured_stdout(output: &CapturedChildOutput) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn captured_stderr(output: &CapturedChildOutput) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub(crate) fn json_stdout(output: &Output) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&stdout(output))?)
}

pub(crate) fn assert_non_guarantees(disclosure: &Value, expected: &[&str]) {
    let values = disclosure["non_guarantees"]
        .as_array()
        .expect("disclosure should include non_guarantees");
    for expected_value in expected {
        assert!(
            values
                .iter()
                .any(|value| value.as_str() == Some(expected_value)),
            "missing non-guarantee {expected_value}: {disclosure}"
        );
    }
}

pub(crate) fn assert_report_line(report: &str, expected: &str) {
    assert!(
        report.lines().any(|line| line == expected),
        "missing report line `{expected}` in:\n{report}"
    );
}

pub(crate) fn assert_report_line_names(report: &str, expected: &[&str]) {
    let actual = report
        .lines()
        .map(|line| {
            let separator = line
                .find(':')
                .unwrap_or_else(|| panic!("report line missing `:` separator: {line}"));
            &line[..=separator]
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "unexpected preflight report line names");
}

pub(crate) fn assert_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().any(|candidate| candidate == code),
        "expected close blocker code {code}, got {codes:?}"
    );
}

pub(crate) fn assert_no_close_blocker(response_value: &Value, code: &str) {
    let codes = close_blocker_codes(response_value);
    assert!(
        codes.iter().all(|candidate| candidate != code),
        "did not expect close blocker code {code}, got {codes:?}"
    );
}

pub(crate) fn close_blocker_codes(response_value: &Value) -> Vec<String> {
    response_value
        .get("blockers")
        .or_else(|| response_value.get("close_blockers"))
        .and_then(Value::as_array)
        .expect("blockers or close_blockers should be present")
        .iter()
        .filter_map(|blocker| blocker["code"].as_str().map(str::to_owned))
        .collect()
}
