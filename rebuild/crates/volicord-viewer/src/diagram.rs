//! Bounded, dependency-free local SVG. No generated markup, fonts, assets or I/O.
use volicord_projections::*;

pub(crate) const RENDERER_ID: &str = "volicord-static-svg/2";
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn words(s: &str, width: usize) -> Vec<String> {
    // Unicode scalar chunks preserve every character, including unspaced labels.
    // This is layout only; the full admitted label remains in the anchor title.
    s.chars()
        .collect::<Vec<_>>()
        .chunks(width)
        .map(|c| c.iter().collect())
        .collect()
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
            let rows = rank.iter().max().copied().unwrap_or(0) + 1;
            let columns = (0..rows)
                .map(|r| rank.iter().filter(|v| **v == r).count())
                .max()
                .unwrap_or(1);
            let w = ((columns + 1) * 260).max(720);
            for row in 0..rows {
                let entries = rank
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| **r == row)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>();
                let count = entries.len();
                for (col, i) in entries.into_iter().enumerate() {
                    positions[i] = (w * (col + 1) / (count + 1), 80 + row * 280);
                }
            }
            (w, rows * 280 + 40)
        }
        DiagramMeaning::Sequence { groups, .. } => {
            let w = ((n + 1) * 240).max(720);
            for (i, p) in positions.iter_mut().enumerate() {
                *p = (w * (i + 1) / (n + 1), 40);
            }
            let rows = groups.iter().map(|g| g.messages.len() + 1).sum::<usize>();
            (w, 200 + rows * 125)
        }
    };
    let marker = format!("{ns}:arrow");
    let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" role=\"group\" aria-label=\"{}\" data-renderer=\"{RENDERER_ID}\"><defs><marker id=\"{marker}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M0 0L10 5L0 10Z\"/></marker></defs>",esc(label(&d.title)?));
    if matches!(d.meaning, DiagramMeaning::Sequence { .. }) {
        for (x, _) in &positions {
            out.push_str(&format!(
                "<line class=\"lifeline\" x1=\"{x}\" y1=\"160\" x2=\"{x}\" y2=\"{}\"/>",
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
            open(&mut out, ns, &r.id, &r.label, label(&r.label)?, target);
            // Long edges use an outside lane, never pass through an unrelated
            // node. Adjacent edges place keys above their distinct endpoints.
            let (x, mid, key_width) = if a == b {
                out.push_str(&format!("<path class=\"relation-line\" d=\"M{} {}h55v70h-55\" marker-end=\"url(#{marker})\"/>", x1+110,y1+25));
                (x1 + 150, y1 + 130, 180)
            } else if rank[b] != rank[a] + 1 {
                let left = x1 <= x2;
                let lane = if left {
                    20 + (i % 3) * 12
                } else {
                    width - 20 - (i % 3) * 12
                };
                let side1 = if left { x1 - 110 } else { x1 + 110 };
                let side2 = if left { x2 - 110 } else { x2 + 110 };
                let sy = y1 + 60;
                let ey = y2 + 60;
                out.push_str(&format!("<path class=\"relation-line\" d=\"M{side1} {sy}H{lane}V{ey}H{side2}\" marker-end=\"url(#{marker})\"/>"));
                (if left { 75 } else { width - 75 }, (sy + ey) / 2, 140)
            } else {
                let start = y1 + 120;
                let end = y2;
                let mid = (start + end) / 2 + (i % 3) * 15;
                out.push_str(&format!("<path class=\"relation-line\" d=\"M{x1} {start}V{mid}H{x2}V{end}\" marker-end=\"url(#{marker})\"/>"));
                (x2, mid, 180)
            };
            out.push_str(&format!("<rect class=\"edge-key\" x=\"{}\" y=\"{}\" width=\"{key_width}\" height=\"27\" rx=\"4\"/>",x.saturating_sub(key_width/2),mid.saturating_sub(21)));
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
            DiagramNodeKind::Branch => out.push_str(&format!("<path class=\"participant-box\" d=\"M{x} {y}l110 60l-110 60l-110 -60Z\"/>")),
            DiagramNodeKind::Initial | DiagramNodeKind::Final => {
                out.push_str(&format!("<ellipse class=\"participant-box\" cx=\"{x}\" cy=\"{}\" rx=\"110\" ry=\"60\"/>",y+60));
                if node.kind == DiagramNodeKind::Final {out.push_str(&format!("<ellipse class=\"participant-box\" cx=\"{x}\" cy=\"{}\" rx=\"105\" ry=\"55\"/>",y+60));}
            }
            _ => out.push_str(&format!("<rect class=\"participant-box\" x=\"{}\" y=\"{y}\" width=\"220\" height=\"120\" rx=\"{}\"/>",x.saturating_sub(110),if node.kind == DiagramNodeKind::State {20} else {6})),
        }
        text(&mut out, x, y + 20, node.kind.text(), 28);
        text(&mut out, x, y + 42, label(&node.label)?, 26);
        out.push_str("</a>");
    }
    if let DiagramMeaning::Sequence { groups, .. } = &d.meaning {
        let mut y = 185;
        for (i, g) in groups.iter().enumerate() {
            let guard = g.condition.as_ref().map(label).transpose()?;
            let name = format!(
                "{} · {}{}",
                i + 1,
                match g.order {
                    DiagramOrder::Sequential => "seq",
                    DiagramOrder::Parallel => "par · no relative order",
                },
                guard.map(|s| format!(" [if {s}]")).unwrap_or_default()
            );
            out.push_str(&format!(
                "<rect class=\"sequence-frame\" x=\"10\" y=\"{y}\" width=\"{}\" height=\"{}\"/>",
                width - 20,
                125 * (g.messages.len() + 1) - 15
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
            y += 125;
            for (j, m) in g.messages.iter().enumerate() {
                // Each independent message is its own UML par operand. Stacking
                // messages in one undivided operand would imply local ordering.
                if g.order == DiagramOrder::Parallel && j > 0 {
                    out.push_str(&format!("<path class=\"relation-line parallel-operand\" d=\"M10 {}H{}\" stroke-dasharray=\"6 4\"/>",y-25,width-10));
                }
                let (x1, _) = positions[index(&m.from)?];
                let (x2, _) = positions[index(&m.to)?];
                open(&mut out, ns, &m.id, &m.label, label(&m.label)?, target);
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
                    40,
                );
                out.push_str("</a>");
                y += 125;
            }
        }
    }
    if let DiagramMeaning::State { owner, .. } = &d.meaning {
        open(&mut out, ns, "state:owner", owner, label(owner)?, target);
        text(
            &mut out,
            width / 2,
            height - 12,
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
}
