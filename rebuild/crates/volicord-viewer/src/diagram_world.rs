//! Room for directed layers and their own relationship labels, independent of a viewport.
use super::{graph_layout::ranks, index, node_kind, panel, path, relations, routing, Layout};
use crate::diagram::{measure_at, node_header, sized_text, words_at};
use routing::{Point, Rect};
use volicord_projections::*;

fn lines(text: &str, width: usize, size: usize) -> usize {
    words_at(text, width, size).len()
}
fn quiet(out: &mut String, x: usize, y: usize, text: &str, width: usize) {
    out.push_str("<g class=\"diagram-node-kind\">");
    sized_text(out, x, y, text, width, 13, 20);
    out.push_str("</g>");
}
fn port(r: Rect, right: bool, ordinal: usize, total: usize, branch: bool) -> (Point, Point) {
    let offset = r.h * (ordinal + 1) / (total + 1);
    let x = if branch {
        let extent = r.w * offset.min(r.h - offset) / r.h;
        if right {
            r.x + r.w / 2 + extent
        } else {
            r.x + r.w / 2 - extent
        }
    } else if right {
        r.x + r.w
    } else {
        r.x
    };
    (
        (x, r.y + offset),
        (if right { r.x + r.w + 10 } else { r.x - 10 }, r.y + offset),
    )
}

