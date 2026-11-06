//! Spacious temporal drawing with labels beside their own directed message.
use super::{index, panel, path, Layout};
use crate::diagram::{esc, measure_at, sized_text, words_at};
use volicord_projections::*;

fn left_text(
    out: &mut String,
    x: usize,
    y: usize,
    value: &str,
    width: usize,
    size: usize,
    leading: usize,
) {
    out.push_str(&format!("<text x=\"{x}\" y=\"{y}\" text-anchor=\"start\">"));
    for (i, line) in words_at(value, width, size).iter().enumerate() {
        out.push_str(&format!(
            "<tspan x=\"{x}\" dy=\"{}\">{}</tspan>",
            if i == 0 { 0 } else { leading },
            esc(line)
        ));
    }
    out.push_str("</text>");
}

pub(super) fn sequence(
    diagram: &GroundedDiagram,
    texts: &[&str],
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    let DiagramMeaning::Sequence {
        participants: nodes,
        groups,
    } = &diagram.meaning
    else {
        return Err("sequence unavailable".into());
    };
    if nodes.is_empty() {
        return Err("sequence participants unavailable".into());
    }
    let label = |value| diagram_label(value, texts);
    let lines = |value: &str, width: usize, size: usize| words_at(value, width.max(16), size).len();
    let node_width = nodes
        .iter()
        .map(|node| label(&node.label).map(|name| (measure_at(name, 16) + 32).clamp(280, 360)))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .unwrap_or(280);
    let lane = node_width + 144;
    let padding = 40;
    let width = (padding * 2 + node_width + (nodes.len() - 1) * lane).max(if nodes.len() == 1 {
        padding * 2 + node_width + 144 + 440
    } else {
        0
    });
    let center = |i: usize| padding + node_width / 2 + i * lane;
    let header = nodes
        .iter()
        .map(|node| {
            Ok((36
                + 24 * lines(label(&node.label)?, node_width - 24, 16)
                + node
                    .identifier
                    .as_ref()
                    .map(|id| label(id).map(|value| 20 * lines(value, node_width - 24, 13)))
                    .transpose()?
                    .unwrap_or(0)
                + 20 * lines(node.kind.localized_text(locale), node_width - 24, 13))
            .max(100))
        })
        .collect::<Result<Vec<usize>, String>>()?
        .into_iter()
        .max()
        .unwrap_or(100);
    let mut layout = Layout {
        width,
        ..Layout::default()
    };
    for (i, node) in nodes.iter().enumerate() {
        let x = center(i);
        let mut drawing = String::new();
        panel(
            &mut drawing,
            x - node_width / 2,
            24,
            node_width,
            header,
            "participant-box",
        );
        super::super::node_header(&mut drawing, x, 52, node, texts, node_width - 24)?;
        let mut baseline = 52 + 24 * lines(label(&node.label)?, node_width - 24, 16);
        if let Some(identifier) = &node.identifier {
            let mut value = String::new();
            value.push_str("<g class=\"diagram-label-support\">");
            sized_text(
                &mut value,
                x,
                baseline + 4,
                label(identifier)?,
                node_width - 24,
                13,
                20,
            );
            value.push_str("</g>");
            baseline += 20 * lines(label(identifier)?, node_width - 24, 13);
            layout
                .elements
                .insert(format!("{}:identifier", node.id), value);
        }
        drawing.push_str("<g class=\"diagram-node-kind\">");
        sized_text(
            &mut drawing,
            x,
            baseline + 13,
            node.kind.localized_text(locale),
            node_width - 24,
            13,
            20,
        );
        drawing.push_str("</g>");
        layout.elements.insert(node.id.clone(), drawing);
        layout.order.push(node.id.clone());
        if node.identifier.is_some() {
            layout.order.push(format!("{}:identifier", node.id));
        }
    }
    let endpoint = |i: usize| -> Result<String, String> {
        let name = label(&nodes[i].label)?;
        if nodes
            .iter()
            .filter(|node| label(&node.label) == Ok(name))
            .count()
            > 1
        {
            diagram_node_identity(&nodes[i], texts)
        } else {
            Ok(name.into())
        }
    };
    let mut y = header + 54;
    let mut ordinal = 0;
    for (gi, group) in groups.iter().enumerate() {
        let start = y;
        let mut context = String::new();
        let order = match group.order {
            DiagramOrder::Sequential => diagram_ui(locale, "strict · sequential", "strict · 순차"),
            DiagramOrder::Parallel => {
                diagram_ui(locale, "par · no relative order", "par · 상대 순서 없음")
            }
        };
        let heading = format!("{}. {order}", gi + 1);
        let scope = match &group.condition {
            Some(guard) => format!(
                "{heading} · opt · {} {}",
                diagram_ui(locale, "condition:", "조건:"),
                label(guard)?
            ),
            None => heading.clone(),
        };
        let heading_width = (width - 80).min(320);
        context.push_str("<g class=\"diagram-node-kind\">");
        left_text(&mut context, 45, y + 24, &heading, heading_width, 13, 20);
        context.push_str("</g>");
        let mut context_height = 20 * lines(&heading, heading_width, 13);
        if let Some(guard) = &group.condition {
            let condition = label(guard)?;
            let inline = width >= padding + heading_width + 24 + 280 + padding;
            let x = if inline {
                padding + heading_width + 24
            } else {
                45
            };
            let guard_y = if inline {
                y + 24
            } else {
                y + 24 + context_height + 8
            };
            let guard_width = width - x - padding;
            let mut value = String::from("<g class=\"diagram-condition\">");
            left_text(&mut value, x, guard_y, condition, guard_width, 16, 24);
            value.push_str("</g>");
            let key = format!("guard:{}", gi + 1);
            layout.elements.insert(key.clone(), value);
            layout.order.push(key);
            let guard_height = 24 * lines(condition, guard_width, 16);
            context_height = if inline {
                context_height.max(guard_height)
            } else {
                context_height + 8 + guard_height
            };
        }
        y += context_height + 15;
        for (mi, message) in group.messages.iter().enumerate() {
            if mi > 0 && group.order == DiagramOrder::Parallel {
                layout.background.push_str(&format!("<path class=\"relation-line parallel-operand\" d=\"M25 {}H{}\" stroke-dasharray=\"3 5\"/>",y-20,width-25));
            }
            ordinal += 1;
            let from = index(nodes, &message.from)?;
            let to = index(nodes, &message.to)?;
            let a = center(from);
            let b = center(to);
            let self_call = from == to;
            let message_width = if self_call {
                440.min(width - padding * 2 - 100)
            } else {
                a.abs_diff(b).saturating_sub(50).max(260)
            };
            let left = if self_call {
                if a + 100 + message_width <= width - padding {
                    a + 100
                } else {
                    a.saturating_sub(100 + message_width)
                }
            } else {
                a.min(b) + 25
            };
            let name = label(&message.label)?;
            let support = format!(
                "{ordinal} · {} → {} · {scope} · {}",
                endpoint(from)?,
                endpoint(to)?,
                message.kind.localized_text(locale)
            );
            let primary_height = 24 * lines(name, message_width, 16);
            let support_height = 20 * lines(&support, message_width, 13);
            let baseline = y + (primary_height + support_height + 20).max(78);
            let mut drawing = format!(
                "<g class=\"sequence-message\" data-from=\"{}\" data-to=\"{}\">",
                message.from, message.to
            );
            panel(
                &mut drawing,
                left,
                y + 6,
                message_width,
                primary_height + support_height + 12,
                "sequence-message-hitbox",
            );
            left_text(&mut drawing, left, y + 26, name, message_width, 16, 24);
            drawing.push_str("<g class=\"diagram-node-kind sequence-endpoints\">");
            left_text(
                &mut drawing,
                left,
                y + 26 + primary_height,
                &support,
                message_width,
                13,
                20,
            );
            drawing.push_str("</g>");
            let route = if self_call {
                let turn = if left > a {
                    a + 65
                } else {
                    a.saturating_sub(65)
                };
                format!("M{a} {}H{turn}V{}H{a}", baseline - 35, baseline + 12)
            } else {
                format!("M{a} {baseline}H{b}")
            };
            path(
                &mut drawing,
                &route,
                marker,
                message.kind == DiagramMessageKind::Return,
            );
            drawing.push_str("</g>");
            layout.elements.insert(message.id.clone(), drawing);
            layout.order.push(message.id.clone());
            y = baseline + 54;
        }
        layout.background.push_str(&format!("<g class=\"sequence-group\" data-group-index=\"{}\"><rect class=\"sequence-frame\" x=\"25\" y=\"{start}\" width=\"{}\" height=\"{}\" rx=\"8\"/>{context}</g>",gi+1,width-50,y-start+35));
        y += 57;
    }
    layout.height = y + 30;
    let mut lifelines = String::new();
    for i in 0..nodes.len() {
        let x = center(i);
        lifelines.push_str(&format!(
            "<line class=\"lifeline\" x1=\"{x}\" y1=\"{}\" x2=\"{x}\" y2=\"{}\"/>",
            header + 30,
            layout.height - 30
        ));
    }
    layout.background = lifelines + &layout.background;
    Ok(layout)
}
