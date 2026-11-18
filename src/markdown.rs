//! A small, dependency-free markdown renderer for `egui`.
//!
//! Supported: headings, bold/italic/strikethrough/inline-code, bullet and
//! numbered lists, task-list checkboxes, blockquotes, callouts (`> [!note]`),
//! fenced code blocks, GitHub-style tables, footnotes, horizontal rules,
//! markdown links `[text](url)`, Obsidian `[[wikilinks]]` (incl. `[[t|alias]]`),
//! and `![[embeds]]`.

use eframe::egui::{self, Color32, RichText, Sense, Vec2};

thread_local! {
    /// Equation numbers and `\label` targets for the note being rendered.
    static EQ_COUNTER: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static EQ_LABELS: std::cell::RefCell<std::collections::HashMap<String, usize>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Reset per-note equation state (called at the top level of `render`).
fn reset_equations() {
    EQ_COUNTER.with(|c| c.set(0));
    EQ_LABELS.with(|m| m.borrow_mut().clear());
}

fn next_equation() -> usize {
    EQ_COUNTER.with(|c| {
        let n = c.get() + 1;
        c.set(n);
        n
    })
}

/// Pull `\label{name}` out of a math block and record it under `number`.
fn take_label(tex: &mut String, number: usize) {
    while let Some(start) = tex.find("\\label{") {
        if let Some(end) = tex[start..].find('}') {
            let name = tex[start + 7..start + end].to_string();
            EQ_LABELS.with(|m| m.borrow_mut().insert(name, number));
            tex.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
}

/// Replace `\ref{name}` with the recorded equation number.
fn expand_refs(tex: &mut String) {
    // cleveref-style commands first (longest names first so `\\crefrange`
    // is not mistaken for `\\cref`).
    expand_multi_ref(tex, "\\crefrange", RefStyle::Range);
    expand_multi_ref(tex, "\\Crefrange", RefStyle::Range);
    expand_multi_ref(tex, "\\cpageref", RefStyle::Plain);
    expand_multi_ref(tex, "\\Cref", RefStyle::Named);
    expand_multi_ref(tex, "\\cref", RefStyle::Plain);
    expand_multi_ref(tex, "\\labelcref", RefStyle::Plain);
    for cmd in ["\\eqref", "\\autoref", "\\ref"] {
        let paren = cmd != "\\ref";
        let open = format!("{cmd}{{");
        while let Some(start) = tex.find(&open) {
            let Some(end) = tex[start..].find('}') else {
                break;
            };
            let name = tex[start + open.len()..start + end].to_string();
            let n = EQ_LABELS.with(|m| m.borrow().get(&name).copied());
            let num = n.map(|n| n.to_string()).unwrap_or_else(|| "??".into());
            let repl = if paren { format!("({num})") } else { num };
            tex.replace_range(start..start + end + 1, &repl);
        }
    }
}

#[derive(Clone, Copy)]
enum RefStyle {
    /// `(1)`, `(1, 2)`.
    Plain,
    /// `Eq. (1)`, `Eq. (1, 2)`.
    Named,
    /// `(1)–(3)` for `\\crefrange{a}{b}`.
    Range,
}

fn label_number(name: &str) -> String {
    EQ_LABELS.with(|m| {
        m.borrow()
            .get(name.trim())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "??".into())
    })
}

/// Expand `\\cref{a,b}`-style commands. `Range` takes two braced arguments.
fn expand_multi_ref(tex: &mut String, cmd: &str, style: RefStyle) {
    let open = format!("{cmd}{{");
    while let Some(start) = tex.find(&open) {
        let Some(end) = tex[start..].find('}') else {
            break;
        };
        let body = tex[start + open.len()..start + end].to_string();
        let mut replaced_end = start + end + 1;
        // `\\crefrange{a}{b}` reads a second brace group.
        let second = if matches!(style, RefStyle::Range) {
            let rest = &tex[replaced_end..];
            if let Some(open2) = rest.strip_prefix('{') {
                if let Some(close2) = open2.find('}') {
                    let value = open2[..close2].to_string();
                    replaced_end += close2 + 2;
                    Some(value)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let repl = match style {
            RefStyle::Range => {
                let first = label_number(body.split(',').next().unwrap_or(""));
                let last = second.as_deref().map(label_number).unwrap_or_default();
                if last.is_empty() {
                    format!("({first})")
                } else {
                    format!("({first})–({last})")
                }
            }
            RefStyle::Plain | RefStyle::Named => {
                let nums: Vec<String> = body.split(',').map(label_number).collect();
                let list = nums.join(", ");
                match style {
                    RefStyle::Named => format!("{} ({list})", tr_eq_ref()),
                    _ => format!("({list})"),
                }
            }
        };
        tex.replace_range(start..replaced_end, &repl);
    }
}

/// The word used by `\\Cref` (localised).
fn tr_eq_ref() -> &'static str {
    match crate::i18n::current() {
        crate::i18n::Lang::Tr => "Eş.",
        _ => "Eq.",
    }
}

/// Extract an explicit `\tag{...}` (returns it and removes it from `tex`).
fn take_tag(tex: &mut String) -> Option<String> {
    let start = tex.find("\\tag{")?;
    let end = tex[start..].find('}')?;
    let tag = tex[start + 5..start + end].to_string();
    tex.replace_range(start..start + end + 1, "");
    Some(tag)
}

/// A clickable thing the preview produced this frame.
pub enum LinkClick {
    /// A `[[wikilink]]` target (to be resolved to a note).
    Wiki(String),
    /// An external `[text](url)` link.
    External(String),
}

/// One styled run within a line of markdown.
enum Inline {
    Text {
        text: String,
        bold: bool,
        italic: bool,
        strike: bool,
        code: bool,
    },
    Wiki {
        target: String,
        alias: String,
    },
    External {
        text: String,
        url: String,
    },
    Footnote(String),
    Math(String),
    Highlight(String),
}

/// Extract all wikilink targets appearing in a block of markdown.
pub fn wikilinks(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        match after.find("]]") {
            Some(end) => {
                let inner = &after[..end];
                let target = inner.split('|').next().unwrap_or(inner).trim();
                if !target.is_empty() {
                    out.push(target.to_string());
                }
                rest = &after[end + 2..];
            }
            None => break,
        }
    }
    out
}

/// Collect document headings as `(level, text)` pairs.
pub fn headings(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) {
            let rest = &trimmed[hashes..];
            if rest.is_empty() || rest.starts_with(' ') {
                out.push((hashes, rest.trim().to_string()));
            }
        }
    }
    out
}

/// Parse a single line into styled inline runs.
fn parse_inline(line: &str) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    let mut buf = String::new();
    let mut bold = false;
    let mut italic = false;
    let mut strike = false;
    let mut code = false;

    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                out.push(Inline::Text {
                    text: std::mem::take(&mut buf),
                    bold,
                    italic,
                    strike,
                    code,
                });
            }
        };
    }

    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Obsidian comment: %% ... %% (drop it entirely).
        if c == '%' && i + 1 < chars.len() && chars[i + 1] == '%' {
            flush!();
            let mut j = i + 2;
            let mut end = chars.len();
            while j + 1 < chars.len() {
                if chars[j] == '%' && chars[j + 1] == '%' {
                    end = j + 2;
                    break;
                }
                j += 1;
            }
            i = end;
            continue;
        }
        // Highlight: ==text==
        if c == '=' && i + 1 < chars.len() && chars[i + 1] == '=' {
            if let Some(close) = chars[i + 2..].iter().position(|&x| x == '=') {
                let inner: String = chars[i + 2..i + 2 + close].iter().collect();
                flush!();
                out.push(Inline::Highlight(inner));
                i = i + 2 + close + 1;
                continue;
            }
        }
        // Inline HTML: drop tags (keep <br> as a line break).
        if c == '<' {
            if let Some(close) = chars[i..].iter().position(|&x| x == '>') {
                let tag: String = chars[i + 1..i + close].iter().collect();
                let lname = tag.trim_start_matches('/').to_ascii_lowercase();
                flush!();
                if lname == "br" || lname == "br/" {
                    out.push(Inline::Text {
                        text: "\n".into(),
                        bold,
                        italic,
                        strike,
                        code,
                    });
                }
                i = i + close + 1;
                continue;
            }
        }
        // Wikilink: [[target]] or [[target|alias]]
        if c == '[' && i + 1 < chars.len() && chars[i + 1] == '[' {
            if let Some(close) = find_close(&chars, i) {
                let inner: String = chars[i + 2..close].iter().collect();
                let mut split = inner.splitn(2, '|');
                let target = split.next().unwrap_or("").trim().to_string();
                let alias = split
                    .next()
                    .map(|a| a.trim().to_string())
                    .unwrap_or_else(|| target.clone());
                if !target.is_empty() {
                    flush!();
                    out.push(Inline::Wiki { target, alias });
                    i = close + 2;
                    continue;
                }
            }
        }
        // Footnote reference: [^id]
        if c == '[' && i + 1 < chars.len() && chars[i + 1] == '^' {
            if let Some(close) = chars[i + 2..].iter().position(|&x| x == ']') {
                let id: String = chars[i + 2..i + 2 + close].iter().collect();
                flush!();
                out.push(Inline::Footnote(id));
                i = i + 2 + close + 1;
                continue;
            }
        }
        // Markdown link: [text](url)
        if c == '[' {
            if let Some((text, url, next)) = parse_md_link(&chars, i) {
                flush!();
                out.push(Inline::External { text, url });
                i = next;
                continue;
            }
        }
        // Inline math: $...$
        if c == '$' {
            if let Some(close) = chars[i + 1..].iter().position(|&x| x == '$') {
                let inner: String = chars[i + 1..i + 1 + close].iter().collect();
                if !inner.is_empty() && !inner.starts_with(' ') {
                    flush!();
                    out.push(Inline::Math(inner));
                    i = i + 1 + close + 1;
                    continue;
                }
            }
        }
        match c {
            '`' => {
                flush!();
                code = !code;
                i += 1;
            }
            '~' if i + 1 < chars.len() && chars[i + 1] == '~' => {
                flush!();
                strike = !strike;
                i += 2;
            }
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    flush!();
                    bold = !bold;
                    i += 2;
                } else {
                    flush!();
                    italic = !italic;
                    i += 1;
                }
            }
            '_' => {
                let prev_word = i > 0 && chars[i - 1].is_alphanumeric();
                let next_word = i + 1 < chars.len() && chars[i + 1].is_alphanumeric();
                if !(prev_word && next_word) {
                    flush!();
                    italic = !italic;
                    i += 1;
                } else {
                    buf.push(c);
                    i += 1;
                }
            }
            _ => {
                buf.push(c);
                i += 1;
            }
        }
    }
    flush!();
    out
}

