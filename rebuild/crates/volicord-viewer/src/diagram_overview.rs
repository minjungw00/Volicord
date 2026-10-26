//! Responsive connected maps. Meaning and origins are shared across both layouts.
//! All coordinates are computed from admitted topology and exact label lengths.
use super::{elements, esc, open, text, words, RENDERER_ID};
use std::collections::BTreeMap;
#[path = "diagram_route.rs"]
mod routing;
use volicord_projections::*;

#[derive(Default)]
struct Layout {
    width: usize,
    height: usize,
    background: String,
    order: Vec<String>,
    elements: BTreeMap<String, String>,
    routes: BTreeMap<String, String>,
}
fn lines(value: &str, pixels: usize) -> usize {
    words(value, pixels.max(16)).len()
}
fn writing(out: &mut String, x: usize, y: usize, value: &str, pixels: usize) {
    text(out, x, y, value, pixels.max(16));
}
fn panel(out: &mut String, x: usize, y: usize, w: usize, h: usize, class: &str) {
    out.push_str(&format!(
        "<rect class=\"{class}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"6\"/>"
    ));
}
fn path(out: &mut String, route: &str, marker: &str, dashed: bool) {
    // A white/foreground bridge distinguishes crossings from joins. Complete
    // endpoint names in each relation annotation also identify its direction.
    out.push_str(&format!("<path class=\"route-bridge\" d=\"{route}\"/><path class=\"relation-line\" d=\"{route}\" marker-end=\"url(#{marker})\"{} />",if dashed {" stroke-dasharray=\"6 4\""}else{""}));
}
fn index(nodes: &[DiagramNode], id: &str) -> Result<usize, String> {
    nodes
        .iter()
        .position(|n| n.id == id)
        .ok_or_else(|| "diagram endpoint unavailable".into())
}
fn relations(d: &GroundedDiagram) -> &[DiagramRelation] {
    match &d.meaning {
        DiagramMeaning::Concept { relations, .. }
        | DiagramMeaning::Component { relations, .. }
        | DiagramMeaning::Flowchart { relations, .. }
        | DiagramMeaning::State { relations, .. } => relations,
        DiagramMeaning::Sequence { .. } => &[],
    }
}
fn node_kind(
    nodes: &[DiagramNode],
    i: usize,
    texts: &[&str],
    locale: FixedLocale,
    common_type: bool,
) -> Result<String, String> {
    let name = diagram_label(&nodes[i].label, texts)?;
    let repeated = nodes
        .iter()
        .filter(|n| diagram_label(&n.label, texts) == Ok(name))
        .count()
        > 1;
    let kind = if common_type {
        ""
    } else {
        nodes[i].kind.localized_text(locale)
    };
    Ok(if repeated {
        format!("{kind} [{}]", nodes[i].id).trim_start().into()
    } else {
        kind.into()
    })
}
// Breadth layers are presentation, never inferred execution or containment.
fn graph(
    d: &GroundedDiagram,
    texts: &[&str],
    width: usize,
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    use routing::Rect;
    let nodes = d.nodes();
    let edges = relations(d);
    let label = |l: &DiagramLabel| diagram_label(l, texts);
    let endpoints = edges
        .iter()
        .map(|e| Ok((index(nodes, &e.from)?, index(nodes, &e.to)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let endpoint_name = |i: usize| -> Result<String, String> {
        let name = label(&nodes[i].label)?;
        Ok(
            if nodes.iter().filter(|n| label(&n.label) == Ok(name)).count() > 1 {
                format!("{name} [{}]", nodes[i].id)
            } else {
                name.into()
            },
        )
    };
    let associations = endpoints
        .iter()
        .map(|&(a, b)| Ok(format!("{} → {}", endpoint_name(a)?, endpoint_name(b)?)))
        .collect::<Result<Vec<String>, String>>()?;
    let mut ranks = vec![usize::MAX; nodes.len()];
    ranks[0] = 0;
    let mut queue = std::collections::VecDeque::from([0]);
    while let Some(a) = queue.pop_front() {
        for &(from, to) in &endpoints {
            if from == a && ranks[to] == usize::MAX {
                ranks[to] = ranks[a] + 1;
                queue.push_back(to);
            }
        }
    }
    while ranks.contains(&usize::MAX) {
        let mut changed = false;
        for &(a, b) in &endpoints {
            if ranks[a] != usize::MAX && ranks[b] == usize::MAX {
                ranks[b] = ranks[a] + 1;
                changed = true;
            } else if ranks[b] != usize::MAX && ranks[a] == usize::MAX {
                ranks[a] = ranks[b] + 1;
                changed = true;
            }
        }
        if !changed {
            if let Some(i) = ranks.iter().position(|r| *r == usize::MAX) {
                ranks[i] = 0;
            }
        }
    }
    let cols = if width < 400 { 2 } else { 3 };
    let margin = 24;
    let gap = 16;
    let content = width - margin * 2;
    let cell = (content - gap * (cols - 1)) / cols;
    let counts = (0..nodes.len())
        .map(|b| endpoints.iter().filter(|(_, to)| *to == b).count())
        .collect::<Vec<_>>();
    let mut layout = Layout {
        width,
        ..Layout::default()
    };
    let mut y = 20;
    // A uniform type belongs to the whole figure. Mixed types remain attached
    // to their elements; no predicate, endpoint or semantic selection is elided.
    let uniform_nodes = nodes.len() > 1 && nodes.iter().all(|n| n.kind == nodes[0].kind);
    let uniform_edges = edges
        .first()
        .map(|first| edges.len() > 1 && edges.iter().all(|e| e.kind == first.kind))
        .unwrap_or(false);
    // Local reminders repeat the admitted owner, never infer it from topology.
    // They also work in static exports after the selectable owner scrolls away.
    let owner = if let DiagramMeaning::State { owner, .. } = &d.meaning {
        format!(
            "{} {}",
            diagram_ui(locale, "State owner:", "상태 소유자:"),
            label(owner)?
        )
    } else {
        String::new()
    };
    for value in [
        uniform_nodes.then(|| {
            format!(
                "{} {}",
                diagram_ui(locale, "All nodes:", "모든 노드:"),
                nodes[0].kind.localized_text(locale)
            )
        }),
        uniform_edges.then(|| {
            format!(
                "{} {}",
                diagram_ui(locale, "All relations:", "모든 관계:"),
                edges[0].kind.localized_text(locale)
            )
        }),
    ]
    .into_iter()
    .flatten()
    {
        writing(
            &mut layout.background,
            width / 2,
            y + 16,
            &value,
            width - 32,
        );
        y += 20 * lines(&value, width - 32) + 12;
    }
    // State ownership retains its own selectable origin after the shared keys.
    if let DiagramMeaning::State { owner, .. } = &d.meaning {
        let value = format!(
            "{} {}",
            diagram_ui(locale, "State owner:", "상태 소유자:"),
            label(owner)?
        );
        let mut out = String::new();
        writing(&mut out, width / 2, y + 16, &value, width - 32);
        y += 20 * lines(&value, width - 32) + 24;
        layout.elements.insert("state:owner".into(), out);
        layout.order.push("state:owner".into());
    }
    let blank = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let mut node_boxes = vec![blank; nodes.len()];
    let mut edge_boxes = vec![blank; edges.len()];
    for rank in 0..=*ranks.iter().max().unwrap_or(&0) {
        let group = (0..nodes.len())
            .filter(|i| ranks[*i] == rank)
            .collect::<Vec<_>>();
        for row in group.chunks(cols) {
            let mut row_h = 0;
            for (col, &i) in row.iter().enumerate() {
                let tw = if nodes[i].kind == DiagramNodeKind::Branch {
                    cell / 2 - 12
                } else {
                    cell - 8
                };
                let kind = node_kind(nodes, i, texts, locale, uniform_nodes)?;
                let h = (24 + 20 * (lines(label(&nodes[i].label)?, tw) + lines(&kind, tw)))
                    + 20 * lines(&owner, tw);
                let h = h.max(counts[i] * 20 + 24);
                let h = if nodes[i].kind == DiagramNodeKind::Branch {
                    h * 2
                } else {
                    h
                };
                let x = margin
                    + (content - (row.len() * cell + (row.len() - 1) * gap)) / 2
                    + col * (cell + gap);
                node_boxes[i] = Rect { x, y, w: cell, h };
                row_h = row_h.max(h);
                layout.order.push(nodes[i].id.clone());
            }
            y += row_h + 28;
        }
        let edge_start = y;
        let mut col = 0;
        let mut row_h = 0;
        for (i, e) in edges
            .iter()
            .enumerate()
            .filter(|(_, e)| group.contains(&index(nodes, &e.from).unwrap_or(usize::MAX)))
        {
            let guard = e.condition.as_ref().map(label).transpose()?;
            let needs_wide = width < 400
                || std::iter::once(label(&e.label)?)
                    .chain(std::iter::once(associations[i].as_str()))
                    .chain(guard)
                    .flat_map(str::split_whitespace)
                    .any(|word| super::measure_text(word) > cell);
            if needs_wide && col > 0 {
                y += row_h + 24;
                row_h = 0;
                col = 0;
            }
            let w = if needs_wide { content } else { cell };
            let x = if needs_wide {
                margin
            } else {
                margin + col * (cell + gap)
            };
            let kind = if uniform_edges {
                ""
            } else {
                e.kind.localized_text(locale)
            };
            let h = 24
                + 20 * (lines(&associations[i], w - 12)
                    + lines(label(&e.label)?, w - 12)
                    + lines(kind, w - 12)
                    + lines(&owner, w - 12))
                + guard.map(|v| 20 * lines(v, w - 12) + 24).unwrap_or(0);
            edge_boxes[i] = Rect { x, y, w, h };
            layout.order.push(e.id.clone());
            if e.condition.is_some() {
                layout.order.push(format!("{}:condition", e.id));
            }
            row_h = row_h.max(h);
            col += 1;
            if needs_wide || col == cols {
                y += row_h + 24;
                col = 0;
                row_h = 0;
            }
        }
        if col > 0 {
            y += row_h + 24;
        }
        // Compare outgoing routes beneath their own source column. This is
        // geometric proximity only: shared nodes, direction and full endpoint
        // names remain, and no column becomes an execution/ownership boundary.
        // Full-width labels and narrow layouts retain full reading width.
        let outgoing = endpoints
            .iter()
            .enumerate()
            .filter(|(_, (a, _))| group.contains(a))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if width >= 400
            && group.len() > 1
            && group.len() <= cols
            && outgoing.iter().all(|i| edge_boxes[*i].w == cell)
        {
            let mut bottoms = vec![edge_start; nodes.len()];
            for i in outgoing {
                let a = endpoints[i].0;
                edge_boxes[i].x = node_boxes[a].x;
                edge_boxes[i].y = bottoms[a];
                bottoms[a] += edge_boxes[i].h + 24;
            }
            y = group
                .iter()
                .map(|a| bottoms[*a])
                .max()
                .unwrap_or(edge_start);
        }
        y += 16;
    }
    layout.height = y + 16;
    let obstacles = node_boxes
        .iter()
        .chain(&edge_boxes)
        .map(|r| r.padded())
        .collect::<Vec<_>>();
    // Reserve every future connector port. Otherwise a previous crossing can
    // land exactly on a later start, leaving no distinct direction to escape.
    let mut ports = Vec::new();
    let mut incoming_port = vec![0; nodes.len()];
    for (i, &(a, b)) in endpoints.iter().enumerate() {
        let from = node_boxes[a];
        let to = node_boxes[b];
        let card = edge_boxes[i];
        ports.extend([
            (from.x + from.w / 2, from.y + from.h + 8),
            (card.x + card.w / 2, card.y - 8),
            (card.x + card.w / 2, card.y + card.h + 8),
            (
                to.x + to.w + 8,
                to.y + to.h / 2 + incoming_port[b] * 20 - counts[b].saturating_sub(1) * 10,
            ),
        ]);
        incoming_port[b] += 1;
    }
    let route = |start, end, old: &[routing::Segment], group| {
        let mut protected = obstacles.clone();
        protected.extend(
            ports
                .iter()
                .filter(|p| **p != start && **p != end)
                .map(|p| Rect {
                    x: p.0 - 2,
                    y: p.1 - 2,
                    w: 4,
                    h: 4,
                }),
        );
        routing::route(start, end, &protected, old, group, width, layout.height)
    };
    let mut routes = Vec::new();
    let mut incoming = vec![0; nodes.len()];
    for (i, e) in edges.iter().enumerate() {
        let (a, b) = endpoints[i];
        let from = node_boxes[a];
        let to = node_boxes[b];
        let card = edge_boxes[i];
        let sy = from.y + from.h;
        let sx = from.x + from.w / 2;
        let ly = card.y;
        let lx = card.x + card.w / 2;
        let ty = to.y + to.h / 2 + incoming[b] * 20 - counts[b].saturating_sub(1) * 10;
        incoming[b] += 1;
        let tx = to.x + to.w
            - if nodes[b].kind == DiagramNodeKind::Branch {
                ty.abs_diff(to.y + to.h / 2) * to.w / to.h
            } else {
                0
            };
        let first = route((sx, sy + 8), (lx, ly - 8), &routes, a)
            .map_err(|error| format!("{} input: {error}", e.id))?;
        let mut input = vec![(sx, sy)];
        input.extend(first);
        input.push((lx, ly));
        routing::remember(&input, a, &mut routes);
        let cy = card.y + card.h;
        let second = route(
            (lx, cy + 8),
            (to.x + to.w + 8, ty),
            &routes,
            nodes.len() + i,
        )
        .map_err(|error| format!("{} output: {error}", e.id))?;
        let mut output = vec![(lx, cy)];
        output.extend(second);
        output.push((tx, ty));
        routing::remember(&output, nodes.len() + i, &mut routes);
        let mut out = String::new();
        path(
            &mut out,
            &(routing::path(&input) + " " + &routing::path(&output)),
            marker,
            e.kind == DiagramRelationKind::DataFlow,
        );
        // The native fragment belongs to the readable annotation. Including
        // a long backward route in its bounds can land a narrow reader above
        // the actual wording. The route remains an alias to the same claim.
        layout.routes.insert(e.id.clone(), out);
        let mut out = String::new();
        panel(&mut out, card.x, card.y, card.w, card.h, "edge-key");
        writing(&mut out, lx, card.y + 20, &associations[i], card.w - 12);
        let py = card.y + 20 + 20 * lines(&associations[i], card.w - 12);
        writing(&mut out, lx, py, label(&e.label)?, card.w - 12);
        let ky = py + 20 * lines(label(&e.label)?, card.w - 12);
        let kind = if uniform_edges {
            ""
        } else {
            e.kind.localized_text(locale)
        };
        if !kind.is_empty() {
            writing(&mut out, lx, ky, kind, card.w - 12);
        }
        let context_y = ky + 20 * lines(kind, card.w - 12);
        if !owner.is_empty() {
            writing(&mut out, lx, context_y, &owner, card.w - 12);
        }
        if let Some(guard) = &e.condition {
            let mut g = String::new();
            writing(
                &mut g,
                lx,
                context_y + 20 * lines(&owner, card.w - 12) + 20,
                label(guard)?,
                card.w - 12,
            );
            layout.elements.insert(format!("{}:condition", e.id), g);
        }
        layout.elements.insert(e.id.clone(), out);
    }
    for (i, node) in nodes.iter().enumerate() {
        let r = node_boxes[i];
        let mut out = String::new();
        if node.kind == DiagramNodeKind::Branch {
            out.push_str(&format!(
                "<polygon class=\"participant-box\" points=\"{},{} {},{} {},{} {},{}\"/>",
                r.x + r.w / 2,
                r.y,
                r.x + r.w,
                r.y + r.h / 2,
                r.x + r.w / 2,
                r.y + r.h,
                r.x,
                r.y + r.h / 2
            ));
        } else {
            panel(&mut out, r.x, r.y, r.w, r.h, "participant-box");
        }
        let tw = if node.kind == DiagramNodeKind::Branch {
            r.w / 2 - 12
        } else {
            r.w - 8
        };
        let name = label(&node.label)?;
        let kind = node_kind(nodes, i, texts, locale, uniform_nodes)?;
        let th = 20 * (lines(name, tw) + lines(&kind, tw) + lines(&owner, tw));
        let top = r.y + (r.h - th) / 2 + 16;
        writing(&mut out, r.x + r.w / 2, top, name, tw);
        if !kind.is_empty() {
            writing(
                &mut out,
                r.x + r.w / 2,
                top + 20 * lines(name, tw),
                &kind,
                tw,
            );
        }
        if !owner.is_empty() {
            writing(
                &mut out,
                r.x + r.w / 2,
                top + 20 * (lines(name, tw) + lines(&kind, tw)),
                &owner,
                tw,
            );
        }
        layout.elements.insert(node.id.clone(), out);
    }
    Ok(layout)
}

fn sequence(
    d: &GroundedDiagram,
    texts: &[&str],
    width: usize,
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    let DiagramMeaning::Sequence {
        participants: nodes,
        groups,
    } = &d.meaning
    else {
        return Err("sequence unavailable".into());
    };
    let label = |l: &DiagramLabel| diagram_label(l, texts);
    let n = nodes.len();
    let endpoint = |i: usize| -> Result<String, String> {
        let name = label(&nodes[i].label)?;
        Ok(
            if nodes.iter().filter(|n| label(&n.label) == Ok(name)).count() > 1 {
                format!("{name} [{}]", nodes[i].id)
            } else {
                name.into()
            },
        )
    };
    let lane = (width - 24) / n;
    let node_width = lane - 2;
    let repeated = nodes
        .iter()
        .map(|node| {
            label(&node.label)
                .map(|name| nodes.iter().filter(|n| label(&n.label) == Ok(name)).count() > 1)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let header = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            label(&n.label).map(|s| {
                20 * (lines(s, (node_width - 4).max(16))
                    + if repeated[i] {
                        lines(&n.id, (node_width - 4).max(16)) + 1
                    } else {
                        0
                    })
                    + 24
            })
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(48);
    let mut layout = Layout {
        width,
        ..Layout::default()
    };
    let x = |i: usize| 12 + lane * i + lane / 2;
    for (i, node) in nodes.iter().enumerate() {
        layout.order.push(node.id.clone());
        let mut out = String::new();
        panel(
            &mut out,
            x(i) - node_width / 2,
            12,
            node_width,
            header,
            "participant-box",
        );
        writing(
            &mut out,
            x(i),
            34,
            label(&node.label)?,
            (node_width - 4).max(16),
        );
        if repeated[i] {
            writing(
                &mut out,
                x(i),
                54 + 20 * lines(label(&node.label)?, (node_width - 4).max(16)),
                &node.id,
                (node_width - 4).max(16),
            );
        }
        layout.elements.insert(node.id.clone(), out);
    }
    let mut y = header + 36;
    for (gi, group) in groups.iter().enumerate() {
        let start = y;
        let order = match group.order {
            DiagramOrder::Sequential => diagram_ui(locale, "strict · sequential", "strict · 순차"),
            DiagramOrder::Parallel => {
                diagram_ui(locale, "par · no relative order", "par · 상대 순서 없음")
            }
        };
        let heading = format!("{}. {order}", gi + 1);
        let heading_height = 20 * lines(&heading, width - 32) + 12;
        // Mask lifelines behind the complete group heading and guard, just as
        // behind message wording. A line through a condition changes its scope.
        let guard_text = group
            .condition
            .as_ref()
            .map(|guard| {
                label(guard)
                    .map(|s| format!("opt · {} {s}", diagram_ui(locale, "condition:", "조건:")))
            })
            .transpose()?;
        let guard_height = guard_text
            .as_ref()
            .map_or(0, |s| 20 * lines(s, width - 32) + 16);
        panel(
            &mut layout.background,
            8,
            y,
            width - 16,
            heading_height + guard_height,
            "message-label",
        );
        writing(
            &mut layout.background,
            width / 2,
            y + 18,
            &heading,
            width - 32,
        );
        y += heading_height;
        if let Some(value) = &guard_text {
            layout.order.push(format!("guard:{}", gi + 1));
            let mut out = String::new();
            writing(&mut out, width / 2, y + 18, value, width - 32);
            y += guard_height;
            layout.elements.insert(format!("guard:{}", gi + 1), out);
        }
        for (mi, message) in group.messages.iter().enumerate() {
            layout.order.push(message.id.clone());
            let a = index(nodes, &message.from)?;
            let b = index(nodes, &message.to)?;
            let mut out = String::new();
            let name = label(&message.label)?;
            // Long scenes must remain readable after the participant headers
            // scroll away. These names repeat sealed endpoints; no new label or
            // ordering is inferred. Equal names keep their admitted identities.
            let association = format!("{} → {}", endpoint(a)?, endpoint(b)?);
            let kind = format!(
                "{}.{} · {}",
                gi + 1,
                mi + 1,
                message.kind.localized_text(locale)
            );
            // A group ordinal alone cannot recover strict/par/opt meaning
            // when the header is outside the reading viewport. Repeat only
            // this message's sealed scope; never carry a previous guard forward.
            let context = if let Some(guard) = &guard_text {
                format!("{order} · {guard}")
            } else {
                order.to_owned()
            };
            let context = format!("{kind} · {context}");
            let association_height = 20 * lines(&association, width - 48);
            let h = 20 * (lines(name, width - 48) + lines(&context, width - 48))
                + association_height
                + 16;
            panel(&mut out, 16, y, width - 32, h, "message-label");
            writing(&mut out, width / 2, y + 20, &association, width - 48);
            writing(
                &mut out,
                width / 2,
                y + 20 + association_height,
                name,
                width - 48,
            );
            writing(
                &mut out,
                width / 2,
                y + 20 + association_height + 20 * lines(name, width - 48),
                &context,
                width - 48,
            );
            let baseline = y + h + 16;
            let route = if a == b {
                // The last participant's loop turns left, staying in bounds.
                let loop_x = if a + 1 == n { x(a) - 18 } else { x(a) + 18 };
                format!("M{} {baseline}H{loop_x}V{}H{}", x(a), baseline + 18, x(a))
            } else {
                format!("M{} {baseline}H{}", x(a), x(b))
            };
            path(
                &mut out,
                &route,
                marker,
                message.kind == DiagramMessageKind::Return,
            );
            layout.elements.insert(message.id.clone(), out);
            y = baseline + 38;
            if group.order == DiagramOrder::Parallel && mi + 1 < group.messages.len() {
                layout.background.push_str(&format!("<path class=\"relation-line parallel-operand\" d=\"M8 {y}H{}\" stroke-dasharray=\"3 5\"/>",width-8));
                y += 16;
            }
        }
        layout.background.push_str(&format!(
            "<rect class=\"sequence-frame\" x=\"4\" y=\"{start}\" width=\"{}\" height=\"{}\"/>",
            width - 8,
            y - start
        ));
        y += 20;
    }
    layout.height = y + 16;
    // Lifelines precede opaque labels; they never run through the visible text.
    let mut lifelines = String::new();
    for i in 0..n {
        lifelines.push_str(&format!(
            "<line class=\"lifeline\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            x(i),
            header + 24,
            x(i),
            layout.height - 16
        ));
    }
    layout.background = lifelines + &layout.background;
    Ok(layout)
}
pub(super) fn svg(
    d: &GroundedDiagram,
    texts: &[&str],
    ns: &str,
    target: &dyn Fn(&str, &DiagramLabel) -> String,
    locale: FixedLocale,
) -> Result<String, String> {
    let marker = format!("{ns}:arrow");
    let layout = |width| {
        if matches!(d.meaning, DiagramMeaning::Sequence { .. }) {
            sequence(d, texts, width, &marker, locale)
        } else {
            graph(d, texts, width, &marker, locale)
        }
    };
    let wide = layout(760)?;
    let narrow = layout(272)?;
    let title = diagram_label(&d.title, texts)?;
    // No viewBox: each variant keeps native 16px text. CSS selects geometry,
    // never scales an overview to a miniature. The semantic anchors exist once.
    let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" style=\"--map-wide-width:{}px;--map-wide-height:{}px;--map-narrow-width:{}px;--map-narrow-height:{}px\" class=\"connected-overview\" role=\"group\" aria-label=\"{}\" data-renderer=\"{RENDERER_ID}\"><defs><marker id=\"{marker}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M0 0L10 5L0 10Z\"/></marker></defs><g class=\"diagram-content\"><g class=\"map-wide\">{}</g><g class=\"map-narrow\">{}</g>",wide.width,wide.height,wide.width,wide.height,narrow.width,narrow.height,esc(title),wide.background,narrow.background);
    let labels = elements(d).into_iter().collect::<BTreeMap<_, _>>();
    if wide.order.len() != labels.len() || wide.order != narrow.order {
        return Err("incomplete responsive elements".into());
    }
    for (id, route) in &wide.routes {
        let label = labels.get(id).ok_or("missing route label")?;
        let name =
            super::element_caption(d, id, texts, locale) + ": " + diagram_label(label, texts)?;
        out.push_str(&format!(
            "<a class=\"diagram-route diagram-text-link\" data-element=\"{}-{}\" href=\"#{}\" tabindex=\"-1\" aria-label=\"{}\"><g class=\"map-wide\">{route}</g><g class=\"map-narrow\">{}</g></a>",
            esc(ns), esc(id), esc(&target(id, label)), esc(&name), narrow.routes.get(id).ok_or("missing narrow route")?
        ));
    }
    for id in &wide.order {
        let label = labels.get(id).ok_or("missing element label")?;
        let name =
            super::element_caption(d, id, texts, locale) + ": " + diagram_label(label, texts)?;
        open(&mut out, ns, id, label, &name, target);
        out.push_str(&format!(
            "<g class=\"map-wide\">{}</g><g class=\"map-narrow\">{}</g></a>",
            wide.elements.get(id).ok_or("missing wide element")?,
            narrow.elements.get(id).ok_or("missing narrow element")?
        ));
    }
    out.push_str("</g></svg>");
    if out.len() > 262144 {
        return Err("local SVG output budget exceeded".into());
    }
    Ok(out)
}
