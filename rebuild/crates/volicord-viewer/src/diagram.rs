//! Bounded, dependency-free local SVG. No generated markup, fonts, assets or I/O.
use volicord_projections::*;
#[path = "diagram_route.rs"]
mod routing;
use routing::Rect;

pub(crate) const RENDERER_ID: &str = "volicord-static-svg/4";
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn words(s: &str, width: usize) -> Vec<String> {
    // Conservative pixels at the native 16px font. ASCII is not uniformly
    // eight pixels wide. Browser glyph checks remain the independent oracle.
    let measure = |ch: char| {
        if !ch.is_ascii() || "MWmw@%&#".contains(ch) {
            16
        } else if " iljft.,:;'!|()[]".contains(ch) {
            6
        } else {
            12
        }
    };
    let limit = width * 8;
    let mut result = Vec::new();
    let mut line = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let pixels = measure(ch);
        if used + pixels > limit && !line.is_empty() {
            // Preserve every original byte, including whitespace; prefer a
            // word boundary without inventing abbreviations or shorter prose.
            let cut = line
                .char_indices()
                .rev()
                .find(|(_, c)| c.is_whitespace())
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(line.len());
            let rest = line.split_off(cut);
            result.push(std::mem::replace(&mut line, rest));
            used = line.chars().map(measure).sum();
            if used + pixels > limit && !line.is_empty() {
                result.push(std::mem::take(&mut line));
                used = 0;
            }
        }
        line.push(ch);
        used += pixels;
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
    locale: FixedLocale,
) -> Result<String, String> {
    // The renderer is a separate allocation/markup boundary. Admission normally
    // supplies these bounds; do not rely on it to constrain layout or a future
    // caller before the final output-size check can run.
    bounded_input(d, texts, ns)?;
    let label = |l: &DiagramLabel| diagram_label(l, texts);
    let n = d.nodes().len();
    if n == 0 || n > 12 {
        return Err("unsupported diagram node budget".into());
    }
    let mut node_height = d
        .nodes()
        .iter()
        .map(|n| label(&n.label).map(|s| 62 + 20 * words(s, 26).len()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(80)
        .max(80);
    // A diamond's text block sits in its wide middle, away from sloping edges.
    if d.nodes().iter().any(|n| n.kind == DiagramNodeKind::Branch) {
        node_height = node_height.max(
            d.nodes()
                .iter()
                .filter(|n| n.kind == DiagramNodeKind::Branch)
                .map(|n| label(&n.label).map(|s| 2 * (44 + 20 * words(s, 26).len())))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .max()
                .unwrap_or(80),
        );
    }
    let node_width = |i: usize| {
        if d.nodes()[i].kind == DiagramNodeKind::Branch {
            360
        } else {
            220
        }
    };
    let column_step = if d.nodes().iter().any(|n| n.kind == DiagramNodeKind::Branch) {
        400
    } else {
        320
    };
    let owner_header = if let DiagramMeaning::State { owner, .. } = &d.meaning {
        30 + 20
            * words(
                &format!(
                    "{} {}",
                    diagram_ui(locale, "State owner:", "상태 소유자:"),
                    label(owner)?
                ),
                40,
            )
            .len()
    } else {
        0
    };
    let mut key_y = Vec::new();
    let mut key_slots = Vec::new();
    let mut message_heights = Vec::new();
    let mut group_heights = Vec::new();
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
            // Rank the condensation graph. A cycle stays together without
            // reversing edges or pretending vector order is execution order.
            let mut reaches = vec![vec![false; n]; n];
            for r in relations {
                reaches[index(&r.from)?][index(&r.to)?] = true;
            }
            for k in 0..n {
                for a in 0..n {
                    for b in 0..n {
                        reaches[a][b] |= reaches[a][k] && reaches[k][b];
                    }
                }
            }
            for _ in 0..n {
                let old = rank.clone();
                for r in relations {
                    let a = index(&r.from)?;
                    let b = index(&r.to)?;
                    if !reaches[b][a] {
                        rank[b] = rank[b].max(old[a] + 1);
                    }
                }
                for a in 0..n {
                    for b in 0..n {
                        if reaches[a][b] && reaches[b][a] {
                            rank[a] = rank[a].max(rank[b]);
                        }
                    }
                }
                if old == rank {
                    break;
                }
            }
            let slots = relations
                .iter()
                .map(|r| {
                    let lines = words(label(&r.label)?, 32).len();
                    let guard = r
                        .condition
                        .as_ref()
                        .map(|l| {
                            label(l).map(|v| {
                                words(&format!("{} {v}", diagram_ui(locale, "if", "조건:")), 30)
                                    .len()
                            })
                        })
                        .transpose()?
                        .unwrap_or(0);
                    Ok(52 + 20 * (lines + guard))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let rows = rank.iter().max().copied().unwrap_or(0) + 1;
            let columns = (0..rows)
                .map(|r| rank.iter().filter(|v| **v == r).count())
                .max()
                .unwrap_or(1);
            // Independent columns share a label band; only labels attached
            // to the same target need separate vertical slots.
            let mut target_slots = vec![0usize; n];
            let mut self_slots = vec![0usize; n];
            for (r, slot) in relations.iter().zip(&slots) {
                let b = index(&r.to)?;
                if r.from == r.to {
                    self_slots[b] += slot;
                } else {
                    target_slots[b] += slot;
                }
            }
            let row_slots = (0..rows)
                .map(|row| {
                    (0..n)
                        .filter(|i| rank[*i] == row)
                        .map(|i| target_slots[i])
                        .max()
                        .unwrap_or(0)
                })
                .collect::<Vec<_>>();
            let row_self = (0..rows)
                .map(|row| {
                    (0..n)
                        .filter(|i| rank[*i] == row)
                        .map(|i| self_slots[i])
                        .max()
                        .unwrap_or(0)
                })
                .collect::<Vec<_>>();
            let top = owner_header + row_slots[0] + 40;
            let mut row_y = vec![top; rows];
            for row in 1..rows {
                row_y[row] = row_y[row - 1] + node_height + row_self[row - 1] + row_slots[row] + 48;
            }
            let w = columns * column_step + 80;
            for (row, y) in row_y.iter().enumerate() {
                let entries = (0..n).filter(|i| rank[*i] == row).collect::<Vec<_>>();
                let offset = (w - entries.len() * column_step) / 2;
                for (col, i) in entries.into_iter().enumerate() {
                    positions[i] = (offset + column_step / 2 + col * column_step, *y);
                }
            }
            let mut consumed = vec![0usize; n];
            let mut self_consumed = vec![0usize; n];
            for (r, slot) in relations.iter().zip(&slots) {
                let b = index(&r.to)?;
                if r.from == r.to {
                    key_y.push(positions[b].1 + node_height + 40 + self_consumed[b]);
                    self_consumed[b] += slot;
                } else {
                    key_y.push(positions[b].1 - target_slots[b] + consumed[b]);
                    consumed[b] += slot;
                }
            }
            key_slots = slots;
            (w, row_y[rows - 1] + node_height + row_self[rows - 1] + 64)
        }
        DiagramMeaning::Sequence { groups, .. } => {
            // Participant order is admitted vector order. Each self-call owns
            // its complete multiline interior and a separate return baseline.
            let extra = groups
                .iter()
                .flat_map(|g| &g.messages)
                .any(|m| m.from == m.to && index(&m.from).ok() == Some(n - 1));
            let w = 40 + n * 300 + if extra { 190 } else { 0 };
            for (i, p) in positions.iter_mut().enumerate() {
                *p = (150 + i * 300, 24);
            }
            let guard_width = ((w - 64) / 8).min(80);
            for g in groups {
                let guard = g.condition.as_ref().map(label).transpose()?;
                let header = 44
                    + guard
                        .map(|v| 20 * words(v, guard_width).len() + 32)
                        .unwrap_or(0);
                let mut gh = header;
                for m in &g.messages {
                    let lines = words(label(&m.label)?, 30).len();
                    let mh = 60 + lines * 20;
                    message_heights.push(mh);
                    gh += mh;
                }
                group_heights.push((header, gh));
            }
            (
                w,
                node_height + 56 + group_heights.iter().map(|(_, h)| h).sum::<usize>(),
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
        let mut obstacles = positions
            .iter()
            .enumerate()
            .map(|(i, (x, y))| {
                Rect {
                    x: x - node_width(i) / 2,
                    y: *y,
                    w: node_width(i),
                    h: node_height,
                }
                .padded()
            })
            .collect::<Vec<_>>();
        for (i, r) in relations.iter().enumerate() {
            let x = positions[index(&r.to)?].0;
            obstacles.push(
                Rect {
                    x: x - 140,
                    y: key_y[i] - 21,
                    w: 280,
                    h: key_slots[i] - 16,
                }
                .padded(),
            );
        }
        if owner_header > 0 {
            obstacles.push(Rect {
                x: 20,
                y: 0,
                w: width - 40,
                h: owner_header,
            });
        }
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
                &format!(
                    "{}: {}",
                    element_caption(d, &r.id, texts, locale),
                    label(&r.label)?
                ),
                target,
            );
            let slot = key_slots[i];
            let label_lines = words(label(&r.label)?, 32).len();
            let mid = key_y[i];
            let x = x2;
            let key_width = 280;
            let card = Rect {
                x: x - 140,
                y: mid - 21,
                w: 280,
                h: slot - 16,
            };
            let (start, end, start_tip, end_tip) = if a == b {
                (
                    (x1 + node_width(a) / 2 + 8, y1 + node_height / 4),
                    (x1 + node_width(a) / 2 + 8, y1 + node_height * 3 / 4),
                    (
                        x1 + if matches!(
                            d.nodes()[a].kind,
                            DiagramNodeKind::Branch
                                | DiagramNodeKind::Initial
                                | DiagramNodeKind::Final
                        ) {
                            90
                        } else {
                            110
                        },
                        y1 + node_height / 4,
                    ),
                    (
                        x1 + if matches!(
                            d.nodes()[a].kind,
                            DiagramNodeKind::Branch
                                | DiagramNodeKind::Initial
                                | DiagramNodeKind::Final
                        ) {
                            90
                        } else {
                            110
                        },
                        y1 + node_height * 3 / 4,
                    ),
                )
            } else {
                let port = |id: &str, outgoing: bool, center: usize| {
                    let matches =
                        |r: &&DiagramRelation| if outgoing { r.from == id } else { r.to == id };
                    let total = relations.iter().filter(matches).count();
                    let ordinal = relations[..i].iter().filter(matches).count();
                    center - 96 + (ordinal + 1) * 192 / (total + 1)
                };
                let from_x = if matches!(
                    d.nodes()[a].kind,
                    DiagramNodeKind::Branch | DiagramNodeKind::Initial | DiagramNodeKind::Final
                ) {
                    x1
                } else {
                    port(&r.from, true, x1)
                };
                let to_x = if matches!(
                    d.nodes()[b].kind,
                    DiagramNodeKind::Branch | DiagramNodeKind::Initial | DiagramNodeKind::Final
                ) {
                    x2
                } else {
                    port(&r.to, false, x2)
                };
                (
                    (from_x, y1 + node_height + 8),
                    (to_x, y2 - 8),
                    (from_x, y1 + node_height),
                    (to_x, y2),
                )
            };
            let before = routing::route(start, (x, card.y - 8), &obstacles, width, height)?;
            let after = routing::route((x, card.y + card.h + 8), end, &obstacles, width, height)?;
            let connector = |a: (usize, usize), b: (usize, usize)| {
                if a.0 == b.0 {
                    format!("V{}", b.1)
                } else {
                    format!("H{}", b.0)
                }
            };
            let path = format!(
                "M{} {}{}{}{}{}",
                start_tip.0,
                start_tip.1,
                connector(start_tip, start),
                before,
                after,
                connector(end, end_tip)
            );
            out.push_str(&format!(
                "<path class=\"relation-line\" d=\"{path}\" marker-end=\"url(#{marker})\"/>"
            ));
            if matches!(
                r.kind,
                DiagramRelationKind::DataFlow | DiagramRelationKind::DependsOn
            ) {
                let start = out
                    .rfind("<path class=\"relation-line\"")
                    .ok_or("relation path missing")?;
                out.insert_str(start + "<path".len(), " stroke-dasharray=\"6 4\"");
            }
            out.push_str(&format!("<rect class=\"edge-key\" x=\"{}\" y=\"{}\" width=\"{key_width}\" height=\"{}\" rx=\"4\"/>",x.saturating_sub(key_width/2),mid.saturating_sub(21),slot-16));
            text(&mut out, x, mid - 2, label(&r.label)?, 32);
            text(
                &mut out,
                x,
                mid + label_lines * 20,
                &format!("E{} · {}", i + 1, r.kind.localized_text(locale)),
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
                    mid + label_lines * 20 + 22,
                    &format!(
                        "{} {}",
                        diagram_ui(locale, "if", "조건:"),
                        label(condition)?
                    ),
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
            DiagramNodeKind::Branch => out.push_str(&format!("<path class=\"participant-box\" d=\"M{x} {y}l180 {}l-180 {}l-180 -{}Z\"/>",node_height/2,node_height/2,node_height/2)),
            DiagramNodeKind::Initial | DiagramNodeKind::Final => {
                out.push_str(&format!("<ellipse class=\"participant-box\" cx=\"{x}\" cy=\"{}\" rx=\"110\" ry=\"{}\"/>",y+node_height/2,node_height/2));
                if node.kind == DiagramNodeKind::Final {out.push_str(&format!("<ellipse class=\"participant-box\" cx=\"{x}\" cy=\"{}\" rx=\"105\" ry=\"{}\"/>",y+node_height/2,(node_height/2).saturating_sub(5)));}
            }
            _ => out.push_str(&format!("<rect class=\"participant-box\" x=\"{}\" y=\"{y}\" width=\"220\" height=\"{node_height}\" rx=\"{}\"/>",x.saturating_sub(110),if node.kind == DiagramNodeKind::State {20} else {6})),
        }
        let lines = words(label(&node.label)?, 26).len();
        let baseline = if node.kind == DiagramNodeKind::Branch {
            y + node_height / 2 - lines * 10 + 18
        } else {
            y + 42
        };
        text(
            &mut out,
            x,
            baseline - 22,
            node.kind.localized_text(locale),
            28,
        );
        text(&mut out, x, baseline, label(&node.label)?, 26);
        out.push_str("</a>");
    }
    if let DiagramMeaning::Sequence { groups, .. } = &d.meaning {
        let mut y = node_height + 48;
        let mut message_index = 0;
        for (i, g) in groups.iter().enumerate() {
            let (header, gh) = group_heights[i];
            out.push_str(&format!(
                "<rect class=\"sequence-frame\" x=\"10\" y=\"{y}\" width=\"{}\" height=\"{}\"/>",
                width - 20,
                gh - 8
            ));
            let order = match g.order {
                DiagramOrder::Sequential => {
                    diagram_ui(locale, "strict · sequential", "strict · 순차")
                }
                DiagramOrder::Parallel => {
                    diagram_ui(locale, "par · no relative order", "par · 상대 순서 없음")
                }
            };
            if let Some(condition) = &g.condition {
                open(
                    &mut out,
                    ns,
                    &format!("guard:{}", i + 1),
                    condition,
                    label(condition)?,
                    target,
                );
                text(
                    &mut out,
                    width / 2,
                    y + 22,
                    &format!(
                        "opt · {} {}",
                        diagram_ui(locale, "if", "조건:"),
                        label(condition)?
                    ),
                    ((width - 64) / 8).min(80),
                );
                out.push_str("</a>");
                let inner = header - 32;
                out.push_str(&format!(
                    "<rect class=\"sequence-frame\" x=\"18\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    y + inner,
                    width - 36,
                    gh - inner - 12
                ));
            }
            text(
                &mut out,
                width / 2,
                y + header - 16,
                &format!("{}. {order}", i + 1),
                80,
            );
            y += header;
            for (j, m) in g.messages.iter().enumerate() {
                let mh = message_heights[message_index];
                message_index += 1;
                if g.order == DiagramOrder::Parallel && j > 0 {
                    out.push_str(&format!("<path class=\"relation-line parallel-operand\" d=\"M18 {}H{}\" stroke-dasharray=\"6 4\"/>",y-8,width-18));
                }
                let x1 = positions[index(&m.from)?].0;
                let x2 = positions[index(&m.to)?].0;
                open(
                    &mut out,
                    ns,
                    &m.id,
                    &m.label,
                    &format!(
                        "{}: {}",
                        element_caption(d, &m.id, texts, locale),
                        label(&m.label)?
                    ),
                    target,
                );
                let dash = if m.kind == DiagramMessageKind::Return {
                    "stroke-dasharray=\"6 4\""
                } else {
                    ""
                };
                let lines = words(label(&m.label)?, 30).len();
                let x = if x1 == x2 { x1 + 145 } else { (x1 + x2) / 2 };
                let label_y = y + 18;
                let arrow_y = y + lines * 20 + 26;
                if x1 == x2 {
                    // Both horizontal legs stay outside the full label block.
                    out.push_str(&format!("<path class=\"relation-line\" d=\"M{x1} {}H{}V{arrow_y}H{x1}\" marker-end=\"url(#{marker})\" {dash}/>",y+2,x1+280));
                } else {
                    out.push_str(&format!("<path class=\"relation-line\" d=\"M{x1} {arrow_y}H{x2}\" marker-end=\"url(#{marker})\" {dash}/>"));
                }
                // Mask lifelines only beneath the label, never a message path.
                out.push_str(&format!(
                    "<rect class=\"message-label\" x=\"{}\" y=\"{}\" width=\"256\" height=\"{}\"/>",
                    x - 128,
                    label_y - 16,
                    lines * 20
                ));
                text(&mut out, x, label_y, label(&m.label)?, 30);
                text(
                    &mut out,
                    x,
                    arrow_y + 20,
                    &format!("{}.{} · {}", i + 1, j + 1, m.kind.localized_text(locale)),
                    30,
                );
                out.push_str("</a>");
                y += mh;
            }
        }
    }
    if let DiagramMeaning::State { owner, .. } = &d.meaning {
        open(&mut out, ns, "state:owner", owner, label(owner)?, target);
        text(
            &mut out,
            width / 2,
            24,
            &format!(
                "{} {}",
                diagram_ui(locale, "State owner:", "상태 소유자:"),
                label(owner)?
            ),
            40,
        );
        out.push_str("</a>");
    }
    out.push_str("</svg>");
    if out.len() > 262144 {
        return Err("local SVG render budget exceeded".into());
    }
    Ok(out)
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
pub(crate) fn element_caption(
    d: &GroundedDiagram,
    id: &str,
    texts: &[&str],
    locale: FixedLocale,
) -> String {
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
            return format!(
                "{} {}",
                diagram_ui(locale, "State owner:", "상태 소유자:"),
                label(owner)
            );
        }
    }
    if let Some(n) = d.nodes().iter().find(|n| n.id == id) {
        return format!("{} · {}", label(&n.label), n.kind.localized_text(locale));
    }
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => {
            for r in relations {
                if r.id == id {
                    return format!(
                        "{} → {} · {}",
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
                            "{}. {} → {} · {}",
                            i + 1,
                            node(&m.from),
                            node(&m.to),
                            m.kind.localized_text(locale)
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
    fn svg(
        d: &GroundedDiagram,
        texts: &[&str],
        ns: &str,
        target: &dyn Fn(&str, &DiagramLabel) -> String,
    ) -> Result<String, String> {
        super::svg(d, texts, ns, target, FixedLocale::English)
    }
    #[test]
    fn renderer_refuses_excess_work_and_markup_namespaces_before_layout() {
        let label = DiagramLabel {
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
            let label=DiagramLabel{unit:0,start_byte:0,end_byte:value.len()};
            let nodes=["first","second","third"].iter().map(|id|DiagramNode{id:(*id).into(),kind:DiagramNodeKind::Participant,label:label.clone()}).collect::<Vec<_>>();
            let messages=[("self","first","first",DiagramMessageKind::Call),("cross","first","third",DiagramMessageKind::Await),("last-self","third","third",DiagramMessageKind::Return)].iter().map(|(id,from,to,kind)|DiagramMessage{id:(*id).into(),from:(*from).into(),to:(*to).into(),kind:*kind,label:label.clone()}).collect::<Vec<_>>();
            let sequence=GroundedDiagram{id:"long-sequence".into(),title:label.clone(),caption_unit:0,semantic_fingerprint:String::new(),meaning:DiagramMeaning::Sequence{participants:nodes,groups:vec![DiagramGroup{order:DiagramOrder::Parallel,condition:Some(label.clone()),messages:messages.clone()},DiagramGroup{order:DiagramOrder::Sequential,condition:Some(label.clone()),messages}]}};
            let state=GroundedDiagram{id:"long-state".into(),title:label.clone(),caption_unit:0,semantic_fingerprint:String::new(),meaning:DiagramMeaning::State{owner:label.clone(),nodes:vec![DiagramNode{id:"only".into(),kind:DiagramNodeKind::State,label:label.clone()}],relations:vec![DiagramRelation{id:"self".into(),from:"only".into(),to:"only".into(),kind:DiagramRelationKind::Transition,label:label.clone(),condition:Some(label.clone())}]}};
            let flow=GroundedDiagram{id:"long-flow".into(),title:label.clone(),caption_unit:0,semantic_fingerprint:String::new(),meaning:DiagramMeaning::Flowchart{nodes:[("branch",DiagramNodeKind::Branch),("left",DiagramNodeKind::Process),("right",DiagramNodeKind::Process)].iter().map(|(id,kind)|DiagramNode{id:(*id).into(),kind:*kind,label:label.clone()}).collect(),relations:["left","right"].iter().map(|id|DiagramRelation{id:format!("to-{id}"),from:"branch".into(),to:(*id).into(),kind:DiagramRelationKind::ControlFlow,label:label.clone(),condition:Some(label.clone())}).collect()}};
            for (kind,d) in [("sequence",sequence),("state",state),("flow",flow)] {
                for (ui,locale) in [("en",FixedLocale::English),("ko",FixedLocale::Korean)] {
                    let ns=format!("fixture-{kind}-{script}-{ui}");
                    let svg=super::svg(&d,&[&value],&ns,&|id,_|format!("{ns}-{id}-claim"),locale)?;
                    assert!(svg.contains(&format!("aria-label=\"{value}\"")));
                    let file=format!("{ns}.svg");if let Some(root)=&root {std::fs::write(root.join(&file),svg).map_err(|e|e.to_string())?;}
                    fixtures.push(serde_json::json!({"id":ns,"file":file,"diagram":d,"prose":[{"text":value}]}));
                }
            }
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
