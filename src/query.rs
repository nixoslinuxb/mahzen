//! A small Dataview/Bases-style query language for ```query fenced blocks.
//!
//! Example:
//!
//! ```text
//! table title, status, length(tags) as "tag count"
//! from #work or "Projects"
//! where status = active
//! flatten tags
//! group by status
//! sort title asc
//! limit 20
//! ```
//!
//! Supported modes: `table`, `list`, `task`. Fields: `title`/`file.name`,
//! `path`/`file.path`, `folder`/`file.folder`, `tags`, `aliases`, `todo`,
//! `done`, `tasks`, and any frontmatter property. Expressions may use the
//! functions `length`, `upper`, `lower`, `contains`, `startswith`, `endswith`,
//! `round`, `default`, `dateformat`, `sum`, `min`, `max`, `file.link`, and
//! string/number literals. `where` supports `=`, `!=`, `>`, `<`, `>=`, `<=`,
//! `contains`, and bare truthiness. `sort` takes an expression plus
//! `asc`/`desc`; `flatten` expands list fields into one row per item;
//! `group by` aggregates rows.

use eframe::egui::{self, RichText};

/// A note as seen by the query engine.
#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub title: String,
    pub path: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub props: Vec<(String, String)>,
    pub tasks_todo: usize,
    pub tasks_done: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Table,
    List,
    Task,
}

#[derive(Clone, Debug)]
struct Column {
    expr: String,
    alias: String,
}

#[derive(Clone, Debug)]
struct Query {
    mode: Mode,
    columns: Vec<Column>,
    from: Vec<String>,
    where_clauses: Vec<Clause>,
    sort: Option<(String, bool)>,
    limit: usize,
    flatten: Option<String>,
    group_by: Option<String>,
    /// Bases-style presentation: a plain table or a grid of cards.
    cards: bool,
}

#[derive(Clone, Debug)]
struct Clause {
    field: String,
    op: String,
    value: String,
}

fn property<'a>(doc: &'a Doc, name: &str) -> &'a str {
    doc.props
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
        .unwrap_or("")
}

fn field(doc: &Doc, name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "title" | "file.name" | "name" | "note" | "file.link" => doc.title.clone(),
        "path" | "file.path" => doc.path.clone(),
        "folder" | "file.folder" => doc.folder.clone(),
        "tags" | "tag" => doc.tags.join(", "),
        "aliases" | "alias" => doc.aliases.join(", "),
        "tasks" => (doc.tasks_todo + doc.tasks_done).to_string(),
        "todo" => doc.tasks_todo.to_string(),
        "done" => doc.tasks_done.to_string(),
        _ => property(doc, &n).to_string(),
    }
}

/// Split an argument list on top-level commas (respecting quotes/parens).
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '"' | '\'' if quote.is_none() => {
                quote = Some(c);
                cur.push(c);
            }
            q if Some(q) == quote => {
                quote = None;
                cur.push(c);
            }
            '(' if quote.is_none() => {
                depth += 1;
                cur.push(c);
            }
            ')' if quote.is_none() => {
                depth -= 1;
                cur.push(c);
            }
            ',' if quote.is_none() && depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// Evaluate a (small) expression against a note. `group` supplies the row
