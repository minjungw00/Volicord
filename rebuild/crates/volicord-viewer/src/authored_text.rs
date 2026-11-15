//! Presentation of explicit authored examples and comparison tables. Original
//! unit text, claim offsets, citations and seals remain unchanged. This is not a
//! Markdown/HTML interpreter for Sources, canonical reports or arbitrary links.
use crate::render::escape;

/// A complete fenced example or pipe table may occupy one grounded reader unit.
/// Malformed/mixed notation falls back to the complete ordinary escaped text.
pub(crate) fn block(text: &str, korean: bool) -> Option<String> {
    block_selection(text, None, korean, &[])
}

pub(crate) fn with_actions(
    text: &str,
    korean: bool,
    actions: &[(usize, usize, String)],
) -> Option<String> {
    if actions.is_empty() {
        block(text, korean)
    } else {
        block_selection(text, None, korean, actions)
    }
}

/// The full admitted unit supplies context while its exact cited claim selects
/// table rows. Never reconstruct a table from a partial Markdown substring.
pub(crate) fn selected(text: &str, start: usize, end: usize, korean: bool) -> String {
    block_selection(text, Some((start, end)), korean, &[])
        .unwrap_or_else(|| inline(text.get(start..end).unwrap_or("")))
}

fn block_selection(
    text: &str,
    selection: Option<(usize, usize)>,
    korean: bool,
    actions: &[(usize, usize, String)],
) -> Option<String> {
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
        html.push_str(&format!("<th scope=\"col\">{}</th>", inline(cell)));
    }
    if !actions.is_empty() {
        html.push_str(if korean {
            "<th scope=\"col\">근거</th>"
        } else {
            "<th scope=\"col\">Evidence</th>"
        });
    }
    html.push_str("</tr></thead><tbody>");
    let mut offset = text.len() - text.trim_start().len();
    let original_lines: Vec<_> = text.trim().split_inclusive('\n').collect();
    for (index, row) in rows.iter().enumerate() {
        let end = offset + original_lines[index].trim_end_matches(['\r', '\n']).len();
        if index < 2 {
            offset += original_lines[index].len();
            continue;
        }
        let selected = selection.is_some_and(|(start, stop)| {
            (start != 0 || stop != text.len()) && start < end && stop > offset
        });
        html.push_str(if selected {
            "<tr class=\"claim-selected\" aria-current=\"true\">"
        } else {
            "<tr>"
        });
        for cell in row {
            html.push_str(&format!("<td>{}</td>", inline(cell)));
        }
        if !actions.is_empty() {
            html.push_str("<td class=\"table-evidence-actions\">");
            for (_, _, action) in actions
                .iter()
                .filter(|(start, stop, _)| *start < end && *stop > offset)
            {
                html.push_str(action);
            }
            html.push_str("</td>");
        }
        html.push_str("</tr>");
        offset += original_lines[index].len();
    }
    html.push_str("</tbody></table></div>");
    Some(html)
}

/// Only explicit same-Project Work references are navigation. Everything else,
/// including HTTP/HTML/JavaScript and malformed references, stays literal text.
/// Resolution is a final presentation pass, never a canonical relation/write.
pub(crate) fn inline(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        out.push_str(&escape(&rest[..start]));
        rest = &rest[start..];
        if let Some((label, tail)) = rest[1..].split_once("](work:") {
            if !label.is_empty() && !label.contains(['[', ']', '\n', '\r']) {
                if let Some(id) = tail.get(..32).filter(|id| {
                    id.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                }) {
                    if let Some(tail) = tail[32..].strip_prefix(')') {
                        out.push_str(&format!(
                            "<span data-authored-work=\"{id}\">{}</span>",
                            escape(label)
                        ));
                        rest = tail;
                        continue;
                    }
                }
            }
        }
        out.push('[');
        rest = &rest[1..];
    }
    out.push_str(&escape(rest));
    out
}

