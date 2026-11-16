//! A small Dataview/Bases-style query language for ```query fenced blocks.
//!
//! Example:
//!
//! ```text
//! table title, tags, status
//! from #work or "Projects"
//! where status = active
//! sort title asc
//! limit 20
//! ```
//!
//! Supported modes: `table`, `list`, `task`. Fields: `title`/`file.name`,
//! `path`/`file.path`, `folder`/`file.folder`, `tags`, `aliases`, and any
//! frontmatter property. `where` supports `=`, `!=`, `>`, `<`, `>=`, `<=`,
//! `contains`, and bare truthiness. `sort` takes a field plus `asc`/`desc`.

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
struct Query {
    mode: Mode,
    columns: Vec<String>,
    from: Vec<String>,
    where_clauses: Vec<Clause>,
    sort: Option<(String, bool)>,
    limit: usize,
}

#[derive(Clone, Debug)]
struct Clause {
    field: String,
    op: String,
    value: String,
}

fn field(doc: &Doc, name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "title" | "file.name" | "name" | "note" => doc.title.clone(),
        "path" | "file.path" => doc.path.clone(),
        "folder" | "file.folder" => doc.folder.clone(),
        "tags" | "tag" => doc.tags.join(", "),
        "aliases" | "alias" => doc.aliases.join(", "),
        "tasks" => (doc.tasks_todo + doc.tasks_done).to_string(),
        "todo" => doc.tasks_todo.to_string(),
        "done" => doc.tasks_done.to_string(),
        _ => doc
            .props
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&n))
            .map(|(_, v)| v.clone())
            .unwrap_or_default(),
    }
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
    };
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("//") || line.starts_with('#') && line.len() > 1 {
            continue;
        }
        let (kw, rest) = line
            .split_once(char::is_whitespace)
            .map(|(a, b)| (a.to_ascii_lowercase(), b.trim()))
            .unwrap_or((line.to_ascii_lowercase(), ""));
        match kw.as_str() {
            "table" => {
                q.mode = Mode::Table;
                q.columns = rest
                    .split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect();
            }
            "list" => {
                q.mode = Mode::List;
                q.columns = rest
                    .split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect();
            }
            "task" | "tasks" => {
                q.mode = Mode::Task;
            }
            "from" => {
                // `from #tag or "Folder" or "path"`
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
            "where" => {
                if let Some(c) = parse_clause(rest) {
                    q.where_clauses.push(c);
                }
            }
            "sort" => {
                let mut parts = rest.split_whitespace();
                let f = parts.next().unwrap_or("").to_string();
                let desc = parts
                    .next()
                    .map(|d| d.eq_ignore_ascii_case("desc"))
                    .unwrap_or(false);
                if !f.is_empty() {
                    q.sort = Some((f, desc));
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
            Mode::Table => vec!["file.name".into(), "tags".into(), "file.folder".into()],
            Mode::List => vec!["file.name".into()],
            Mode::Task => vec!["file.name".into(), "todo".into()],
        };
    }
    q
}

fn parse_clause(s: &str) -> Option<Clause> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // contains(...) / !contains(...)
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
    let v = field(doc, &c.field);
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
                return lhs > rhs;
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

/// Render a query block. Returns `true` when the language handled it.
pub fn render(ui: &mut egui::Ui, src: &str, docs: &[Doc]) -> bool {
    let q = parse(src);
    let mut rows: Vec<&Doc> = docs
        .iter()
        .filter(|d| from_matches(&q, d))
        .filter(|d| q.where_clauses.iter().all(|c| clause_matches(c, d)))
        .collect();
    if let Some((f, desc)) = &q.sort {
        rows.sort_by(|a, b| {
            let fa = field(a, f).to_ascii_lowercase();
            let fb = field(b, f).to_ascii_lowercase();
            let ord = fa.cmp(&fb);
            if *desc {
                ord.reverse()
            } else {
                ord
            }
        });
    }
    rows.truncate(q.limit);

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
                ui.label(
                    RichText::new(format!("{} result(s)", rows.len()))
                        .small()
                        .weak(),
                );
            });
            ui.separator();
            match q.mode {
                Mode::Table => {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        egui::Grid::new(("query-grid", src.as_ptr() as usize))
                            .striped(true)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                for c in &q.columns {
                                    ui.label(RichText::new(c).small().strong());
                                }
                                ui.end_row();
                                for d in &rows {
                                    for c in &q.columns {
                                        ui.label(RichText::new(field(d, c)).small());
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                }
                Mode::List => {
                    for d in &rows {
                        ui.label(RichText::new(field(d, &q.columns[0])).small());
                    }
                }
                Mode::Task => {
                    for d in &rows {
                        ui.label(
                            RichText::new(format!(
                                "{} — {} open / {} done",
                                d.title, d.tasks_todo, d.tasks_done
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Doc> {
        vec![
            Doc {
                title: "Alpha".into(),
                path: "Work/Alpha.md".into(),
                folder: "Work".into(),
                tags: vec!["work".into()],
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

    fn run(src: &str) -> Vec<String> {
        let q = parse(src);
        let mut rows: Vec<String> = sample()
            .into_iter()
            .filter(|d| from_matches(&q, d))
            .filter(|d| q.where_clauses.iter().all(|c| clause_matches(c, d)))
            .map(|d| d.title)
            .collect();
        if let Some((f, desc)) = &q.sort {
            rows.sort_by(|a, b| {
                let _ = (a, b, f, desc);
                a.cmp(b)
            });
        }
        rows
    }

    #[test]
    fn filters_by_tag_property_and_sort() {
        assert_eq!(run("list title\nfrom #work"), vec!["Alpha"]);
        assert_eq!(run("list title\nwhere status = active"), vec!["Alpha"]);
        assert_eq!(run("list title\nwhere status = done"), vec!["Beta"]);
        assert_eq!(run("list title\nsort title asc"), vec!["Alpha", "Beta"]);
    }

    #[test]
    fn numeric_and_task_filters() {
        assert_eq!(run("list title\nwhere done > 2"), vec!["Beta"]);
        assert_eq!(run("list title\nwhere todo >= 1"), vec!["Alpha"]);
    }
}