/// count for `length(rows)` inside a `group by`.
fn eval(doc: &Doc, expr: &str, group: Option<usize>) -> String {
    let e = expr.trim();
    if e.is_empty() {
        return String::new();
    }
    if (e.starts_with('"') && e.ends_with('"') && e.len() >= 2)
        || (e.starts_with('\'') && e.ends_with('\'') && e.len() >= 2)
    {
        return e[1..e.len() - 1].to_string();
    }
    if let Some(inner) = e.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
        return eval(doc, inner, group);
    }
    // Numeric literals (and signed ones) evaluate to themselves.
    if e.trim().parse::<f64>().is_ok() {
        return e.trim().to_string();
    }
    // Function call.
    if let Some(open) = e.find('(') {
        if e.ends_with(')') {
            let name = e[..open].trim().to_ascii_lowercase();
            let args = split_args(&e[open + 1..e.len() - 1]);
            let arg = |i: usize| -> String {
                args.get(i).map(|a| eval(doc, a, group)).unwrap_or_default()
            };
            let raw = |i: usize| args.get(i).cloned().unwrap_or_default();
            return match name.as_str() {
                "length" | "len" => {
                    if raw(0).trim().eq_ignore_ascii_case("rows") {
                        group.unwrap_or(0).to_string()
                    } else if raw(0).trim().eq_ignore_ascii_case("tags")
                        || field(doc, &raw(0)) == doc.tags.join(", ")
                    {
                        doc.tags.len().to_string()
                    } else {
                        arg(0).chars().count().to_string()
                    }
                }
                "upper" => arg(0).to_uppercase(),
                "lower" => arg(0).to_lowercase(),
                "trim" => arg(0).trim().to_string(),
                "contains" => (arg(0).to_lowercase().contains(&arg(1).to_lowercase())).to_string(),
                "startswith" => arg(0)
                    .to_lowercase()
                    .starts_with(&arg(1).to_lowercase())
                    .to_string(),
                "endswith" => arg(0)
                    .to_lowercase()
                    .ends_with(&arg(1).to_lowercase())
                    .to_string(),
                "round" => {
                    let v: f64 = arg(0).parse().unwrap_or(0.0);
                    let digits: u32 = raw(1).trim().parse().unwrap_or(0);
                    format!("{:.*}", digits as usize, v)
                }
                "default" => {
                    let v = arg(0);
                    if v.trim().is_empty() {
                        arg(1)
                    } else {
                        v
                    }
                }
                "dateformat" | "format" => crate::dates::reformat(&arg(0), &arg(1)),
                "date" => {
                    // Normalise anything date-like to YYYY-MM-DD.
                    let v = arg(0);
                    if v.trim().eq_ignore_ascii_case("today") {
                        let (y, m, d) = crate::dates::today();
                        format!("{y:04}-{m:02}-{d:02}")
                    } else if let Some((y, m, d)) = crate::dates::parse_iso(&v) {
                        format!("{y:04}-{m:02}-{d:02}")
                    } else {
                        v
                    }
                }
                "year" | "month" | "day" => {
                    let v = arg(0);
                    match crate::dates::parse_iso(&v) {
                        Some((y, m, d)) => match name.as_str() {
                            "year" => y.to_string(),
                            "month" => m.to_string(),
                            _ => d.to_string(),
                        },
                        None => String::new(),
                    }
                }
                "weekday" => {
                    let v = arg(0);
                    match crate::dates::parse_iso(&v) {
                        Some((y, m, d)) => crate::dates::WEEKDAY_NAMES
                            [crate::dates::weekday(y, m, d) % 7]
                            .to_string(),
                        None => String::new(),
                    }
                }
                "reldate" => {
                    // Whole days between the value and today ("2 days ago").
                    let v = arg(0);
                    match crate::dates::parse_iso(&v) {
                        Some((y, m, d)) => {
                            let then = crate::dates::days_from_civil(y, m, d);
                            let (ty, tm, td) = crate::dates::today();
                            let now = crate::dates::days_from_civil(ty, tm, td);
                            let diff = now - then;
                            match diff {
                                0 => crate::tr!("today").to_string(),
                                1 => crate::tr!("yesterday").to_string(),
                                -1 => crate::tr!("tomorrow").to_string(),
                                n if n > 1 => crate::tr!("{} days ago", n),
                                n => crate::tr!("in {} days", -n),
                            }
                        }
                        None => String::new(),
                    }
                }
                "number" | "toNumber" => {
                    let v = arg(0);
                    match v.trim().parse::<f64>() {
                        Ok(n) => format!("{n}"),
                        Err(_) => "0".into(),
                    }
                }
                "floor" => arg(0)
                    .trim()
                    .parse::<f64>()
                    .map(|n| format!("{}", n.floor()))
                    .unwrap_or_default(),
                "ceil" => arg(0)
                    .trim()
                    .parse::<f64>()
                    .map(|n| format!("{}", n.ceil()))
                    .unwrap_or_default(),
                "abs" => arg(0)
                    .trim()
                    .parse::<f64>()
                    .map(|n| format!("{}", n.abs()))
                    .unwrap_or_default(),
                "mod" => {
                    let a: f64 = arg(0).trim().parse().unwrap_or(0.0);
                    let b: f64 = arg(1).trim().parse().unwrap_or(1.0);
                    if b == 0.0 {
                        "0".into()
                    } else {
                        format!("{}", a % b)
                    }
                }
                "replace" => arg(0).replace(&arg(1), &arg(2)),
                "substring" | "substr" => {
                    let s = arg(0);
                    let start: usize = raw(1).trim().parse().unwrap_or(0);
                    let len: Option<usize> = raw(2).trim().parse().ok();
                    s.chars()
                        .skip(start)
                        .take(len.unwrap_or(usize::MAX))
                        .collect()
                }
                "split" => {
                    let s = arg(0);
                    let sep = arg(1);
                    s.split(&sep)
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
                "join" => arg(0)
                    .split(',')
                    .map(str::trim)
                    .collect::<Vec<_>>()
                    .join(&arg(1)),
                "unique" => {
                    let mut seen: Vec<String> = Vec::new();
                    for part in arg(0).split(',') {
                        let p = part.trim().to_string();
                        if !p.is_empty() && !seen.contains(&p) {
                            seen.push(p);
                        }
                    }
                    seen.join(", ")
                }
                "count" => arg(0)
                    .split(',')
                    .filter(|p| !p.trim().is_empty())
                    .count()
                    .to_string(),
                "truncate" => {
                    let s = arg(0);
                    let n: usize = raw(1).trim().parse().unwrap_or(40);
                    if s.chars().count() <= n {
                        s
                    } else {
                        let mut t: String = s.chars().take(n).collect();
                        t.push('…');
                        t
                    }
                }
                "pad" => {
                    let s = arg(0);
                    let n: usize = raw(1).trim().parse().unwrap_or(8);
                    format!("{s:0>n$}")
                }
                "striptime" => arg(0).split('T').next().unwrap_or("").to_string(),
                "nonnull" | "coalesce" => {
                    for i in 0..args.len() {
                        let v = arg(i);
                        if !v.trim().is_empty() {
                            return v;
                        }
                    }
                    String::new()
                }
                "sum" => {
                    let vals: Vec<f64> = arg(0)
                        .split(',')
                        .filter_map(|v| v.trim().parse().ok())
                        .collect();
                    format!("{}", vals.iter().sum::<f64>())
                }
                "min" => arg(0)
                    .split(',')
                    .filter_map(|v| v.trim().parse::<f64>().ok())
                    .fold(f64::INFINITY, f64::min)
                    .to_string(),
                "max" => arg(0)
                    .split(',')
                    .filter_map(|v| v.trim().parse::<f64>().ok())
                    .fold(f64::NEG_INFINITY, f64::max)
                    .to_string(),
                _ => field(doc, e),
            };
        }
    }
    if e.eq_ignore_ascii_case("key") {
        return field(doc, e);
    }
    field(doc, e)
}

