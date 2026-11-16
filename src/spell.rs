//! A lightweight, offline spell checker for the editor.
//!
//! It loads a system word list once (`/usr/share/dict/*`, cracklib's word list,
//! or a Hunspell dictionary's `.dic` file) and flags words that are not in it.
//! Markdown syntax, code, URLs, wikilinks and frontmatter are ignored. Common
//! English inflections are accepted when the stem is known, which keeps the
//! false-positive rate low without pulling in a full stemming library.

use std::collections::HashSet;
use std::sync::OnceLock;

use crate::frontmatter;

fn load_words() -> HashSet<String> {
    let mut words: HashSet<String> = HashSet::new();
    let candidates = [
        "/usr/share/dict/words",
        "/usr/share/dict/american-english",
        "/usr/share/dict/british-english",
        "/usr/share/dict/cracklib-small",
    ];
    for path in candidates {
        if let Ok(contents) = std::fs::read_to_string(path) {
            for line in contents.lines() {
                let w = line.trim().to_lowercase();
                if w.len() > 1 && w.chars().all(|c| c.is_alphabetic() || c == '\'') {
                    words.insert(w);
                }
            }
            if !words.is_empty() {
                return words;
            }
        }
    }
    // Fall back to a Hunspell dictionary's word list (first column of each line).
    for dir in [
        "/usr/share/hunspell",
        "/usr/share/myspell",
        "/usr/share/myspell/dicts",
    ] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("dic") {
                    continue;
                }
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    for line in contents.lines().skip(1) {
                        let w = line.split('/').next().unwrap_or("").trim().to_lowercase();
                        if w.len() > 1 && w.chars().all(|c| c.is_alphabetic() || c == '\'') {
                            words.insert(w);
                        }
                    }
                    if !words.is_empty() {
                        return words;
                    }
                }
            }
        }
    }
    words
}

fn dictionary() -> &'static HashSet<String> {
    static DICT: OnceLock<HashSet<String>> = OnceLock::new();
    DICT.get_or_init(load_words)
}

/// Whether any dictionary could be loaded.
pub fn available() -> bool {
    !dictionary().is_empty()
}

/// Whether a lowercase word is known, allowing simple English inflections.
fn known(word: &str) -> bool {
    let dict = dictionary();
    if dict.contains(word) {
        return true;
    }
    let mut stems: Vec<String> = Vec::new();
    if let Some(s) = word.strip_suffix("'s") {
        stems.push(s.to_string());
    }
    for suffix in ["s", "es", "ed", "ing", "ly", "er", "est"] {
        if let Some(s) = word.strip_suffix(suffix) {
            if s.len() >= 2 {
                stems.push(s.to_string());
                // Doubled final consonant: running → run.
                if let Some(last) = s.chars().last() {
                    if s.len() > 2 && last.is_alphabetic() {
                        stems.push(s[..s.len() - 1].to_string());
                    }
                }
            }
        }
    }
    stems.iter().any(|s| dict.contains(s))
}

/// Return the set of misspelled words (lowercase) in a note. Code spans, code
/// fences, frontmatter, URLs, wikilink targets and HTML tags are skipped.
pub fn misspellings(text: &str) -> HashSet<String> {
    if !available() {
        return HashSet::new();
    }
    let body = frontmatter::parse(text).body;
    let mut out: HashSet<String> = HashSet::new();
    let mut in_fence = false;
    for line in body.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let scrubbed = scrub_line(line);
        let mut token = String::new();
        for c in scrubbed.chars() {
            if c.is_alphabetic() || c == '\'' {
                token.push(c);
            } else {
                check_token(&token, &mut out);
                token.clear();
            }
        }
        check_token(&token, &mut out);
    }
    out
}

fn check_token(token: &str, out: &mut HashSet<String>) {
    let len = token.chars().filter(|c| c.is_alphabetic()).count();
    if len < 3 {
        return;
    }
    let lower = token.to_lowercase();
    if !known(&lower) {
        out.insert(lower);
    }
}

/// Drop markdown syntax that should never be spell-checked.
fn scrub_line(line: &str) -> String {
    let mut s = line.to_string();
    // Inline code spans.
    while let Some(start) = s.find('`') {
        if let Some(end) = s[start + 1..].find('`') {
            let end = start + 1 + end;
            s.replace_range(start..=end, " ");
        } else {
            s.replace_range(start..start + 1, " ");
            break;
        }
    }
    // Wikilink targets (keep the alias).
    while let Some(start) = s.find("[[") {
        if let Some(end) = s[start + 2..].find("]]") {
            let end = start + 2 + end;
            let inner = s[start + 2..end].to_string();
            let shown = inner.rsplit('|').next().unwrap_or("").to_string();
            s.replace_range(start..end + 2, &format!(" {shown} "));
        } else {
            break;
        }
    }
    // Markdown links: keep the label, drop the URL.
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(start) = rest.find("](") {
        if let Some(close) = rest[start + 2..].find(')') {
            out.push_str(&rest[..start]);
            out.push(' ');
            rest = &rest[start + 2 + close + 1..];
        } else {
            break;
        }
    }
    out.push_str(rest);
    // Bare URLs and HTML tags.
    let mut cleaned = String::with_capacity(out.len());
    for word in out.split_whitespace() {
        let w = word.trim_matches(|c: char| {
            !(c.is_alphanumeric() || c == '\'' || c == '/' || c == '-' || c == '_')
        });
        if w.contains("://")
            || w.contains("www.")
            || (w.contains('.') && w.rsplit('.').next().is_some_and(|t| t.len() <= 4))
        {
            cleaned.push(' ');
        } else {
            cleaned.push_str(word);
            cleaned.push(' ');
        }
    }
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_inflections() {
        if !available() {
            return;
        }
        assert!(known("notes"));
        assert!(known("running"));
        assert!(!known("qwertyuiopzz"));
    }

    #[test]
    fn ignores_code_and_urls() {
        if !available() {
            return;
        }
        let text =
            "```\nzzzzqqqq\n```\nSee https://example.com/zzzzqqqq and `zzzzqqqq`.\nHello world.\n";
        let m = misspellings(text);
        assert!(!m.contains("zzzzqqqq"));
    }
}
