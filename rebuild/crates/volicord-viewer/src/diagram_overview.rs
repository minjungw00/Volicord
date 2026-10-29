//! Responsive connected maps. Meaning and origins are shared across both layouts.
//! All coordinates are computed from admitted topology and exact label lengths.
use super::{elements, esc, open, RENDERER_ID};
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
fn panel(out: &mut String, x: usize, y: usize, w: usize, h: usize, class: &str) {
    out.push_str(&format!(
        "<rect class=\"{class}\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"6\"/>"
    ));
}
fn path(out: &mut String, route: &str, marker: &str, dashed: bool) {
    // A foreground bridge distinguishes crossings from joins; arrowheads retain
    // direction independently of the authored action and its quiet label leader.
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
fn node_kind(node: &DiagramNode, locale: FixedLocale, common_type: bool) -> String {
    if common_type {
        String::new()
    } else {
        node.kind.localized_text(locale).into()
    }
}
#[path = "diagram_graph.rs"]
mod graph_layout;
use graph_layout::graph;
#[path = "diagram_stages.rs"]
mod stages;

fn sequence<'a>(
    d: &'a GroundedDiagram,
    texts: &[&str],
    width: usize,
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    use super::{sized_text, words_at};
    let DiagramMeaning::Sequence {
        participants: nodes,
        groups,
    } = &d.meaning
    else {
        return Err("sequence unavailable".into());
    };
    let label = |l: &'a DiagramLabel| diagram_label(l, texts);
    let count = |s: &str, w: usize, px: usize| words_at(s, w.max(16), px).len();
    let n = nodes.len();
    // A narrow article pans the temporal scene instead of squeezing participant
    // names into one-glyph columns. Message wording still uses the article's
    // reading width; expanding the canvas must not create unreadable wide bands.
    let reading_width = width - 32;
    let message_width = reading_width.min(400);
    let width = width.max(24 + n * 128);
    let endpoint = |i: usize| -> Result<String, String> {
        let name = label(&nodes[i].label)?;
        Ok(
            if nodes.iter().filter(|n| label(&n.label) == Ok(name)).count() > 1 {
                diagram_node_identity(&nodes[i], texts)?
            } else {
                name.into()
            },
        )
    };
    let lane = (width - 24) / n;
    let node_width = lane.saturating_sub(4).max(16);
    let x = |i: usize| 12 + lane * i + lane / 2;
    let header = nodes
        .iter()
        .map(|node| {
            Ok(24
                * count(
                    label(&node.label)?,
                    node_width.saturating_sub(8).max(16),
                    16,
                )
                + node
                    .identifier
                    .as_ref()
                    .map(|l| {
                        label(l).map(|s| 18 * count(s, node_width.saturating_sub(8).max(16), 12))
                    })
                    .transpose()?
                    .unwrap_or(0)
                + 24)
        })
        .collect::<Result<Vec<usize>, String>>()?
        .into_iter()
        .max()
        .unwrap_or(48);
    let mut layout = Layout {
        width,
        ..Layout::default()
    };
    for (i, node) in nodes.iter().enumerate() {
        let mut out = String::new();
        panel(
            &mut out,
            x(i) - node_width / 2,
            12,
            node_width,
            header,
            "participant-box",
        );
        super::node_header(
            &mut out,
            x(i),
            36,
            node,
            texts,
            node_width.saturating_sub(8).max(16),
        )?;
        layout.order.push(node.id.clone());
        layout.elements.insert(node.id.clone(), out);
        if let Some(identifier) = &node.identifier {
            let mut support = String::new();
            super::identifier_text(
                &mut support,
                x(i),
                36 + 24
                    * count(
                        label(&node.label)?,
                        node_width.saturating_sub(8).max(16),
                        16,
                    ),
                label(identifier)?,
                node_width.saturating_sub(8).max(16),
            );
            layout
                .elements
                .insert(format!("{}:identifier", node.id), support);
            layout.order.push(format!("{}:identifier", node.id));
        }
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
        let guard = group
            .condition
            .as_ref()
            .map(|g| {
                label(g).map(|s| format!("opt · {} {s}", diagram_ui(locale, "condition:", "조건:")))
            })
            .transpose()?;
        // Group context belongs to the article reading column, independently
        // of the wider panning participant canvas.
        let context_width = reading_width - 8;
        let context_center = 16 + reading_width / 2;
        let hh = 18 * count(&heading, context_width, 12) + 12;
        let gh = guard
            .as_ref()
            .map_or(0, |s| 20 * count(s, context_width, 13) + 12);
        panel(
            &mut layout.background,
            8,
            y,
            width - 16,
            hh + gh,
            "message-label",
        );
        layout
            .background
            .push_str("<g class=\"diagram-node-kind\">");
        sized_text(
            &mut layout.background,
            context_center,
            y + 16,
            &heading,
            context_width,
            12,
            18,
        );
        layout.background.push_str("</g>");
        y += hh;
        if let Some(value) = &guard {
            let mut out = String::from("<g class=\"diagram-condition\">");
            sized_text(
                &mut out,
                context_center,
                y + 16,
                value,
                context_width,
                13,
                20,
            );
            out.push_str("</g>");
            layout.elements.insert(format!("guard:{}", gi + 1), out);
            layout.order.push(format!("guard:{}", gi + 1));
            y += gh;
        }
        for (mi, message) in group.messages.iter().enumerate() {
            let a = index(nodes, &message.from)?;
            let b = index(nodes, &message.to)?;
            let name = label(&message.label)?;
            let association = format!("{} → {}", endpoint(a)?, endpoint(b)?);
            let context = format!(
                "{}.{} · {} · {}{}",
                gi + 1,
                mi + 1,
                message.kind.localized_text(locale),
                if group.order == DiagramOrder::Sequential {
                    "strict"
                } else {
                    order
                },
                guard
                    .as_ref()
                    .map(|s| format!(" · {s}"))
                    .unwrap_or_default()
            );
            // Action first. Endpoint names and order/guard remain locally visible
            // with subordinate styles; full identifiers live in headers/detail.
            let w = message_width;
            let center = ((x(a) + x(b)) / 2).clamp(w / 2 + 16, width - w / 2 - 16);
            let h = 24 * count(name, w - 16, 16)
                + 18 * count(&association, w - 16, 12)
                + 18 * count(&context, w - 16, 12)
                + 16;
            let mut out = String::new();
            panel(&mut out, center - w / 2, y, w, h, "message-label");
            let mut ty = y + 24;
            sized_text(&mut out, center, ty, name, w - 16, 16, 24);
            ty += 24 * count(name, w - 16, 16);
            out.push_str("<g class=\"diagram-node-kind sequence-endpoints\">");
            sized_text(&mut out, center, ty, &association, w - 16, 12, 18);
            out.push_str("</g>");
            ty += 18 * count(&association, w - 16, 12);
            out.push_str("<g class=\"diagram-node-kind sequence-context\">");
            sized_text(&mut out, center, ty, &context, w - 16, 12, 18);
            out.push_str("</g>");
            let baseline = y + h + 12;
            let route = if a == b {
                let turn = if a + 1 == n {
                    x(a).saturating_sub(18)
                } else {
                    x(a) + 18
                };
                format!("M{} {baseline}H{turn}V{}H{}", x(a), baseline + 18, x(a))
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
            layout.order.push(message.id.clone());
            y = baseline + 36;
            if group.order == DiagramOrder::Parallel && mi + 1 < group.messages.len() {
                layout.background.push_str(&format!("<path class=\"relation-line parallel-operand\" d=\"M8 {y}H{}\" stroke-dasharray=\"3 5\"/>",width-8));
                y += 12;
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
    render(d, texts, ns, target, locale, false)
}

pub(super) fn timeline_svg(
    d: &GroundedDiagram,
    texts: &[&str],
    ns: &str,
    target: &dyn Fn(&str, &DiagramLabel) -> String,
    locale: FixedLocale,
) -> Result<String, String> {
    render(d, texts, ns, target, locale, true)
}

fn render(
    d: &GroundedDiagram,
    texts: &[&str],
    ns: &str,
    target: &dyn Fn(&str, &DiagramLabel) -> String,
    locale: FixedLocale,
    timeline: bool,
) -> Result<String, String> {
    let marker = format!("{ns}:arrow");
    let layout = |width| {
        if matches!(d.meaning, DiagramMeaning::Sequence { .. }) {
            if timeline {
                sequence(d, texts, width, &marker, locale)
            } else {
                stages::sequence(d, texts, width, &marker, locale)
            }
        } else {
            graph(d, texts, width, &marker, locale)
        }
    };
    let wide = layout(760)?;
    let medium = layout(520)?;
    let narrow = layout(272)?;
    let title = diagram_label(&d.title, texts)?;
    // No viewBox: each variant keeps native 16px text. CSS selects geometry,
    // never scales an overview to a miniature. The semantic anchors exist once.
    let mut out=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" style=\"--map-wide-width:{}px;--map-wide-height:{}px;--map-medium-width:{}px;--map-medium-height:{}px;--map-narrow-width:{}px;--map-narrow-height:{}px\" class=\"connected-overview\" role=\"group\" aria-label=\"{}\" data-renderer=\"{RENDERER_ID}\"><defs><marker id=\"{marker}\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\"><path d=\"M0 0L10 5L0 10Z\"/></marker></defs><g class=\"diagram-content\"><g class=\"map-wide\">{}</g><g class=\"map-medium\">{}</g><g class=\"map-narrow\">{}</g>",wide.width,wide.height,wide.width,wide.height,medium.width,medium.height,narrow.width,narrow.height,esc(title),wide.background,medium.background,narrow.background);
    let labels = elements(d).into_iter().collect::<BTreeMap<_, _>>();
    if wide.order.len() != labels.len() || wide.order != narrow.order || wide.order != medium.order
    {
        return Err("incomplete responsive elements".into());
    }
    let mode = if matches!(d.meaning, DiagramMeaning::Sequence { .. }) {
        if timeline {
            "timeline"
        } else {
            "stages"
        }
    } else {
        "graph"
    };
    out = out.replacen("class=\"connected-overview\"", &format!("class=\"connected-overview\" data-sequence-layout=\"{mode}\" data-origin-namespace=\"{}\"", esc(ns)), 1);
    let mut relationship: Option<&str> = None;
    for id in &wide.order {
        if relationship.is_some_and(|r| id != &format!("{r}:condition")) {
            out.push_str("</g>");
            relationship = None;
        }
        let label = labels.get(id).ok_or("missing element label")?;
        let name =
            super::element_caption(d, id, texts, locale) + ": " + diagram_label(label, texts)?;
        if let Some(route) = wide.routes.get(id) {
            // One native relationship scope connects its label, optional guard
            // and pointer alias. CSS focus/selection works without scripting.
            out.push_str("<g class=\"diagram-relationship\">");
            relationship = Some(id);
            out.push_str(&format!(
                "<a class=\"diagram-route diagram-text-link\" data-element=\"{}-{}\" href=\"#{}\" tabindex=\"-1\" aria-label=\"{}\"><g class=\"map-wide\">{route}</g><g class=\"map-medium\">{}</g><g class=\"map-narrow\">{}</g></a>",
                esc(ns), esc(id), esc(&target(id, label)), esc(&name), medium.routes.get(id).ok_or("missing medium route")?, narrow.routes.get(id).ok_or("missing narrow route")?
            ));
        }
        open(&mut out, ns, id, label, &name, target);
        out.push_str(&format!(
            "<g class=\"map-wide\">{}</g><g class=\"map-medium\">{}</g><g class=\"map-narrow\">{}</g></a>",
            wide.elements.get(id).ok_or("missing wide element")?,
            medium.elements.get(id).ok_or("missing medium element")?,
            narrow.elements.get(id).ok_or("missing narrow element")?
        ));
    }
    if relationship.is_some() {
        out.push_str("</g>");
    }
    out.push_str("</g></svg>");
    if out.len() > 262144 {
        return Err("local SVG output budget exceeded".into());
    }
    Ok(out)
}
