//! Bounded, dependency-free local SVG. No generated markup, fonts, assets or I/O.
use volicord_projections::*;
#[path = "diagram_overview.rs"]
mod overview;

pub(crate) const RENDERER_ID: &str = "volicord-static-svg/13";
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn measure_text(s: &str) -> usize {
    s.chars()
        .map(|ch| {
            if ch == '\u{2028}' {
                0
            } else if !ch.is_ascii() || "MWmw@%&#".contains(ch) {
                18
            } else if " iljft.,:;'!|()[]".contains(ch) {
                6
            } else if ch.is_ascii_uppercase() {
                13
            } else {
                12
            }
        })
        .sum()
}
fn words(s: &str, limit: usize) -> Vec<String> {
    // Reserve native 16px semibold glyph width. Every word must fit its own
    // text rectangle, including narrow participant headers; using the box's
    // padding as overflow space lets its border cut through the name.
    let mut result = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for token in s.split_inclusive(char::is_whitespace) {
        let size = measure_text(token);
        if used > 0 && used + size > limit {
            result.push(std::mem::take(&mut line));
            used = 0;
        }
        if size <= limit {
            line.push_str(token);
            used += size;
            // U+2028 is an authored printable line separator, already sealed
            // with the label. Never infer a name/translation from its text.
            if token.ends_with('\u{2028}') {
                result.push(std::mem::take(&mut line));
                used = 0;
            }
            continue;
        }
        for ch in token.chars() {
            let size = measure_text(&ch.to_string());
            if used + size > limit && !line.is_empty() {
                result.push(std::mem::take(&mut line));
                used = 0;
            }
            line.push(ch);
            used += size;
            if ch == '\u{2028}' {
                result.push(std::mem::take(&mut line));
                used = 0;
            }
        }
    }
    if !line.is_empty() {
        result.push(line)
    }
    result
}

// Conservative local advances at the actual role size. Export uses the same
// bounded metrics as live output; browser conformance measures native glyphs.
fn words_at(s: &str, width: usize, size: usize) -> Vec<String> {
    words(s, width.saturating_mul(16) / size)
}
fn measure_at(s: &str, size: usize) -> usize {
    measure_text(s).saturating_mul(size).div_ceil(16)
}

fn node_text(out: &mut String, x: usize, y: usize, value: &str, width: usize) {
    out.push_str(&format!(
        "<text class=\"diagram-node-label\" x=\"{x}\" y=\"{y}\" text-anchor=\"middle\">"
    ));
    for (i, line) in words(value, width).iter().enumerate() {
        out.push_str(&format!(
            "<tspan x=\"{x}\" dy=\"{}\">{}</tspan>",
            if i == 0 { 0 } else { 24 },
            esc(line)
        ));
    }
    out.push_str("</text>");
}

fn node_header(
    out: &mut String,
    x: usize,
    y: usize,
    node: &DiagramNode,
    texts: &[&str],
    width: usize,
) -> Result<(), String> {
    let name = diagram_label(&node.label, texts)?;
    node_text(out, x, y, name, width);
    Ok(())
}
fn identifier_text(out: &mut String, x: usize, y: usize, value: &str, width: usize) {
    out.push_str("<g class=\"diagram-label-support\">");
    sized_text(out, x, y, value, width, 12, 18);
    out.push_str("</g>");
}

fn sized_text(
    out: &mut String,
    x: usize,
    y: usize,
    value: &str,
    width: usize,
    size: usize,
    leading: usize,
) {
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\">"
    ));
    for (i, line) in words_at(value, width, size).iter().enumerate() {
        out.push_str(&format!(
            "<tspan x=\"{x}\" dy=\"{}\">{}</tspan>",
            if i == 0 { 0 } else { leading },
            esc(line)
        ));
    }
    out.push_str("</text>");
}

fn open(
    out: &mut String,
    ns: &str,
    id: &str,
    label: &DiagramLabel,
    name: &str,
    target: &dyn Fn(&str, &DiagramLabel) -> String,
) {
    out.push_str(&format!("<a id=\"{}-{}\" href=\"#{}\" class=\"diagram-element\" aria-label=\"{}\"><title>{}</title>",esc(ns),esc(id),esc(&target(id,label)),esc(name),esc(name)));
}
/// SVG and the alternative read the same labels/endpoints/order. IDs are scoped
/// to the exact publication; there is no author-controlled resource reference.
pub(crate) fn svg(
    d: &GroundedDiagram,
    texts: &[&str],
    ns: &str,
    target: &dyn Fn(&str, &DiagramLabel) -> String,
    locale: FixedLocale,
) -> Result<String, String> {
    bounded_input(d, texts, ns)?;
    overview::svg(d, texts, ns, target, locale)
}

