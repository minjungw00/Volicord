//! Joint topology/port/label geometry, rendered only from admitted roles.
use super::{geometry, node_kind, panel, path, relations, Layout};
use crate::diagram::{measure_at, node_header, sized_text, words_at};
use geometry::{Point, Rect};
use volicord_projections::*;

fn lines(text: &str, width: usize, size: usize) -> usize {
    words_at(text, width, size).len()
}
fn quiet(out: &mut String, x: usize, y: usize, text: &str, width: usize) {
    out.push_str("<g class=\"diagram-node-kind\">");
    sized_text(out, x, y, text, width, 13, 20);
    out.push_str("</g>");
}
pub(super) fn graph(
    d: &GroundedDiagram,
    texts: &[&str],
    width: usize,
    marker: &str,
    locale: FixedLocale,
    spatial: bool,
) -> Result<Layout, String> {
    let nodes = d.nodes();
    let edges = relations(d);
    let label = |l| diagram_label(l, texts);
    let common_nodes = nodes.iter().all(|n| n.kind == nodes[0].kind);
    let common_edges = edges.iter().all(|e| e.kind == edges[0].kind);
    let kinds = nodes
        .iter()
        .map(|n| node_kind(n, locale, common_nodes))
        .collect::<Vec<_>>();
    let measured_width = nodes
        .iter()
        .map(|n| label(&n.label).map(|s| (measure_at(s, 16) + 32).clamp(200, 320)))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(240);
    let node_width = if spatial {
        measured_width
    } else {
        measured_width.min(width - 48)
    };
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
    let label_width = if spatial { 320 } else { (width - 32).min(320) };
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
                + if spatial {
                    0
                } else {
                    owner.as_ref().map_or(0, |s| 24 * lines(s, tw, 16) + 8)
                }
                + 32;
            let ports = edges
                .iter()
                .map(|e| usize::from(e.from == n.id) + usize::from(e.to == n.id))
                .sum::<usize>();
            let h = h.max(32 + ports * 14);
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
    let associations = edges
        .iter()
        .map(|e| {
            let a = nodes
                .iter()
                .position(|n| n.id == e.from)
                .ok_or("diagram source unavailable")?;
            let b = nodes
                .iter()
                .position(|n| n.id == e.to)
                .ok_or("diagram target unavailable")?;
            Ok(format!("{} → {}", endpoint(a)?, endpoint(b)?))
        })
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
                + if spatial {
                    0
                } else {
                    owner
                        .as_ref()
                        .map_or(0, |s| 24 * lines(s, label_width - 24, 16) + 8)
                }
                + 24)
        })
        .collect::<Result<Vec<usize>, String>>()?;
    let top = (48
        + legend
            .iter()
            .map(|s| 20 * lines(s, node_width, 13) + 8)
            .sum::<usize>()
        + owner
            .as_ref()
            .map_or(0, |s| 24 * lines(s, node_width, 16) + 16))
    .max(112);
    let computed = geometry::compute(
        nodes,
        edges,
        node_width,
        &heights,
        label_width,
        &label_heights,
        spatial,
    )?;
    let left = 24;
    let width = computed.width;
    let height = computed.height + top;
    let boxes = computed
        .nodes
        .iter()
        .map(|r| Rect { y: r.y + top, ..*r })
        .collect::<Vec<_>>();
    let labels = computed
        .labels
        .iter()
        .map(|r| Rect { y: r.y + top, ..*r })
        .collect::<Vec<_>>();
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
    if let Some(owner) = owner.as_ref() {
        let mut out = String::new();
        sized_text(
            &mut out,
            left + node_width / 2,
            legend_y + 18,
            owner,
            node_width,
            16,
            24,
        );
        layout.elements.insert("state:owner".into(), out);
        layout.order.push("state:owner".into());
    }
    for (i, e) in edges.iter().enumerate() {
        let mut points = computed.routes[i]
            .iter()
            .map(|&(x, y)| (x, y + top))
            .collect::<Vec<Point>>();
        geometry::clip_branches(&mut points, nodes, e, &boxes)?;
        let mut route = String::new();
        path(
            &mut route,
            &geometry::path(&points),
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
        if !spatial {
            if let Some(owner) = &owner {
                sized_text(&mut out, r.x + r.w / 2, y, owner, r.w - 24, 16, 24);
                y += 24 * lines(owner, r.w - 24, 16) + 8;
            }
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
        if !spatial {
            if let Some(owner) = &owner {
                y += 20 * lines(&kinds[i], tw, 13);
                sized_text(&mut out, r.x + r.w / 2, y, owner, tw, 16, 24);
            }
        }
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
