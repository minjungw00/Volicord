//! Bounded orthogonal visibility routing, including already drawn connections.
//! Crossings may be bridged; independent routes never share a straight run.
use std::{cmp::Reverse, collections::BinaryHeap};
pub(super) type Point = (usize, usize);
#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}
impl Rect {
    pub fn padded(self) -> Self {
        Self {
            x: self.x.saturating_sub(4),
            y: self.y.saturating_sub(4),
            w: self.w + 8,
            h: self.h + 8,
        }
    }
    fn blocks(self, a: Point, b: Point) -> bool {
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
}
#[derive(Clone, Copy)]
pub(super) struct Segment {
    pub a: Point,
    pub b: Point,
    pub group: usize,
}
fn shared(a: usize, b: usize, c: usize, d: usize) -> usize {
    a.max(b)
        .min(c.max(d))
        .saturating_sub(a.min(b).max(c.min(d)))
}
fn penalty(a: Point, b: Point, old: &[Segment], group: usize) -> Option<usize> {
    let horizontal = a.1 == b.1;
    let mut cost = 0;
    for s in old {
        if s.group == group {
            continue;
        }
        if horizontal == (s.a.1 == s.b.1) {
            let (distance, overlap) = if horizontal {
                (a.1.abs_diff(s.a.1), shared(a.0, b.0, s.a.0, s.b.0))
            } else {
                (a.0.abs_diff(s.a.0), shared(a.1, b.1, s.a.1, s.b.1))
            };
            if overlap > 0 {
                if distance == 0 {
                    return None;
                }
                if distance < 6 {
                    cost += overlap * 32;
                }
            }
        } else {
            let (h1, h2, v1, v2) = if horizontal {
                (a, b, s.a, s.b)
            } else {
                (s.a, s.b, a, b)
            };
            if v1.0 >= h1.0.min(h2.0)
                && v1.0 <= h1.0.max(h2.0)
                && h1.1 >= v1.1.min(v2.1)
                && h1.1 <= v1.1.max(v2.1)
            {
                cost += 24;
            }
        }
    }
    Some(cost)
}
pub(super) fn remember(points: &[Point], group: usize, old: &mut Vec<Segment>) {
    for p in points.windows(2) {
        if p[0] != p[1] {
            old.push(Segment {
                a: p[0],
                b: p[1],
                group,
            });
        }
    }
}
pub(super) fn path(points: &[Point]) -> String {
    let Some(first) = points.first() else {
        return String::new();
    };
    let mut s = format!("M{} {}", first.0, first.1);
    for i in 1..points.len() {
        if points[i] == points[i - 1] {
            continue;
        }
        if i + 1 < points.len()
            && ((points[i - 1].0 == points[i].0 && points[i].0 == points[i + 1].0)
                || (points[i - 1].1 == points[i].1 && points[i].1 == points[i + 1].1))
        {
            continue;
        }
        if points[i - 1].0 == points[i].0 {
            s.push_str(&format!("V{}", points[i].1))
        } else {
            s.push_str(&format!("H{}", points[i].0))
        }
    }
    s
}
pub(super) fn route(
    start: Point,
    end: Point,
    obstacles: &[Rect],
    old: &[Segment],
    group: usize,
    width: usize,
    height: usize,
) -> Result<Vec<Point>, String> {
    let mut xs = vec![8, width - 8, start.0, end.0];
    let mut ys = vec![8, height - 8, start.1, end.1];
    for r in obstacles {
        xs.extend([r.x, r.x + r.w]);
        ys.extend([r.y, r.y + r.h]);
    }
    for s in old {
        if s.a.0 == s.b.0 {
            xs.extend([s.a.0.saturating_sub(6), s.a.0, s.a.0 + 6]);
        } else {
            ys.extend([s.a.1.saturating_sub(6), s.a.1, s.a.1 + 6]);
        }
    }
    xs.retain(|x| *x >= 4 && *x <= width - 4);
    ys.retain(|y| *y >= 4 && *y <= height - 4);
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    let nx = xs.len();
    let ny = ys.len();
    if nx > 256 || ny > 256 {
        return Err("local routing grid budget exceeded".into());
    }
    let source = ys.binary_search(&start.1).map_err(|_| "route source")? * nx
        + xs.binary_search(&start.0).map_err(|_| "route source")?;
    let dest = ys.binary_search(&end.1).map_err(|_| "route target")? * nx
        + xs.binary_search(&end.0).map_err(|_| "route target")?;
    let mut distances = vec![usize::MAX; nx * ny * 3];
    let mut previous = vec![usize::MAX; nx * ny * 3];
    let mut heap = BinaryHeap::new();
    let point = |p: usize| (xs[p % nx], ys[p / nx]);
    distances[source * 3] = 0;
    heap.push(Reverse((
        start.0.abs_diff(end.0) + start.1.abs_diff(end.1),
        0,
        source * 3,
    )));
    let mut finish = None;
    while let Some(Reverse((_, cost, state))) = heap.pop() {
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
            let Some(extra) = penalty(a, b, old, group) else {
                continue;
            };
            let g = cost
                + a.0.abs_diff(b.0)
                + a.1.abs_diff(b.1)
                + extra
                + if direction != 0 && direction != dir {
                    16
                } else {
                    0
                };
            let ns = next * 3 + dir;
            if g < distances[ns] {
                distances[ns] = g;
                previous[ns] = state;
                heap.push(Reverse((
                    g + b.0.abs_diff(end.0) + b.1.abs_diff(end.1),
                    g,
                    ns,
                )));
            }
        }
    }
    let mut at = finish.ok_or("no distinct unobstructed local diagram route")?;
    let mut points = vec![point(at / 3)];
    while at != source * 3 {
        at = previous[at];
        if at == usize::MAX {
            return Err("incomplete local route".into());
        }
        points.push(point(at / 3));
    }
    points.reverse();
    let compact = points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            if i > 0
                && i + 1 < points.len()
                && ((points[i - 1].0 == p.0 && p.0 == points[i + 1].0)
                    || (points[i - 1].1 == p.1 && p.1 == points[i + 1].1))
            {
                None
            } else {
                Some(*p)
            }
        })
        .collect();
    Ok(compact)
}