pub(super) fn graph(
    d: &GroundedDiagram,
    texts: &[&str],
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    let nodes = d.nodes();
    let edges = relations(d);
    let label = |l| diagram_label(l, texts);
    let endpoints = edges
        .iter()
        .map(|e| Ok((index(nodes, &e.from)?, index(nodes, &e.to)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let rank = ranks(nodes.len(), &endpoints);
    let levels = rank.iter().max().copied().unwrap_or(0) + 1;
    let common_nodes = nodes.iter().all(|n| n.kind == nodes[0].kind);
    let common_edges = edges.iter().all(|e| e.kind == edges[0].kind);
    let kinds = nodes
        .iter()
        .map(|n| node_kind(n, locale, common_nodes))
        .collect::<Vec<_>>();
    let node_width = nodes
        .iter()
        .map(|n| label(&n.label).map(|s| (measure_at(s, 16) + 32).clamp(200, 320)))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(240);
    let legend = [
        common_nodes.then(|| {
            format!(
                "{} {}",
                diagram_ui(locale, "All nodes:", "모든 노드:"),
                nodes[0].kind.localized_text(locale)
            )
        }),
        common_edges.then(|| {
            format!(
                "{} {}",
                diagram_ui(locale, "All relations:", "모든 관계:"),
                edges[0].kind.localized_text(locale)
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    let owner = if let DiagramMeaning::State { owner, .. } = &d.meaning {
        Some(format!(
            "{} {}",
            diagram_ui(locale, "State owner:", "상태 소유자:"),
            label(owner)?
        ))
    } else {
        None
    };
    let label_width = 320;
    let heights = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let tw = if n.kind == DiagramNodeKind::Branch {
                node_width / 2 - 16
            } else {
                node_width - 24
            };
            let h = 24 * lines(label(&n.label)?, tw, 16)
                + n.identifier
                    .as_ref()
                    .map(|l| label(l).map(|s| 20 * lines(s, tw, 13)))
                    .transpose()?
                    .unwrap_or(0)
                + 20 * lines(&kinds[i], tw, 13)
                + 32;
            Ok(if n.kind == DiagramNodeKind::Branch {
                h * 2
            } else {
                h
            })
        })
        .collect::<Result<Vec<usize>, String>>()?;
    let endpoint = |i: usize| -> Result<String, String> {
        let name = label(&nodes[i].label)?;
        if nodes.iter().filter(|n| label(&n.label) == Ok(name)).count() > 1 {
            diagram_node_identity(&nodes[i], texts)
        } else {
            Ok(name.to_owned())
        }
    };
    let associations = endpoints
        .iter()
        .map(|&(a, b)| Ok(format!("{} → {}", endpoint(a)?, endpoint(b)?)))
        .collect::<Result<Vec<_>, String>>()?;
    let label_heights = edges
        .iter()
        .enumerate()
        .map(|(i, e)| {
            Ok(24 * lines(label(&e.label)?, label_width - 24, 16)
                + 20 * lines(&associations[i], label_width - 24, 13)
                + e.condition
                    .as_ref()
                    .map(|l| label(l).map(|s| 24 * lines(s, label_width - 24, 16)))
                    .transpose()?
                    .unwrap_or(0)
                + if common_edges {
                    0
                } else {
                    20 * lines(e.kind.localized_text(locale), label_width - 24, 13)
                }
                + 24)
        })
        .collect::<Result<Vec<usize>, String>>()?;
    let slot = heights
        .iter()
        .copied()
        .max()
        .unwrap_or(80)
        .max(label_heights.iter().copied().max().unwrap_or(80))
        + 64;
    let mut order = (0..nodes.len()).collect::<Vec<_>>();
    // Stable barycenter passes reduce crossing without changing identity/order semantics.
    for _ in 0..4 {
        for level in 0..levels {
            let positions = (0..nodes.len())
                .map(|n| order.iter().position(|&i| i == n).unwrap_or(n))
                .collect::<Vec<_>>();
            let mut column = order
                .iter()
                .copied()
                .filter(|&i| rank[i] == level)
                .collect::<Vec<_>>();
            column.sort_by_key(|&n| {
                let neighbors = endpoints
                    .iter()
                    .filter_map(|&(a, b)| {
                        if b == n && rank[a] < level {
                            Some(positions[a])
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                (
                    neighbors.iter().sum::<usize>() * 1024 / neighbors.len().max(1),
                    n,
                )
            });
            let mut next = column.into_iter();
            for i in &mut order {
                if rank[*i] == level {
                    if let Some(n) = next.next() {
                        *i = n;
                    }
                }
            }
        }
    }
    let gap = label_width + 112;
    let left = if endpoints.iter().any(|&(a, b)| rank[b] <= rank[a]) {
        label_width + 72
    } else {
        24
    }; // Separate bounded channels for cycles/backward edges.
    let width = left + levels * (node_width + gap) + 48;
    let rows = (0..levels)
        .map(|r| rank.iter().filter(|&&v| v == r).count())
        .max()
        .unwrap_or(1);
    let top = (48
        + legend
            .iter()
            .map(|s| 20 * lines(s, node_width, 13) + 8)
            .sum::<usize>()
        + owner
            .as_ref()
            .map_or(0, |s| 24 * lines(s, node_width, 16) + 16))
    .max(112);
    let body_height = rows * slot;
    let mut boxes = vec![
        Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        };
        nodes.len()
    ];
    for level in 0..levels {
        let column = order
            .iter()
            .copied()
            .filter(|&i| rank[i] == level)
            .collect::<Vec<_>>();
        for (row, &i) in column.iter().enumerate() {
            let center = top + row * slot + heights[i] / 2;
            boxes[i] = Rect {
                x: left + level * (node_width + gap),
                y: center.saturating_sub(heights[i] / 2),
                w: node_width,
                h: heights[i],
            };
        }
    }
    let mut labels = vec![
        Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0
        };
        edges.len()
    ];
    let mut height = top + body_height + 64;
    // Each source layer owns an ample label corridor. Conditions stay with
    // their relationship, never in a detached global label shelf.
    for corridor in 0..=levels {
        let mut list = edges
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                let (a, b) = endpoints[*i];
                let c = if rank[b] > rank[a] { rank[a] + 1 } else { 0 };
                c == corridor
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        list.sort_by_key(|&i| {
            let (a, b) = endpoints[i];
            (
                (boxes[a].y + boxes[a].h / 2 + boxes[b].y + boxes[b].h / 2) / 2,
                i,
            )
        });
        let total = list.iter().map(|&i| label_heights[i] + 32).sum::<usize>();
        let mut y = top + body_height.saturating_sub(total) / 2;
        for i in list {
            let (a, _) = endpoints[i];
            labels[i] = Rect {
                x: if corridor == 0 {
                    24
                } else {
                    boxes[a].x + node_width + 56
                },
                y,
                w: label_width,
                h: label_heights[i],
            };
            y += label_heights[i] + 32;
        }
        height = height.max(y + 48);
    }
    let mut layout = Layout {
        width,
        height,
        ..Layout::default()
    };
    let mut legend_y = 20;
    for value in &legend {
        quiet(
            &mut layout.background,
            left + node_width / 2,
            legend_y,
            value,
            node_width,
        );
        legend_y += 20 * lines(value, node_width, 13) + 8;
    }
    if let Some(owner) = owner {
        let mut out = String::new();
        sized_text(
            &mut out,
            left + node_width / 2,
            legend_y + 18,
            &owner,
            node_width,
            16,
            24,
        );
        layout.elements.insert("state:owner".into(), out);
        layout.order.push("state:owner".into());
    }
    let incoming = (0..nodes.len())
        .map(|i| endpoints.iter().filter(|&&(_, b)| b == i).count())
        .collect::<Vec<_>>();
    let outgoing = (0..nodes.len())
        .map(|i| endpoints.iter().filter(|&&(a, _)| a == i).count())
        .collect::<Vec<_>>();
    let mut emitted = vec![0; nodes.len()];
    let mut received = vec![0; nodes.len()];
    let obstacles = boxes
        .iter()
        .chain(&labels)
        .map(|r| r.padded())
        .collect::<Vec<_>>();
    let mut connections = Vec::new();
    for (i, _) in edges.iter().enumerate() {
        let (a, b) = endpoints[i];
        let forward = rank[b] > rank[a];
        let (source, s) = port(
            boxes[a],
            forward,
            emitted[a],
            outgoing[a] + incoming[a],
            nodes[a].kind == DiagramNodeKind::Branch,
        );
        let (target, t) = port(
            boxes[b],
            !forward,
            outgoing[b] + received[b],
            outgoing[b] + incoming[b],
            nodes[b].kind == DiagramNodeKind::Branch,
        );
        emitted[a] += 1;
        received[b] += 1;
        let r = labels[i];
        let entry = (r.x - 12, r.y + r.h + 10);
        let exit = (r.x + r.w + 12, r.y + r.h + 10);
        connections.push((
            source,
            s,
            target,
            t,
            if forward { entry } else { exit },
            if forward { exit } else { entry },
        ));
    }
    let reserved = connections
        .iter()
        .flat_map(|&(_, s, _, t, entry, exit)| [s, t, entry, exit])
        .collect::<Vec<_>>();
    let mut old = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        let (source, s, target, t, entry, exit) = connections[i];
        let protect =
            |start: Point, end: Point| {
                let mut result = obstacles.clone();
                result.extend(reserved.iter().filter(|&&p| p != start && p != end).map(
                    |&(x, y)| Rect {
                        x: x.saturating_sub(2),
                        y: y.saturating_sub(2),
                        w: 4,
                        h: 4,
                    },
                ));
                result
            };
        let first = routing::route(s, entry, &protect(s, entry), &old, 2 * i + 2, width, height)
            .map_err(|s| format!("{} world entry: {s}", e.id))?;
        let last = routing::route(exit, t, &protect(exit, t), &old, 2 * i + 2, width, height)
            .map_err(|s| format!("{} world exit: {s}", e.id))?;
        let mut points = vec![source];
        points.extend(first);
        points.push(exit);
        points.extend(last);
        points.push(target);
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
        let r = labels[i];
        let mut out = String::new();
        panel(&mut out, r.x, r.y, r.w, r.h, "edge-key");
        out.push_str("<g class=\"relation-wording\">");
        let mut y = r.y + 18;
        sized_text(
            &mut out,
            r.x + r.w / 2,
            y,
            label(&e.label)?,
            r.w - 24,
            16,
            24,
        );
        y += 24 * lines(label(&e.label)?, r.w - 24, 16);
        quiet(&mut out, r.x + r.w / 2, y, &associations[i], r.w - 24);
        y += 20 * lines(&associations[i], r.w - 24, 13);
        if !common_edges {
            quiet(
                &mut out,
                r.x + r.w / 2,
                y,
                e.kind.localized_text(locale),
                r.w - 24,
            );
            y += 20 * lines(e.kind.localized_text(locale), r.w - 24, 13);
        }
        out.push_str("</g>");
        layout.elements.insert(e.id.clone(), out);
        if let Some(condition) = &e.condition {
            let mut out = String::from("<g class=\"diagram-condition\">");
            sized_text(
                &mut out,
                r.x + r.w / 2,
                y,
                label(condition)?,
                r.w - 24,
                16,
                24,
            );
            out.push_str("</g>");
            layout.elements.insert(format!("{}:condition", e.id), out);
        }
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
            r.w / 2 - 16
        } else {
            r.w - 24
        };
        // The central half of a diamond contains its text rectangle. Starting
        // at the top tip lets even a short label cross the sloping boundary.
        let mut y = r.y
            + 24
            + if node.kind == DiagramNodeKind::Branch {
                r.h / 4
            } else {
                0
            };
        node_header(&mut out, r.x + r.w / 2, y, node, texts, tw)?;
        y += 24 * lines(label(&node.label)?, tw, 16);
        if let Some(identifier) = &node.identifier {
            let mut support = String::new();
            support.push_str("<g class=\"diagram-label-support\">");
            sized_text(
                &mut support,
                r.x + r.w / 2,
                y,
                label(identifier)?,
                tw,
                13,
                20,
            );
            support.push_str("</g>");
            layout
                .elements
                .insert(format!("{}:identifier", node.id), support);
            y += 20 * lines(label(identifier)?, tw, 13);
        }
        quiet(&mut out, r.x + r.w / 2, y, &kinds[i], tw);
        layout.elements.insert(node.id.clone(), out);
        layout.order.push(node.id.clone());
        if node.identifier.is_some() {
            layout.order.push(format!("{}:identifier", node.id));
        }
    }
    for e in edges {
        layout.order.push(e.id.clone());
        if e.condition.is_some() {
            layout.order.push(format!("{}:condition", e.id));
        }
    }
    Ok(layout)
}