fn truthy(s: &str) -> bool {
    !matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "" | "false" | "no" | "off" | "0" | "none" | "null"
    )
}

fn parse(src: &str) -> Query {
    let mut q = Query {
        mode: Mode::Table,
        columns: Vec::new(),
        from: Vec::new(),
        where_clauses: Vec::new(),
        sort: None,
        limit: usize::MAX,
        flatten: None,
        group_by: None,
        cards: false,
    };
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || (line.starts_with('#') && line.len() > 1) {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower
            .strip_prefix("group by ")
            .map(|_| &line["group by ".len()..])
        {
            q.group_by = Some(rest.trim().to_string());
            continue;
        }
        if let Some(rest) = lower
            .strip_prefix("flatten ")
            .map(|_| &line["flatten ".len()..])
        {
            q.flatten = Some(rest.trim().to_string());
            continue;
        }
        let (kw, rest) = line
            .split_once(char::is_whitespace)
            .map(|(a, b)| (a.to_ascii_lowercase(), b.trim()))
            .unwrap_or((line.to_ascii_lowercase(), ""));
        match kw.as_str() {
            "table" | "list" => {
                q.mode = if kw == "table" {
                    Mode::Table
                } else {
                    Mode::List
                };
                q.columns = split_args(rest)
                    .into_iter()
                    .map(|c| {
                        // `expr as alias`
                        if let Some((e, a)) = c.split_once(" as ") {
                            Column {
                                expr: e.trim().to_string(),
                                alias: a.trim().trim_matches('"').to_string(),
                            }
                        } else {
                            Column {
                                alias: c.trim().to_string(),
                                expr: c.trim().to_string(),
                            }
                        }
                    })
                    .collect();
            }
            "task" | "tasks" => q.mode = Mode::Task,
            "from" => {
                for part in rest.split(" or ") {
                    let v = part
                        .trim()
                        .trim_start_matches('#')
                        .trim_matches('"')
                        .trim()
                        .to_string();
                    if !v.is_empty() {
                        q.from.push(v);
                    }
                }
            }
            "view" => {
                q.cards = rest.trim().eq_ignore_ascii_case("cards")
                    || rest.trim().eq_ignore_ascii_case("card")
                    || rest.trim().eq_ignore_ascii_case("grid");
            }
            "where" => {
                if let Some(c) = parse_clause(rest) {
                    q.where_clauses.push(c);
                }
            }
            "sort" => {
                let mut parts = rest.rsplitn(2, ' ');
                let last = parts.next().unwrap_or("");
                let desc = last.eq_ignore_ascii_case("desc");
                let asc = last.eq_ignore_ascii_case("asc");
                let expr = if desc || asc {
                    parts.next().unwrap_or("").trim().to_string()
                } else {
                    rest.trim().to_string()
                };
                if !expr.is_empty() {
                    q.sort = Some((expr, desc));
                }
            }
            "limit" => {
                if let Ok(n) = rest.trim().parse() {
                    q.limit = n;
                }
            }
            _ => {}
        }
    }
    if q.columns.is_empty() {
        q.columns = match q.mode {
            Mode::Table => vec![
                Column {
                    expr: "file.name".into(),
                    alias: "file.name".into(),
                },
                Column {
                    expr: "tags".into(),
                    alias: "tags".into(),
                },
                Column {
                    expr: "file.folder".into(),
                    alias: "file.folder".into(),
                },
            ],
            Mode::List => vec![Column {
                expr: "file.name".into(),
                alias: "file.name".into(),
            }],
            Mode::Task => vec![
                Column {
                    expr: "file.name".into(),
                    alias: "file.name".into(),
                },
                Column {
                    expr: "todo".into(),
                    alias: "todo".into(),
                },
            ],
        };
    }
    q
}

