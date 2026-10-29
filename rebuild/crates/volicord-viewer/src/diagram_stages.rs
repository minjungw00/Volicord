//! A folded sequence, using the admitted groups and messages without abstraction.
//! Local actor references replace discovery across remote participant lifelines.
use super::{panel, path, Layout};
use crate::diagram::{identifier_text, node_header, sized_text, words_at};
use volicord_projections::*;

pub(super) fn sequence(
    d: &GroundedDiagram,
    texts: &[&str],
    width: usize,
    marker: &str,
    locale: FixedLocale,
) -> Result<Layout, String> {
    let DiagramMeaning::Sequence {
        participants,
        groups,
    } = &d.meaning
    else {
        return Err("sequence unavailable".into());
    };
    let label = |l| diagram_label(l, texts);
    let count = |s: &str, w, px| words_at(s, w, px).len();
    let endpoint = |id: &str| -> Result<String, String> {
        let node = participants
            .iter()
            .find(|n| n.id == id)
            .ok_or("missing participant")?;
        let name = label(&node.label)?;
        if participants
            .iter()
            .filter(|n| label(&n.label) == Ok(name))
            .count()
            > 1
        {
            diagram_node_identity(node, texts)
        } else {
            Ok(name.into())
        }
    };
    let mut layout = Layout {
        width,
        ..Layout::default()
    };
    let w = width - 64;
    let center = width / 2 + 8;
    let mut y = 24;
    for (gi, group) in groups.iter().enumerate() {
        let start = y;
        let order = match group.order {
            DiagramOrder::Sequential => diagram_ui(locale, "strict · sequential", "strict · 순차"),
            DiagramOrder::Parallel => {
                diagram_ui(locale, "par · no relative order", "par · 상대 순서 없음")
            }
        };
        let heading = format!("{}. {order}", gi + 1);
        layout
            .background
            .push_str("<g class=\"diagram-node-kind\">");
        sized_text(&mut layout.background, center, y + 16, &heading, w, 12, 18);
        layout.background.push_str("</g>");
        y += 18 * count(&heading, w, 12) + 16;
        let guard = group
            .condition
            .as_ref()
            .map(|g| {
                label(g).map(|s| format!("opt · {} {s}", diagram_ui(locale, "condition:", "조건:")))
            })
            .transpose()?;
        if let Some(guard) = &guard {
            let mut out = String::from("<g class=\"diagram-condition\">");
            sized_text(&mut out, center, y + 16, guard, w, 13, 20);
            out.push_str("</g>");
            layout.elements.insert(format!("guard:{}", gi + 1), out);
            y += 20 * count(guard, w, 13) + 12;
        }
        for (mi, message) in group.messages.iter().enumerate() {
            let name = label(&message.label)?;
            let source = endpoint(&message.from)?;
            let destination = endpoint(&message.to)?;
            // Full scope is retained locally for long/reverse scrolling. Order
            // and type share one quiet row instead of separate metadata bands.
            let local_order = match group.order {
                DiagramOrder::Sequential => "strict",
                DiagramOrder::Parallel => {
                    diagram_ui(locale, "par · no relative order", "par · 상대 순서 없음")
                }
            };
            let context = format!(
                "{} · {}{}",
                message.kind.localized_text(locale),
                local_order,
                guard
                    .as_ref()
                    .map(|g| format!(" · {g}"))
                    .unwrap_or_default()
            );
            let mut out = String::from("<g class=\"sequence-stage-message\">");
            let top = y;
            sized_text(&mut out, center, y + 20, name, w, 16, 24);
            y += 24 * count(name, w, 16) + 4;
            let self_call = message.from == message.to;
            let local_width = if self_call { w } else { (w - 16) / 2 };
            let left = if self_call {
                center
            } else {
                32 + local_width / 2
            };
            let right = width - 16 - local_width / 2;
            out.push_str("<g class=\"diagram-node-kind sequence-endpoints\">");
            out.push_str(&format!(
                "<g class=\"sequence-from\" data-node=\"{}\">",
                message.from
            ));
            sized_text(&mut out, left, y + 12, &source, local_width, 12, 18);
            out.push_str("</g>");
            if !self_call {
                out.push_str(&format!(
                    "<g class=\"sequence-to\" data-node=\"{}\">",
                    message.to
                ));
                sized_text(&mut out, right, y + 12, &destination, local_width, 12, 18);
                out.push_str("</g>");
            }
            out.push_str("</g>");
            y +=
                18 * count(&source, local_width, 12).max(if self_call {
                    0
                } else {
                    count(&destination, local_width, 12)
                }) + 8;
            let route = if self_call {
                format!("M{left} {y}h24v12h-24")
            } else {
                format!("M{left} {y}H{right}")
            };
            out.push_str(&format!(
                "<g class=\"sequence-local-route\" data-from=\"{}\" data-to=\"{}\">",
                message.from, message.to
            ));
            path(
                &mut out,
                &route,
                marker,
                message.kind == DiagramMessageKind::Return,
            );
            out.push_str("</g>");
            y += if self_call { 28 } else { 16 };
            out.push_str("<g class=\"diagram-node-kind sequence-context\">");
            sized_text(&mut out, center, y + 12, &context, w, 12, 18);
            out.push_str("</g></g>");
            y += 18 * count(&context, w, 12) + 24;
            // A semantic message is one native link, including the space
            // between its text and directed route. Keep modifier/pointer
            // activation on that exact message rather than the SVG background.
            out.insert_str("<g class=\"sequence-stage-message\">".len(), &format!(
                "<rect class=\"sequence-message-hitbox\" x=\"28\" y=\"{top}\" width=\"{}\" height=\"{}\"/>",
                width - 40, y - top - 12));
            layout.background.push_str(&format!(
                "<circle class=\"sequence-step\" cx=\"20\" cy=\"{}\" r=\"3\"/>",
                top + 16
            ));
            layout.elements.insert(message.id.clone(), out);
            if group.order == DiagramOrder::Parallel && mi + 1 < group.messages.len() {
                layout.background.push_str(&format!(
                    "<path class=\"parallel-operand\" d=\"M28 {y}H{}\"/>",
                    width - 16
                ));
                y += 12;
            }
        }
        panel(
            &mut layout.background,
            8,
            start,
            width - 16,
            y - start,
            "sequence-frame",
        );
        // A continuous order rail links existing groups, never actors. Parallel
        // operands retain separators and the explicit no-relative-order scope.
        layout.background.push_str(&format!(
            "<path class=\"sequence-order-rail\" d=\"M20 {start}V{}\"/>",
            y + 28
        ));
        y += 28;
    }
    // Exact selectable participant/identifier roles remain present once, as a
    // full-width key. Local message references already carry their exact names.
    for node in participants {
        let name = label(&node.label)?;
        let h = 24 * count(name, width - 64, 16)
            + node
                .identifier
                .as_ref()
                .map(|l| label(l).map(|s| 18 * count(s, width - 64, 12)))
                .transpose()?
                .unwrap_or(0)
            + 24;
        let mut out = String::new();
        panel(&mut out, 24, y, width - 48, h, "participant-box");
        node_header(&mut out, width / 2, y + 24, node, texts, width - 64)?;
        layout.elements.insert(node.id.clone(), out);
        layout.order.push(node.id.clone());
        if let Some(identifier) = &node.identifier {
            let mut out = String::new();
            identifier_text(
                &mut out,
                width / 2,
                y + 24 + 24 * count(name, width - 64, 16),
                label(identifier)?,
                width - 64,
            );
            layout
                .elements
                .insert(format!("{}:identifier", node.id), out);
            layout.order.push(format!("{}:identifier", node.id));
        }
        y += h + 12;
    }
    for (i, group) in groups.iter().enumerate() {
        if group.condition.is_some() {
            layout.order.push(format!("guard:{}", i + 1));
        }
        layout
            .order
            .extend(group.messages.iter().map(|m| m.id.clone()));
    }
    layout.height = y + 12;
    Ok(layout)
}
