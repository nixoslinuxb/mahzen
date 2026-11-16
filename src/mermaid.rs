//! A built-in renderer for a useful subset of Mermaid **flowcharts**.
//!
//! No JavaScript engine or external `mmdc` required: this parses the common
//! `graph`/`flowchart` syntax (node shapes, `-->`/`---`/`-.->` edges, edge
//! labels) and lays it out with a simple layered algorithm, then draws it with
//! `egui` shapes. Unsupported diagram types return `false` so the caller can
//! fall back.

use eframe::egui::{self, RichText, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Rect,
    Round,
    Stadium,
    Diamond,
    Circle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: Shape,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: String,
    pub dashed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    TopDown,
    LeftRight,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Flowchart {
    pub dir: Dir,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// Parse Mermaid flowchart source. Returns `None` for unsupported diagrams.
pub fn parse(src: &str) -> Option<Flowchart> {
    let mut dir: Option<Dir> = None;
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();

    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if dir.is_none() {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("graph") || lower.starts_with("flowchart") {
                // Split off the keyword and the direction token; the rest may
                // contain statements on the same line.
                let after_kw = line
                    .split_once(char::is_whitespace)
                    .map(|(_, r)| r)
                    .unwrap_or("");
                let (dir_tok, rest) = after_kw
                    .split_once(char::is_whitespace)
                    .unwrap_or((after_kw, ""));
                let dt = dir_tok.to_ascii_lowercase();
                dir = Some(if dt.starts_with("lr") || dt.starts_with("rl") {
                    Dir::LeftRight
                } else {
                    Dir::TopDown
                });
                for stmt in rest.split(';') {
                    parse_statement(stmt, &mut nodes, &mut edges);
                }
                continue;
            }
            return None;
        }
        for stmt in line.split(';') {
            parse_statement(stmt, &mut nodes, &mut edges);
        }
    }
    let dir = dir?;
    if nodes.is_empty() {
        return None;
    }
    Some(Flowchart { dir, nodes, edges })
}

fn ensure_node(nodes: &mut Vec<Node>, id: &str, label: Option<String>, shape: Option<Shape>) {
    let id = id.trim();
    if id.is_empty() {
        return;
    }
    if let Some(n) = nodes.iter_mut().find(|n| n.id == id) {
        if let Some(l) = label {
            n.label = l;
        }
        if let Some(s) = shape {
            n.shape = s;
        }
        return;
    }
    nodes.push(Node {
        id: id.to_string(),
        label: label.unwrap_or_else(|| id.to_string()),
        shape: shape.unwrap_or(Shape::Rect),
    });
}

fn parse_statement(stmt: &str, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>) {
    let stmt = stmt.trim().trim_end_matches(';').trim();
    if stmt.is_empty() || stmt.starts_with("%%") {
        return;
    }
    if stmt.starts_with("subgraph")
        || stmt == "end"
        || stmt.starts_with("style")
        || stmt.starts_with("classDef")
        || stmt.starts_with("linkStyle")
        || stmt.starts_with("click")
    {
        return;
    }

    // Chain parser: node (arrow node)*, with optional edge labels.
    let mut rest = stmt;
    let mut prev: Option<String> = None;
    let mut pending: Option<(Arrow, String)> = None;
    loop {
        match find_arrow(rest) {
            Some((a, b, kind, label)) => {
                let (id, l, sh) = parse_node_token(&rest[..a]);
                ensure_node(nodes, &id, l, sh);
                if let (Some(p), Some((k, lab))) = (&prev, &pending) {
                    edges.push(Edge {
                        from: p.clone(),
                        to: id.clone(),
                        label: lab.clone(),
                        dashed: *k == Arrow::Dashed,
                    });
                }
                prev = Some(id);
                pending = Some((kind, label));
                rest = &rest[b..];
            }
            None => {
                let (id, l, sh) = parse_node_token(rest);
                if !id.is_empty() {
                    ensure_node(nodes, &id, l, sh);
                    if let (Some(p), Some((k, lab))) = (&prev, &pending) {
                        edges.push(Edge {
                            from: p.clone(),
                            to: id.clone(),
                            label: lab.clone(),
                            dashed: *k == Arrow::Dashed,
                        });
                    }
                }
                break;
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum Arrow {
    Solid,
    Open,
    Dashed,
}

/// Find the earliest arrow in `s`: `-.->`, `-->|l|`, `-- l -->`, `-->`, `---`, `==>`.
fn find_arrow(s: &str) -> Option<(usize, usize, Arrow, String)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if s[i..].starts_with("-.") {
            if let Some(rel) = s[i..].find("->") {
                return Some((i, i + rel + 2, Arrow::Dashed, String::new()));
            }
        }
        if s[i..].starts_with("-->") {
            let after = i + 3;
            if s[after..].starts_with('|') {
                if let Some(close) = s[after + 1..].find('|') {
                    let label = s[after + 1..after + 1 + close].to_string();
                    return Some((i, after + 1 + close + 1, Arrow::Solid, label));
                }
            }
            return Some((i, after, Arrow::Solid, String::new()));
        }
        if s[i..].starts_with("---") {
            return Some((i, i + 3, Arrow::Open, String::new()));
        }
        if s[i..].starts_with("==>") {
            return Some((i, i + 3, Arrow::Solid, String::new()));
        }
        if s[i..].starts_with("--") {
            if let Some(rel) = s[i + 2..].find("-->") {
                let label = s[i + 2..i + 2 + rel].trim().to_string();
                return Some((i, i + 2 + rel + 3, Arrow::Solid, label));
            }
        }
        i += 1;
    }
    None
}

/// Parse `id`, `id[Label]`, `id(Label)`, `id([Label])`, `id{Label}`, `id((Label))`.
fn parse_node_token(tok: &str) -> (String, Option<String>, Option<Shape>) {
    let tok = tok.trim();
    for (open, close, shape) in [
        ("((", "))", Shape::Circle),
        ("([", "])", Shape::Stadium),
        ("[", "]", Shape::Rect),
        ("(", ")", Shape::Round),
        ("{", "}", Shape::Diamond),
    ] {
        if let Some(o) = tok.find(open) {
            if let Some(c) = tok.rfind(close) {
                if c > o {
                    let id = tok[..o].trim().to_string();
                    let mut label = tok[o + open.len()..c].trim().to_string();
                    label = label.trim_matches('"').to_string();
                    return (id, Some(label), Some(shape));
                }
            }
        }
    }
    (tok.trim_matches('"').to_string(), None, None)
}

/// Layered layout. Returns one `(x, y)` per node, roughly in `[-1, 1]`.
pub fn layout(fc: &Flowchart) -> Vec<(f32, f32)> {
    let n = fc.nodes.len();
    if n == 0 {
        return Vec::new();
    }
    let index = |id: &str| fc.nodes.iter().position(|x| x.id == id);
    let mut rank = vec![0usize; n];
    // Relax ranks along edges (capped for cycles).
    for _ in 0..n {
        let mut changed = false;
        for e in &fc.edges {
            if let (Some(a), Some(b)) = (index(&e.from), index(&e.to)) {
                if rank[b] < rank[a] + 1 {
                    rank[b] = rank[a] + 1;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let max_rank = *rank.iter().max().unwrap_or(&0);
    // Group by rank, keeping declaration order within a rank.
    let mut per_rank: Vec<Vec<usize>> = vec![Vec::new(); max_rank + 1];
    for (i, r) in rank.iter().enumerate() {
        per_rank[*r].push(i);
    }
    let mut pos = vec![(0.0f32, 0.0f32); n];
    for (r, group) in per_rank.iter().enumerate() {
        let count = group.len().max(1);
        for (slot, &i) in group.iter().enumerate() {
            // Centre each rank's row/column around 0.
            let line = slot as f32 - (count as f32 - 1.0) / 2.0;
            let depth = r as f32;
            pos[i] = match fc.dir {
                Dir::TopDown => (line, depth),
                Dir::LeftRight => (depth, line),
            };
        }
    }
    pos
}

fn shape_rect(ui: &egui::Ui, shape: Shape, center: egui::Pos2, size: Vec2) -> egui::Rect {
    let _ = (ui, shape);
    egui::Rect::from_center_size(center, size)
}

/// Render a Mermaid diagram (flowchart, sequence diagram or pie chart).
/// Returns `true` if the source was handled natively.
pub fn render(ui: &mut egui::Ui, src: &str) -> bool {
    let first = src
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
        .unwrap_or("")
        .to_ascii_lowercase();
    if first.starts_with("sequencediagram") {
        return render_sequence(ui, src);
    }
    if first.starts_with("pie") {
        return render_pie(ui, src);
    }
    let Some(fc) = parse(src) else {
        return false;
    };
    let pos = layout(&fc);
    let n = fc.nodes.len();

    // Node sizes from label length.
    let sizes: Vec<Vec2> = fc
        .nodes
        .iter()
        .map(|node| {
            let chars = node.label.chars().count().max(3) as f32;
            let w = (chars * 8.0 + 24.0).clamp(60.0, 260.0);
            let h = if node.shape == Shape::Rect || node.shape == Shape::Round {
                40.0
            } else {
                46.0
            };
            Vec2::new(w, h)
        })
        .collect();

    // World bounds.
    let (mut minx, mut miny, mut maxx, mut maxy) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for i in 0..n {
        let (px, py) = pos[i];
        let s = sizes[i];
        // Use columns/rows spacing of ~1 unit => scale later.
        let w = s.x;
        let h = s.y;
        let (cx, cy) = match fc.dir {
            Dir::TopDown => (px * 200.0, py * 120.0),
            Dir::LeftRight => (px * 220.0, py * 90.0),
        };
        minx = minx.min(cx - w / 2.0);
        maxx = maxx.max(cx + w / 2.0);
        miny = miny.min(cy - h / 2.0);
        maxy = maxy.max(cy + h / 2.0);
    }
    let world_w = (maxx - minx).max(1.0);
    let world_h = (maxy - miny).max(1.0);
    let avail = ui.available_width().clamp(200.0, 900.0);
    let scale = (avail / world_w).min(1.4);
    let canvas_size = Vec2::new(world_w * scale + 40.0, world_h * scale + 40.0);

    let (rect, _) = ui.allocate_exact_size(canvas_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let origin = rect.min + Vec2::new(20.0, 20.0);
    let map = |px: f32, py: f32| -> egui::Pos2 {
        let (cx, cy) = match fc.dir {
            Dir::TopDown => (px * 200.0, py * 120.0),
            Dir::LeftRight => (px * 220.0, py * 90.0),
        };
        origin + Vec2::new((cx - minx) * scale, (cy - miny) * scale)
    };

    let accent = ui.visuals().hyperlink_color;
    let fill = ui.visuals().faint_bg_color;
    let text = ui.visuals().text_color();

    // Edges first.
    for e in &fc.edges {
        let (Some(a), Some(b)) = (
            fc.nodes.iter().position(|x| x.id == e.from),
            fc.nodes.iter().position(|x| x.id == e.to),
        ) else {
            continue;
        };
        let ca = map(pos[a].0, pos[a].1);
        let cb = map(pos[b].0, pos[b].1);
        // Trim to node edges.
        let dirv = (cb - ca).normalized();
        let start = ca + dirv * (sizes[a].length() * scale / 2.0).min(40.0);
        let end = cb - dirv * (sizes[b].length() * scale / 2.0).min(40.0);
        let stroke = egui::Stroke::new(1.6_f32, accent.linear_multiply(0.8));
        if e.dashed {
            draw_dashed(&painter, start, end, stroke);
        } else {
            painter.line_segment([start, end], stroke);
        }
        // Arrowhead.
        let perp = Vec2::new(-dirv.y, dirv.x);
        let tip = end;
        let base = end - dirv * 10.0;
        let tri = vec![tip, base + perp * 5.0, base - perp * 5.0];
        painter.add(egui::Shape::convex_polygon(tri, accent, egui::Stroke::NONE));
        if !e.label.is_empty() {
            let mid = start + (end - start) * 0.5;
            painter.text(
                mid,
                egui::Align2::CENTER_CENTER,
                &e.label,
                egui::FontId::proportional(12.0),
                text,
            );
        }
    }

    // Nodes.
    for i in 0..n {
        let node = &fc.nodes[i];
        let center = map(pos[i].0, pos[i].1);
        let size = sizes[i] * scale;
        let rect = shape_rect(ui, node.shape, center, size);
        let stroke = egui::Stroke::new(1.4_f32, accent.linear_multiply(0.7));
        match node.shape {
            Shape::Rect => {
                painter.rect(rect, egui::Rounding::same(4.0), fill, stroke);
            }
            Shape::Round => {
                painter.rect(rect, egui::Rounding::same(12.0), fill, stroke);
            }
            Shape::Stadium => {
                painter.rect(rect, egui::Rounding::same(size.y / 2.0), fill, stroke);
            }
            Shape::Circle => {
                painter.circle(center, size.x.max(size.y) / 2.0, fill, stroke);
            }
            Shape::Diamond => {
                let p = |dx: f32, dy: f32| center + Vec2::new(dx, dy);
                let pts = vec![
                    p(0.0, -size.y / 2.0),
                    p(size.x / 2.0, 0.0),
                    p(0.0, size.y / 2.0),
                    p(-size.x / 2.0, 0.0),
                ];
                painter.add(egui::Shape::convex_polygon(pts, fill, stroke));
            }
        }
        painter.text(
            center,
            egui::Align2::CENTER_CENTER,
            &node.label,
            egui::FontId::proportional(13.0),
            text,
        );
    }

    // A tiny caption noting the direction.
    ui.add_space(2.0);
    ui.label(
        RichText::new(match fc.dir {
            Dir::TopDown => "mermaid · flowchart (top-down)",
            Dir::LeftRight => "mermaid · flowchart (left-right)",
        })
        .weak()
        .small(),
    );
    true
}

// ------------------------------------------------------------ sequence diagram

struct Participant {
    id: String,
    label: String,
}

struct Message {
    from: usize,
    to: usize,
    text: String,
    dashed: bool,
}

/// Parse the subset of `sequenceDiagram` used day-to-day.
fn parse_sequence(src: &str) -> Option<(Vec<Participant>, Vec<Message>, Option<String>)> {
    let mut parts: Vec<Participant> = Vec::new();
    let mut msgs: Vec<Message> = Vec::new();
    let mut note: Option<String> = None;
    let ensure = |id: &str, parts: &mut Vec<Participant>| -> usize {
        if let Some(i) = parts.iter().position(|p| p.id == id) {
            return i;
        }
        parts.push(Participant {
            id: id.to_string(),
            label: id.to_string(),
        });
        parts.len() - 1
    };
    let mut seen_header = false;
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if !seen_header {
            if line.to_ascii_lowercase().starts_with("sequencediagram") {
                seen_header = true;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("participant ") {
            let (id, label) = match rest.split_once(" as ") {
                Some((a, b)) => (a.trim(), b.trim()),
                None => (rest.trim(), rest.trim()),
            };
            let i = ensure(id, &mut parts);
            parts[i].label = label.to_string();
            continue;
        }
        if let Some(rest) = line.strip_prefix("Note ") {
            note = Some(rest.to_string());
            continue;
        }
        // Messages: A->>B: text, A-->>B: text, A->B: text, A-->B: text
        if let Some((from_id, body)) = split_arrow(line) {
            let (text, to_id) = match body.split_once(':') {
                Some((to, t)) => (t.trim().to_string(), to.trim().to_string()),
                None => (String::new(), body.trim().to_string()),
            };
            let a = ensure(&from_id, &mut parts);
            let b = ensure(&to_id, &mut parts);
            msgs.push(Message {
                from: a,
                to: b,
                text,
                dashed: line.contains("--"),
            });
        }
    }
    if !seen_header || parts.is_empty() {
        return None;
    }
    Some((parts, msgs, note))
}

/// Split `A->>B: hello` into `("A", "B: hello")`.
fn split_arrow(line: &str) -> Option<(String, String)> {
    for pat in ["->>", "-->>", "-->", "->"] {
        if let Some((a, b)) = line.split_once(pat) {
            return Some((a.trim().to_string(), b.trim().to_string()));
        }
    }
    None
}

fn render_sequence(ui: &mut egui::Ui, src: &str) -> bool {
    let Some((parts, msgs, _note)) = parse_sequence(src) else {
        return false;
    };
    let col_w = 190.0f32;
    let left = 30.0;
    let top = 20.0;
    let header_h = 34.0;
    let line_h = 40.0;
    let width = left * 2.0 + col_w * parts.len() as f32;
    let height = top + header_h + line_h * (msgs.len().max(1) as f32) + 30.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let origin = rect.min;
    let accent = ui.visuals().hyperlink_color;
    let fill = ui.visuals().faint_bg_color;
    let text = ui.visuals().text_color();
    let cx = |i: usize| origin.x + left + col_w * (i as f32 + 0.5);

    // Participants + lifelines.
    for (i, p) in parts.iter().enumerate() {
        let r = egui::Rect::from_center_size(
            egui::pos2(cx(i), origin.y + top + header_h / 2.0),
            Vec2::new(col_w * 0.8, header_h),
        );
        painter.rect(
            r,
            egui::Rounding::same(4.0),
            fill,
            egui::Stroke::new(1.2_f32, accent),
        );
        painter.text(
            r.center(),
            egui::Align2::CENTER_CENTER,
            &p.label,
            egui::FontId::proportional(13.0),
            text,
        );
        painter.line_segment(
            [
                egui::pos2(cx(i), origin.y + top + header_h),
                egui::pos2(cx(i), origin.y + height - 12.0),
            ],
            egui::Stroke::new(1.0_f32, accent.linear_multiply(0.35)),
        );
    }
    // Messages.
    for (n, m) in msgs.iter().enumerate() {
        let y = origin.y + top + header_h + line_h * (n as f32 + 0.5);
        let x1 = cx(m.from);
        let x2 = cx(m.to);
        let stroke = egui::Stroke::new(1.4_f32, accent);
        if m.dashed {
            draw_dashed(&painter, egui::pos2(x1, y), egui::pos2(x2, y), stroke);
        } else {
            painter.line_segment([egui::pos2(x1, y), egui::pos2(x2, y)], stroke);
        }
        let dir = if x2 >= x1 { 1.0 } else { -1.0 };
        let tip = egui::pos2(x2, y);
        let base = egui::pos2(x2 - dir * 10.0, y);
        painter.add(egui::Shape::convex_polygon(
            vec![tip, base + Vec2::new(0.0, -5.0), base + Vec2::new(0.0, 5.0)],
            accent,
            egui::Stroke::NONE,
        ));
        if !m.text.is_empty() {
            painter.text(
                egui::pos2((x1 + x2) / 2.0, y - 9.0),
                egui::Align2::CENTER_BOTTOM,
                &m.text,
                egui::FontId::proportional(12.0),
                text,
            );
        }
    }
    ui.add_space(2.0);
    ui.label(RichText::new("mermaid · sequence diagram").weak().small());
    true
}

// ---------------------------------------------------------------- pie chart

fn render_pie(ui: &mut egui::Ui, src: &str) -> bool {
    let mut title = String::new();
    let mut slices: Vec<(String, f64)> = Vec::new();
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("pie") {
            if let Some(t) = line.split_once("title").map(|(_, t)| t.trim()) {
                title = t.to_string();
            }
            continue;
        }
        if let Some((label, val)) = line.rsplit_once(':') {
            let label = label.trim().trim_matches('"').to_string();
            if let Ok(v) = val.trim().parse::<f64>() {
                if v > 0.0 {
                    slices.push((label, v));
                }
            }
        }
    }
    if slices.is_empty() {
        return false;
    }
    let total: f64 = slices.iter().map(|(_, v)| *v).sum();
    let radius = 100.0f32;
    let legend_w = 220.0f32;
    let size = Vec2::new(radius * 2.0 + 30.0 + legend_w, radius * 2.0 + 30.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.min + Vec2::new(radius + 15.0, radius + 15.0);
    let colors = [
        egui::Color32::from_rgb(0x8a, 0x5c, 0xf5),
        egui::Color32::from_rgb(0x4f, 0xc1, 0x9a),
        egui::Color32::from_rgb(0xe0, 0x8c, 0x4f),
        egui::Color32::from_rgb(0x5a, 0x9c, 0xe0),
        egui::Color32::from_rgb(0xd0, 0x5f, 0x8f),
        egui::Color32::from_rgb(0xb8, 0xc0, 0x4f),
        egui::Color32::from_rgb(0x9a, 0x7a, 0xd0),
        egui::Color32::from_rgb(0x66, 0xb8, 0xb8),
    ];
    let mut angle = -std::f32::consts::FRAC_PI_2;
    for (i, (_label, v)) in slices.iter().enumerate() {
        let sweep = (*v / total * std::f64::consts::TAU) as f32;
        let steps = ((sweep / 0.12).ceil() as usize).max(3);
        let mut pts = vec![center];
        for s in 0..=steps {
            let a = angle + sweep * (s as f32 / steps as f32);
            pts.push(center + Vec2::new(a.cos(), a.sin()) * radius);
        }
        let color = colors[i % colors.len()];
        painter.add(egui::Shape::convex_polygon(
            pts,
            color,
            egui::Stroke::new(1.0_f32, ui.visuals().panel_fill),
        ));
        angle += sweep;
    }
    // Legend.
    let mut y = rect.min.y + 26.0;
    if !title.is_empty() {
        painter.text(
            egui::pos2(rect.min.x + radius * 2.0 + 40.0, rect.min.y + 6.0),
            egui::Align2::LEFT_TOP,
            &title,
            egui::FontId::proportional(14.0),
            ui.visuals().text_color(),
        );
        y += 12.0;
    }
    for (i, (label, v)) in slices.iter().enumerate() {
        let x = rect.min.x + radius * 2.0 + 34.0;
        let color = colors[i % colors.len()];
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(x, y), Vec2::splat(10.0)),
            egui::Rounding::same(2.0),
            color,
        );
        let pct = *v / total * 100.0;
        painter.text(
            egui::pos2(x + 16.0, y - 1.0),
            egui::Align2::LEFT_TOP,
            format!("{label} — {pct:.1}%"),
            egui::FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
        y += 20.0;
    }
    ui.add_space(2.0);
    ui.label(RichText::new("mermaid · pie chart").weak().small());
    true
}

fn draw_dashed(painter: &egui::Painter, a: egui::Pos2, b: egui::Pos2, stroke: egui::Stroke) {
    let total = (b - a).length();
    let seg = 6.0;
    let mut t = 0.0;
    while t < total {
        let t2 = (t + seg).min(total);
        let p1 = a + (b - a) * (t / total).min(1.0);
        let p2 = a + (b - a) * (t2 / total).min(1.0);
        painter.line_segment([p1, p2], stroke);
        t += seg * 2.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_nodes() {
        let fc = parse("graph TD\n A[Start] --> B{Ok?}\n B -->|yes| C(End)").unwrap();
        assert_eq!(fc.dir, Dir::TopDown);
        assert_eq!(fc.nodes.len(), 3);
        assert_eq!(fc.nodes[0].label, "Start");
        assert_eq!(fc.nodes[1].shape, Shape::Diamond);
        assert_eq!(fc.nodes[2].shape, Shape::Round);
        assert_eq!(fc.edges.len(), 2);
        assert_eq!(fc.edges[1].label, "yes");
    }

    #[test]
    fn parses_dashed_and_open_links() {
        let fc = parse("flowchart LR\n A -.-> B\n B --- C").unwrap();
        assert_eq!(fc.dir, Dir::LeftRight);
        assert!(fc.edges[0].dashed);
        assert!(!fc.edges[1].dashed);
    }

    #[test]
    fn parses_inline_statements_with_semicolons() {
        let fc = parse("flowchart TD; A-->B; B-->C;").unwrap();
        assert_eq!(fc.nodes.len(), 3);
        assert_eq!(fc.edges.len(), 2);
    }

    #[test]
    fn rejects_non_flowcharts() {
        assert!(parse("sequenceDiagram\n A->>B: hi").is_none());
        assert!(parse("").is_none());
    }

    #[test]
    fn layout_ranks_follow_edges() {
        let fc = parse("graph TD\n A --> B --> C").unwrap();
        let pos = layout(&fc);
        // A above B above C in top-down (y increases with rank).
        assert!(pos[0].1 < pos[1].1);
        assert!(pos[1].1 < pos[2].1);
    }

    #[test]
    fn layout_left_right() {
        let fc = parse("flowchart LR\n A --> B").unwrap();
        let pos = layout(&fc);
        assert!(pos[0].0 < pos[1].0);
        assert_eq!(pos[0].1, pos[1].1);
    }
}