/// Try to parse `[text](url)` starting at `start` (which must point at `[`).
fn parse_md_link(chars: &[char], start: usize) -> Option<(String, String, usize)> {
    let close = chars[start..].iter().position(|&c| c == ']')? + start;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars[close + 1..].iter().position(|&c| c == ')')? + close + 1;
    let text: String = chars[start + 1..close].iter().collect();
    let url: String = chars[close + 2..end].iter().collect();
    if url.contains(' ') || url.is_empty() {
        // Not a real link (e.g. `[a] (b)` with a space).
        return None;
    }
    Some((text, url, end + 1))
}

fn find_close(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start + 2;
    while i + 1 < chars.len() {
        if chars[i] == ']' && chars[i + 1] == ']' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Render markdown into `ui`. `embed` resolves `![[target]]` to
/// `(title, body)`; `depth` guards against recursive embeds.
pub fn render(
    ui: &mut egui::Ui,
    text: &str,
    base_size: f32,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
    fence: &dyn Fn(&mut egui::Ui, &str, &str) -> bool,
    depth: usize,
) -> Vec<LinkClick> {
    if depth == 0 {
        reset_equations();
    }
    let mut clicked: Vec<LinkClick> = Vec::new();
    let lines: Vec<&str> = text.split('\n').collect();
    let mut i = 0;

    while i < lines.len() {
        let raw = lines[i];
        let line = raw.trim_end();
        let trimmed = line.trim_start();

        // Fenced code block (or a special block handled by `fence`).
        if trimmed.starts_with("```") {
            let lang = trimmed.trim_start_matches('`').trim().to_string();
            let mut block = String::new();
            i += 1;
            while i < lines.len() && !lines[i].trim_start().starts_with("```") {
                block.push_str(lines[i]);
                block.push('\n');
                i += 1;
            }
            i += 1; // consume closing fence
            if !fence(ui, &lang, &block) {
                code_block(ui, &block, base_size);
            }
            continue;
        }

        // LaTeX display environments: \begin{equation} … \end{equation}
        if trimmed.starts_with("\\begin{") {
            let env_name: String = trimmed
                .trim_start_matches("\\begin{")
                .split('}')
                .next()
                .unwrap_or("")
                .to_string();
            let numbered_env = matches!(
                env_name.as_str(),
                "equation"
                    | "equation*"
                    | "align"
                    | "align*"
                    | "gather"
                    | "gather*"
                    | "multline"
                    | "multline*"
                    | "eqnarray"
                    | "eqnarray*"
            );
            if numbered_env {
                let unnumbered = env_name.ends_with('*');
                let mut tex = String::new();
                let rest = trimmed.split_once('}').map(|(_, r)| r.trim()).unwrap_or("");
                if !rest.is_empty() {
                    tex.push_str(rest);
                }
                i += 1;
                let end_marker = format!("\\end{{{env_name}}}");
                while i < lines.len() && !lines[i].contains(&end_marker) {
                    tex.push('\n');
                    tex.push_str(lines[i]);
                    i += 1;
                }
                if i < lines.len() {
                    let before = lines[i].split(&end_marker).next().unwrap_or("");
                    if !before.trim().is_empty() {
                        tex.push('\n');
                        tex.push_str(before.trim());
                    }
                    i += 1;
                }
                let tag = take_tag(&mut tex);
                let number = if let Some(t) = tag {
                    Some(t)
                } else if unnumbered {
                    None
                } else {
                    let n = next_equation();
                    take_label(&mut tex, n);
                    Some(n.to_string())
                };
                expand_refs(&mut tex);
                display_math_tagged(ui, &tex, base_size, number.as_deref());
                continue;
            }
        }
        if trimmed.starts_with("\\[") {
            let mut tex = trimmed.trim_start_matches("\\[").to_string();
            i += 1;
            while i < lines.len() && !lines[i].contains("\\]") {
                tex.push('\n');
                tex.push_str(lines[i]);
                i += 1;
            }
            if i < lines.len() {
                let before = lines[i].split("\\]").next().unwrap_or("");
                tex.push_str(before);
                i += 1;
            }
            expand_refs(&mut tex);
            display_math_tagged(ui, &tex, base_size, None);
            continue;
        }

        // Display math: $$ ... $$, either on one line or fenced by $$ lines.
        if trimmed.starts_with("$$") {
            let mut tex = String::new();
            if trimmed.len() > 4 && trimmed.ends_with("$$") {
                tex = trimmed[2..trimmed.len() - 2].to_string();
                i += 1;
            } else {
                let first = trimmed.strip_prefix("$$").unwrap_or("").to_string();
                if !first.trim().is_empty() {
                    tex.push_str(&first);
                }
                i += 1;
                while i < lines.len() && !lines[i].trim().ends_with("$$") {
                    tex.push('\n');
                    tex.push_str(lines[i]);
                    i += 1;
                }
                if i < lines.len() {
                    let body = lines[i].trim().trim_end_matches("$$");
                    if !body.trim().is_empty() {
                        tex.push('\n');
                        tex.push_str(body);
                    }
                    i += 1;
                }
            }
            let tag = take_tag(&mut tex);
            let number = match tag {
                Some(t) => Some(t),
                None if tex.contains("\\label{") => {
                    let n = next_equation();
                    take_label(&mut tex, n);
                    Some(n.to_string())
                }
                None => None,
            };
            expand_refs(&mut tex);
            display_math_tagged(ui, &tex, base_size, number.as_deref());
            continue;
        }

        // Obsidian multi-line comment block: %% ... %%
        if trimmed.starts_with("%%") {
            let mut closed = trimmed.len() > 4 && trimmed.ends_with("%%");
            i += 1;
            while i < lines.len() && !closed {
                if lines[i].trim_end().ends_with("%%") {
                    closed = true;
                }
                i += 1;
            }
            continue;
        }

        // Embed: ![[target]] (media embeds are drawn by the attachment strip).
        if trimmed.starts_with("![[") {
            if let Some(end) = trimmed.find("]]") {
                let target = trimmed[3..end].trim();
                let path = target.split('|').next().unwrap_or(target).trim();
                if is_media_target(path) {
                    i += 1;
                    continue;
                }
                render_embed(ui, target, base_size, embed, fence, depth, &mut clicked);
                i += 1;
                continue;
            }
        }

        // Table: a header row followed by a separator row.
        if trimmed.starts_with('|') && i + 1 < lines.len() && is_table_sep(lines[i + 1]) {
            let mut rows: Vec<Vec<String>> = Vec::new();
            rows.push(parse_table_row(line));
            i += 2; // skip header + separator
            while i < lines.len() && lines[i].trim_start().starts_with('|') {
                rows.push(parse_table_row(lines[i]));
                i += 1;
            }
            render_table(ui, &rows, base_size, &mut clicked, i, embed);
            continue;
        }

        // Callout: > [!type] Title
        if trimmed.starts_with("> [!") {
            let header = &trimmed[2..];
            let title = header
                .split_once(']')
                .map(|(_, t)| t.trim().to_string())
                .unwrap_or_default();
            let kind = header
                .trim_start_matches('[')
                .trim_start_matches('!')
                .split(']')
                .next()
                .unwrap_or("note")
                .trim()
                .to_string();
            let mut body = String::new();
            i += 1;
            while i < lines.len() && lines[i].trim_start().starts_with('>') {
                // Strip exactly one quote level so nested callouts stay nested.
                let raw = lines[i].trim_start();
                let inner = raw.strip_prefix('>').unwrap_or(raw);
                body.push_str(inner.trim_start());
                body.push('\n');
                i += 1;
            }
            render_callout(
                ui,
                &kind,
                &title,
                &body,
                base_size,
                embed,
                fence,
                depth,
                &mut clicked,
            );
            continue;
        }

        if trimmed.is_empty() {
            ui.add_space(base_size * 0.5);
            i += 1;
            continue;
        }

        // Horizontal rule.
        if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            ui.separator();
            i += 1;
            continue;
        }

        // Footnote definition: [^id]: text
        if trimmed.starts_with("[^") {
            if let Some(rest) = trimmed.strip_prefix("[^") {
                if let Some(colon) = rest.find("]:") {
                    let id = &rest[..colon];
                    let body = rest[colon + 2..].trim();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!("[^{id}]")).small().weak());
                        render_segments(ui, body, base_size * 0.9, &mut clicked, embed);
                    });
                    i += 1;
                    continue;
                }
            }
        }

        // Headings.
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) {
            let rest = &line[hashes..];
            if rest.is_empty() || rest.starts_with(' ') {
                let size = match hashes {
                    1 => base_size * 1.9,
                    2 => base_size * 1.6,
                    3 => base_size * 1.35,
                    4 => base_size * 1.2,
                    5 => base_size * 1.1,
                    _ => base_size,
                };
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label(
                        RichText::new("▾")
                            .size(size * 0.62)
                            .color(crate::theme::TEXT_MUTED),
                    );
                    ui.label(
                        RichText::new(rest.trim())
                            .size(size)
                            .strong()
                            .color(crate::theme::heading_color(hashes)),
                    );
                });
                ui.add_space(3.0);
                i += 1;
                continue;
            }
        }

        // Blockquote.
        if let Some(rest) = trimmed.strip_prefix("> ") {
            egui::Frame::none()
                .fill(crate::theme::ACCENT.linear_multiply(0.06))
                .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("▏").color(crate::theme::ACCENT));
                        render_segments(ui, rest, base_size, &mut clicked, embed);
                    });
                });
            i += 1;
            continue;
        }

        // Task list checkbox.
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("- [ ] ") || lower.starts_with("- [x] ") {
            let checked = lower.starts_with("- [x] ");
            let rest = trimmed[6..].to_string();
            ui.horizontal_wrapped(|ui| {
                ui.label(if checked { "☑" } else { "☐" });
                render_segments(ui, &rest, base_size, &mut clicked, embed);
            });
            i += 1;
            continue;
        }

        // Bullet list.
        if let Some(rest) = bullet_prefix(trimmed) {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("•").color(crate::theme::GOLD));
                render_segments(ui, rest, base_size, &mut clicked, embed);
            });
            i += 1;
            continue;
        }

        // Numbered list.
        if let Some((num, rest)) = numbered_prefix(trimmed) {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(num).color(crate::theme::GOLD));
                render_segments(ui, rest, base_size, &mut clicked, embed);
            });
            i += 1;
            continue;
        }

        // Regular paragraph.
        let segs = parse_inline(trimmed);
        if segs.iter().all(|s| matches!(s, Inline::Text { .. })) {
            if let Some(Inline::Text {
                text,
                bold,
                italic,
                strike,
                code,
            }) = segs.into_iter().next()
            {
                let mut rt = RichText::new(text).size(base_size);
                if bold {
                    rt = rt.strong();
                }
                if italic {
                    rt = rt.italics();
                }
                if strike {
                    rt = rt.strikethrough();
                }
                if code {
                    rt = rt
                        .monospace()
                        .background_color(ui.visuals().extreme_bg_color);
                }
                ui.add(egui::Label::new(rt).wrap(true));
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                render_segments(ui, trimmed, base_size, &mut clicked, embed);
            });
        }
        i += 1;
    }

    clicked
}

