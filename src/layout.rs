//! Deterministic tree layout: sections are columns left to right by `order`, levels are bands
//! (1 at the bottom by the roots, 3 at the top), topics fill each cell bottom-up.
//! The browser only draws what is computed here.

use crate::model::Content;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};

pub const NODE_W: f64 = 200.0;
pub const NODE_H: f64 = 70.0;
pub const GAP_X: f64 = 16.0;
pub const GAP_Y: f64 = 22.0;
pub const COL_PAD: f64 = 18.0;
pub const COL_GAP: f64 = 26.0;
pub const HEADER_H: f64 = 84.0;
pub const BAND_PAD: f64 = 26.0;
pub const MARGIN: f64 = 32.0;
pub const GUTTER: f64 = 44.0;
pub const ROOTS_H: f64 = 120.0;
/// Rows per level cell before a section column gets wider.
pub const ROWS_BEFORE_WIDEN: usize = 3;
pub const MAX_SLOTS: usize = 3;
/// Characters per title line that fit in a node (system sans, 14 px).
pub const LINE_CHARS: usize = 22;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Column {
    pub section: String,
    pub x: f64,
    pub w: f64,
    pub slots: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Band {
    pub level: u8,
    pub y: f64,
    pub h: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Node {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub d: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    /// Top of the column headers.
    pub header_y: f64,
    pub columns: Vec<Column>,
    pub bands: Vec<Band>,
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
    /// Decorative root curves from each column down to the trunk.
    pub roots: Vec<String>,
    pub trunk: (f64, f64),
}

/// Greedy word wrap into at most two lines, the last one cut with «…».
pub fn wrap_title(title: &str, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in title.split_whitespace() {
        let len = cur.chars().count();
        if len > 0 && len + 1 + word.chars().count() > max {
            lines.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.len() > 2 {
        let rest = lines[1..].join(" ");
        lines.truncate(1);
        lines.push(rest);
    }
    if let Some(last) = lines.last_mut()
        && last.chars().count() > max
    {
        let cut: String = last.chars().take(max - 1).collect();
        *last = format!("{}…", cut.trim_end());
    }
    if let Some(first) = lines.first_mut()
        && first.chars().count() > max
    {
        let cut: String = first.chars().take(max - 1).collect();
        *first = format!("{}…", cut.trim_end());
    }
    lines
}

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// The longest chain of `needs` inside one cell ending at each topic, so prerequisites come first.
fn depths(ids: &[&str], needs: &HashMap<&str, &[String]>) -> HashMap<String, usize> {
    fn depth<'a>(
        id: &'a str,
        ids: &[&str],
        needs: &HashMap<&'a str, &'a [String]>,
        memo: &mut HashMap<String, usize>,
        guard: usize,
    ) -> usize {
        if let Some(d) = memo.get(id) {
            return *d;
        }
        let mut best = 0;
        if guard < 64 {
            for n in needs.get(id).copied().unwrap_or_default() {
                if ids.contains(&n.as_str()) {
                    best = best.max(1 + depth(n.as_str(), ids, needs, memo, guard + 1));
                }
            }
        }
        memo.insert(id.to_string(), best);
        best
    }
    let mut memo = HashMap::new();
    for id in ids {
        depth(id, ids, needs, &mut memo, 0);
    }
    memo
}

pub fn compute(c: &Content) -> Layout {
    let mut sections: Vec<_> = c.sections.iter().collect();
    sections.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let needs: HashMap<&str, &[String]> = c
        .topics
        .iter()
        .map(|t| (t.id(), t.meta.needs.as_slice()))
        .collect();

    // cells[(section, level)] = topic ids in placement order
    let mut cells: BTreeMap<(&str, u8), Vec<&str>> = BTreeMap::new();
    for t in &c.topics {
        cells
            .entry((t.meta.section.as_str(), t.meta.level.clamp(1, 3)))
            .or_default()
            .push(t.id());
    }
    let order: HashMap<&str, i64> = c.topics.iter().map(|t| (t.id(), t.meta.order)).collect();
    for ids in cells.values_mut() {
        let dep = depths(ids, &needs);
        ids.sort_by(|a, b| {
            dep[*a]
                .cmp(&dep[*b])
                .then(order[a].cmp(&order[b]))
                .then(a.cmp(b))
        });
    }

    // Column widths
    let mut columns = Vec::new();
    let mut slots_of: HashMap<&str, usize> = HashMap::new();
    let mut x = MARGIN + GUTTER;
    for s in &sections {
        let max_count = (1..=3u8)
            .map(|l| cells.get(&(s.id.as_str(), l)).map_or(0, Vec::len))
            .max()
            .unwrap_or(0);
        let slots = max_count.div_ceil(ROWS_BEFORE_WIDEN).clamp(1, MAX_SLOTS);
        let w = slots as f64 * (NODE_W + GAP_X) - GAP_X + 2.0 * COL_PAD;
        columns.push(Column {
            section: s.id.clone(),
            x: r1(x),
            w: r1(w),
            slots,
        });
        slots_of.insert(s.id.as_str(), slots);
        x += w + COL_GAP;
    }
    let width = r1(x - COL_GAP + MARGIN);

    // Band heights: the most rows any section needs on that level.
    let rows_on = |level: u8| -> usize {
        sections
            .iter()
            .map(|s| {
                let n = cells.get(&(s.id.as_str(), level)).map_or(0, Vec::len);
                n.div_ceil(slots_of[s.id.as_str()])
            })
            .max()
            .unwrap_or(0)
            .max(1)
    };
    let header_y = MARGIN;
    let mut y = header_y + HEADER_H;
    let mut bands = Vec::new();
    for level in [3u8, 2, 1] {
        let rows = rows_on(level) as f64;
        let h = rows * (NODE_H + GAP_Y) - GAP_Y + 2.0 * BAND_PAD;
        bands.push(Band {
            level,
            y: r1(y),
            h: r1(h),
        });
        y += h;
    }
    let ground = y;
    let height = r1(ground + ROOTS_H + MARGIN);

    // Nodes
    let mut nodes = BTreeMap::new();
    let titles: HashMap<&str, &str> = c
        .topics
        .iter()
        .map(|t| (t.id(), t.meta.title.as_str()))
        .collect();
    for col in &columns {
        for band in &bands {
            let Some(ids) = cells.get(&(col.section.as_str(), band.level)) else {
                continue;
            };
            let slots = col.slots;
            let bottom = band.y + band.h - BAND_PAD;
            for (row, chunk) in ids.chunks(slots).enumerate() {
                let inset = (slots - chunk.len()) as f64 * (NODE_W + GAP_X) / 2.0;
                for (i, id) in chunk.iter().enumerate() {
                    let nx = col.x + COL_PAD + inset + i as f64 * (NODE_W + GAP_X);
                    let ny = bottom - (row as f64 + 1.0) * NODE_H - row as f64 * GAP_Y;
                    nodes.insert(
                        id.to_string(),
                        Node {
                            x: r1(nx),
                            y: r1(ny),
                            w: NODE_W,
                            h: NODE_H,
                            lines: wrap_title(titles[id], LINE_CHARS),
                        },
                    );
                }
            }
        }
    }

    // Edges: need → topic
    let mut edges = Vec::new();
    for t in &c.topics {
        let Some(to) = nodes.get(t.id()) else {
            continue;
        };
        for n in &t.meta.needs {
            let Some(from) = nodes.get(n.as_str()) else {
                continue;
            };
            edges.push(Edge {
                from: n.clone(),
                to: t.meta.id.clone(),
                d: edge_path(from, to),
            });
        }
    }
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    // Roots: every column's foot curves down to one trunk point under the middle.
    let trunk = (r1(width / 2.0), r1(ground + ROOTS_H - 12.0));
    let roots = columns
        .iter()
        .map(|col| {
            let fx = col.x + col.w / 2.0;
            let mid = ground + ROOTS_H * 0.55;
            format!(
                "M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
                fx,
                ground,
                fx,
                mid,
                trunk.0,
                mid - 10.0,
                trunk.0,
                trunk.1
            )
        })
        .collect();

    Layout {
        width,
        height,
        header_y,
        columns,
        bands,
        nodes,
        edges,
        roots,
        trunk,
    }
}

/// A smooth curve from the prerequisite to the topic: vertical when the topic is above,
/// sideways when they share a row.
pub fn edge_path(a: &Node, b: &Node) -> String {
    let (acx, bcx) = (a.x + a.w / 2.0, b.x + b.w / 2.0);
    if b.y + b.h <= a.y {
        // b above a: top of a → bottom of b
        let (sx, sy, tx, ty) = (acx, a.y, bcx, b.y + b.h);
        let dy = ((sy - ty) / 2.0).max(24.0);
        format!(
            "M{sx:.1},{sy:.1} C{sx:.1},{:.1} {tx:.1},{:.1} {tx:.1},{ty:.1}",
            sy - dy,
            ty + dy
        )
    } else if a.y + a.h <= b.y {
        // b below a (a need of a higher level): bottom of a → top of b
        let (sx, sy, tx, ty) = (acx, a.y + a.h, bcx, b.y);
        let dy = ((ty - sy) / 2.0).max(24.0);
        format!(
            "M{sx:.1},{sy:.1} C{sx:.1},{:.1} {tx:.1},{:.1} {tx:.1},{ty:.1}",
            sy + dy,
            ty - dy
        )
    } else {
        // same row: side to side
        let (sx, tx) = if bcx >= acx {
            (a.x + a.w, b.x)
        } else {
            (a.x, b.x + b.w)
        };
        let (sy, ty) = (a.y + a.h / 2.0, b.y + b.h / 2.0);
        let dx = ((tx - sx).abs() / 2.0).max(16.0) * if tx >= sx { 1.0 } else { -1.0 };
        format!(
            "M{sx:.1},{sy:.1} C{:.1},{sy:.1} {:.1},{ty:.1} {tx:.1},{ty:.1}",
            sx + dx,
            tx - dx
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_two_lines_and_ellipsis() {
        assert_eq!(wrap_title("Символ веры", 22), vec!["Символ веры"]);
        let w = wrap_title(
            "Догматическое богословие: Троица, христология и экклезиология",
            22,
        );
        assert_eq!(w.len(), 2);
        assert!(w[1].ends_with('…'));
        assert!(w.iter().all(|l| l.chars().count() <= 22));
    }
}
