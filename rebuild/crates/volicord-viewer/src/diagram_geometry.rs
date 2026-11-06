//! ELK schema adapter. Unordered inputs are sorted; output identity, direction,
//! dimensions, ports, labels and complete routes are checked before SVG emission.
use super::engine;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use volicord_projections::{DiagramNode, DiagramNodeKind, DiagramRelation};

pub(super) type Point = (usize, usize);
#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
pub(super) struct Computed {
    pub width: usize,
    pub height: usize,
    pub nodes: Vec<Rect>,
    pub labels: Vec<Rect>,
    pub routes: Vec<Vec<Point>>,
}
#[derive(Clone, Copy, Deserialize)]
struct Position {
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
struct Bounds {
    id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
#[derive(Deserialize)]
struct Node {
    #[serde(flatten)]
    bounds: Bounds,
    #[serde(default)]
    ports: Vec<Bounds>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Section {
    start_point: Position,
    end_point: Position,
    #[serde(default)]
    bend_points: Vec<Position>,
}
#[derive(Deserialize)]
struct Edge {
    id: String,
    sources: Vec<String>,
    targets: Vec<String>,
    labels: Vec<Bounds>,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
struct Geometry {
    width: f64,
    height: f64,
    children: Vec<Node>,
    edges: Vec<Edge>,
}
fn port(edge: &str, source: bool) -> String {
    format!("{edge}:{}", if source { "source" } else { "target" })
}
fn bounded(v: f64) -> bool {
    v.is_finite() && (0.0..=65_536.0).contains(&v)
}
fn inside(r: &Bounds, w: f64, h: f64) -> bool {
    [r.x, r.y, r.width, r.height].into_iter().all(bounded)
        && r.x + r.width <= w + 0.01
        && r.y + r.height <= h + 0.01
}
impl Bounds {
    fn rect(&self) -> Rect {
        Rect {
            x: self.x.round() as usize,
            y: self.y.round() as usize,
            w: self.width as usize,
            h: self.height as usize,
        }
    }
}
fn options(spatial: bool) -> Value {
    json!({"elk.algorithm":"layered", "elk.direction":if spatial {"RIGHT"} else {"DOWN"},
        "elk.edgeRouting":"POLYLINE", "elk.randomSeed":"1",
        "elk.layered.mergeEdges":"false", "elk.layered.mergeHierarchyEdges":"false",
        "elk.spacing.nodeNode":"40", "elk.spacing.edgeEdge":"18", "elk.spacing.edgeLabel":"10",
        "elk.layered.spacing.nodeNodeBetweenLayers":"70",
        "elk.layered.spacing.edgeNodeBetweenLayers":"26",
        "elk.layered.spacing.edgeEdgeBetweenLayers":"18",
        "elk.padding":"[top=24,left=24,bottom=24,right=24]"})
}

pub(super) fn compute(
    nodes: &[DiagramNode],
    edges: &[DiagramRelation],
    node_width: usize,
    heights: &[usize],
    label_width: usize,
    label_heights: &[usize],
    spatial: bool,
) -> Result<Computed, String> {
    let mut sorted_nodes = nodes.iter().enumerate().collect::<Vec<_>>();
    sorted_nodes.sort_by_key(|(_, n)| &n.id);
    let mut sorted_edges = edges.iter().enumerate().collect::<Vec<_>>();
    sorted_edges.sort_by_key(|(_, e)| &e.id);
    let input = json!({"id":":graph","layoutOptions":options(spatial),
        "children":sorted_nodes.iter().map(|(i,n)|json!({"id":n.id,"width":node_width,"height":heights[*i],
            "ports":sorted_edges.iter().flat_map(|(_,e)| {
                let mut ports=Vec::new();
                if e.from==n.id {ports.push(json!({"id":port(&e.id,true),"width":0,"height":0}));}
                if e.to==n.id {ports.push(json!({"id":port(&e.id,false),"width":0,"height":0}));}
                ports
            }).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "edges":sorted_edges.iter().map(|(i,e)|json!({"id":e.id,"sources":[port(&e.id,true)],"targets":[port(&e.id,false)],
            "labels":[{"id":format!("{}:label",e.id),"text":":","width":label_width,"height":label_heights[*i],
                "layoutOptions":{"elk.edgeLabels.placement":"CENTER"}}]})).collect::<Vec<_>>()});
    let output = engine::layout(&input.to_string())?;
    let g: Geometry =
        serde_json::from_str(&output).map_err(|_| "local layout returned invalid geometry JSON")?;
    decode(
        g,
        nodes,
        edges,
        node_width,
        heights,
        label_width,
        label_heights,
    )
}

fn decode(
    g: Geometry,
    nodes: &[DiagramNode],
    edges: &[DiagramRelation],
    node_width: usize,
    heights: &[usize],
    label_width: usize,
    label_heights: &[usize],
) -> Result<Computed, String> {
    let bad = || "local layout returned malformed or incomplete geometry".to_owned();
    if !bounded(g.width)
        || !bounded(g.height)
        || g.width < 1.0
        || g.height < 1.0
        || g.width * g.height > 100_000_000.0
        || g.children.len() != nodes.len()
        || g.edges.len() != edges.len()
    {
        return Err(bad());
    }
    let mut seen = BTreeSet::new();
    let mut ports = BTreeMap::new();
    let mut node_boxes = BTreeMap::new();
    for n in &g.children {
        let b = &n.bounds;
        let Some(i) = nodes.iter().position(|n| n.id == b.id) else {
            return Err(bad());
        };
        if !seen.insert(b.id.as_str())
            || !inside(b, g.width, g.height)
            || b.width != node_width as f64
            || b.height != heights[i] as f64
        {
            return Err(bad());
        }
        let expected = edges
            .iter()
            .flat_map(|e| {
                [(&e.from, true), (&e.to, false)]
                    .into_iter()
                    .filter(|(id, _)| *id == &b.id)
                    .map(|(_, source)| port(&e.id, source))
            })
            .collect::<BTreeSet<_>>();
        if expected.len() != n.ports.len() {
            return Err(bad());
        }
        for p in &n.ports {
            if !expected.contains(&p.id)
                || !inside(p, b.width, b.height)
                || p.width != 0.0
                || p.height != 0.0
                || ports
                    .insert(
                        p.id.clone(),
                        Position {
                            x: b.x + p.x,
                            y: b.y + p.y,
                        },
                    )
                    .is_some()
            {
                return Err(bad());
            }
        }
        node_boxes.insert(b.id.as_str(), b.rect());
    }
    seen.clear();
    let mut label_boxes = BTreeMap::new();
    let mut routes = BTreeMap::new();
    for e in &g.edges {
        let Some(i) = edges.iter().position(|v| v.id == e.id) else {
            return Err(bad());
        };
        if !seen.insert(e.id.as_str())
            || e.sources != [port(&e.id, true)]
            || e.targets != [port(&e.id, false)]
            || e.labels.len() != 1
            || e.sections.len() != 1
        {
            return Err(bad());
        }
        let l = &e.labels[0];
        if l.id != format!("{}:label", e.id)
            || !inside(l, g.width, g.height)
            || l.width != label_width as f64
            || l.height != label_heights[i] as f64
        {
            return Err(bad());
        }
        let s = &e.sections[0];
        if s.bend_points.len() > 128 {
            return Err(bad());
        }
        let points = std::iter::once(s.start_point)
            .chain(s.bend_points.iter().copied())
            .chain(std::iter::once(s.end_point))
            .collect::<Vec<_>>();
        if points
            .iter()
            .any(|p| !bounded(p.x) || !bounded(p.y) || p.x > g.width || p.y > g.height)
        {
            return Err(bad());
        }
        for (p, id) in [(s.start_point, &e.sources[0]), (s.end_point, &e.targets[0])] {
            let Some(port) = ports.get(id) else {
                return Err(bad());
            };
            if (p.x - port.x).abs() > 0.01 || (p.y - port.y).abs() > 0.01 {
                return Err(bad());
            }
        }
        label_boxes.insert(e.id.as_str(), l.rect());
        routes.insert(
            e.id.as_str(),
            points
                .iter()
                .map(|p| (p.x.round() as usize, p.y.round() as usize))
                .collect(),
        );
    }
    Ok(Computed {
        width: g.width.ceil() as usize,
        height: g.height.ceil() as usize,
        nodes: nodes.iter().map(|n| node_boxes[&*n.id]).collect(),
        labels: edges.iter().map(|e| label_boxes[&*e.id]).collect(),
        routes: edges
            .iter()
            .map(|e| routes.remove(&*e.id).ok_or_else(bad))
            .collect::<Result<_, _>>()?,
    })
}

// ELK ports lie on rectangular node envelopes. Project a branch endpoint onto
// its diamond along the engine's first/last segment, preserving route direction.
pub(super) fn clip_branches(
    points: &mut [Point],
    nodes: &[DiagramNode],
    edge: &DiagramRelation,
    boxes: &[Rect],
) -> Result<(), String> {
    if points.len() < 2 {
        return Err("local layout route unavailable".into());
    }
    for (id, index, neighbor) in [
        (&edge.from, 0, 1),
        (&edge.to, points.len() - 1, points.len() - 2),
    ] {
        let i = nodes
            .iter()
            .position(|n| &n.id == id)
            .ok_or("diagram endpoint unavailable")?;
        if nodes[i].kind != DiagramNodeKind::Branch {
            continue;
        }
        let r = boxes[i];
        let (x, y) = points[index];
        points[index] = if points[neighbor].0 != x {
            let offset = y.saturating_sub(r.y).min(r.h);
            let inset = r.w / 2 - r.w * offset.min(r.h - offset) / r.h;
            (
                if x <= r.x {
                    r.x + inset
                } else {
                    r.x + r.w - inset
                },
                y,
            )
        } else {
            let offset = x.saturating_sub(r.x).min(r.w);
            let inset = r.h / 2 - r.h * offset.min(r.w - offset) / r.w;
            (
                x,
                if y <= r.y {
                    r.y + inset
                } else {
                    r.y + r.h - inset
                },
            )
        };
    }
    Ok(())
}
pub(super) fn path(points: &[Point]) -> String {
    let mut result = String::new();
    let mut previous = None;
    for &(x, y) in points {
        match previous {
            None => result.push_str(&format!("M{x} {y}")),
            Some((px, _)) if px == x => result.push_str(&format!("V{y}")),
            Some((_, py)) if py == y => result.push_str(&format!("H{x}")),
            Some(_) => result.push_str(&format!("L{x} {y}")),
        };
        previous = Some((x, y));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use volicord_projections::{DiagramLabel, DiagramRelationKind};
    fn inputs() -> (Vec<DiagramNode>, Vec<DiagramRelation>) {
        let label = DiagramLabel {
            text: "value".into(),
            unit: 0,
            start_byte: 0,
            end_byte: 5,
        };
        let nodes = ["a", "b", "c", "isolated"]
            .iter()
            .map(|id| DiagramNode {
                id: (*id).into(),
                kind: DiagramNodeKind::Component,
                label: label.clone(),
                identifier: None,
            })
            .collect();
        let edges = [
            ("a", "b"),
            ("a", "b"),
            ("b", "a"),
            ("b", "c"),
            ("c", "a"),
            ("a", "a"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (from, to))| DiagramRelation {
            id: format!("e{i}"),
            from: (*from).into(),
            to: (*to).into(),
            kind: DiagramRelationKind::DependsOn,
            label: label.clone(),
            condition: Some(label.clone()),
        })
        .collect();
        (nodes, edges)
    }
    #[test]
    fn unordered_permutations_keep_cycles_multiedges_self_loops_and_isolates() -> Result<(), String>
    {
        let (mut nodes, mut edges) = inputs();
        let snapshot = |nodes: &[DiagramNode], edges: &[DiagramRelation]| -> Result<_, String> {
            let g = compute(
                nodes,
                edges,
                200,
                &vec![80; nodes.len()],
                240,
                &vec![120; edges.len()],
                true,
            )?;
            Ok((
                nodes
                    .iter()
                    .zip(g.nodes)
                    .map(|(n, r)| (n.id.clone(), (r.x, r.y, r.w, r.h)))
                    .collect::<BTreeMap<_, _>>(),
                edges
                    .iter()
                    .zip(g.routes.into_iter().zip(g.labels))
                    .map(|(e, (route, r))| (e.id.clone(), (route, (r.x, r.y, r.w, r.h))))
                    .collect::<BTreeMap<_, _>>(),
            ))
        };
        let original = snapshot(&nodes, &edges)?;
        for _ in 0..8 {
            nodes.rotate_left(1);
            edges.rotate_right(1);
            assert_eq!(snapshot(&nodes, &edges)?, original);
        }
        edges.reverse();
        nodes.reverse();
        assert_eq!(snapshot(&nodes, &edges)?, original);
        Ok(())
    }
    #[test]
    fn malformed_incomplete_reversed_and_foreign_geometry_are_refused() -> Result<(), String> {
        let (mut nodes, mut edges) = inputs();
        nodes.truncate(2);
        edges.truncate(1);
        // Independently authored geometry oracle; no engine output supplies it.
        let valid = json!({"width":600,"height":200,"children":[
            {"id":"a","x":10,"y":20,"width":100,"height":60,"ports":[{"id":"e0:source","x":100,"y":30,"width":0,"height":0}]},
            {"id":"b","x":490,"y":20,"width":100,"height":60,"ports":[{"id":"e0:target","x":0,"y":30,"width":0,"height":0}]}],
            "edges":[{"id":"e0","sources":["e0:source"],"targets":["e0:target"],"labels":[{"id":"e0:label","x":250,"y":70,"width":100,"height":100}],"sections":[{"startPoint":{"x":110,"y":50},"endPoint":{"x":490,"y":50}}]}]});
        let accept = |v: Value| {
            decode(
                serde_json::from_value(v).map_err(|e| e.to_string())?,
                &nodes,
                &edges,
                100,
                &[60, 60],
                100,
                &[100],
            )
        };
        assert!(accept(valid.clone()).is_ok());
        let mut bad = valid.clone();
        bad["edges"][0]["sections"][0]["endPoint"]["x"] = json!(110);
        assert!(accept(bad).is_err());
        let mut bad = valid.clone();
        bad["edges"][0]["sources"] = json!(["e0:target"]);
        assert!(accept(bad).is_err());
        let mut bad = valid.clone();
        bad["edges"][0]["labels"] = json!([]);
        assert!(accept(bad).is_err());
        let mut bad = valid.clone();
        bad["children"][1]["id"] = json!("a");
        assert!(accept(bad).is_err());
        let mut bad = valid.clone();
        bad["edges"][0]["id"] = json!("foreign");
        assert!(accept(bad).is_err());
        let mut bad = valid.clone();
        bad["edges"] = json!([]);
        assert!(accept(bad).is_err());
        let mut bad = valid.clone();
        bad["width"] = json!(1e100);
        assert!(accept(bad).is_err());
        let mut bad = valid;
        bad["edges"][0]["labels"][0]["width"] = json!(200);
        assert!(accept(bad).is_err());
        Ok(())
    }
}