fn ellipsize_text(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max).collect();
        out.push('…');
        out
    }
}

fn is_media_target(target: &str) -> bool {
    let ext = std::path::Path::new(target)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "bmp"
            | "svg"
            | "mp3"
            | "ogg"
            | "oga"
            | "wav"
            | "m4a"
            | "flac"
            | "opus"
            | "mp4"
            | "webm"
            | "mkv"
            | "mov"
            | "avi"
    )
}

/// Draw centred display math with an optional right-aligned equation number.
fn display_math_tagged(ui: &mut egui::Ui, tex: &str, base_size: f32, number: Option<&str>) {
    ui.add_space(3.0);
    let size = base_size * 1.25;
    let layout = crate::math::layout_math(ui, tex.trim(), size, true);
    let h = (layout.ascent + layout.descent).max(size);
    let width = ui.available_width().max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
    let x = rect.min.x + ((width - layout.width) / 2.0).max(0.0);
    crate::math::paint_math(
        ui.painter(),
        egui::Rect::from_min_size(egui::pos2(x, rect.min.y), Vec2::new(layout.width, h)),
        &layout,
        ui.visuals().text_color(),
    );
    if let Some(n) = number {
        ui.painter().text(
            egui::pos2(rect.max.x - 4.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("({n})"),
            egui::FontId::proportional(size * 0.85),
            ui.visuals().weak_text_color(),
        );
    }
    ui.add_space(3.0);
}

fn code_block(ui: &mut egui::Ui, block: &str, base_size: f32) {
    egui::Frame::none()
        .fill(crate::theme::CODE_BG)
        .stroke(egui::Stroke::new(1.0_f32, crate::theme::BORDER))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .rounding(egui::Rounding::same(6.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    RichText::new(block.trim_end())
                        .monospace()
                        .size(base_size * 0.95)
                        .color(Color32::from_rgb(0xcf, 0xd6, 0xe6)),
                )
                .wrap(true),
            );
        });
    ui.add_space(6.0);
}

