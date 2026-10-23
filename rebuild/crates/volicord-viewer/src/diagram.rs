//! Bounded, dependency-free local SVG. No generated markup, fonts, assets or I/O.
use volicord_projections::*;

pub(crate) const RENDERER_ID: &str = "volicord-static-svg/3";
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn words(s: &str, width: usize) -> Vec<String> {
    // Reserve two cells for non-ASCII and wide Latin glyphs. This conservative
    // local measure avoids squeezing CJK text into Latin-width boxes.
    let mut result = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let cells = if !ch.is_ascii() || "MWmw@%&#".contains(ch) {
            2
        } else {
            1
        };
        if used + cells > width && !line.is_empty() {
            result.push(std::mem::take(&mut line));
            used = 0;
        }
        line.push(ch);
        used += cells;
    }
    if !line.is_empty() {
        result.push(line);
    }
    result
}
fn text(out: &mut String, x: usize, y: usize, value: &str, width: usize) {
    out.push_str(&format!(
        "<text x=\"{x}\" y=\"{y}\" text-anchor=\"middle\">"
    ));
    for (i, line) in words(value, width).iter().enumerate() {
        out.push_str(&format!(
            "<tspan x=\"{x}\" dy=\"{}\">{}</tspan>",
            if i == 0 { 0 } else { 20 },
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
) -> Result<String, String> {
    let label = |l: &DiagramLabel| diagram_label(l, texts);
    let n = d.nodes().len();
    if n == 0 || n > 12 {
        return Err("unsupported diagram node budget".into());
    }
    let node_height = d
        .nodes()
        .iter()
        .map(|n| label(&n.label).map(|s| 62 + 20 * words(s, 26).len()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(120)
        .max(120);
    let owner_header = if let DiagramMeaning::State { owner, .. } = &d.meaning {
        30 + 20 * words(&format!("State owner: {}", label(owner)?), 80).len()
    } else {
        0
    };
    let mut incoming = vec![0usize; n];
    let mut key_y = Vec::new();
    let mut key_slots = Vec::new();
    let mut target_order = Vec::new();
    let mut message_height = 125usize;
    let mut source_tail = vec![0usize; n];
    let mut rank = vec![0usize; n];
    let mut positions = vec![(0usize, 0usize); n];
    let index = |id: &str| {
        d.nodes()
            .iter()
            .position(|n| n.id == id)
            .ok_or_else(|| "diagram endpoint unavailable".to_owned())
    };
    let (width, height) = match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            // Bounded layered DAG layout. Cyclic explanatory relationships use
            // a stable grid; neither layout asserts a semantic rank.
            let mut changed = false;
            for _ in 0..n {
                changed = false;
                for r in relations {
                    let a = index(&r.from)?;
                    let b = index(&r.to)?;
                    if rank[b] <= rank[a] {
                        rank[b] = rank[a] + 1;
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
            if changed {
                for (i, r) in rank.iter_mut().enumerate() {
                    *r = i / 3;
                }
            }
            let slots = relations
                .iter()
                .map(|r| {
                    r.condition
                        .as_ref()
                        .map(|l| label(l).map(|v| words(&format!("if {v}"), 30).len()))
                        .transpose()
                        .map(|lines| 60 + 20 * lines.unwrap_or(0))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let rows = rank.iter().max().copied().unwrap_or(0) + 1;
            let columns = (0..rows)
                .map(|r| rank.iter().filter(|v| **v == r).count())
                .max()
                .unwrap_or(1);
            let mut row_slots = vec![0usize; rows];
            for (r, slot) in relations.iter().zip(&slots) {
                if r.from != r.to {
                    row_slots[rank[index(&r.to)?]] += slot;
                }
            }
            let mut self_slots = vec![0usize; rows];
            for (r, slot) in relations.iter().zip(&slots) {
                if r.from == r.to {
                    let row = rank[index(&r.from)?];
                    self_slots[row] += slot;
                }
            }
            let top = 80.max(owner_header + if changed { row_slots[0] + 40 } else { 20 });
            let mut row_y = vec![top; rows];
            for row in 1..rows {
                row_y[row] =
                    row_y[row - 1] + node_height + self_slots[row - 1] + row_slots[row] + 40;
            }
            for i in 0..n {
                source_tail[i] = self_slots[rank[i]];
            }
            let has_self = relations.iter().any(|r| r.from == r.to);
            let w = ((columns + 1) * 260).max(720)
                + relations.len() * 16
                + if has_self { (columns + 1) * 180 } else { 0 };
            for (row, y) in row_y.iter().enumerate() {
                let entries = rank
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| **r == row)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                let count = entries.len();
                for (col, i) in entries.into_iter().enumerate() {
                    positions[i] = (w * (col + 1) / (count + 1), *y);
                }
            }
            let mut consumed = vec![0usize; rows];
            let mut self_consumed = vec![0usize; rows];
            for (r, slot) in relations.iter().zip(&slots) {
                let b = index(&r.to)?;
                let row = rank[b];
                if r.from == r.to {
                    key_y.push(positions[b].1 + node_height + 20 + self_consumed[row]);
                    self_consumed[row] += slot;
                } else {
                    key_y.push(positions[b].1 - row_slots[row] + 2 + consumed[row]);
                    consumed[row] += slot;
                }
                target_order.push(incoming[b]);
                incoming[b] += 1;
            }
            key_slots = slots;
            (w, row_y[rows - 1] + node_height + self_slots[rows - 1] + 40)
        }
        DiagramMeaning::Sequence { groups, .. } => {
            let w = ((n + 1) * 240).max(720);
            for (i, p) in positions.iter_mut().enumerate() {
                *p = (w * (i + 1) / (n + 1), 40);
            }
            for m in groups.iter().flat_map(|g| &g.messages) {
                let distance = positions[index(&m.from)?]
                    .0
                    .abs_diff(positions[index(&m.to)?].0);
                let units = if distance == 0 {
                    26
                } else {
                    (distance / 8).saturating_sub(4).clamp(26, 40)
                };
                message_height = message_height.max(
                    50 + 20
                        * words(
                            &format!("12.20 · {}: {}", m.kind.text(), label(&m.label)?),
                            units,
                        )
                        .len(),
                );
            }
            let rows = groups.iter().map(|g| g.messages.len()).sum::<usize>();
            (
                w,
                node_height + 80 + groups.len() * 125 + rows * message_height,
            )
        }
    };
    let marker = format!("{ns}:arrow");
    let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" role=\"group\" aria-label=\"{}\" data-renderer=\"{RENDERER_ID}\"><defs><marker id=\"{marker}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M0 0L10 5L0 10Z\"/></marker></defs>",esc(label(&d.title)?));
    if matches!(d.meaning, DiagramMeaning::Sequence { .. }) {
        for (x, _) in &positions {
            out.push_str(&format!(
                "<line class=\"lifeline\" x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\"/>",
                node_height + 40,
                height - 15
            ));
        }
    }
    if let DiagramMeaning::Concept { relations, .. }
    | DiagramMeaning::Component { relations, .. }
    | DiagramMeaning::Flowchart { relations, .. }
    | DiagramMeaning::State { relations, .. } = &d.meaning
    {
        for (i, r) in relations.iter().enumerate() {
            let a = index(&r.from)?;
            let b = index(&r.to)?;
            let (x1, y1) = positions[a];
            let (x2, y2) = positions[b];
            open(
                &mut out,
                ns,
                &r.id,
                &r.label,
                &format!("{}: {}", element_caption(d, &r.id, texts), label(&r.label)?),
                target,
            );
            let slot = key_slots[i];
            let guard_lines = (slot - 60) / 20;
            let mid = key_y[i];
            let out_y = mid + 30 + 20 * guard_lines;
            let (x, key_width) = if a == b {
                let side = x1 + 110;
                let upper = y1 + 30;
                let back = y1 + node_height - 30;
                let outer = x1 + 280;
                let return_lane = x1 + 300;
                let key = x1 + 140;
                out.push_str(&format!("<path class=\"relation-line\" d=\"M{side} {upper}H{outer}V{mid}H{key}V{out_y}H{return_lane}V{back}H{side}\" marker-end=\"url(#{marker})\"/>"));
                (key, if guard_lines > 0 { 260 } else { 180 })
            } else {
                let lane = if x2 <= width / 2 {
                    20 + i * 6
                } else {
                    width - 20 - i * 6
                };
                let start = y1 + node_height;
                let exit = start + source_tail[a] + 20;
                let stem = x2 + 134 + target_order[i] * 2;
                let approach = y2 - 15;
                // Distinct outer lanes, disjoint label rows and side stems
                // keep paths clear of nodes, guards and other edge keys.
                out.push_str(&format!("<path class=\"relation-line\" d=\"M{x1} {start}V{exit}H{lane}V{mid}H{x2}V{out_y}H{stem}V{approach}H{x2}V{y2}\" marker-end=\"url(#{marker})\"/>"));
                (x2, if guard_lines > 0 { 260 } else { 180 })
            };
            if matches!(
                r.kind,
                DiagramRelationKind::DataFlow | DiagramRelationKind::DependsOn
            ) {
                let start = out
                    .rfind("<path class=\"relation-line\"")
                    .ok_or("relation path missing")?;
                out.insert_str(start + "<path".len(), " stroke-dasharray=\"6 4\"");
            }
            out.push_str(&format!("<rect class=\"edge-key\" x=\"{}\" y=\"{}\" width=\"{key_width}\" height=\"{}\" rx=\"4\"/>",x.saturating_sub(key_width/2),mid.saturating_sub(21),33+guard_lines*20));
            text(
                &mut out,
                x,
                mid - 2,
                &format!(
                    "E{} · {}{}",
                    i + 1,
                    r.kind.text(),
                    if r.condition.is_some() { " [if]" } else { "" }
                ),
                32,
            );
            out.push_str("</a>");
            if let Some(condition) = &r.condition {
                open(
                    &mut out,
                    ns,
                    &format!("{}:condition", r.id),
                    condition,
                    label(condition)?,
                    target,
                );
                text(
                    &mut out,
                    x,
                    mid + 22,
                    &format!("if {}", label(condition)?),
                    30,
                );
                out.push_str("</a>");
            }
        }
    }
    for (i, node) in d.nodes().iter().enumerate() {
        let (x, y) = positions[i];
        open(
            &mut out,
            ns,
            &node.id,
            &node.label,
            label(&node.label)?,
            target,
        );
        match node.kind {
            DiagramNodeKind::Branch => out.push_str(&format!("<path class=\"participant-box\" d=\"M{x} {y}l110 {}l-110 {}l-110 -{}Z\"/>",node_height/2,node_height/2,node_height/2)),
            DiagramNodeKind::Initial | DiagramNodeKind::Final => {
                out.push_str(&format!("<ellipse class=\"participant-box\" cx=\"{x}\" cy=\"{}\" rx=\"110\" ry=\"{}\"/>",y+node_height/2,node_height/2));
                if node.kind == DiagramNodeKind::Final {out.push_str(&format!("<ellipse class=\"participant-box\" cx=\"{x}\" cy=\"{}\" rx=\"105\" ry=\"{}\"/>",y+node_height/2,(node_height/2).saturating_sub(5)));}
            }
            _ => out.push_str(&format!("<rect class=\"participant-box\" x=\"{}\" y=\"{y}\" width=\"220\" height=\"{node_height}\" rx=\"{}\"/>",x.saturating_sub(110),if node.kind == DiagramNodeKind::State {20} else {6})),
        }
        text(&mut out, x, y + 20, node.kind.text(), 28);
        text(&mut out, x, y + 42, label(&node.label)?, 26);
        out.push_str("</a>");
    }
    if let DiagramMeaning::Sequence { groups, .. } = &d.meaning {
        let mut y = node_height + 65;
        for (i, g) in groups.iter().enumerate() {
            let guard = g.condition.as_ref().map(label).transpose()?;
            let name = format!(
                "{} · {}{}",
                i + 1,
                if guard.is_some() {
                    "opt"
                } else {
                    match g.order {
                        DiagramOrder::Sequential => "strict",
                        DiagramOrder::Parallel => "par · no relative order",
                    }
                },
                guard.map(|s| format!(" [if {s}]")).unwrap_or_default()
            );
            out.push_str(&format!(
                "<rect class=\"sequence-frame\" x=\"10\" y=\"{y}\" width=\"{}\" height=\"{}\"/>",
                width - 20,
                125 + message_height * g.messages.len() - 15
            ));
            if let Some(condition) = &g.condition {
                open(
                    &mut out,
                    ns,
                    &format!("guard:{}", i + 1),
                    condition,
                    label(condition)?,
                    target,
                );
            }
            text(&mut out, width / 2, y + 24, &name, 80);
            if g.condition.is_some() {
                out.push_str("</a>");
            }
            if guard.is_some() {
                out.push_str(&format!(
                    "<rect class=\"sequence-frame\" x=\"18\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    y + 75,
                    width - 36,
                    125 + message_height * g.messages.len() - 95
                ));
                text(
                    &mut out,
                    width / 2,
                    y + 100,
                    match g.order {
                        DiagramOrder::Sequential => "strict",
                        DiagramOrder::Parallel => "par · no relative order",
                    },
                    80,
                );
            }
            y += 125;
            for (j, m) in g.messages.iter().enumerate() {
                // Each independent message is its own UML par operand. Stacking
                // messages in one undivided operand would imply local ordering.
                if g.order == DiagramOrder::Parallel && j > 0 {
                    out.push_str(&format!("<path class=\"relation-line parallel-operand\" d=\"M10 {}H{}\" stroke-dasharray=\"6 4\"/>",y-25,width-10));
                }
                let (x1, _) = positions[index(&m.from)?];
                let (x2, _) = positions[index(&m.to)?];
                open(
                    &mut out,
                    ns,
                    &m.id,
                    &m.label,
                    &format!("{}: {}", element_caption(d, &m.id, texts), label(&m.label)?),
                    target,
                );
                let dash = if m.kind == DiagramMessageKind::Return {
                    "stroke-dasharray=\"6 4\""
                } else {
                    ""
                };
                if x1 == x2 {
                    out.push_str(&format!("<path class=\"relation-line\" d=\"M{x1} {y}h65v35h-65\" marker-end=\"url(#{marker})\" {dash}/>"));
                } else {
                    out.push_str(&format!("<path class=\"relation-line\" d=\"M{x1} {y}H{x2}\" marker-end=\"url(#{marker})\" {dash}/>"));
                }
                let x = if x1 == x2 { x1 + 85 } else { (x1 + x2) / 2 };
                text(
                    &mut out,
                    x,
                    y + 20,
                    &format!(
                        "{}.{} · {}: {}",
                        i + 1,
                        j + 1,
                        m.kind.text(),
                        label(&m.label)?
                    ),
                    if x1 == x2 {
                        26
                    } else {
                        (x1.abs_diff(x2) / 8).saturating_sub(4).clamp(26, 40)
                    },
                );
                out.push_str("</a>");
                y += message_height;
            }
        }
    }
    if let DiagramMeaning::State { owner, .. } = &d.meaning {
        open(&mut out, ns, "state:owner", owner, label(owner)?, target);
        text(
            &mut out,
            width / 2,
            24,
            &format!("State owner: {}", label(owner)?),
            80,
        );
        out.push_str("</a>");
    }
    out.push_str("</svg>");
    if out.len() > 262144 {
        return Err("local SVG render budget exceeded".into());
    }
    Ok(out)
}

/// Read-side identities for every selectable semantic element, including guards.
/// ':' separates renderer-owned guard IDs from admitted ASCII element tokens.
pub(crate) fn elements(d: &GroundedDiagram) -> Vec<(String, &DiagramLabel)> {
    let mut result = d
        .nodes()
        .iter()
        .map(|n| (n.id.clone(), &n.label))
        .collect::<Vec<_>>();
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
pub(crate) fn element_caption(d: &GroundedDiagram, id: &str, texts: &[&str]) -> String {
    let label = |l: &DiagramLabel| diagram_label(l, texts).unwrap_or("");
    let node = |id: &str| {
        d.nodes()
            .iter()
            .find(|n| n.id == id)
            .map(|n| label(&n.label))
            .unwrap_or("")
    };
    if id == "state:owner" {
        if let DiagramMeaning::State { owner, .. } = &d.meaning {
            return format!("State owner: {}", label(owner));
        }
    }
    if let Some(n) = d.nodes().iter().find(|n| n.id == id) {
        return format!("{} · {}", label(&n.label), n.kind.text());
    }
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            for r in relations {
                if r.id == id {
                    return format!("{} → {} · {}", node(&r.from), node(&r.to), r.kind.text());
                }
            }
        }
        DiagramMeaning::Sequence { groups, .. } => {
            for (i, g) in groups.iter().enumerate() {
                for m in &g.messages {
                    if m.id == id {
                        return format!(
                            "{}. {} → {} · {}",
                            i + 1,
                            node(&m.from),
                            node(&m.to),
                            m.kind.text()
                        );
                    }
                }
            }
        }
    }
    elements(d)
        .into_iter()
        .find(|(key, _)| key == id)
        .map(|(_, l)| format!("if {}", label(l)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn svg_escapes_text_and_scopes_internal_marker_guard_and_element_ids() -> Result<(), String> {
        // Renderer-only hostile-text control, not an admitted semantic publication.
        let text = "<script>alert(1)</script>";
        let label = DiagramLabel {
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
            1
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
            std::fs::create_dir(root).map_err(|e| e.to_string())?;
            std::fs::write(root.join("long-korean.svg"), &output).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[test]
    fn routed_state_edges_do_not_cross_unrelated_nodes_or_overlay_keys() -> Result<(), String> {
        // Independent geometry invariant for the Source-derived request topology:
        // queued -> delivered; each can finish in success, failure or expiry.
        let label = DiagramLabel {
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
        let mut boxes = Vec::new();
        for name in names {
            let group = output
                .split_once(&format!("id=\"geometry-{name}\""))
                .ok_or("node")?
                .1
                .split("</a>")
                .next()
                .ok_or("node close")?;
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
                    a.0 + a.2 <= b.0 || b.0 + b.2 <= a.0 || a.1 + a.3 <= b.1 || b.1 + b.3 <= a.1,
                    "distinct key bounds"
                );
            }
        }
        for (i, (from, to)) in edges.iter().enumerate() {
            let group = output
                .split_once(&format!("id=\"geometry-e{i}\""))
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
        Ok(())
    }
}
