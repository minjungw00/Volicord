//! Presentation of explicit authored examples and comparison tables. Original
//! unit text, claim offsets, citations and seals remain unchanged. This is not a
//! Markdown/HTML interpreter for Sources, canonical reports or arbitrary links.
use crate::render::escape;

/// A complete fenced example or pipe table may occupy one grounded reader unit.
/// Malformed/mixed notation falls back to the complete ordinary escaped text.
pub(crate) fn block(text: &str, korean: bool) -> Option<String> {
    let lines: Vec<_> = text.trim().lines().collect();
    if let Some(language) = lines.first()?.strip_prefix("```") {
        if lines.len() < 3
            || lines.last()?.trim() != "```"
            || !language
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
            || lines[1..lines.len() - 1]
                .iter()
                .any(|line| line.trim() == "```")
        {
            return None;
        }
        return Some(format!(
            "<pre class=\"authored-example\" tabindex=\"0\"><code data-example-language=\"{}\">{}</code></pre>",
            escape(language), escape(&lines[1..lines.len() - 1].join("\n"))
        ));
    }
    if lines.len() < 3 {
        return None;
    }
    let rows: Vec<Vec<String>> = lines
        .iter()
        .map(|line| cells(line))
        .collect::<Option<_>>()?;
    let columns = rows.first()?.len();
    if columns < 2
        || rows.iter().any(|row| row.len() != columns)
        || !rows[1].iter().all(|cell| {
            let separator = cell.trim_matches(':');
            separator.len() >= 3 && separator.bytes().all(|c| c == b'-')
        })
    {
        return None;
    }
    let label = if korean {
        "설명 비교표"
    } else {
        "Explanation comparison table"
    };
    let mut html = format!("<div class=\"authored-table-scroll\" role=\"region\" aria-label=\"{label}\" tabindex=\"0\"><table class=\"authored-table\"><thead><tr>");
    for cell in &rows[0] {
        html.push_str(&format!("<th scope=\"col\">{}</th>", escape(cell)));
    }
    html.push_str("</tr></thead><tbody>");
    for row in &rows[2..] {
        html.push_str("<tr>");
        for cell in row {
            html.push_str(&format!("<td>{}</td>", escape(cell)));
        }
        html.push_str("</tr>");
    }
    html.push_str("</tbody></table></div>");
    Some(html)
}

fn cells(line: &str) -> Option<Vec<String>> {
    let line = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                cell.push('|');
            }
            '|' => {
                cells.push(cell.trim().into());
                cell.clear();
            }
            _ => cell.push(c),
        }
    }
    cells.push(cell.trim().into());
    Some(cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_examples_escape_code_without_interpreting_or_executing_it() {
        let output = block(
            "```json\n{\"unset\": null, \"empty\": \"\", \"html\": \"<script>\"}\n```",
            false,
        )
        .expect("example");
        assert!(output.contains("data-example-language=\"json\""));
        assert!(output.contains("&lt;script&gt;"));
        assert!(!output.contains("<script>"));
        assert!(output.contains("null"));
    }
    #[test]
    fn tables_keep_multilingual_cells_and_escaped_pipes_without_active_markup() {
        let output = block("| 입력 | 결과 |\n| :--- | ---: |\n| A \\| B | <img src=x onerror=alert(1)> |\n| λ | 중단 |", true).expect("table");
        assert!(output.contains("<th scope=\"col\">입력</th>"));
        assert!(output.contains("<td>A | B</td>"));
        assert!(output.contains("<td>λ</td>"));
        assert!(!output.contains("<img"));
    }
    #[test]
    fn incomplete_or_mixed_formats_fall_back_without_losing_text() {
        for input in [
            "plain prose",
            "```sh\nmissing end",
            "```<script>\na\n```",
            "| a | b |\n| --- | --- |\n| missing |",
            "Prose\n| a | b |\n| --- | --- |\n| c | d |",
            "```\na\n```\nother\n```",
        ] {
            assert!(block(input, false).is_none(), "{input}");
        }
    }
}