fn parse_clause(s: &str) -> Option<Clause> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(inner) = s
        .strip_prefix("contains(")
        .and_then(|x| x.strip_suffix(')'))
    {
        if let Some((f, v)) = inner.split_once(',') {
            return Some(Clause {
                field: f.trim().to_string(),
                op: "contains".into(),
                value: v.trim().trim_matches('"').to_string(),
            });
        }
    }
    for op in ["!=", ">=", "<=", "=", ">", "<"] {
        if let Some((f, v)) = s.split_once(op) {
            return Some(Clause {
                field: f.trim().to_string(),
                op: op.to_string(),
                value: v.trim().trim_matches('"').to_string(),
            });
        }
    }
    Some(Clause {
        field: s.to_string(),
        op: "truthy".into(),
        value: String::new(),
    })
}

fn clause_matches(c: &Clause, doc: &Doc) -> bool {
    let v = eval(doc, &c.field, None);
    let lhs = v.to_ascii_lowercase();
    let rhs = c.value.to_ascii_lowercase();
    match c.op.as_str() {
        "=" => lhs == rhs,
        "!=" => lhs != rhs,
        "contains" => lhs.contains(&rhs),
        ">" | "<" | ">=" | "<=" => {
            let a: f64 = v.trim().parse().unwrap_or(f64::NAN);
            let b: f64 = c.value.trim().parse().unwrap_or(f64::NAN);
            if a.is_nan() || b.is_nan() {
                return match c.op.as_str() {
                    ">" => lhs > rhs,
                    "<" => lhs < rhs,
                    ">=" => lhs >= rhs,
                    _ => lhs <= rhs,
                };
            }
            match c.op.as_str() {
                ">" => a > b,
                "<" => a < b,
                ">=" => a >= b,
                _ => a <= b,
            }
        }
        _ => truthy(&v),
    }
}

fn from_matches(q: &Query, doc: &Doc) -> bool {
    if q.from.is_empty() {
        return true;
    }
    let path = doc.path.to_ascii_lowercase();
    let folder = doc.folder.to_ascii_lowercase();
    q.from.iter().any(|f| {
        let f = f.to_ascii_lowercase();
        doc.tags.iter().any(|t| {
            let t = t.to_ascii_lowercase();
            t == f || t.starts_with(&format!("{f}/"))
        }) || folder.contains(&f)
            || path.contains(&f)
    })
}