fn bounded_input(d: &GroundedDiagram, texts: &[&str], ns: &str) -> Result<(), String> {
    if ns.is_empty()
        || ns.len() > 256
        || !ns
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err("unsupported local SVG namespace".into());
    }
    if d.nodes().is_empty() || d.nodes().len() > 12 || texts.len() > 32 {
        return Err("unsupported local SVG input budget".into());
    }
    if texts.iter().any(|s| {
        s.chars()
            .take(EXPLANATION_PARAGRAPH_CHARACTER_LIMIT + 1)
            .count()
            > EXPLANATION_PARAGRAPH_CHARACTER_LIMIT
    }) {
        return Err("unsupported local SVG claim budget".into());
    }
    match &d.meaning {
        DiagramMeaning::Sequence { groups, .. } => {
            if groups.is_empty()
                || groups.len() > 12
                || groups
                    .iter()
                    .any(|g| g.messages.is_empty() || g.messages.len() > 20)
                || groups.iter().map(|g| g.messages.len()).sum::<usize>() > 20
            {
                return Err("unsupported local SVG sequence budget".into());
            }
        }
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            if relations.is_empty() || relations.len() > 20 {
                return Err("unsupported local SVG relationship budget".into());
            }
        }
    }
    for label in d.labels() {
        let text = diagram_label(label, texts)?;
        if text.len() > 384
            || text.trim().is_empty()
            || text.chars().count() > 96
            || text.chars().any(char::is_control)
        {
            return Err("unsupported local SVG label".into());
        }
    }
    // Validate borrowed tokens before elements() can clone/format identities.
    let token = |id: &str| {
        if id.is_empty()
            || id.len() > 48
            || !id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            Err("unsupported local SVG element identity".to_owned())
        } else {
            Ok(())
        }
    };
    token(&d.id)?;
    for n in d.nodes() {
        token(&n.id)?;
    }
    match &d.meaning {
        DiagramMeaning::Sequence { groups, .. } => {
            for m in groups.iter().flat_map(|g| &g.messages) {
                token(&m.id)?;
                token(&m.from)?;
                token(&m.to)?;
            }
        }
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            for edge in relations {
                token(&edge.id)?;
                token(&edge.from)?;
                token(&edge.to)?;
            }
        }
    }
    Ok(())
}

