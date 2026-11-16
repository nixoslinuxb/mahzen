//! Minimal YAML frontmatter parsing and note metadata (title, aliases, tags).
//!
//! Only the handful of frontmatter keys Obsidian users actually rely on are
//! supported: `title`, `aliases`, and `tags`. Inline `#tags` in the body are
//! collected too. This is deliberately a small, dependency-free subset of YAML.

/// Parsed frontmatter fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
}

/// Everything we extract from a note's text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Meta {
    pub frontmatter: Frontmatter,
    /// The note body with any frontmatter block removed.
    pub body: String,
    /// Title resolved from frontmatter (falls back to the caller's file stem).
    pub title: Option<String>,
    /// Aliases, from frontmatter.
    pub aliases: Vec<String>,
    /// All tags: frontmatter tags plus inline `#tags`, deduplicated, no `#`.
    pub tags: Vec<String>,
    /// Every frontmatter property, in document order (for search / properties).
    pub props: Vec<(String, PropValue)>,
}

impl Meta {
    #[cfg(test)]
    pub fn has_frontmatter(&self) -> bool {
        self.frontmatter != Frontmatter::default()
    }
}

/// A generic frontmatter value, as edited in the Properties panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropValue {
    Text(String),
    List(Vec<String>),
}

impl PropValue {
    /// A human-readable, editable representation (lists become comma-separated).
    pub fn display(&self) -> String {
        match self {
            PropValue::Text(s) => s.clone(),
            PropValue::List(items) => items.join(", "),
        }
    }

    /// The unquoted, trimmed list items (empty for scalar values).
    #[allow(dead_code)]
    pub fn items(&self) -> Vec<String> {
        match self {
            PropValue::List(items) => items.clone(),
            PropValue::Text(_) => Vec::new(),
        }
    }
}

/// Parse a note's full text into [`Meta`].
pub fn parse(text: &str) -> Meta {
    let mut fm = Frontmatter::default();
    let body = split_frontmatter(text, &mut fm);
    let mut tags: Vec<String> = fm.tags.clone();
    for tag in inline_tags(body) {
        if !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
            tags.push(tag);
        }
    }
    Meta {
        title: fm.title.clone(),
        aliases: fm.aliases.clone(),
        tags,
        body: body.to_string(),
        props: properties(text),
        frontmatter: fm,
    }
}

/// Remove a leading `---` frontmatter block, parsing it into `fm`, and return
/// the remaining body.
fn split_frontmatter<'a>(text: &'a str, fm: &mut Frontmatter) -> &'a str {
    match frontmatter_block(text) {
        Some((yaml, body)) => {
            parse_yaml(yaml, fm);
            body
        }
        None => text,
    }
}

/// Split a leading `---` block into `(yaml, body)`. Returns `None` when there is
/// no well-formed frontmatter block.
fn frontmatter_block(text: &str) -> Option<(&str, &str)> {
    let rest = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    // Find a closing fence line ("---" possibly with trailing spaces).
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let yaml = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return Some((yaml, body));
        }
        offset += line.len();
    }
    None
}

/// Parse every scalar / list property in the frontmatter, in document order.
pub fn properties(text: &str) -> Vec<(String, PropValue)> {
    let Some((yaml, _)) = frontmatter_block(text) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PropValue)> = Vec::new();
    let lines: Vec<&str> = yaml.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            i += 1;
            continue;
        };
        let key = key.trim().to_string();
        if key.is_empty() {
            i += 1;
            continue;
        }
        let value = value.trim();
        let mut val = PropValue::Text(unquote(value));
        if value.is_empty() {
            // Block list on following indented `- ` lines.
            let mut items: Vec<String> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let l = lines[j];
                if l.trim().is_empty() {
                    j += 1;
                    continue;
                }
                let lt = l.trim_start();
                if let Some(item) = lt.strip_prefix("- ") {
                    if l.len() == lt.len() {
                        break;
                    }
                    items.push(unquote(item.trim()));
                    j += 1;
                } else {
                    break;
                }
            }
            if !items.is_empty() {
                val = PropValue::List(items);
                i = j;
            } else {
                i += 1;
            }
        } else {
            if value.starts_with('[') {
                val = PropValue::List(parse_list(value));
            }
            i += 1;
        }
        out.push((key, val));
    }
    out
}