/// A row of the result: a note plus an optional flattened field override.
struct Row<'a> {
    doc: &'a Doc,
    flat: Option<(String, String)>,
}

impl<'a> Row<'a> {
    fn eval(&self, expr: &str, group: Option<usize>) -> String {
        if let Some((key, val)) = &self.flat {
            if expr.trim().eq_ignore_ascii_case(key) {
                return val.clone();
            }
        }
        eval(self.doc, expr, group)
    }
}

fn flatten_rows<'a>(q: &Query, docs: &'a [Doc]) -> Vec<Row<'a>> {
    let mut rows: Vec<Row<'a>> = Vec::new();
    for d in docs {
        if !from_matches(q, d) || !q.where_clauses.iter().all(|c| clause_matches(c, d)) {
            continue;
        }
        match &q.flatten {
            Some(f) if f.eq_ignore_ascii_case("tags") => {
                if d.tags.is_empty() {
                    rows.push(Row { doc: d, flat: None });
                }
                for t in &d.tags {
                    rows.push(Row {
                        doc: d,
                        flat: Some((f.clone(), t.clone())),
                    });
                }
            }
            Some(f) if f.eq_ignore_ascii_case("aliases") => {
                if d.aliases.is_empty() {
                    rows.push(Row { doc: d, flat: None });
                }
                for t in &d.aliases {
                    rows.push(Row {
                        doc: d,
                        flat: Some((f.clone(), t.clone())),
                    });
                }
            }
            Some(f) => {
                // Generic list-valued property ("a, b, c").
                let val = property(d, f);
                let items: Vec<&str> = val
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if items.len() <= 1 {
                    rows.push(Row { doc: d, flat: None });
                } else {
                    for item in items {
                        rows.push(Row {
                            doc: d,
                            flat: Some((f.clone(), item.to_string())),
                        });
                    }
                }
            }
            None => rows.push(Row { doc: d, flat: None }),
        }
    }
    rows
}