pub(crate) fn resolve_work_links(
    html: &str,
    request: &crate::ViewerRequest,
    snapshot: bool,
) -> String {
    const MARKER: &str = "<span data-authored-work=\"";
    let targets = html
        .match_indices("id=\"work-")
        .filter_map(|(start, _)| {
            let tail = &html[start + 9..];
            tail.get(..32)
                .filter(|_| tail.as_bytes().get(32) == Some(&b'"'))
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find(MARKER) {
        out.push_str(&rest[..start]);
        let tail = &rest[start + MARKER.len()..];
        let Some((id, tail)) = tail.split_once("\">") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let Some((label, tail)) = tail.split_once("</span>") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let known = targets.contains(id);
        if snapshot && !known {
            let note = if request.locale == crate::ViewerLocale::Korean {
                "이 스냅샷에 작업 미포함"
            } else {
                "Work not included in this snapshot"
            };
            out.push_str(&format!(
                "<span class=\"unavailable-work-reference\">{label} · {note}</span>"
            ));
        } else if let Some(work) = work_id(id) {
            let target = if snapshot {
                format!("#work-{id}")
            } else {
                crate::render::reader_url(request, &crate::ViewerView::Work { work: Some(work) })
            };
            out.push_str(&format!("<a class=\"authored-work-link\" data-work-reference=\"{id}\" href=\"{}\">{label}</a>",escape(&target)));
        } else {
            out.push_str(label);
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

fn work_id(id: &str) -> Option<volicord_context::ContextItemId> {
    if id.len() != 32 {
        return None;
    }
    let mut bytes = [0; 16];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(id.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(volicord_context::ContextItemId::from_bytes(bytes))
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
    fn row_selection_uses_exact_utf8_coordinates_including_crlf() {
        let text = "| 이름 | 값 |\r\n| --- | --- |\r\n| 첫째 | A |\r\n| 둘째 | B |\r\n";
        let start = text.find("| 둘째").expect("row");
        let rendered = selected(text, start, text.len(), true);
        assert_eq!(rendered.matches("class=\"claim-selected\"").count(), 1);
        assert!(rendered.contains("aria-current=\"true\"><td>둘째</td>"));
        assert!(rendered.contains("<td>첫째</td>"));
    }
    #[test]
    fn only_explicit_internal_work_links_are_resolved_without_dead_native_fragments() {
        let id = "00000000000000000000000000000001";
        let request = crate::ViewerRequest {
            project_id: volicord_context::ProjectId::from_bytes([1; 16]),
            locale: crate::ViewerLocale::English,
            requested_language: "fr".into(),
            view: crate::ViewerView::Overview,
            guarded_request: None,
        };
        let prose=inline(&format!("Read [Current <title>](work:{id}); [unsafe](javascript:alert(1)); [external](https://example.com)."));
        let live = resolve_work_links(&prose, &request, false);
        assert!(live.contains("Current &lt;title&gt;</a>"));
        assert!(live.contains(&format!("work={id}&amp;project={}", request.project_id)));
        assert!(!live.contains("href=\"javascript:"));
        assert!(!live.contains("href=\"https:"));
        let missing = resolve_work_links(&prose, &request, true);
        assert!(!missing.contains("href=\"#work-"));
        assert!(missing.contains("Work not included in this snapshot"));
        let native = resolve_work_links(
            &format!("<section id=\"work-{id}\"></section>{prose}"),
            &request,
            true,
        );
        assert!(native.contains(&format!("href=\"#work-{id}\"")));
        let fenced = block(&format!("```sh\n[Literal](work:{id})\n```"), false).expect("fence");
        assert!(!fenced.contains("data-authored-work"));
        for text in [
            "[bad](work:x)",
            "[bad](work:0000000000000000000000000000000g)",
            "[bad](work:00000000000000000000000000000001?project=foreign)",
        ] {
            assert!(!inline(text).contains("data-authored-work"));
        }
    }
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
