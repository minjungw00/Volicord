//! Condensed directed layers, content-sized rows and facing-port connections.
//! Label shelves reserve reading space; labels never become intermediate nodes.
use super::{index, node_kind, panel, path, relations, routing, Layout};
use crate::diagram::{identifier_text, measure_at, node_header, sized_text, words_at};
use routing::{Point, Rect};
use volicord_projections::*;

fn count(s: &str, w: usize, px: usize) -> usize {
    words_at(s, w.max(16), px).len()
}
fn quiet(out: &mut String, x: usize, y: usize, s: &str, w: usize) {
    out.push_str("<g class=\"diagram-node-kind\">");
    sized_text(out, x, y, s, w, 12, 18);
    out.push_str("</g>");
}
fn action(out: &mut String, x: usize, y: usize, s: &str, w: usize) {
    sized_text(out, x, y, s, w, 13, 20);
}

fn ranks(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    // The bounded transitive closure gives stable SCC identities even for cycles.
    let mut reach = vec![vec![false; n]; n];
    for (i, row) in reach.iter_mut().enumerate() {
        row[i] = true;
    }
    for &(a, b) in edges {
        reach[a][b] = true;
    }
    for k in 0..n {
        for a in 0..n {
            for b in 0..n {
                reach[a][b] |= reach[a][k] && reach[k][b];
            }
        }
    }
    let component = (0..n)
        .map(|a| (0..n).find(|&b| reach[a][b] && reach[b][a]).unwrap_or(a))
        .collect::<Vec<_>>();
    let mut rank = vec![0usize; n];
    for _ in 0..n {
        for &(a, b) in edges {
            if component[a] != component[b] {
                rank[component[b]] = rank[component[b]].max(rank[component[a]] + 1);
            }
        }
    }
    let mut result = component.iter().map(|&c| rank[c]).collect::<Vec<_>>();
    // A separate root can sit near its target without manufacturing containment.
    for a in 0..n {
        if !edges.iter().any(|&(_, b)| b == a) {
            if let Some(next) = edges
                .iter()
                .filter(|&&(f, _)| f == a)
                .map(|&(_, b)| result[b])
                .min()
            {
                result[a] = next.saturating_sub(1);
            }
        }
    }
    result
}

fn port(rect: Rect, other: Rect, ordinal: usize, total: usize, branch: bool) -> (Point, Point) {
    let offset = (rect.w * (ordinal + 1) / (total + 1)).clamp(4, rect.w - 4);
    if other.y >= rect.y + rect.h + 8 {
        let boundary = if branch {
            rect.y + rect.h / 2 + rect.h * offset.min(rect.w - offset) / rect.w
        } else {
            rect.y + rect.h
        };
        (
            (rect.x + offset, boundary),
            (rect.x + offset, rect.y + rect.h + 8),
        )
    } else if rect.y >= other.y + other.h + 8 {
        let boundary = if branch {
            rect.y + rect.h / 2 - rect.h * offset.min(rect.w - offset) / rect.w
        } else {
            rect.y
        };
        ((rect.x + offset, boundary), (rect.x + offset, rect.y - 8))
    } else {
        let offset = (rect.h * (ordinal + 1) / (total + 1)).clamp(4, rect.h - 4);
        if other.x > rect.x {
            let boundary = if branch {
                rect.x + rect.w / 2 + rect.w * offset.min(rect.h - offset) / rect.h
            } else {
                rect.x + rect.w
            };
            (
                (boundary, rect.y + offset),
                (rect.x + rect.w + 8, rect.y + offset),
            )
        } else {
            let boundary = if branch {
                rect.x + rect.w / 2 - rect.w * offset.min(rect.h - offset) / rect.h
            } else {
                rect.x
            };
            ((boundary, rect.y + offset), (rect.x - 8, rect.y + offset))
        }
    }
}