/// Render a query block. Returns `true` when the language handled it.
pub fn render(ui: &mut egui::Ui, src: &str, docs: &[Doc]) -> bool {
    let q = parse(src);
    let mut rows = flatten_rows(&q, docs);
    if let Some((expr, desc)) = &q.sort {
        rows.sort_by(|a, b| {
            let fa = a.eval(expr, None).to_ascii_lowercase();
            let fb = b.eval(expr, None).to_ascii_lowercase();
            let ord = match (fa.parse::<f64>(), fb.parse::<f64>()) {
                (Ok(x), Ok(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                _ => fa.cmp(&fb),
            };
            if *desc {
                ord.reverse()
            } else {
                ord
            }
        });
    }

    // Optional grouping: preserve first-appearance order.
    let mut groups: Vec<(String, Vec<&Row>)> = Vec::new();
    if let Some(g) = &q.group_by {
        for row in &rows {
            let key = row.eval(g, None);
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, list)) => list.push(row),
                None => groups.push((key, vec![row])),
            }
        }
        groups.truncate(q.limit);
    } else {
        rows.truncate(q.limit);
    }

    egui::Frame::none()
        .fill(ui.visuals().faint_bg_color)
        .inner_margin(egui::Margin::same(8.0))
        .rounding(egui::Rounding::same(5.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(match q.mode {
                        Mode::Table => "QUERY · TABLE",
                        Mode::List => "QUERY · LIST",
                        Mode::Task => "QUERY · TASKS",
                    })
                    .small()
                    .strong()
                    .color(ui.visuals().hyperlink_color),
                );
                let count = if q.group_by.is_some() {
                    groups.len()
                } else {
                    rows.len()
                };
                ui.label(RichText::new(format!("{count} result(s)")).small().weak());
                if q.flatten.is_some() {
                    ui.label(RichText::new("· flattened").small().weak());
                }
                if let Some(g) = &q.group_by {
                    ui.label(RichText::new(format!("· grouped by {g}")).small().weak());
                }
            });
            ui.separator();
            match q.mode {
                Mode::Table if q.cards => {
                    // Bases-style card grid: one card per row of results.
                    let avail = ui.available_width().max(200.0);
                    let card_w = 240.0f32;
                    let per_row = ((avail / (card_w + 12.0)).floor() as usize).max(1);
                    let items: Vec<&Row> = if q.group_by.is_some() {
                        Vec::new()
                    } else {
                        rows.iter().collect()
                    };
                    egui::ScrollArea::vertical()
                        .max_height(520.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let mut idx = 0usize;
                            while idx < items.len() {
                                ui.horizontal_top(|ui| {
                                    for r in items.iter().skip(idx).take(per_row) {
                                        egui::Frame::none()
                                            .fill(ui.visuals().extreme_bg_color)
                                            .stroke(egui::Stroke::new(
                                                1.0_f32,
                                                ui.visuals().hyperlink_color.linear_multiply(0.3),
                                            ))
                                            .rounding(egui::Rounding::same(6.0))
                                            .inner_margin(egui::Margin::same(10.0))
                                            .show(ui, |ui| {
                                                ui.set_width(card_w - 20.0);
                                                query_card_title(ui, r, &q);
                                                ui.add_space(4.0);
                                                for c in q.columns.iter().skip(1) {
                                                    let v = r.eval(&c.expr, None);
                                                    if v.trim().is_empty() {
                                                        continue;
                                                    }
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            RichText::new(&c.alias)
                                                                .size(9.5)
                                                                .color(
                                                                    ui.visuals().weak_text_color(),
                                                                ),
                                                        );
                                                        ui.label(
                                                            RichText::new(ellipsize(&v, 34))
                                                                .size(10.5),
                                                        );
                                                    });
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }
                                });
                                idx += per_row;
                            }
                        });
                }
                Mode::Table => {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        egui::Grid::new(("query-grid", src.as_ptr() as usize))
                            .striped(true)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                for c in &q.columns {
                                    ui.label(RichText::new(&c.alias).small().strong());
                                }
                                ui.end_row();
                                if let Some(_g) = &q.group_by {
                                    for (key, list) in &groups {
                                        for c in &q.columns {
                                            let val = if c.expr.trim().eq_ignore_ascii_case("key") {
                                                key.clone()
                                            } else {
                                                list.first()
                                                    .map(|r| r.eval(&c.expr, Some(list.len())))
                                                    .unwrap_or_default()
                                            };
                                            ui.label(RichText::new(val).small());
                                        }
                                        ui.end_row();
                                    }
                                } else {
                                    for r in &rows {
                                        for c in &q.columns {
                                            ui.label(RichText::new(r.eval(&c.expr, None)).small());
                                        }
                                        ui.end_row();
                                    }
                                }
                            });
                    });
                }
                Mode::List => {
                    if q.group_by.is_some() {
                        for (key, list) in &groups {
                            ui.label(
                                RichText::new(format!("{} — {} note(s)", key, list.len())).small(),
                            );
                        }
                    } else {
                        for r in &rows {
                            ui.label(RichText::new(r.eval(&q.columns[0].expr, None)).small());
                        }
                    }
                }
                Mode::Task => {
                    for r in &rows {
                        ui.label(
                            RichText::new(format!(
                                "{} — {} open / {} done",
                                r.doc.title, r.doc.tasks_todo, r.doc.tasks_done
                            ))
                            .small(),
                        );
                    }
                }
            }
        });
    ui.add_space(4.0);
    true
}

/// Card title: the first column's value, falling back to the note title.
fn query_card_title(ui: &mut egui::Ui, r: &Row, q: &Query) {
    let first = q
        .columns
        .first()
        .map(|c| r.eval(&c.expr, None))
        .unwrap_or_default();
    let title = if first.trim().is_empty() {
        r.doc.title.clone()
    } else {
        first
    };
    ui.label(RichText::new(ellipsize(&title, 32)).small().strong());
}

fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Doc> {
        vec![
            Doc {
                title: "Alpha".into(),
                path: "Work/Alpha.md".into(),
                folder: "Work".into(),
                tags: vec!["work".into(), "rust".into()],
                props: vec![("status".into(), "active".into())],
                tasks_todo: 2,
                tasks_done: 1,
                ..Default::default()
            },
            Doc {
                title: "Beta".into(),
                path: "Home/Beta.md".into(),
                folder: "Home".into(),
                tags: vec!["home".into()],
                props: vec![("status".into(), "done".into())],
                tasks_todo: 0,
                tasks_done: 3,
                ..Default::default()
            },
        ]
    }

    fn titles(src: &str) -> Vec<String> {
        let q = parse(src);
        let docs = sample();
        let mut rows = flatten_rows(&q, &docs);
        if let Some((expr, desc)) = &q.sort {
            rows.sort_by(|a, b| {
                let fa = a.eval(expr, None);
                let fb = b.eval(expr, None);
                let o = fa.cmp(&fb);
                if *desc {
                    o.reverse()
                } else {
                    o
                }
            });
        }
        rows.iter().map(|r| r.doc.title.clone()).collect()
    }

    #[test]
    fn filters_by_tag_property_and_sort() {
        assert_eq!(titles("list title\nfrom #work"), vec!["Alpha"]);
        assert_eq!(titles("list title\nwhere status = active"), vec!["Alpha"]);
        assert_eq!(titles("list title\nwhere status = done"), vec!["Beta"]);
        assert_eq!(titles("list title\nsort title desc"), vec!["Beta", "Alpha"]);
    }

    #[test]
    fn extended_functions_and_dates() {
        let docs = sample();
        let d = &docs[0];
        assert_eq!(eval(d, "upper(title)", None), "ALPHA");
        assert_eq!(eval(d, "truncate(title, 2)", None), "Al…");
        assert_eq!(eval(d, "number(\"12\")", None), "12");
        assert_eq!(eval(d, "mod(7, 4)", None), "3");
        assert_eq!(eval(d, "replace(title, \"A\", \"B\")", None), "Blpha");
        assert_eq!(eval(d, "count(\"a,b,c\")", None), "3");
        assert_eq!(eval(d, "unique(\"a,b,a\")", None), "a, b");
        assert_eq!(eval(d, "pad(7, 3)", None), "007");
        assert_eq!(eval(d, "date(\"2024-03-05T10:00\")", None), "2024-03-05");
        assert_eq!(eval(d, "year(\"2024-03-05\")", None), "2024");
        assert_eq!(eval(d, "month(\"2024-03-05\")", None), "3");
        assert_eq!(eval(d, "day(\"2024-03-05\")", None), "5");
        assert_eq!(eval(d, "weekday(\"2024-03-05\")", None), "Tuesday");
        assert_eq!(
            eval(d, "dateformat(\"2024-03-05\", \"YYYY/MM\")", None),
            "2024/03"
        );
        assert!(eval(d, "reldate(\"2024-03-05\")", None).contains(' '));
        assert_eq!(eval(d, "coalesce(\"\", \"x\")", None), "x");
    }

    #[test]
    fn card_view_is_parsed() {
        let q = parse("table title, tags\nview cards");
        assert!(q.cards);
        let q = parse("table title");
        assert!(!q.cards);
    }

    #[test]
    fn numeric_and_task_filters() {
        assert_eq!(titles("list title\nwhere done > 2"), vec!["Beta"]);
        assert_eq!(titles("list title\nwhere todo >= 1"), vec!["Alpha"]);
    }

    #[test]
    fn functions_and_aliases() {
        let q = parse(
            "table upper(title) as \"T\", length(tags) as tags\nwhere contains(tags, \"rust\")",
        );
        assert_eq!(q.columns[0].alias, "T");
        let docs = sample();
        let rows = flatten_rows(&q, &docs);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].eval(&q.columns[0].expr, None), "ALPHA");
        assert_eq!(rows[0].eval(&q.columns[1].expr, None), "2");
    }

    #[test]
    fn flatten_expands_list_fields() {
        let q = parse("list tags\nflatten tags\nfrom #work");
        let docs = sample();
        let rows = flatten_rows(&q, &docs);
        assert_eq!(rows.len(), 2);
        let vals: Vec<String> = rows.iter().map(|r| r.eval("tags", None)).collect();
        assert_eq!(vals, vec!["work", "rust"]);
    }

    #[test]
    fn group_by_counts_rows() {
        let q = parse("table key, length(rows)\ngroup by folder");
        let docs = sample();
        let rows = flatten_rows(&q, &docs);
        let mut groups: Vec<(String, usize)> = Vec::new();
        for r in &rows {
            let key = r.eval(q.group_by.as_ref().unwrap(), None);
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, n)) => *n += 1,
                None => groups.push((key, 1)),
            }
        }
        groups.sort();
        assert_eq!(
            groups,
            vec![("Home".to_string(), 1), ("Work".to_string(), 1)]
        );
    }
}