fn render_embed(
    ui: &mut egui::Ui,
    target: &str,
    base_size: f32,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
    fence: &dyn Fn(&mut egui::Ui, &str, &str) -> bool,
    depth: usize,
    clicked: &mut Vec<LinkClick>,
) {
    let Some((title, body)) = embed(target) else {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("!").color(ui.visuals().warn_fg_color));
            ui.label(RichText::new(format!("[[{target}]] not found")).weak());
        });
        return;
    };
    egui::Frame::none()
        .fill(crate::theme::ACCENT.linear_multiply(0.06))
        .stroke(egui::Stroke::new(
            1.0_f32,
            crate::theme::ACCENT.linear_multiply(0.4),
        ))
        .inner_margin(egui::Margin::same(11.0))
        .rounding(egui::Rounding::same(7.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(&title).strong().small());
            ui.add_space(2.0);
            if depth < 2 {
                let inner = render(ui, &body, base_size * 0.92, embed, fence, depth + 1);
                clicked.extend(inner);
            } else {
                ui.label(RichText::new("(embed depth limit reached)").weak().small());
            }
        });
    ui.add_space(4.0);
}

#[allow(clippy::too_many_arguments)]
fn render_callout(
    ui: &mut egui::Ui,
    kind: &str,
    title: &str,
    body: &str,
    base_size: f32,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
    fence: &dyn Fn(&mut egui::Ui, &str, &str) -> bool,
    depth: usize,
    clicked: &mut Vec<LinkClick>,
) {
    let accent = crate::theme::ACCENT;
    egui::Frame::none()
        .fill(accent.linear_multiply(0.07))
        .stroke(egui::Stroke::new(1.0_f32, accent.linear_multiply(0.25)))
        .inner_margin(egui::Margin::symmetric(11.0, 9.0))
        .rounding(egui::Rounding::same(7.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(3.0, 16.0), Sense::hover());
                ui.painter().rect_filled(rect, 1.5, accent);
                let label = if title.is_empty() {
                    kind.to_uppercase()
                } else {
                    title.to_string()
                };
                ui.label(RichText::new(label).strong().color(accent));
            });
            ui.add_space(2.0);
            let inner = render(ui, body, base_size * 0.95, embed, fence, depth + 1);
            clicked.extend(inner);
        });
    ui.add_space(4.0);
}

fn is_table_sep(line: &str) -> bool {
    let cells = parse_table_row(line);
    !cells.is_empty()
        && cells.iter().all(|c| {
            let c = c.trim_matches(':').trim();
            c.len() >= 2 && c.chars().all(|ch| ch == '-')
        })
}

fn parse_table_row(line: &str) -> Vec<String> {
    let t = line.trim();
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn render_table(
    ui: &mut egui::Ui,
    rows: &[Vec<String>],
    base_size: f32,
    clicked: &mut Vec<LinkClick>,
    salt: usize,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
) {
    if rows.is_empty() {
        return;
    }
    // Spread the table across the reading column instead of letting it hug the
    // left edge: every column gets an equal share of the available width, with
    // cell text wrapping inside it.
    let cols = rows.iter().map(Vec::len).max().unwrap_or(0).max(1);
    let spacing = 0.0f32;
    let pad = 8.0f32;
    let avail = ui.available_width().max(120.0);
    let col_w = ((avail - spacing * (cols.saturating_sub(1) as f32)) / cols as f32).max(60.0);

    egui::Frame::none()
        .stroke(egui::Stroke::new(1.0_f32, crate::theme::BORDER))
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::same(1.0))
        .show(ui, |ui| {
            ui.set_width(avail);
            egui::Grid::new(("md-table", salt))
                .striped(false)
                .spacing([spacing, 0.0])
                .min_col_width(col_w)
                .show(ui, |ui| {
                    for (r, row) in rows.iter().enumerate() {
                        let is_header = r == 0;
                        for c in 0..cols {
                            let cell = row.get(c).map(String::as_str).unwrap_or("");
                            let bg = if is_header {
                                Some(crate::theme::PANEL)
                            } else if r % 2 == 0 {
                                Some(crate::theme::CODE_BG.linear_multiply(0.6))
                            } else {
                                None
                            };
                            let mut frame =
                                egui::Frame::none().inner_margin(egui::Margin::symmetric(pad, 6.0));
                            if let Some(bg) = bg {
                                frame = frame.fill(bg);
                            }
                            frame.show(ui, |ui| {
                                ui.set_min_width(col_w - pad * 2.0);
                                ui.set_max_width(col_w - pad * 2.0);
                                ui.vertical(|ui| {
                                    ui.style_mut().wrap = Some(true);
                                    if is_header {
                                        render_segments_bold(ui, cell, base_size, clicked, embed);
                                    } else {
                                        render_segments(ui, cell, base_size, clicked, embed);
                                    }
                                });
                            });
                        }
                        ui.end_row();
                    }
                });
        });
    ui.add_space(6.0);
}