/// Quote a scalar when YAML would otherwise misparse it.
fn yaml_scalar(s: &str) -> String {
    let needs = s.is_empty()
        || s.contains(':')
        || s.contains('#')
        || s.contains('"')
        || s.starts_with(' ')
        || s.ends_with(' ')
        || s.starts_with('[')
        || s.starts_with('{');
    if needs {
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

fn render_property(key: &str, value: &PropValue) -> String {
    match value {
        PropValue::Text(s) => format!("{key}: {}\n", yaml_scalar(s)),
        PropValue::List(items) => {
            if items.is_empty() {
                format!("{key}: []\n")
            } else {
                let body = items
                    .iter()
                    .map(|i| yaml_scalar(i))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{key}: [{body}]\n")
            }
        }
    }
}

/// Replace the note's frontmatter with `props` (re-rendering it), keeping the
/// body unchanged. Passing an empty list removes the block entirely.
pub fn replace_properties(text: &str, props: &[(String, PropValue)]) -> String {
    let body = frontmatter_block(text)
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|| text.to_string());
    if props.is_empty() {
        return body;
    }
    let mut out = String::from("---\n");
    for (k, v) in props {
        out.push_str(&render_property(k, v));
    }
    out.push_str("---\n");
    out.push_str(&body);
    out
}

/// Set (or add) a single frontmatter property, preserving the other ones.
pub fn set_property(text: &str, key: &str, value: PropValue) -> String {
    let mut props = properties(text);
    match props.iter_mut().find(|(k, _)| k.eq_ignore_ascii_case(key)) {
        Some((_, v)) => *v = value,
        None => props.push((key.to_string(), value)),
    }
    replace_properties(text, &props)
}

/// Remove a frontmatter property (case-insensitive key match).
pub fn remove_property(text: &str, key: &str) -> String {
    let mut props = properties(text);
    props.retain(|(k, _)| !k.eq_ignore_ascii_case(key));
    replace_properties(text, &props)
}

/// Whether `tag` matches `from` exactly or as a `/`-separated subtag.
fn tag_matches(tag: &str, from: &str) -> bool {
    let t = tag.to_ascii_lowercase();
    let f = from.to_ascii_lowercase();
    t == f || t.starts_with(&format!("{f}/"))
}

fn rename_tag_value(tag: &str, from: &str, to: Option<&str>) -> Option<String> {
    if !tag_matches(tag, from) {
        return Some(tag.to_string());
    }
    let to = to?;
    // Keep the subtag suffix when renaming a parent (`work` → `job`).
    let lower_tag = tag.to_ascii_lowercase();
    let lower_from = from.to_ascii_lowercase();
    if lower_tag.len() > lower_from.len() {
        let suffix = &tag[lower_from.len()..];
        Some(format!("{to}{suffix}"))
    } else {
        Some(to.to_string())
    }
}

/// Rename (or, with `to = None`, delete) a tag everywhere in a note: both the
/// frontmatter `tags` list and inline `#tags` in the body. Subtags are renamed
/// too (`work` → `job` turns `work/rust` into `job/rust`).
pub fn rename_tag(text: &str, from: &str, to: Option<&str>) -> String {
    let mut props = properties(text);
    for (k, v) in props.iter_mut() {
        if !(k.eq_ignore_ascii_case("tags") || k.eq_ignore_ascii_case("tag")) {
            continue;
        }
        let items = match v {
            PropValue::List(items) => std::mem::take(items),
            PropValue::Text(s) => parse_list(s),
        };
        let renamed: Vec<String> = items
            .iter()
            .filter_map(|t| rename_tag_value(t, from, to))
            .collect();
        *v = PropValue::List(renamed);
    }
    let body = frontmatter_block(text)
        .map(|(_, b)| b.to_string())
        .unwrap_or_else(|| text.to_string());
    let body = rename_inline_tags(&body, from, to);
    replace_properties(&body, &props)
}

fn rename_inline_tags(body: &str, from: &str, to: Option<&str>) -> String {
    let mut out = String::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let prev_ok = i == 0
                || matches!(
                    bytes[i - 1],
                    b' ' | b'\t' | b'\n' | b'(' | b'[' | b'{' | b'>' | b','
                );
            if prev_ok {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() {
                    let c = bytes[j];
                    if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'/' {
                        j += 1;
                    } else {
                        break;
                    }
                }
                if j > start {
                    let tag = &body[start..j];
                    match rename_tag_value(tag, from, to) {
                        Some(new_tag) => {
                            out.push('#');
                            out.push_str(&new_tag);
                        }
                        None => {
                            // Tag removed: drop the `#tag` token.
                        }
                    }
                    i = j;
                    continue;
                }
            }
        }
        let ch = body[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn parse_yaml(yaml: &str, fm: &mut Frontmatter) {
    let lines: Vec<&str> = yaml.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            i += 1;
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        let mut items: Vec<String> = Vec::new();
        if value.is_empty() {
            // Possibly a block list on the following indented `- ` lines.
            let mut j = i + 1;
            while j < lines.len() {
                let l = lines[j];
                if l.trim().is_empty() {
                    j += 1;
                    continue;
                }
                let lt = l.trim_start();
                if let Some(item) = lt.strip_prefix("- ") {
                    if l.len() == lt.len() {
                        // Not indented: belongs to the next key.
                        break;
                    }
                    items.push(unquote(item.trim()));
                    j += 1;
                } else {
                    break;
                }
            }
            i = j;
        } else {
            items = parse_list(value);
            i += 1;
        }
        match key.as_str() {
            "title" => {
                fm.title = if value.is_empty() {
                    items.into_iter().next()
                } else {
                    Some(unquote(value))
                };
            }
            "aliases" | "alias" => fm.aliases = items,
            "tags" | "tag" => fm.tags = items,
            _ => {}
        }
    }
}

/// Parse an inline YAML list: `[a, b]` or `a, b` or `a b`.
fn parse_list(value: &str) -> Vec<String> {
    let v = value.trim();
    if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        // Comma-separated; keep spaces inside quoted entries.
        inner
            .split(',')
            .map(|s| unquote(s.trim()))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        v.split([',', ' '])
            .map(|s| unquote(s.trim()))
            .filter(|s| !s.is_empty())
            .collect()
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .unwrap_or(s);
    let s = s
        .strip_prefix('\'')
        .and_then(|x| x.strip_suffix('\''))
        .unwrap_or(s);
    s.to_string()
}

/// Collect inline `#tags` from a note body, ignoring headings and code fences.
pub fn inline_tags(body: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    let mut in_code = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        // A heading is `#` at line start followed by space.
        if trimmed.starts_with("# ") || trimmed == "#" {
            continue;
        }
        for tag in tags_in_line(line) {
            if !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag)) {
                tags.push(tag);
            }
        }
    }
    tags
}

