//! Search query language.
//!
//! Supported: plain terms, `"quoted phrases"`, `tag:`, `path:`, `file:`,
//! `property:key=value` (or `prop:`), `task:todo|done`, `section:"Heading"`,
//! `line:N` / `line:>N` / `line:10-20`, `/regex/`, `-` negation and `OR`
//! (each `OR`-separated group is ANDed internally, groups are alternatives).

use regex::Regex;

/// A document as seen by the search engine.
pub struct Doc<'a> {
    pub title: &'a str,
    pub path: &'a str,
    pub tags: &'a [String],
    pub content: &'a str,
    /// Frontmatter properties (`key`, display value).
    pub props: &'a [(String, String)],
}

enum Term {
    Text(String),
    Tag(String),
    Path(String),
    File(String),
    Property(String, Option<String>),
    Task(bool),
    Section(String),
    Line(String),
    Regex(Regex),
}

pub struct Query {
    /// ANDed terms, ORed across groups.
    groups: Vec<Vec<(bool, Term)>>,
}

impl Query {
    pub fn parse(input: &str) -> Query {
        // Split on the standalone `OR` operator (case-insensitive).
        let mut groups: Vec<Vec<(bool, Term)>> = vec![Vec::new()];
        for raw in tokenize(input) {
            if raw.eq_ignore_ascii_case("OR") {
                groups.push(Vec::new());
                continue;
            }
            let mut tok = raw.as_str();
            let mut negate = false;
            if let Some(rest) = tok.strip_prefix('-') {
                if !rest.is_empty() && !rest.starts_with('-') {
                    negate = true;
                    tok = rest;
                }
            }
            if tok.is_empty() {
                continue;
            }
            let lower = tok.to_ascii_lowercase();
            let term = if let Some(v) = lower.strip_prefix("tag:").map(|_| &tok[4..]) {
                Term::Tag(v.to_string())
            } else if let Some(v) = lower.strip_prefix("path:").map(|_| &tok[5..]) {
                Term::Path(v.to_string())
            } else if let Some(v) = lower.strip_prefix("file:").map(|_| &tok[5..]) {
                Term::File(v.to_string())
            } else if let Some(v) = lower
                .strip_prefix("property:")
                .map(|_| &tok[9..])
                .or_else(|| lower.strip_prefix("prop:").map(|_| &tok[5..]))
            {
                match v.split_once('=') {
                    Some((k, val)) => {
                        Term::Property(k.trim().to_string(), Some(val.trim().to_string()))
                    }
                    None => Term::Property(v.trim().to_string(), None),
                }
            } else if let Some(v) = lower.strip_prefix("task:").map(|_| &tok[5..]) {
                Term::Task(!matches!(
                    v.to_ascii_lowercase().as_str(),
                    "done" | "x" | "checked" | "complete" | "completed"
                ))
            } else if let Some(v) = lower.strip_prefix("section:").map(|_| &tok[8..]) {
                Term::Section(v.trim_matches('"').to_string())
            } else if let Some(v) = lower.strip_prefix("line:").map(|_| &tok[5..]) {
                Term::Line(v.to_string())
            } else if tok.len() >= 2 && tok.starts_with('/') && tok.ends_with('/') {
                let pat = &tok[1..tok.len() - 1];
                match Regex::new(pat) {
                    Ok(re) => Term::Regex(re),
                    Err(_) => Term::Text(pat.to_string()),
                }
            } else {
                Term::Text(tok.to_string())
            };
            groups.last_mut().unwrap().push((negate, term));
        }
        groups.retain(|g| !g.is_empty());
        if groups.is_empty() {
            groups.push(Vec::new());
        }
        Query { groups }
    }

    pub fn matches(&self, doc: &Doc) -> bool {
        self.groups.iter().any(|group| {
            group.iter().all(|(neg, term)| {
                let m = term_matches(term, doc);
                if *neg {
                    !m
                } else {
                    m
                }
            })
        })
    }

