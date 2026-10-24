//! Bounded orthogonal routing around all node and label rectangles.
//! Routes carry no semantic identity, direction or grouping authority.
use std::{cmp::Reverse, collections::BinaryHeap};

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    fn blocks(self, a: (usize, usize), b: (usize, usize)) -> bool {
        if a.0 == b.0 {
            a.0 > self.x
                && a.0 < self.x + self.w
                && a.1.min(b.1) < self.y + self.h
                && a.1.max(b.1) > self.y
        } else {
            a.1 > self.y
                && a.1 < self.y + self.h
                && a.0.min(b.0) < self.x + self.w
                && a.0.max(b.0) > self.x
        }
    }
    pub fn padded(self) -> Self {
        Self {
            x: self.x.saturating_sub(8),
            y: self.y.saturating_sub(8),
            w: self.w + 16,
            h: self.h + 16,
        }
    }
}

/// A rectilinear visibility grid has at most 70 x 70 intersections for the
/// admitted 12 nodes + 20 relation labels. Dijkstra includes incoming direction
/// so a short direct route wins over unnecessary outer lanes and extra bends.
pub(super) fn route(
    start: (usize, usize),
    end: (usize, usize),
    obstacles: &[Rect],
    width: usize,
    height: usize,
) -> Result<String, String> {
    let mut xs = vec![8, width - 8, start.0, end.0];
    let mut ys = vec![8, height - 8, start.1, end.1];
    for r in obstacles {
        xs.extend([r.x, r.x + r.w]);
        ys.extend([r.y, r.y + r.h]);
    }
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let nx = xs.len();
    let ny = ys.len();
    if nx > 70 || ny > 70 {
        return Err("local route grid budget exceeded".into());
    }
    let source = ys.binary_search(&start.1).map_err(|_| "route source")? * nx
        + xs.binary_search(&start.0).map_err(|_| "route source")?;
    let dest = ys.binary_search(&end.1).map_err(|_| "route target")? * nx
        + xs.binary_search(&end.0).map_err(|_| "route target")?;
    let mut distances = vec![usize::MAX; nx * ny * 3];
    let mut previous = vec![usize::MAX; nx * ny * 3];
    let mut heap = BinaryHeap::new();
    distances[source * 3] = 0;
    heap.push(Reverse((0, source * 3)));
    let point = |p: usize| (xs[p % nx], ys[p / nx]);
    let mut finish = None;
    while let Some(Reverse((cost, state))) = heap.pop() {
        if distances[state] != cost {
            continue;
        }
        let at = state / 3;
        let direction = state % 3;
        if at == dest {
            finish = Some(state);
            break;
        }
        let x = at % nx;
        let y = at / nx;
        let neighbors = [
            (x.checked_sub(1).map(|v| y * nx + v), 1),
            (if x + 1 < nx { Some(at + 1) } else { None }, 1),
            (y.checked_sub(1).map(|v| v * nx + x), 2),
            (if y + 1 < ny { Some(at + nx) } else { None }, 2),
        ];
        for (next, dir) in neighbors {
            let Some(next) = next else { continue };
            let a = point(at);
            let b = point(next);
            if obstacles.iter().any(|r| r.blocks(a, b)) {
                continue;
            }
            let next_state = next * 3 + dir;
            let next_cost = cost
                + a.0.abs_diff(b.0)
                + a.1.abs_diff(b.1)
                + if direction != 0 && direction != dir {
                    32
                } else {
                    0
                };
            if next_cost < distances[next_state] {
                distances[next_state] = next_cost;
                previous[next_state] = state;
                heap.push(Reverse((next_cost, next_state)));
            }
        }
    }
    let mut at = finish.ok_or("no unobstructed local diagram route")?;
    let mut points = vec![point(at / 3)];
    while at != source * 3 {
        at = previous[at];
        if at == usize::MAX {
            return Err("incomplete local route".into());
        }
        points.push(point(at / 3));
    }
    points.reverse();
    let mut out = format!("M{} {}", start.0, start.1);
    // Collapse collinear grid segments, retaining turns exactly.
    for i in 1..points.len() {
        if i + 1 < points.len()
            && ((points[i - 1].0 == points[i].0 && points[i].0 == points[i + 1].0)
                || (points[i - 1].1 == points[i].1 && points[i].1 == points[i + 1].1))
        {
            continue;
        }
        if points[i - 1].0 == points[i].0 {
            out.push_str(&format!("V{}", points[i].1));
        } else {
            out.push_str(&format!("H{}", points[i].0));
        }
    }
    Ok(out)
}