fn tags_in_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' {
            let prev_ok = i == 0
                || matches!(
                    bytes[i - 1],
                    b' ' | b'\t' | b'(' | b'[' | b'{' | b'>' | b','
                );
            if prev_ok {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() {
                    let c = bytes[j];
                    if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'/' {
                        j += 1;
                    } else {
                        break;
                    }
                }
                if j > start {
                    out.push(line[start..j].to_string());
                    i = j;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_and_block_lists() {
        let text = "---\ntitle: My Note\naliases: [mn, \"my note\"]\ntags:\n  - project\n  - rust\n---\n# Body\n";
        let meta = parse(text);
        assert_eq!(meta.title.as_deref(), Some("My Note"));
        assert_eq!(meta.aliases, vec!["mn", "my note"]);
        assert_eq!(meta.tags, vec!["project", "rust"]);
        assert!(meta.body.starts_with("# Body"));
    }

    #[test]
    fn collects_inline_tags_and_skips_headings_and_code() {
        let text = "# Heading not a tag\n\nsome #work and #rust/async here\n\n```\n#notatag\n```\n";
        let meta = parse(text);
        assert_eq!(meta.tags, vec!["work", "rust/async"]);
    }

    #[test]
    fn merges_frontmatter_and_inline_tags() {
        let text = "---\ntags: [a]\n---\nbody #b\n";
        let meta = parse(text);
        assert_eq!(meta.tags, vec!["a", "b"]);
    }

    #[test]
    fn no_frontmatter_is_all_body() {
        let meta = parse("just text #tag\n");
        assert!(!meta.has_frontmatter());
        assert_eq!(meta.body, "just text #tag\n");
        assert_eq!(meta.tags, vec!["tag"]);
    }

    #[test]
    fn unterminated_frontmatter_is_body() {
        let meta = parse("---\ntitle: x\n");
        assert!(!meta.has_frontmatter());
    }

    #[test]
    fn url_fragments_are_not_tags() {
        let meta = parse("see https://example.com/page#section for details\n");
        assert!(meta.tags.is_empty());
    }

    #[test]
    fn generic_properties_roundtrip() {
        let text = "---\ntitle: Hello: world\naliases: [a, b]\nrating: 5\nlist:\n  - x\n  - y\n---\nbody #inline\n";
        let props = properties(text);
        assert_eq!(props[0].0, "title");
        assert_eq!(props[0].1, PropValue::Text("Hello: world".into()));
        assert_eq!(props[1].1, PropValue::List(vec!["a".into(), "b".into()]));
        assert_eq!(props[2].1, PropValue::Text("5".into()));
        assert_eq!(props[3].1, PropValue::List(vec!["x".into(), "y".into()]));
        let out = set_property(text, "rating", PropValue::Text("4".into()));
        let again = properties(&out);
        assert_eq!(again[2].1, PropValue::Text("4".into()));
        assert!(out.ends_with("body #inline\n"));
        let removed = remove_property(&out, "list");
        assert!(!removed.contains("list:"));
        assert!(removed.contains("title:"));
    }

    #[test]
    fn renames_tags_in_frontmatter_and_body() {
        let text = "---\ntags: [work, work/rust, other]\n---\n# heading\nbody #work and #work/rust and #workshop\n";
        let out = rename_tag(text, "work", Some("job"));
        let meta = parse(&out);
        assert_eq!(meta.tags, vec!["job", "job/rust", "other", "workshop"]);
        assert!(out.contains("#job and #job/rust and #workshop"));
        let deleted = rename_tag(text, "work", None);
        let meta = parse(&deleted);
        assert_eq!(meta.tags, vec!["other", "workshop"]);
        assert!(!deleted.contains("#work "));
        assert!(!deleted.contains("#work/"));
        assert!(deleted.contains("#workshop"));
    }
}