pub(super) fn graph<'a>(
    d: &'a GroundedDiagram,
    texts: &[&str],
    width: usize,
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    let nodes = d.nodes();
    let edges = relations(d);
    let label = |l: &'a DiagramLabel| diagram_label(l, texts);
    let endpoints = edges
        .iter()
        .map(|e| Ok((index(nodes, &e.from)?, index(nodes, &e.to)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let mut rank = ranks(nodes.len(), &endpoints);
    if matches!(d.meaning, DiagramMeaning::Concept { .. }) {
        for (i, node) in nodes.iter().enumerate() {
            let roles = edges
                .iter()
                .filter(|e| e.from == node.id && e.kind == DiagramRelationKind::RoleOf)
                .collect::<Vec<_>>();
            if node.kind == DiagramNodeKind::ContextRole
                && !endpoints.iter().any(|&(_, b)| b == i)
                && roles.len() == 1
            {
                rank[i] = rank[index(nodes, &roles[0].to)?];
            }
        }
    }
    let incoming = (0..nodes.len())
        .map(|i| endpoints.iter().filter(|&&(_, b)| b == i).count())
        .collect::<Vec<_>>();
    let outgoing = (0..nodes.len())
        .map(|i| endpoints.iter().filter(|&&(a, _)| a == i).count())
        .collect::<Vec<_>>();
    let uniform_nodes = nodes.iter().all(|n| n.kind == nodes[0].kind);
    let uniform_edges = edges.iter().all(|e| e.kind == edges[0].kind);
    let owner = match &d.meaning {
        DiagramMeaning::State { owner, .. } => format!(
            "{} {}",
            diagram_ui(locale, "State owner:", "상태 소유자:"),
            label(owner)?
        ),
        _ => String::new(),
    };
    let kinds = nodes
        .iter()
        .map(|node| node_kind(node, locale, uniform_nodes))
        .collect::<Vec<_>>();
    let mut layout = Layout {
        width,
        ..Layout::default()
    };
    let mut y = 24;
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
        quiet(
            &mut layout.background,
            width / 2,
            y + 12,
            &value,
            width - 48,
        );
        y += 18 * count(&value, width - 48, 12) + 10;
    }
    if !owner.is_empty() {
        let mut s = String::new();
        sized_text(&mut s, width / 2, y + 16, &owner, width - 48, 16, 24);
        layout.elements.insert("state:owner".into(), s);
        layout.order.push("state:owner".into());
        y += 24 * count(&owner, width - 48, 16) + 20;
    }
    let blank = Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    };
    let mut boxes = vec![blank; nodes.len()];
    let mut labels = vec![blank; edges.len()];
    let content = width - 48;
    // Barycenter ordering within directed layers; SCC peers retain stable ties.
    let mut order = (0..nodes.len()).collect::<Vec<_>>();
    for _ in 0..4 {
        for level in 0..=*rank.iter().max().unwrap_or(&0) {
            let positions = (0..nodes.len())
                .map(|i| order.iter().position(|&x| x == i).unwrap_or(i))
                .collect::<Vec<_>>();
            let mut layer = order
                .iter()
                .copied()
                .filter(|&i| rank[i] == level)
                .collect::<Vec<_>>();
            layer.sort_by_key(|&i| {
                let neighbors = endpoints
                    .iter()
                    .filter_map(|&(a, b)| {
                        if b == i && rank[a] < level {
                            Some(positions[a])
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                (
                    neighbors.iter().sum::<usize>() * 1024 / neighbors.len().max(1),
                    i,
                )
            });
            let mut iter = layer.into_iter();
            for slot in &mut order {
                if rank[*slot] == level {
                    if let Some(i) = iter.next() {
                        *slot = i;
                    }
                }
            }
        }
    }
    for level in 0..=*rank.iter().max().unwrap_or(&0) {
        let group = order
            .iter()
            .copied()
            .filter(|&i| rank[i] == level)
            .collect::<Vec<_>>();
        let mut rows: Vec<Vec<(usize, usize)>> = Vec::new();
        let mut row = Vec::new();
        let mut used = 0;
        for i in group {
            let name = label(&nodes[i].label)?;
            let identifier = nodes[i]
                .identifier
                .as_ref()
                .map(label)
                .transpose()?
                .unwrap_or("");
            let desired = (measure_at(name, 16)
                .min(184)
                .max(measure_at(identifier, 12).min(160))
                + 24)
                .max(if width >= 520 { 144 } else { 96 })
                .max((incoming[i] + outgoing[i]) * 8 + 24)
                .min(content);
            if !row.is_empty() && used + 32 + desired > content {
                rows.push(std::mem::take(&mut row));
                used = 0;
            }
            used += desired + if row.is_empty() { 0 } else { 32 };
            row.push((i, desired));
        }
        if !row.is_empty() {
            rows.push(row);
        }
        for mut row in rows {
            let row_sources = row.iter().map(|&(i, _)| i).collect::<Vec<_>>();
            let total = row.iter().map(|&(_, w)| w).sum::<usize>() + 32 * (row.len() - 1);
            let share = ((content - total) / row.len()).min(48);
            for (_, w) in &mut row {
                *w += share;
            }
            let total = total + share * row.len();
            let mut x = 24 + (content - total) / 2;
            let mut h = 0;
            for (i, w) in row {
                let tw = if nodes[i].kind == DiagramNodeKind::Branch {
                    w / 2 - 8
                } else {
                    w - 16
                };
                let primary = 24 * count(label(&nodes[i].label)?, tw, 16);
                let secondary = nodes[i]
                    .identifier
                    .as_ref()
                    .map(|l| label(l).map(|s| 18 * count(s, tw, 12)))
                    .transpose()?
                    .unwrap_or(0);
                let height = primary
                    + secondary
                    + 18 * count(&kinds[i], tw, 12)
                    + 20 * count(&owner, tw, 13)
                    + 24;
                let height = if nodes[i].kind == DiagramNodeKind::Branch {
                    height * 2
                } else {
                    height
                };
                let height = height.max((incoming[i] + outgoing[i]) * 12 + 24);
                boxes[i] = Rect { x, y, w, h: height };
                h = h.max(height);
                x += w + 32;
            }
            y += h + 20;
            // Pack complete label rectangles into shelves around their own edge's
            // midpoint. Geometry depends on actual wording, topology and role sizes.
            let mut shelves: Vec<Vec<(usize, usize, usize, usize)>> = Vec::new();
            for (i, e) in edges
                .iter()
                .enumerate()
                .filter(|(i, _)| row_sources.contains(&endpoints[*i].0))
            {
                let a = boxes[endpoints[i].0];
                let b = endpoints[i].1;
                let target_hint = if boxes[b].w > 0 {
                    boxes[b].x + boxes[b].w / 2
                } else {
                    let peers = order
                        .iter()
                        .filter(|&&j| rank[j] == rank[b])
                        .copied()
                        .collect::<Vec<_>>();
                    24 + content * (peers.iter().position(|&j| j == b).unwrap_or(0) + 1)
                        / (peers.len() + 1)
                };
                let kind = if uniform_edges {
                    ""
                } else {
                    e.kind.localized_text(locale)
                };
                let guard = e.condition.as_ref().map(label).transpose()?.unwrap_or("");
                let w = (measure_at(label(&e.label)?, 13)
                    .max(measure_at(guard, 13))
                    .max(measure_at(kind, 12))
                    + 16)
                    .clamp(64, content.min(280));
                let center = if rank[b] > rank[endpoints[i].0] {
                    if incoming[b] > 1 {
                        a.x + a.w / 2
                    } else {
                        target_hint
                    }
                } else {
                    (a.x + a.w / 2 + target_hint) / 2
                };
                let x = center.saturating_sub(w / 2).clamp(24, width - 24 - w);
                let h = 20 * count(label(&e.label)?, w - 12, 13)
                    + 18 * count(kind, w - 12, 12)
                    + 20 * count(guard, w - 12, 13)
                    + 20 * count(&owner, w - 12, 13)
                    + 12;
                let slot = shelves
                    .iter()
                    .position(|s| {
                        s.iter()
                            .all(|&(_, sx, sw, _)| x + w + 12 <= sx || sx + sw + 12 <= x)
                    })
                    .unwrap_or(shelves.len());
                if slot == shelves.len() {
                    shelves.push(Vec::new());
                }
                shelves[slot].push((i, x, w, h));
            }
            for shelf in shelves {
                let h = shelf.iter().map(|&(_, _, _, h)| h).max().unwrap_or(0);
                for (i, x, w, h) in shelf {
                    labels[i] = Rect { x, y, w, h };
                }
                y += h + 24;
            }
            y += 20;
        }
    }
    layout.height = y + 24;
    let obstacles = boxes
        .iter()
        .chain(&labels)
        .map(|b| b.padded())
        .collect::<Vec<_>>();
    let mut emitted = vec![0; nodes.len()];
    let mut received = vec![0; nodes.len()];
    let mut connections = Vec::new();
    for (i, _) in edges.iter().enumerate() {
        let (a, b) = endpoints[i];
        let other_a = if a == b {
            Rect {
                x: boxes[a].x + boxes[a].w + 1,
                ..boxes[a]
            }
        } else {
            boxes[b]
        };
        let other_b = if a == b { other_a } else { boxes[a] };
        let (source, s) = port(
            boxes[a],
            other_a,
            emitted[a],
            outgoing[a] + incoming[a],
            nodes[a].kind == DiagramNodeKind::Branch,
        );
        let (target, t) = port(
            boxes[b],
            other_b,
            outgoing[b] + received[b],
            outgoing[b] + incoming[b],
            nodes[b].kind == DiagramNodeKind::Branch,
        );
        emitted[a] += 1;
        received[b] += 1;
        let r = labels[i];
        let top = (r.x - 8, r.y - 4);
        let bottom = (r.x - 8, r.y + r.h + 4);
        let (entry, exit) = if boxes[b].y >= boxes[a].y {
            (top, bottom)
        } else {
            (bottom, top)
        };
        connections.push((source, s, target, t, entry, exit));
    }
    let reserved = connections
        .iter()
        .flat_map(|&(_, s, _, t, entry, exit)| [s, t, entry, exit])
        .collect::<Vec<_>>();
    let mut old = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        let (source, s, target, t, entry, exit) = connections[i];
        let r = labels[i];
        let protect = |start: Point, end: Point| {
            let mut protected = obstacles.clone();
            protected.extend(reserved.iter().filter(|&&p| p != start && p != end).map(
                |&(x, y)| Rect {
                    x: x.saturating_sub(2),
                    y: y.saturating_sub(2),
                    w: 4,
                    h: 4,
                },
            ));
            protected
        };
        // The predicate sits beside a reserved segment of its actual directed
        // route. There is no separate leader to follow to a remote connection.
        let first = routing::route(
            s,
            entry,
            &protect(s, entry),
            &old,
            2 * i + 2,
            width,
            layout.height,
        )
        .map_err(|s| format!("{} entry route: {s}", e.id))?;
        let last = routing::route(
            exit,
            t,
            &protect(exit, t),
            &old,
            2 * i + 2,
            width,
            layout.height,
        )
        .map_err(|s| format!("{} exit route: {s}", e.id))?;
        let mut points = vec![source];
        points.extend(first);
        points.push(exit);
        points.extend(last);
        points.push(target);
        // Each independently routed leg includes its endpoints. Remove the
        // shared boundary before path compaction, so a duplicate exit cannot
        // swallow the vertical segment beside the predicate.
        points.dedup();
        routing::remember(&points, 2 * i + 2, &mut old);
        let mut route = String::new();
        path(
            &mut route,
            &routing::path(&points),
            marker,
            e.kind == DiagramRelationKind::DataFlow,
        );
        layout.routes.insert(e.id.clone(), route);
        let mut out = String::new();
        panel(&mut out, r.x, r.y, r.w, r.h, "edge-key");
        out.push_str("<g class=\"relation-wording\">");
        let join = (r.x + r.w / 2, r.y - 8);
        let mut ty = r.y + 16;
        action(&mut out, join.0, ty, label(&e.label)?, r.w - 12);
        ty += 20 * count(label(&e.label)?, r.w - 12, 13);
        if !uniform_edges {
            quiet(
                &mut out,
                join.0,
                ty,
                e.kind.localized_text(locale),
                r.w - 12,
            );
            ty += 18 * count(e.kind.localized_text(locale), r.w - 12, 12);
        }
        if !owner.is_empty() {
            out.push_str("<g class=\"diagram-condition\">");
            action(&mut out, join.0, ty, &owner, r.w - 12);
            out.push_str("</g>");
            ty += 20 * count(&owner, r.w - 12, 13);
        }
        if let Some(condition) = &e.condition {
            let mut guard = String::from("<g class=\"diagram-condition\">");
            action(&mut guard, join.0, ty, label(condition)?, r.w - 12);
            guard.push_str("</g>");
            layout.elements.insert(format!("{}:condition", e.id), guard);
        }
        out.push_str("</g>");
        layout.elements.insert(e.id.clone(), out);
    }
    for (i, node) in nodes.iter().enumerate() {
        let r = boxes[i];
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
            panel(
                &mut out,
                r.x,
                r.y,
                r.w,
                r.h,
                if node.kind == DiagramNodeKind::ReadingGroup {
                    "participant-box node-group"
                } else {
                    "participant-box"
                },
            );
        }
        let tw = if node.kind == DiagramNodeKind::Branch {
            r.w / 2 - 8
        } else {
            r.w - 16
        };
        let body_height = 24 * count(label(&node.label)?, tw, 16)
            + node
                .identifier
                .as_ref()
                .map(|l| label(l).map(|s| 18 * count(s, tw, 12)))
                .transpose()?
                .unwrap_or(0)
            + 18 * count(&kinds[i], tw, 12)
            + 20 * count(&owner, tw, 13);
        let mut ty = r.y + (r.h - body_height) / 2 + 16;
        node_header(&mut out, r.x + r.w / 2, ty, node, texts, tw)?;
        ty += 24 * count(label(&node.label)?, tw, 16);
        if let Some(identifier) = &node.identifier {
            let mut support = String::new();
            identifier_text(&mut support, r.x + r.w / 2, ty, label(identifier)?, tw);
            layout
                .elements
                .insert(format!("{}:identifier", node.id), support);
            ty += 18 * count(label(identifier)?, tw, 12);
        }
        quiet(&mut out, r.x + r.w / 2, ty, &kinds[i], tw);
        ty += 18 * count(&kinds[i], tw, 12);
        if !owner.is_empty() {
            out.push_str("<g class=\"diagram-condition\">");
            action(&mut out, r.x + r.w / 2, ty, &owner, tw);
            out.push_str("</g>");
        }
        layout.elements.insert(node.id.clone(), out);
    }
    // Semantic DOM order is stable across responsive placements and never claims
    // that graph enumeration is temporal execution order.
    for node in nodes {
        layout.order.push(node.id.clone());
        if node.identifier.is_some() {
            layout.order.push(format!("{}:identifier", node.id));
        }
    }
    for edge in edges {
        layout.order.push(edge.id.clone());
        if edge.condition.is_some() {
            layout.order.push(format!("{}:condition", edge.id));
        }
    }
    Ok(layout)
}