fn render_segments_bold(
    ui: &mut egui::Ui,
    line: &str,
    base_size: f32,
    clicked: &mut Vec<LinkClick>,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
) {
    for seg in parse_inline(line) {
        match seg {
            Inline::Text {
                text, code, strike, ..
            } => {
                let mut rt = RichText::new(text).size(base_size).strong();
                if code {
                    rt = rt.monospace();
                }
                if strike {
                    rt = rt.strikethrough();
                }
                ui.label(rt);
            }
            other => render_one(ui, other, base_size, clicked, embed),
        }
    }
}

fn render_segments(
    ui: &mut egui::Ui,
    line: &str,
    base_size: f32,
    clicked: &mut Vec<LinkClick>,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
) {
    for seg in parse_inline(line) {
        render_one(ui, seg, base_size, clicked, embed);
    }
}

fn render_one(
    ui: &mut egui::Ui,
    seg: Inline,
    base_size: f32,
    clicked: &mut Vec<LinkClick>,
    embed: &dyn Fn(&str) -> Option<(String, String)>,
) {
    match seg {
        Inline::Text {
            text,
            bold,
            italic,
            strike,
            code,
        } => {
            let mut rt = RichText::new(text).size(base_size);
            if bold {
                rt = rt.strong();
            }
            if italic {
                rt = rt.italics();
            }
            if strike {
                rt = rt.strikethrough();
            }
            if code {
                rt = rt
                    .monospace()
                    .color(Color32::from_rgb(0xcf, 0xd6, 0xe6))
                    .background_color(crate::theme::CODE_BG);
            }
            ui.label(rt);
        }
        Inline::Wiki { target, alias } => {
            // Resolved links are periwinkle; unresolved ones are warm orange.
            let resolved = embed(&target);
            let color = if resolved.is_some() {
                crate::theme::LINK
            } else {
                crate::theme::ORANGE
            };
            let resp = ui.link(RichText::new(alias).size(base_size).color(color));
            // Hover preview: show the linked note's title and opening lines.
            if let Some((title, body)) = resolved {
                resp.clone().on_hover_ui(|ui| {
                    ui.set_max_width(360.0);
                    ui.label(RichText::new(title).strong());
                    ui.separator();
                    let preview: String = body.lines().take(12).collect::<Vec<_>>().join("\n");
                    ui.label(RichText::new(ellipsize_text(&preview, 600)).small());
                });
            }
            if resp.clicked() {
                clicked.push(LinkClick::Wiki(target));
            }
        }
        Inline::External { text, url } => {
            let color = ui.visuals().hyperlink_color;
            let shown = if text.is_empty() { url.clone() } else { text };
            if ui
                .link(
                    RichText::new(shown)
                        .size(base_size)
                        .color(color)
                        .underline(),
                )
                .clicked()
            {
                clicked.push(LinkClick::External(url));
            }
        }
        Inline::Footnote(id) => {
            ui.label(
                RichText::new(format!("[{id}]"))
                    .size(base_size * 0.7)
                    .color(ui.visuals().hyperlink_color),
            );
        }
        Inline::Math(tex) => {
            draw_math(ui, &tex, base_size * 1.05, false);
        }
        Inline::Highlight(text) => {
            ui.label(
                RichText::new(text)
                    .size(base_size)
                    .color(crate::theme::HIGHLIGHT_TEXT)
                    .background_color(crate::theme::HIGHLIGHT),
            );
        }
    }
}

/// Lay out and draw a math expression with the native LaTeX renderer.
fn draw_math(ui: &mut egui::Ui, tex: &str, size: f32, display: bool) {
    let layout = crate::math::layout_math(ui, tex, size, display);
    if layout.width <= 0.0 {
        return;
    }
    let h = (layout.ascent + layout.descent).max(1.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(layout.width, h), Sense::hover());
    crate::math::paint_math(ui.painter(), rect, &layout, ui.visuals().text_color());
}