/// Read-side identities for every selectable semantic element, including guards.
/// ':' separates renderer-owned guard IDs from admitted ASCII element tokens.
pub(crate) fn elements(d: &GroundedDiagram) -> Vec<(String, &DiagramLabel)> {
    let mut result = d
        .nodes()
        .iter()
        .map(|n| (n.id.clone(), &n.label))
        .collect::<Vec<_>>();
    for node in d.nodes() {
        if let Some(identifier) = &node.identifier {
            result.push((format!("{}:identifier", node.id), identifier));
        }
    }
    if let DiagramMeaning::State { owner, .. } = &d.meaning {
        result.push(("state:owner".into(), owner));
    }
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            for r in relations {
                result.push((r.id.clone(), &r.label));
                if let Some(l) = &r.condition {
                    result.push((format!("{}:condition", r.id), l));
                }
            }
        }
        DiagramMeaning::Sequence { groups, .. } => {
            for (i, g) in groups.iter().enumerate() {
                if let Some(l) = &g.condition {
                    result.push((format!("guard:{}", i + 1), l));
                }
                result.extend(g.messages.iter().map(|m| (m.id.clone(), &m.label)));
            }
        }
    }
    result
}
pub(crate) fn element_caption<'a>(
    d: &'a GroundedDiagram,
    id: &str,
    texts: &[&str],
    locale: FixedLocale,
) -> String {
    let label = |l: &'a DiagramLabel| diagram_label(l, texts).unwrap_or("");
    let node = |id: &str| {
        let name = d
            .nodes()
            .iter()
            .find(|n| n.id == id)
            .and_then(|n| diagram_node_identity(n, texts).ok())
            .unwrap_or_default();
        name
    };
    let owner = if let DiagramMeaning::State { owner, .. } = &d.meaning {
        format!(
            " · {} {}",
            diagram_ui(locale, "State owner:", "상태 소유자:"),
            label(owner)
        )
    } else {
        String::new()
    };
    if id == "state:owner" {
        if let DiagramMeaning::State { owner, .. } = &d.meaning {
            return format!(
                "{} {}",
                diagram_ui(locale, "State owner:", "상태 소유자:"),
                label(owner)
            );
        }
    }
    if let Some(n) = d.nodes().iter().find(|n| n.id == id) {
        return format!("{} · {}{owner}", node(&n.id), n.kind.localized_text(locale));
    }
    if let Some(n) = d
        .nodes()
        .iter()
        .find(|n| format!("{}:identifier", n.id) == id)
    {
        return format!(
            "{} · {}",
            node(&n.id),
            diagram_ui(locale, "Identifier", "식별자")
        );
    }
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            for r in relations {
                if r.id == id {
                    return format!(
                        "{} → {} · {}{owner}",
                        node(&r.from),
                        node(&r.to),
                        r.kind.localized_text(locale)
                    );
                }
            }
        }
        DiagramMeaning::Sequence { groups, .. } => {
            for (i, g) in groups.iter().enumerate() {
                for m in &g.messages {
                    if m.id == id {
                        return format!(
                            "{}. {} → {} · {} · {}{}",
                            i + 1,
                            node(&m.from),
                            node(&m.to),
                            m.kind.localized_text(locale),
                            match g.order {
                                DiagramOrder::Sequential =>
                                    diagram_ui(locale, "strict · sequential", "strict · 순차"),
                                DiagramOrder::Parallel => diagram_ui(
                                    locale,
                                    "par · no relative order",
                                    "par · 상대 순서 없음"
                                ),
                            },
                            g.condition
                                .as_ref()
                                .map(|guard| format!(
                                    " · opt · {} {}",
                                    diagram_ui(locale, "condition:", "조건:"),
                                    label(guard)
                                ))
                                .unwrap_or_default()
                        );
                    }
                }
            }
        }
    }
    elements(d)
        .into_iter()
        .find(|(key, _)| key == id)
        .map(|(_, l)| format!("{} {}", diagram_ui(locale, "if", "조건:"), label(l)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_identifier_uses_supporting_typography_without_prose_or_separator_inference() {
        let mut out = String::new();
        node_text(&mut out, 100, 24, "진행 기록", 300);
        identifier_text(&mut out, 100, 48, "Checkpoint", 300);
        assert!(out.contains("diagram-node-label"));
        assert!(out.contains("<g class=\"diagram-label-support\">"));
        assert!(out.contains(">Checkpoint</tspan>"));
        assert!(!out.contains('\u{2028}'));
        assert_eq!(words("동기화", 43), vec!["동기", "화"]);
    }
    fn svg(
        d: &GroundedDiagram,
        texts: &[&str],
        ns: &str,
        target: &dyn Fn(&str, &DiagramLabel) -> String,
    ) -> Result<String, String> {
        super::svg(d, texts, ns, target, FixedLocale::English)
    }
    #[test]
    fn cyclic_components_route_both_directions_and_parallel_channels() -> Result<(), String> {
        // Source-derived topology control; these are renderer inputs, not an
        // admitted repository explanation or a substitute for public reading.
        let values = [
            "Plugin",
            "Worker",
            "Content",
            "Page",
            "sends a DOM snapshot",
            "augments top-frame state",
            "returns observed state",
            "submits /sync",
            "delivers an ID-bearing command",
            "messages Isolated content runtime",
            "sends playback/volume requests back to the worker",
            "router invokes MAIN-world page code",
            "reports results by ID",
        ];
        let labels = values
            .iter()
            .enumerate()
            .map(|(unit, s)| DiagramLabel {
                text: (*s).to_owned(),
                unit,
                start_byte: 0,
                end_byte: s.len(),
            })
            .collect::<Vec<_>>();
        let ids = ["plugin", "worker", "content", "page"];
        let endpoints = [
            (2, 1),
            (1, 3),
            (3, 1),
            (1, 0),
            (0, 1),
            (1, 2),
            (2, 1),
            (1, 3),
            (1, 0),
        ];
        let d = GroundedDiagram {
            id: "cyclic-components".into(),
            title: labels[0].clone(),
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::Component {
                nodes: ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| DiagramNode {
                        identifier: None,
                        id: (*id).into(),
                        kind: DiagramNodeKind::Component,
                        label: labels[i].clone(),
                    })
                    .collect(),
                relations: endpoints
                    .iter()
                    .enumerate()
                    .map(|(i, (from, to))| DiagramRelation {
                        id: format!("edge-{i}"),
                        from: ids[*from].into(),
                        to: ids[*to].into(),
                        kind: DiagramRelationKind::ControlFlow,
                        label: labels[i + 4].clone(),
                        condition: None,
                    })
                    .collect(),
            },
        };
        let output = svg(&d, &values, "cyclic-component-control", &|id, _| {
            format!("{id}-claim")
        })?;
        assert!(output.len() < 262144);
        let column = |id: &str| -> Result<usize, String> {
            let group = output
                .split_once(&format!("id=\"cyclic-component-control-{id}\""))
                .ok_or("element")?
                .1
                .split("</a>")
                .next()
                .ok_or("element close")?;
            group
                .split_once("<rect ")
                .ok_or("box")?
                .1
                .split_once(" x=\"")
                .and_then(|(_, v)| v.split_once('"'))
                .ok_or("box column")?
                .0
                .parse::<usize>()
                .map_err(|e| e.to_string())
        };
        // Column equality belonged to the replaced annotation-box policy.
        // Every actual source/route keeps its own origin and visible predicate;
        // browser controls independently check endpoints and glyph intersections.
        for id in ids {
            assert!(column(id)? < 760);
        }
        for (i, _) in endpoints.iter().enumerate() {
            assert!(column(&format!("edge-{i}"))? < 760);
            assert!(output.contains(&esc(values[i + 4])));
        }
        assert_eq!(
            output
                .matches("class=\"diagram-route diagram-text-link\"")
                .count(),
            endpoints.len()
        );
        Ok(())
    }
    #[test]
    fn renderer_refuses_excess_work_and_markup_namespaces_before_layout() {
        let label = DiagramLabel {
            text: "value".into(),
            unit: 0,
            start_byte: 0,
            end_byte: 5,
        };
        let mut d = GroundedDiagram {
            id: "bounded".into(),
            title: label.clone(),
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::Sequence {
                participants: vec![DiagramNode {
                    identifier: None,
                    id: "actor".into(),
                    kind: DiagramNodeKind::Participant,
                    label: label.clone(),
                }],
                groups: vec![DiagramGroup {
                    order: DiagramOrder::Sequential,
                    condition: None,
                    messages: vec![DiagramMessage {
                        id: "call".into(),
                        from: "actor".into(),
                        to: "actor".into(),
                        kind: DiagramMessageKind::Call,
                        label,
                    }],
                }],
            },
        };
        if let DiagramMeaning::Sequence { groups, .. } = &mut d.meaning {
            groups[0].messages = (0..21)
                .map(|i| {
                    let mut m = groups[0].messages[0].clone();
                    m.id = format!("call-{i}");
                    m
                })
                .collect();
        }
        let calls = std::cell::Cell::new(0);
        let target = |_: &str, _: &DiagramLabel| {
            calls.set(calls.get() + 1);
            "claim".to_owned()
        };
        assert!(svg(&d, &["value"], "bounded", &target).is_err());
        assert_eq!(
            calls.get(),
            0,
            "over-budget work must not start layout/output"
        );
        if let DiagramMeaning::Sequence { groups, .. } = &mut d.meaning {
            groups[0].messages.truncate(1);
        }
        assert!(svg(
            &d,
            &["value"],
            "x\"><image href=\"https://invalid.example/",
            &target
        )
        .is_err());
        assert_eq!(calls.get(), 0);
        let enormous = "한".repeat(100_000);
        d.title.end_byte = enormous.len();
        assert!(svg(&d, &[&enormous], "bounded", &target).is_err());
        assert_eq!(calls.get(), 0);
        d.title.end_byte = 5;
        if let DiagramMeaning::Sequence { participants, .. } = &mut d.meaning {
            participants[0].id = "a".repeat(100_000);
        }
        assert!(svg(&d, &["value"], "bounded", &target).is_err());
        assert_eq!(calls.get(), 0);
    }
    #[test]
    fn svg_escapes_text_and_scopes_internal_marker_guard_and_element_ids() -> Result<(), String> {
        // Renderer-only hostile-text control, not an admitted semantic publication.
        let text = "<script>alert(1)</script>";
        let label = DiagramLabel {
            text: text.to_owned(),
            unit: 0,
            start_byte: 0,
            end_byte: text.len(),
        };
        let mut d = GroundedDiagram {
            id: "fixture".into(),
            title: label.clone(),
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::Sequence {
                participants: vec![DiagramNode {
                    identifier: None,
                    id: "guard-1".into(),
                    kind: DiagramNodeKind::Participant,
                    label: label.clone(),
                }],
                groups: vec![DiagramGroup {
                    order: DiagramOrder::Parallel,
                    condition: Some(label.clone()),
                    messages: vec![DiagramMessage {
                        id: "arrow".into(),
                        from: "guard-1".into(),
                        to: "guard-1".into(),
                        kind: DiagramMessageKind::Return,
                        label,
                    }],
                }],
            },
        };
        if let DiagramMeaning::Sequence { groups, .. } = &mut d.meaning {
            let mut independent = groups[0].messages[0].clone();
            independent.id = "arrow-two".into();
            groups[0].messages.push(independent);
        }
        let first = svg(&d, &[text], "publication-en", &|_, _| "claim".into())?;
        assert_eq!(
            first
                .matches("class=\"relation-line parallel-operand\"")
                .count(),
            3, // One geometric operand in each responsive layout.
        );
        assert!(first.contains("par · no relative order"));
        let mut sequential = d.clone();
        if let DiagramMeaning::Sequence { groups, .. } = &mut sequential.meaning {
            groups[0].order = DiagramOrder::Sequential;
        }
        assert!(
            !svg(&sequential, &[text], "sequential", &|_, _| "claim".into())?
                .contains("parallel-operand")
        );
        assert!(first.contains("&lt;script&gt;"));
        assert!(!first.contains("<script>"));
        for id in [
            "publication-en:arrow",
            "publication-en-guard:1",
            "publication-en-guard-1",
            "publication-en-arrow",
        ] {
            assert_eq!(first.matches(&format!("id=\"{id}\"")).count(), 1);
        }
        let second = svg(&d, &[text], "publication-ko", &|_, _| "claim".into())?;
        assert!(!second.contains("publication-en"));
        assert!(first.len() < 262144);
        let mut missing = d;
        if let DiagramMeaning::Sequence { participants, .. } = &mut missing.meaning {
            participants.clear();
        }
        assert!(svg(&missing, &[text], "publication-en", &|_, _| "claim".into()).is_err());
        Ok(())
    }
    #[test]
    fn long_korean_state_labels_keep_native_size_and_selectable_owner() -> Result<(), String> {
        // Layout/resource control only; no semantic admission or human observation.
        let name = "가나다라마바사아".repeat(12); // Exactly 96 Unicode scalars.
        let label = DiagramLabel {
            text: name.clone(),
            unit: 0,
            start_byte: 0,
            end_byte: name.len(),
        };
        let d = GroundedDiagram {
            id: "long-label".into(),
            title: label.clone(),
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::State {
                owner: label.clone(),
                nodes: vec![DiagramNode {
                    identifier: None,
                    id: "state".into(),
                    kind: DiagramNodeKind::State,
                    label: label.clone(),
                }],
                relations: vec![DiagramRelation {
                    id: "repeat".into(),
                    from: "state".into(),
                    to: "state".into(),
                    kind: DiagramRelationKind::Transition,
                    label: label.clone(),
                    condition: Some(label),
                }],
            },
        };
        let output = svg(&d, &[&name], "ko-publication", &|id, _| {
            format!("{id}-claim")
        })?;
        assert!(output.contains("id=\"ko-publication-state:owner\""));
        assert!(output.contains(&format!("aria-label=\"{name}\"")));
        assert!(output.contains("<tspan"));
        assert!(output.len() < 262144);
        if let Some(root) = std::env::var_os("VOLICORD_LAYOUT_REVIEW_ROOT") {
            let root = std::path::Path::new(&root);
            std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
            std::fs::write(root.join("long-korean.svg"), &output).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[test]
    fn mixed_script_self_calls_guards_and_repeated_names_keep_complete_labels() -> Result<(), String>
    {
        let root = std::env::var_os("VOLICORD_LAYOUT_REVIEW_ROOT").map(std::path::PathBuf::from);
        if let Some(root) = &root {
            std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        }
        let mut fixtures = Vec::new();
        for (script,value) in [("ko","긴 조건과 반환 경로를 확인하는 설명입니다 ".repeat(6)),("en","Long conditional return paths keep every authored word and repeated participant name exact. ".repeat(2)),("dense-en","p".repeat(96))] {
            let value=value.chars().take(96).collect::<String>();
            let label=DiagramLabel{ text: value.clone(),unit:0,start_byte:0,end_byte:value.len()};
            let nodes=["first","second","third"].iter().map(|id|DiagramNode{ identifier: Some(DiagramLabel{text:(*id).into(),..label.clone()}),id:(*id).into(),kind:DiagramNodeKind::Participant,label:label.clone()}).collect::<Vec<_>>();
            let messages=[("self","first","first",DiagramMessageKind::Call),("cross","first","third",DiagramMessageKind::Await),("last-self","third","third",DiagramMessageKind::Return)].iter().map(|(id,from,to,kind)|DiagramMessage{id:(*id).into(),from:(*from).into(),to:(*to).into(),kind:*kind,label:label.clone()}).collect::<Vec<_>>();
            let sequential_messages=messages.iter().cloned().map(|mut m|{m.id=format!("{}-sequential",m.id);m}).collect();
            let sequence=GroundedDiagram{id:"long-sequence".into(),title:label.clone(),caption_unit:0,semantic_fingerprint:String::new(),meaning:DiagramMeaning::Sequence{participants:nodes,groups:vec![DiagramGroup{order:DiagramOrder::Parallel,condition:Some(label.clone()),messages:messages.clone()},DiagramGroup{order:DiagramOrder::Sequential,condition:None,messages:sequential_messages}]}};
            let state=GroundedDiagram{id:"long-state".into(),title:label.clone(),caption_unit:0,semantic_fingerprint:String::new(),meaning:DiagramMeaning::State{owner:label.clone(),nodes:vec![DiagramNode{ identifier: None,id:"only".into(),kind:DiagramNodeKind::State,label:label.clone()}],relations:vec![DiagramRelation{id:"self".into(),from:"only".into(),to:"only".into(),kind:DiagramRelationKind::Transition,label:label.clone(),condition:Some(label.clone())}]}};
            let flow=GroundedDiagram{id:"long-flow".into(),title:label.clone(),caption_unit:0,semantic_fingerprint:String::new(),meaning:DiagramMeaning::Flowchart{nodes:[("branch",DiagramNodeKind::Branch),("left",DiagramNodeKind::Process),("right",DiagramNodeKind::Process)].iter().map(|(id,kind)|DiagramNode{ identifier: Some(DiagramLabel{text:(*id).into(),..label.clone()}),id:(*id).into(),kind:*kind,label:label.clone()}).collect(),relations:["left","right"].iter().map(|id|DiagramRelation{id:format!("to-{id}"),from:"branch".into(),to:(*id).into(),kind:DiagramRelationKind::ControlFlow,label:label.clone(),condition:Some(label.clone())}).collect()}};
            for (kind,d) in [("sequence",sequence),("state",state),("flow",flow)] {
                for (ui,locale) in [("en",FixedLocale::English),("ko",FixedLocale::Korean)] {
                    let ns=format!("fixture-{kind}-{script}-{ui}");
                    let render_started=std::time::Instant::now();
                    let svg=super::svg(&d,&[&value],&ns,&|id,_|format!("{ns}-{id}-claim"),locale)?;
                    let renderer_elapsed_ns=render_started.elapsed().as_nanos();
                    let svg_utf8_bytes=svg.len();
                    assert!(svg.contains(&format!("aria-label=\"{value}\"")));
                    let file=format!("{ns}.svg");if let Some(root)=&root {std::fs::write(root.join(&file),svg).map_err(|e|e.to_string())?;}
                    fixtures.push(serde_json::json!({"id":ns,"ui_locale":ui,"file":file,"diagram":d,"prose":[{"text":value}],"renderer_elapsed_ns":renderer_elapsed_ns,"svg_utf8_bytes":svg_utf8_bytes,"classification":"renderer stress control; no source or comprehension judgment"}));
                }
            }
        }
        // Independent synthetic task variant: two export workers with equal
        // names, concurrent responses, a self-check and conditional publication.
        // This is a layout control, never a new real Work or recorded execution.
        for (ui, locale, values) in [
            ("en", FixedLocale::English, ["Export coordinator", "Export worker", "Two independent exports", "when both exports have been requested and their original revisions remain available", "returns the complete document with its exact revision", "checks both captured revisions before publishing either result"]),
            ("ko", FixedLocale::Korean, ["내보내기 조정", "내보내기 워커", "독립된 두 내보내기", "두 내보내기를 요청했고 원래 revision을 모두 읽을 수 있는 경우", "정확한 revision과 완전한 문서를 반환합니다", "어느 결과든 발행하기 전에 캡처한 두 revision을 검사합니다"]),
        ] {
            let select = |unit: usize| DiagramLabel { text: values[unit].to_owned(), unit, start_byte: 0, end_byte: values[unit].len() };
            let d = GroundedDiagram {
                id: "independent-exports".into(), title: select(2), caption_unit: 2,
                semantic_fingerprint: String::new(),
                meaning: DiagramMeaning::Sequence {
                    participants: ["coordinator", "first-worker", "second-worker"].iter().enumerate()
                        .map(|(i,id)| DiagramNode { identifier: Some(DiagramLabel{text:(*id).into(),..select(usize::from(i > 0))}), id: (*id).into(), kind: DiagramNodeKind::Participant, label: select(usize::from(i > 0)) }).collect(),
                    groups: vec![
                        DiagramGroup { order: DiagramOrder::Parallel, condition: Some(select(3)),
                            messages: ["first-worker", "second-worker"].iter().enumerate().map(|(i,id)| DiagramMessage {
                                id: format!("response-{i}"), from: (*id).into(), to: "coordinator".into(),
                                kind: DiagramMessageKind::Return, label: select(4),
                            }).collect() },
                        DiagramGroup { order: DiagramOrder::Sequential, condition: None,
                            messages: vec![DiagramMessage { id: "verify".into(), from: "coordinator".into(), to: "coordinator".into(), kind: DiagramMessageKind::Call, label: select(5) }] },
                    ],
                },
            };
            let ns = format!("independent-exports-{ui}");
            let file = format!("{ns}.svg");
            let svg = super::svg(&d, &values, &ns, &|id,_| format!("{id}-claim"), locale)?;
            if let Some(root) = &root { std::fs::write(root.join(&file), &svg).map_err(|e|e.to_string())?; }
            fixtures.push(serde_json::json!({"id":ns,"ui_locale":ui,"file":file,"diagram":d,"prose":values.iter().map(|text|serde_json::json!({"text":text})).collect::<Vec<_>>(),"classification":"independent synthetic export task; no Source or real Work claim"}));
        }
        // Different bounded synthetic topology: twelve components, forward and
        // backward routes, and a larger guarded/parallel participant scene.
        for (kind, count, edge_count) in [("many-components", 12, 20), ("many-participants", 8, 14)]
        {
            let values = [
                "Payload 경로와 revision",
                "조건: exact original revision remains available",
            ];
            let select = |unit: usize| DiagramLabel {
                text: values[unit].to_owned(),
                unit,
                start_byte: 0,
                end_byte: values[unit].len(),
            };
            let nodes = (0..count)
                .map(|i| DiagramNode {
                    identifier: Some(DiagramLabel {
                        text: format!("node-{i}"),
                        ..select(0)
                    }),
                    id: format!("node-{i}"),
                    kind: if kind == "many-components" {
                        DiagramNodeKind::Component
                    } else {
                        DiagramNodeKind::Participant
                    },
                    label: select(0),
                })
                .collect::<Vec<_>>();
            let meaning = if kind == "many-components" {
                DiagramMeaning::Component {
                    nodes,
                    relations: (0..edge_count)
                        .map(|i| DiagramRelation {
                            id: format!("edge-{i}"),
                            from: format!("node-{}", i % count),
                            to: format!(
                                "node-{}",
                                if i < count {
                                    (i + 1) % count
                                } else {
                                    (i + count - 1) % count
                                }
                            ),
                            kind: DiagramRelationKind::DependsOn,
                            label: select(0),
                            condition: (i % 3 == 0).then(|| select(1)),
                        })
                        .collect(),
                }
            } else {
                DiagramMeaning::Sequence {
                    participants: nodes,
                    groups: vec![DiagramGroup {
                        order: DiagramOrder::Parallel,
                        condition: Some(select(1)),
                        messages: (0..edge_count)
                            .map(|i| DiagramMessage {
                                id: format!("message-{i}"),
                                from: format!("node-{}", i % count),
                                to: format!(
                                    "node-{}",
                                    if i % 3 == 0 {
                                        i % count
                                    } else {
                                        (i + 1) % count
                                    }
                                ),
                                kind: if i % 2 == 0 {
                                    DiagramMessageKind::Await
                                } else {
                                    DiagramMessageKind::Return
                                },
                                label: select(0),
                            })
                            .collect(),
                    }],
                }
            };
            let d = GroundedDiagram {
                id: kind.into(),
                title: select(0),
                caption_unit: 0,
                semantic_fingerprint: String::new(),
                meaning,
            };
            let output = super::svg(
                &d,
                &values,
                kind,
                &|id, _| format!("{id}-claim"),
                FixedLocale::Korean,
            )?;
            assert!(output.len() <= 262144);
            for i in 0..count {
                assert!(output.contains(&format!("id=\"{kind}-node-{i}\"")));
            }
            if let Some(root) = &root {
                std::fs::write(root.join(format!("{kind}.svg")), &output)
                    .map_err(|e| e.to_string())?;
            }
            fixtures.push(serde_json::json!({"id":kind,"ui_locale":"ko","file":format!("{kind}.svg"),"diagram":d,"prose":values.iter().map(|text|serde_json::json!({"text":text})).collect::<Vec<_>>(),"classification":"synthetic many-node/participant topology; not Product records or semantic acceptance"}));
        }
        if let Some(root) = &root {
            std::fs::write(
                root.join("fixtures.json"),
                serde_json::to_vec_pretty(&fixtures).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[test]
    #[ignore = "explicit local admitted-publication renderer review"]
    fn admitted_publication_renderer_review() -> Result<(), String> {
        let input = std::env::var_os("VOLICORD_LAYOUT_PUBLICATION")
            .ok_or("VOLICORD_LAYOUT_PUBLICATION is required")?;
        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(input).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let prose = value["prose"].as_array().ok_or("public prose")?;
        let texts = prose
            .iter()
            .map(|p| p["text"].as_str().ok_or("public text"))
            .collect::<Result<Vec<_>, _>>()?;
        for raw in value["diagrams"].as_array().ok_or("public diagrams")? {
            let d: GroundedDiagram =
                serde_json::from_value(raw.clone()).map_err(|e| e.to_string())?;
            let started = std::time::Instant::now();
            let output = super::svg(
                &d,
                &texts,
                "review-publication",
                &|id, _| format!("{id}-claim"),
                FixedLocale::English,
            )
            .map_err(|e| format!("{}: {e}", d.id))?;
            assert!(output.contains("connected-overview"));
            println!(
                "{}",
                serde_json::json!({"diagram":d.id,"renderer":super::RENDERER_ID,"elapsed_ns":started.elapsed().as_nanos(),"svg_utf8_bytes":output.len(),"scope":"all three responsive variants; diagnostic profile; no authority reads"})
            );
        }
        Ok(())
    }
    #[test]
    fn routed_state_edges_do_not_cross_unrelated_nodes_or_overlay_keys() -> Result<(), String> {
        // Independent geometry invariant for the Source-derived request topology:
        // queued -> delivered; each can finish in success, failure or expiry.
        let label = DiagramLabel {
            text: "value".into(),
            unit: 0,
            start_byte: 0,
            end_byte: 5,
        };
        let names = ["q", "d", "ok", "fail", "exp"];
        let edges = [
            ("q", "q"),
            ("q", "q"),
            ("q", "d"),
            ("q", "ok"),
            ("d", "ok"),
            ("q", "fail"),
            ("d", "fail"),
            ("q", "exp"),
            ("d", "exp"),
        ];
        let d = GroundedDiagram {
            id: "states".into(),
            title: label.clone(),
            caption_unit: 0,
            semantic_fingerprint: String::new(),
            meaning: DiagramMeaning::State {
                owner: label.clone(),
                nodes: names
                    .iter()
                    .map(|id| DiagramNode {
                        identifier: Some(DiagramLabel {
                            text: (*id).into(),
                            ..label.clone()
                        }),
                        id: (*id).into(),
                        kind: DiagramNodeKind::State,
                        label: label.clone(),
                    })
                    .collect(),
                relations: edges
                    .iter()
                    .enumerate()
                    .map(|(i, (from, to))| DiagramRelation {
                        id: format!("e{i}"),
                        from: (*from).into(),
                        to: (*to).into(),
                        kind: DiagramRelationKind::Transition,
                        label: label.clone(),
                        condition: (i > 2).then(|| label.clone()),
                    })
                    .collect(),
            },
        };
        let output = svg(&d, &["state"], "geometry", &|id, _| format!("{id}-claim"))?;
        assert_eq!(output.matches("All nodes: state").count(), 3);
        assert_eq!(output.matches("All relations: transition").count(), 3);
        fn attribute<'a>(s: &'a str, key: &str) -> Result<&'a str, String> {
            s.split_once(&format!("{key}=\""))
                .and_then(|(_, v)| v.split_once('"'))
                .map(|(v, _)| v)
                .ok_or_else(|| format!("missing {key}"))
        }
        fn rect(s: &str) -> Result<(i64, i64, i64, i64), String> {
            let value = |key| attribute(s, key)?.parse::<i64>().map_err(|e| e.to_string());
            Ok((value("x")?, value("y")?, value("width")?, value("height")?))
        }
        fn crosses(a: (i64, i64), b: (i64, i64), r: (i64, i64, i64, i64)) -> bool {
            let (x, y, w, h) = r;
            if a.0 == b.0 {
                a.0 > x && a.0 < x + w && a.1.min(b.1) < y + h && a.1.max(b.1) > y
            } else {
                a.1 > y && a.1 < y + h && a.0.min(b.0) < x + w && a.0.max(b.0) > x
            }
        }
        // A side-entry line through a third state is rejected by this oracle.
        assert!(crosses((1008, 700), (370, 700), (410, 640, 220, 120)));
        for visible_variant in ["map-wide", "map-medium", "map-narrow"] {
            // Isolate each complete variant, including nested label groups.
            let mut output = output.clone();
            for hidden in ["map-wide", "map-medium", "map-narrow"]
                .into_iter()
                .filter(|v| *v != visible_variant)
            {
                let start = format!("<g class=\"{hidden}\">");
                while let Some(a) = output.find(&start) {
                    let mut cursor = a + start.len();
                    let mut depth = 1;
                    while depth > 0 {
                        let close = output[cursor..].find("</g>").ok_or("variant close")? + cursor;
                        let open = output[cursor..].find("<g ").map(|n| cursor + n);
                        if open.is_some_and(|n| n < close) {
                            depth += 1;
                            cursor = open.ok_or("group")? + 3;
                        } else {
                            depth -= 1;
                            cursor = close + 4;
                        }
                    }
                    output.replace_range(a..cursor, "");
                }
            }
            let mut boxes = Vec::new();
            for name in names {
                let group = output
                    .split_once(&format!("id=\"geometry-{name}\""))
                    .ok_or("node")?
                    .1
                    .split("</a>")
                    .next()
                    .ok_or("node close")?;
                let identifier = output
                    .split_once(&format!("id=\"geometry-{name}:identifier\""))
                    .ok_or("separately selectable identifier")?
                    .1
                    .split("</a>")
                    .next()
                    .ok_or("identifier close")?;
                assert!(
                    identifier.contains(&format!(">{name}</tspan>")),
                    "common type keys must retain each authored repeated-name identifier"
                );
                boxes.push(rect(group.split_once("<rect ").ok_or("box")?.1)?);
            }
            let keys = output
                .split("<rect class=\"edge-key\"")
                .skip(1)
                .map(rect)
                .collect::<Result<Vec<_>, _>>()?;
            for (i, a) in keys.iter().enumerate() {
                for b in &keys[i + 1..] {
                    assert!(
                        a.0 + a.2 <= b.0
                            || b.0 + b.2 <= a.0
                            || a.1 + a.3 <= b.1
                            || b.1 + b.3 <= a.1,
                        "distinct key bounds"
                    );
                }
            }
            for (i, (from, to)) in edges.iter().enumerate() {
                let group = output
                    .split_once(&format!("data-element=\"geometry-e{i}\""))
                    .ok_or("edge")?
                    .1
                    .split("</a>")
                    .next()
                    .ok_or("edge close")?;
                let path = attribute(group.split_once("<path ").ok_or("path")?.1, "d")?;
                let tokens = path
                    .replace('M', " M ")
                    .replace('H', " H ")
                    .replace('V', " V ");
                let mut iter = tokens.split_whitespace();
                let mut point = (0, 0);
                let mut segments = Vec::new();
                while let Some(command) = iter.next() {
                    let mut number = || {
                        iter.next()
                            .ok_or("coordinate".to_owned())?
                            .parse::<i64>()
                            .map_err(|e| e.to_string())
                    };
                    let next = match command {
                        "M" => (number()?, number()?),
                        "H" => (number()?, point.1),
                        "V" => (point.0, number()?),
                        _ => return Err("unsupported path command".into()),
                    };
                    if command != "M" {
                        segments.push((point, next));
                    }
                    point = next;
                }
                for (a, b) in segments {
                    for (j, name) in names.iter().enumerate() {
                        if name != from && name != to {
                            assert!(
                                !crosses(a, b, boxes[j]),
                                "edge {i} passes through unrelated {name}"
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