    /// First content line matching a positive plain/regex/section term, for a
    /// snippet. Groups are tried in order.
    pub fn snippet(&self, doc: &Doc) -> Option<(usize, String)> {
        for group in &self.groups {
            if !group.iter().all(|(neg, term)| {
                let m = term_matches(term, doc);
                if *neg {
                    !m
                } else {
                    m
                }
            }) {
                continue;
            }
            for (neg, term) in group {
                if *neg {
                    continue;
                }
                match term {
                    Term::Text(s) => {
                        if let Some(r) = find_line(doc.content, s, None) {
                            return Some(r);
                        }
                    }
                    Term::Regex(re) => {
                        if let Some(r) = find_line(doc.content, "", Some(re)) {
                            return Some(r);
                        }
                    }
                    Term::Property(k, _v) => {
                        if let Some((_, val)) = doc
                            .props
                            .iter()
                            .find(|(key, _)| key.eq_ignore_ascii_case(k))
                        {
                            return Some((0, format!("{k}: {val}")));
                        }
                    }
                    Term::Section(name) => {
                        if let Some(r) = section_line(doc.content, name) {
                            return Some(r);
                        }
                    }
                    Term::Task(todo) => {
                        for (i, line) in doc.content.lines().enumerate() {
                            let t = line.trim_start();
                            let unchecked = t.starts_with("- [ ]") || t.starts_with("* [ ]");
                            let checked = t.starts_with("- [x]") || t.starts_with("* [x]");
                            if (*todo && unchecked) || (!*todo && checked) {
                                return Some((i + 1, line.trim().chars().take(100).collect()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }
}

fn term_matches(term: &Term, doc: &Doc) -> bool {
    match term {
        Term::Text(s) => {
            let s = s.to_lowercase();
            doc.content.to_lowercase().contains(&s) || doc.title.to_lowercase().contains(&s)
        }
        Term::Tag(s) => {
            let s = s.to_lowercase();
            doc.tags.iter().any(|tag| {
                let tl = tag.to_lowercase();
                tl == s || tl.starts_with(&format!("{s}/"))
            })
        }
        Term::Path(s) => doc.path.to_lowercase().contains(&s.to_lowercase()),
        Term::File(s) => doc.title.to_lowercase().contains(&s.to_lowercase()),
        Term::Property(k, v) => doc.props.iter().any(|(key, val)| {
            if !key.eq_ignore_ascii_case(k) {
                return false;
            }
            match v {
                None => true,
                Some(want) => {
                    let want = want.to_lowercase();
                    let val = val.to_lowercase();
                    val == want || val.contains(&want)
                }
            }
        }),
        Term::Task(todo) => doc.content.lines().any(|line| {
            let t = line.trim_start();
            let unchecked = t.starts_with("- [ ]") || t.starts_with("* [ ]");
            let checked = t.starts_with("- [x]") || t.starts_with("* [x]");
            if *todo {
                unchecked
            } else {
                checked
            }
        }),
        Term::Section(name) => section_line(doc.content, name).is_some(),
        Term::Line(spec) => line_matches(doc.content.lines().count(), spec),
        Term::Regex(re) => re.is_match(doc.content) || re.is_match(doc.title),
    }
}

/// Find a heading containing `name`; return the first non-empty line under it.
fn section_line(content: &str, name: &str) -> Option<(usize, String)> {
    let needle = name.trim_matches('"').to_lowercase();
    let lines: Vec<&str> = content.lines().collect();
    let mut found: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if t.starts_with('#') {
            let text = t.trim_start_matches('#').trim().to_lowercase();
            if found.is_some() {
                break;
            }
            if text.contains(&needle) {
                found = Some(i);
            }
        }
    }
    let start = found?;
    for (i, line) in lines.iter().enumerate().skip(start + 1) {
        let t = line.trim();
        if t.starts_with('#') {
            break;
        }
        if !t.is_empty() {
            return Some((i + 1, t.chars().take(100).collect()));
        }
    }
    Some((start + 1, lines[start].trim().to_string()))
}

fn line_matches(count: usize, spec: &str) -> bool {
    let spec = spec.trim();
    let parse = |s: &str| s.trim().parse::<usize>().ok();
    if let Some(rest) = spec.strip_prefix('>') {
        return parse(rest).is_some_and(|n| count > n);
    }
    if let Some(rest) = spec.strip_prefix('<') {
        return parse(rest).is_some_and(|n| count < n);
    }
    if let Some((a, b)) = spec.split_once('-') {
        if let (Some(a), Some(b)) = (parse(a), parse(b)) {
            return count >= a && count <= b;
        }
    }
    parse(spec) == Some(count)
}

fn find_line(content: &str, needle: &str, re: Option<&Regex>) -> Option<(usize, String)> {
    let needle = needle.to_lowercase();
    for (i, line) in content.lines().enumerate() {
        let hit = match re {
            Some(r) => r.is_match(line),
            None => line.to_lowercase().contains(&needle),
        };
        if hit {
            let snippet: String = line.trim().chars().take(100).collect();
            return Some((i + 1, snippet));
        }
    }
    None
}

/// Split a query into tokens, keeping `"quoted phrases"` and `/regex/` intact.
fn tokenize(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != '"' {
                s.push(chars[i]);
                i += 1;
            }
            i += 1; // closing quote (or end)
            if !s.is_empty() {
                out.push(s);
            }
            continue;
        }
        if c == '/' {
            let mut j = i + 1;
            let mut s = String::from("/");
            while j < chars.len() && chars[j] != '/' {
                s.push(chars[j]);
                j += 1;
            }
            if j < chars.len() {
                s.push('/');
                j += 1;
                out.push(s);
                i = j;
                continue;
            }
            // No closing slash: fall through and treat as a normal token.
        }
        let mut s = String::new();
        while i < chars.len() && !chars[i].is_whitespace() {
            s.push(chars[i]);
            i += 1;
        }
        if !s.is_empty() {
            out.push(s);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc<'a>(title: &'a str, path: &'a str, tags: &'a [String], content: &'a str) -> Doc<'a> {
        Doc {
            title,
            path,
            tags,
            content,
            props: &[],
        }
    }

    #[test]
    fn tokenizes_phrases_and_regex() {
        assert_eq!(
            tokenize(r#"foo "bar baz" /a.c/ qux"#),
            vec!["foo", "bar baz", "/a.c/", "qux"]
        );
    }

    #[test]
    fn plain_terms_are_anded() {
        let re = vec!["work".to_string()];
        let q = Query::parse("alpha beta");
        assert!(q.matches(&doc("Alpha", "/v/a.md", &re, "alpha and beta")));
        assert!(!q.matches(&doc("Alpha", "/v/a.md", &re, "only alpha")));
    }

    #[test]
    fn or_groups_are_alternatives() {
        let tags: Vec<String> = vec![];
        let d = doc("A", "/v/a.md", &tags, "hello world");
        assert!(Query::parse("hello OR goodbye").matches(&d));
        assert!(Query::parse("goodbye OR world").matches(&d));
        assert!(!Query::parse("goodbye OR farewell").matches(&d));
        assert!(Query::parse("hello OR goodbye -world").matches(&d));
    }

    #[test]
    fn tag_path_file_operators() {
        let tags = vec!["work/rust".to_string()];
        let d = doc("Rust Notes", "/v/notes/rust.md", &tags, "x");
        assert!(Query::parse("tag:work").matches(&d)); // nested tag prefix
        assert!(Query::parse("tag:work/rust").matches(&d));
        assert!(Query::parse("path:notes").matches(&d));
        assert!(Query::parse("file:rust").matches(&d));
        assert!(!Query::parse("tag:home").matches(&d));
    }

    #[test]
    fn property_task_section_and_line() {
        let tags: Vec<String> = vec![];
        let props = vec![
            ("status".to_string(), "done".to_string()),
            ("rating".to_string(), "5".to_string()),
        ];
        let content = "# Intro\nfirst body\n## Tasks\n- [ ] todo one\n- [x] done one\n";
        let d = Doc {
            title: "A",
            path: "/v/a.md",
            tags: &tags,
            content,
            props: &props,
        };
        assert!(Query::parse("property:status=done").matches(&d));
        assert!(!Query::parse("property:status=open").matches(&d));
        assert!(Query::parse("prop:rating").matches(&d));
        assert!(Query::parse("task:todo").matches(&d));
        assert!(Query::parse("task:done").matches(&d));
        assert!(Query::parse("section:tasks").matches(&d));
        assert!(Query::parse("line:>3").matches(&d));
        assert!(!Query::parse("line:<3").matches(&d));
        assert!(Query::parse("line:5").matches(&d));
    }

    #[test]
    fn negation() {
        let tags: Vec<String> = vec![];
        let d = doc("A", "/v/a.md", &tags, "hello world");
        assert!(Query::parse("hello -goodbye").matches(&d));
        assert!(!Query::parse("hello -world").matches(&d));
    }

    #[test]
    fn regex_terms() {
        let tags: Vec<String> = vec![];
        let d = doc("A", "/v/a.md", &tags, "color and colour");
        assert!(Query::parse("/colou?r/").matches(&d));
        assert!(!Query::parse("/xyz/").matches(&d));
    }

    #[test]
    fn invalid_regex_falls_back_to_text() {
        let tags: Vec<String> = vec![];
        let d = doc("A", "/v/a.md", &tags, "a(b text");
        // `/a(b/` is not a valid regex, so it becomes a literal search.
        assert!(Query::parse("/a(b/").matches(&d));
    }

    #[test]
    fn snippet_finds_the_matching_line() {
        let tags: Vec<String> = vec![];
        let d = doc("A", "/v/a.md", &tags, "line one\nfind me here\nline three");
        let (n, line) = Query::parse("find").snippet(&d).unwrap();
        assert_eq!(n, 2);
        assert!(line.contains("find me here"));
    }
}