fn bullet_prefix(line: &str) -> Option<&str> {
    for prefix in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

fn numbered_prefix(line: &str) -> Option<(String, &str)> {
    let digits: String = line.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &line[digits.len()..];
    if let Some(rest) = rest.strip_prefix(". ") {
        Some((format!("{digits}."), rest))
    } else {
        None
    }
}

// ---------------------------------------------------------- WYSIWYG editing

/// Inline emphasis state while scanning a line of markdown.
#[derive(Clone, Copy, Default)]
struct InlineFlags {
    bold: bool,
    italic: bool,
    strike: bool,
    code: bool,
    highlight: bool,
}

/// Build a [`egui::text::LayoutJob`] for the editor that renders markdown with
/// its syntax markers hidden (dimensionally collapsed) and its content styled.
///
/// The job text is byte-for-byte identical to `text`, so the caret/selection
/// indices used by `TextEdit` stay valid. Misspelled words are underlined.
pub fn wysiwyg_job(
    text: &str,
    base_size: f32,
    conceal: bool,
    misspelled: &std::collections::HashSet<String>,
    caret: Option<usize>,
    sel: Option<(usize, usize)>,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob {
        break_on_newline: true,
        ..Default::default()
    };
    job.wrap.max_width = f32::INFINITY;

    let mut start = 0usize;
    loop {
        let end = text[start..]
            .find('\n')
            .map(|p| start + p)
            .unwrap_or(text.len());
        let line = &text[start..end];
        push_wysiwyg_line(
            &mut job, line, base_size, conceal, misspelled, start, caret, sel,
        );
        if end >= text.len() {
            break;
        }
        job.append("\n", 0.0, base_format(base_size, crate::theme::TEXT));
        start = end + 1;
    }
    job
}

/// Whether the caret / selection touches the absolute byte span `[a, b]`.
fn reveal_span(a: usize, b: usize, caret: Option<usize>, sel: Option<(usize, usize)>) -> bool {
    if let Some(c) = caret {
        if c >= a && c <= b {
            return true;
        }
    }
    if let Some((x, y)) = sel {
        if x < y && a < y && b > x {
            return true;
        }
    }
    false
}

/// Line-start anchor used to know whether the caret sits on this line.
fn line_is_active(
    line_start: usize,
    line_len: usize,
    caret: Option<usize>,
    sel: Option<(usize, usize)>,
) -> bool {
    let a = line_start;
    let b = line_start + line_len;
    if let Some(c) = caret {
        if c >= a && c <= b {
            return true;
        }
    }
    if let Some((x, y)) = sel {
        if x < y && a < y && b > x {
            return true;
        }
    }
    false
}

/// Overlay find-match highlights onto an existing [`LayoutJob`] without
/// changing its text or byte offsets.
pub fn apply_highlights(job: &mut egui::text::LayoutJob, ranges: &[(usize, usize)]) {
    if ranges.is_empty() {
        return;
    }
    let mut out: Vec<egui::text::LayoutSection> = Vec::new();
    for section in &job.sections {
        let (mut a, b) = (section.byte_range.start, section.byte_range.end);
        let mut cuts: Vec<usize> = ranges
            .iter()
            .flat_map(|(s, e)| [*s, *e])
            .filter(|p| *p > a && *p < b)
            .collect();
        cuts.sort_unstable();
        cuts.dedup();
        cuts.push(b);
        let mut first = true;
        for cut in cuts {
            let mut fmt = section.format.clone();
            let hit = ranges.iter().any(|(s, e)| a >= *s && a < *e);
            if hit {
                fmt.background = crate::theme::ACCENT.linear_multiply(0.35);
            }
            out.push(egui::text::LayoutSection {
                leading_space: if first { section.leading_space } else { 0.0 },
                byte_range: a..cut,
                format: fmt,
            });
            first = false;
            a = cut;
        }
    }
    job.sections = out;
}

fn base_format(size: f32, color: Color32) -> egui::text::TextFormat {
    egui::text::TextFormat {
        font_id: egui::FontId::monospace(size),
        color,
        line_height: Some(size * 1.55),
        ..Default::default()
    }
}

fn hidden_format() -> egui::text::TextFormat {
    egui::text::TextFormat {
        font_id: egui::FontId::monospace(1.0),
        color: Color32::TRANSPARENT,
        extra_letter_spacing: 0.0,
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn push_wysiwyg_line(
    job: &mut egui::text::LayoutJob,
    line: &str,
    base_size: f32,
    conceal: bool,
    misspelled: &std::collections::HashSet<String>,
    line_start: usize,
    caret: Option<usize>,
    sel: Option<(usize, usize)>,
) {
    if line.is_empty() {
        return;
    }
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let active = line_is_active(line_start, line.len(), caret, sel);

    // Fenced code lines are shown verbatim in a muted monospace face.
    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        job.append(
            line,
            0.0,
            egui::text::TextFormat {
                font_id: egui::FontId::monospace(base_size),
                color: Color32::from_gray(0x9a),
                background: crate::theme::CODE_BG,
                line_height: Some(base_size * 1.5),
                ..Default::default()
            },
        );
        return;
    }

    // Headings: hide the `#` markers and scale the text (revealed on the
    // caret's line, like Obsidian).
    if conceal {
        let hashes = trimmed.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && trimmed.chars().nth(hashes).is_some_and(|c| c == ' ') {
            let prefix = indent + hashes + 1;
            if active {
                job.append(
                    &line[..prefix],
                    0.0,
                    base_format(base_size, Color32::from_gray(0x8a)),
                );
            } else {
                job.append(&line[..prefix], 0.0, hidden_format());
            }
            let scale = match hashes {
                1 => 1.55,
                2 => 1.33,
                3 => 1.18,
                4 => 1.08,
                _ => 1.03,
            };
            let fmt = base_format(base_size * scale, crate::theme::heading_color(hashes));
            emit_inline(
                job,
                &line[prefix..],
                fmt,
                InlineFlags::default(),
                conceal,
                misspelled,
                line_start + prefix,
                caret,
                sel,
            );
            return;
        }
        // Blockquotes: hide `> ` and mute the text (revealed on the caret line).
        if trimmed.starts_with('>') {
            let mut prefix = indent;
            while line[prefix..].starts_with('>') {
                prefix += 1;
                if line[prefix..].starts_with(' ') {
                    prefix += 1;
                }
            }
            if active {
                job.append(
                    &line[..prefix],
                    0.0,
                    base_format(base_size, Color32::from_gray(0x8a)),
                );
            } else {
                job.append(&line[..prefix], 0.0, hidden_format());
            }
            let fmt = base_format(base_size, Color32::from_gray(0xa8));
            let flags = InlineFlags {
                italic: true,
                ..Default::default()
            };
            emit_inline(
                job,
                &line[prefix..],
                fmt,
                flags,
                conceal,
                misspelled,
                line_start + prefix,
                caret,
                sel,
            );
            return;
        }
    }

    let fmt = base_format(base_size, crate::theme::TEXT);
    emit_inline(
        job,
        line,
        fmt,
        InlineFlags::default(),
        conceal,
        misspelled,
        line_start,
        caret,
        sel,
    );
}

fn style_for(
    fmt: &egui::text::TextFormat,
    flags: InlineFlags,
    link: bool,
) -> egui::text::TextFormat {
    let mut out = fmt.clone();
    if flags.bold {
        out.color = Color32::from_gray(0xf4);
        out.extra_letter_spacing = 0.2;
    }
    if flags.italic {
        out.italics = true;
    }
    if flags.strike {
        out.strikethrough = egui::Stroke::new(1.0_f32, out.color);
    }
    if flags.highlight {
        out.background = crate::theme::HIGHLIGHT;
        out.color = crate::theme::HIGHLIGHT_TEXT;
    }
    if flags.code {
        out.font_id = egui::FontId::monospace(out.font_id.size * 0.96);
        out.background = crate::theme::CODE_BG;
        out.color = Color32::from_rgb(0xcf, 0xd6, 0xe6);
    }
    if link {
        out.color = crate::theme::LINK;
        out.underline = egui::Stroke::new(1.0_f32, crate::theme::LINK);
    }
    out
}

fn emit_text(
    job: &mut egui::text::LayoutJob,
    text: &str,
    fmt: egui::text::TextFormat,
    misspelled: &std::collections::HashSet<String>,
) {
    if text.is_empty() {
        return;
    }
    if misspelled.is_empty() {
        job.append(text, 0.0, fmt);
        return;
    }
    // Split into words and separators, marking unknown words.
    let mut token = String::new();
    let mut token_start = 0usize;
    let mut last = 0usize;
    for (i, c) in text.char_indices() {
        let w = c.is_alphabetic() || c == '\'' || c == '-';
        if w {
            if token.is_empty() {
                token_start = i;
                if token_start > last {
                    job.append(&text[last..token_start], 0.0, fmt.clone());
                }
            }
            token.push(c);
        } else if !token.is_empty() {
            push_word(job, &token, &text[token_start..i], &fmt, misspelled);
            token.clear();
            last = i;
        }
    }
    if !token.is_empty() {
        push_word(job, &token, &text[token_start..], &fmt, misspelled);
        last = text.len();
    }
    if last < text.len() {
        job.append(&text[last..], 0.0, fmt.clone());
    }
}

fn push_word(
    job: &mut egui::text::LayoutJob,
    word: &str,
    original: &str,
    fmt: &egui::text::TextFormat,
    misspelled: &std::collections::HashSet<String>,
) {
    let known =
        word.chars().all(|c| !c.is_alphabetic()) || !misspelled.contains(&word.to_lowercase());
    if known {
        job.append(original, 0.0, fmt.clone());
    } else {
        let mut flagged = fmt.clone();
        flagged.underline = egui::Stroke::new(1.2_f32, crate::theme::DANGER);
        job.append(original, 0.0, flagged);
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_inline(
    job: &mut egui::text::LayoutJob,
    s: &str,
    base: egui::text::TextFormat,
    initial: InlineFlags,
    conceal: bool,
    misspelled: &std::collections::HashSet<String>,
    offset: usize,
    caret: Option<usize>,
    sel: Option<(usize, usize)>,
) {
    #[derive(Clone, Copy)]
    enum Marker {
        Bold,
        Strike,
        Code,
        Italic,
        Highlight,
    }
    let mut flags = initial;
    let mut i = 0usize;
    let mut plain = 0usize;
    // Closing marker positions that were revealed together with their opening.
    let mut revealed: Vec<(usize, Marker)> = Vec::new();

    macro_rules! flush {
        ($end:expr) => {
            if $end > plain {
                let fmt = style_for(&base, flags, false);
                if flags.code {
                    job.append(&s[plain..$end], 0.0, fmt);
                } else {
                    emit_text(job, &s[plain..$end], fmt, misspelled);
                }
            }
        };
    }
    macro_rules! toggle {
        ($m:expr) => {
            match $m {
                Marker::Bold => flags.bold = !flags.bold,
                Marker::Strike => flags.strike = !flags.strike,
                Marker::Code => flags.code = !flags.code,
                Marker::Italic => flags.italic = !flags.italic,
                Marker::Highlight => flags.highlight = !flags.highlight,
            }
        };
    }

    while i < s.len() {
        let rest = &s[i..];
        if conceal {
            // A closing marker whose opener we revealed: show it literally.
            if let Some(pos) = revealed.iter().position(|(at, _)| *at == i) {
                let (_, kind) = revealed.remove(pos);
                let len = match kind {
                    Marker::Bold | Marker::Strike | Marker::Highlight => 2,
                    Marker::Code | Marker::Italic => 1,
                };
                flush!(i);
                job.append(&s[i..i + len], 0.0, base.clone());
                toggle!(kind);
                i += len;
                plain = i;
                continue;
            }
            // Bold / strikethrough / code / italic markers.
            let marker = if rest.starts_with("**") || rest.starts_with("__") {
                Some(("**", 2, Marker::Bold))
            } else if rest.starts_with("~~") {
                Some(("~~", 2, Marker::Strike))
            } else if rest.starts_with("==") {
                Some(("==", 2, Marker::Highlight))
            } else if rest.starts_with('`') {
                Some(("`", 1, Marker::Code))
            } else if rest.starts_with('*') || rest.starts_with('_') {
                Some(("*", 1, Marker::Italic))
            } else {
                None
            };
            if let Some((pat, len, kind)) = marker {
                let close = s[i + len..]
                    .find(pat)
                    .map(|p| i + len + p)
                    .filter(|c| *c > i + len - 1);
                let show =
                    close.is_some_and(|c| reveal_span(offset + i, offset + c + len, caret, sel));
                flush!(i);
                if show {
                    let c = close.unwrap();
                    job.append(&s[i..i + len], 0.0, base.clone());
                    revealed.push((c, kind));
                } else {
                    job.append(&s[i..i + len], 0.0, hidden_format());
                }
                toggle!(kind);
                i += len;
                plain = i;
                continue;
            }
            // Embeds `![[target]]`: show the target as a link.
            if rest.starts_with("![[") {
                if let Some(close) = rest.find("]]") {
                    let inner = &rest[3..close];
                    let (target, alias) = split_wiki(inner);
                    flush!(i);
                    if reveal_span(offset + i, offset + i + close + 2, caret, sel) {
                        job.append(&s[i..i + close + 2], 0.0, base.clone());
                    } else {
                        job.append("![[", 0.0, hidden_format());
                        let shown = if alias.is_empty() { target } else { alias };
                        let fmt = style_for(&base, flags, true);
                        emit_text(job, shown, fmt, misspelled);
                        job.append("]]", 0.0, hidden_format());
                    }
                    i += close + 2;
                    plain = i;
                    continue;
                }
            }
            // Wikilinks `[[target|alias]]`: show the alias (or target).
            if rest.starts_with("[[") {
                if let Some(close) = rest.find("]]") {
                    flush!(i);
                    if reveal_span(offset + i, offset + i + close + 2, caret, sel) {
                        job.append(&s[i..i + close + 2], 0.0, base.clone());
                    } else {
                        let inner = &rest[2..close];
                        let (target, alias) = split_wiki(inner);
                        job.append("[[", 0.0, hidden_format());
                        let fmt = style_for(&base, flags, true);
                        if alias.is_empty() {
                            emit_text(job, target, fmt, misspelled);
                        } else {
                            job.append(&inner[..inner.len() - alias.len()], 0.0, hidden_format());
                            emit_text(job, alias, fmt, misspelled);
                        }
                        job.append("]]", 0.0, hidden_format());
                    }
                    i += close + 2;
                    plain = i;
                    continue;
                }
            }
            // Markdown links `[text](url)`: show the text, hide the URL.
            if rest.starts_with('[') {
                if let Some(mid) = rest.find("](") {
                    if let Some(close) = rest[mid + 2..].find(')') {
                        let label = &rest[1..mid];
                        if !label.is_empty() && !label.contains('\n') {
                            flush!(i);
                            let end = mid + 3 + close;
                            if reveal_span(offset + i, offset + i + end, caret, sel) {
                                job.append(&s[i..i + end], 0.0, base.clone());
                            } else {
                                job.append("[", 0.0, hidden_format());
                                let fmt = style_for(&base, flags, true);
                                emit_text(job, label, fmt, misspelled);
                                job.append(&rest[mid..end], 0.0, hidden_format());
                            }
                            i += end;
                            plain = i;
                            continue;
                        }
                    }
                }
            }
            // Backslash escapes: hide the backslash itself.
            if rest.starts_with('\\') && rest.len() > 1 {
                flush!(i);
                job.append("\\", 0.0, hidden_format());
                i += 1;
                plain = i;
                continue;
            }
        }
        i += s[i..].chars().next().map_or(1, |c| c.len_utf8());
    }
    flush!(s.len());
}

/// Split a wikilink body into `(target, alias)`.
fn split_wiki(inner: &str) -> (&str, &str) {
    match inner.rsplit_once('|') {
        Some((target, alias)) => (target, alias),
        None => (inner, ""),
    }
}

/// A no-op colour used by tests only.
#[allow(dead_code)]
const _UNUSED: Color32 = Color32::WHITE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_plain_wikilinks() {
        assert_eq!(
            wikilinks("see [[Alpha]] and [[Beta]]"),
            vec!["Alpha", "Beta"]
        );
    }

    #[test]
    fn extracts_aliased_wikilinks() {
        assert_eq!(wikilinks("[[Target|shown text]]"), vec!["Target"]);
    }

    #[test]
    fn ignores_unterminated_wikilinks() {
        assert!(wikilinks("[[oops").is_empty());
    }

    #[test]
    fn collects_headings_with_levels() {
        let md = "# Title\n\ntext\n\n## Sub\n### Deep\n";
        assert_eq!(
            headings(md),
            vec![
                (1, "Title".to_string()),
                (2, "Sub".to_string()),
                (3, "Deep".to_string()),
            ]
        );
    }

    #[test]
    fn ignores_headings_inside_code_fences() {
        let md = "```\n# not a heading\n```\n# real\n";
        assert_eq!(headings(md), vec![(1, "real".to_string())]);
    }

    #[test]
    fn parses_bold_italic_code_and_strike() {
        let segs = parse_inline("a **b** c *d* `e` ~~f~~");
        let texts: Vec<(String, bool, bool, bool, bool)> = segs
            .into_iter()
            .filter_map(|s| match s {
                Inline::Text {
                    text,
                    bold,
                    italic,
                    strike,
                    code,
                } => Some((text, bold, italic, strike, code)),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|(t, b, _, _, _)| t == "b" && *b));
        assert!(texts.iter().any(|(t, _, i, _, _)| t == "d" && *i));
        assert!(texts.iter().any(|(t, _, _, _, c)| t == "e" && *c));
        assert!(texts.iter().any(|(t, _, _, s, _)| t == "f" && *s));
    }

    #[test]
    fn parses_markdown_links() {
        let segs = parse_inline("go [here](https://example.com) now");
        assert!(segs.iter().any(|s| matches!(
            s,
            Inline::External { url, .. } if url == "https://example.com"
        )));
    }

    #[test]
    fn table_separator_detected() {
        assert!(is_table_sep("|---|:---:|---|"));
        assert!(!is_table_sep("| a | b |"));
        assert!(is_table_sep("|---|---|"));
    }

    fn section_at(job: &egui::text::LayoutJob, byte: usize) -> &egui::text::LayoutSection {
        job.sections
            .iter()
            .find(|s| s.byte_range.start <= byte && byte < s.byte_range.end)
            .expect("section at byte")
    }

    #[test]
    fn wysiwyg_conceals_markers_away_from_caret() {
        let miss = std::collections::HashSet::new();
        let text = "a **bold** b";
        let job = wysiwyg_job(text, 14.0, true, &miss, Some(0), Some((0, 0)));
        assert_eq!(job.text, text, "layout text must stay byte-identical");
        let marker = section_at(&job, 2);
        assert!(marker.format.font_id.size < 2.0, "marker should be hidden");
    }

    #[test]
    fn wysiwyg_reveals_markers_at_caret() {
        let miss = std::collections::HashSet::new();
        let text = "a **bold** b";
        // Caret inside the bold run.
        let job = wysiwyg_job(text, 14.0, true, &miss, Some(5), Some((5, 5)));
        let marker = section_at(&job, 2);
        assert!(
            marker.format.font_id.size >= 13.0,
            "marker should be visible"
        );
        // Heading markers appear on the caret's line.
        let head = "# Title";
        let job = wysiwyg_job(head, 14.0, true, &miss, Some(3), Some((3, 3)));
        let hash = section_at(&job, 0);
        assert!(hash.format.font_id.size >= 13.0);
        let job = wysiwyg_job(head, 14.0, true, &miss, Some(0), Some((0, 0)));
        let hash = section_at(&job, 0);
        assert!(hash.format.font_id.size >= 13.0);
        let job = wysiwyg_job(
            "# Title\n\ntext",
            14.0,
            true,
            &miss,
            Some(12),
            Some((12, 12)),
        );
        let hash = section_at(&job, 0);
        assert!(hash.format.font_id.size < 2.0, "inactive heading hides #");
    }

    #[test]
    fn eqref_and_ref_expand_to_numbers() {
        reset_equations();
        let n = next_equation();
        take_label(&mut "\\label{energy}".to_string(), n);
        let mut t = "see \\eqref{energy} and \\ref{energy}".to_string();
        expand_refs(&mut t);
        assert_eq!(t, "see (1) and 1");
        let mut missing = "\\eqref{nope}".to_string();
        expand_refs(&mut missing);
        assert_eq!(missing, "(??)");
        reset_equations();
    }

    #[test]
    fn cleveref_commands_expand() {
        reset_equations();
        take_label(&mut "\\label{a}".to_string(), next_equation());
        take_label(&mut "\\label{b}".to_string(), next_equation());
        let mut t = "\\cref{a,b}".to_string();
        expand_refs(&mut t);
        assert_eq!(t, "(1, 2)");
        let mut t = "\\Cref{a}".to_string();
        expand_refs(&mut t);
        assert_eq!(t, "Eq. (1)");
        let mut t = "\\crefrange{a}{b}".to_string();
        expand_refs(&mut t);
        assert_eq!(t, "(1)–(2)");
        reset_equations();
    }

    #[test]
    fn tables_fill_the_reading_column() {
        use eframe::egui;
        let md = "| Name | Status | Notes |\n|---|---|---|\n| Alpha | done | short |\n| Beta | open | a much longer note body that should wrap |\n";
        let avail = 640.0f32;
        let mut measured = 0.0f32;
        egui::__run_test_ctx(|ctx| {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::pos2(0.0, 0.0),
                    egui::vec2(avail, 400.0),
                )),
                focused: true,
                ..Default::default()
            };
            ctx.begin_frame(input);
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.set_width(avail);
                let no_embed = |_t: &str| -> Option<(String, String)> { None };
                let no_fence = |_u: &mut egui::Ui, _l: &str, _c: &str| -> bool { false };
                render(ui, md, 14.0, &no_embed, &no_fence, 0);
                measured = ui.min_rect().width();
            });
            let _ = ctx.end_frame();
        });
        assert!(
            measured >= avail * 0.95,
            "table should span the column, got {measured} of {avail}"
        );
    }

    #[test]
    fn table_rows_parse_cells() {
        assert_eq!(parse_table_row("| a | b |"), vec!["a", "b"]);
        assert_eq!(parse_table_row("a | b"), vec!["a", "b"]);
    }
}
