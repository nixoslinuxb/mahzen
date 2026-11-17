//! A small LaTeX → Unicode "prettifier" for inline and display math.
//!
//! This is *not* a full typesetting engine: it maps common LaTeX control
//! sequences to their Unicode equivalents and converts simple `^`/`_` groups to
//! Unicode super/subscripts, which covers the overwhelming majority of everyday
//! note math. Anything it doesn't understand is left as-is.

/// Convert a LaTeX math snippet to a Unicode string.
#[allow(dead_code)]
pub fn latex_to_unicode(tex: &str) -> String {
    let mut out = String::with_capacity(tex.len());
    let chars: Vec<char> = tex.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            // Control word or symbol.
            let mut j = i + 1;
            let mut name = String::new();
            while j < chars.len() && chars[j].is_ascii_alphabetic() {
                name.push(chars[j]);
                j += 1;
            }
            if name.is_empty() {
                // A single escaped char (\%, \{, ...).
                if j < chars.len() {
                    out.push(chars[j]);
                    i = j + 1;
                } else {
                    i = j;
                }
                continue;
            }
            match symbol(&name) {
                Some(sym) => out.push_str(sym),
                None => {
                    // Unknown control word: keep the name (drop the backslash).
                    out.push_str(&name);
                }
            }
            i = j;
            continue;
        }
        if (c == '^' || c == '_') && i + 1 < chars.len() {
            let (text, next) = read_group(&chars, i + 1);
            let mapped = map_scripts(&text, c == '^');
            out.push_str(&mapped);
            i = next;
            continue;
        }
        if c == '{' || c == '}' {
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Read a `{...}` group (or a single char) starting at `start`.
fn read_group(chars: &[char], start: usize) -> (String, usize) {
    if start < chars.len() && chars[start] == '{' {
        let mut depth = 0usize;
        let mut i = start;
        let mut s = String::new();
        while i < chars.len() {
            match chars[i] {
                '{' => {
                    depth += 1;
                    if depth > 1 {
                        s.push('{');
                    }
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return (s, i + 1);
                    }
                    s.push('}');
                }
                ch => s.push(ch),
            }
            i += 1;
        }
        (s, i)
    } else if start < chars.len() {
        (chars[start].to_string(), start + 1)
    } else {
        (String::new(), start)
    }
}

fn map_scripts(text: &str, sup: bool) -> String {
    let table: &[(char, char)] = if sup {
        &[
            ('0', '⁰'),
            ('1', '¹'),
            ('2', '²'),
            ('3', '³'),
            ('4', '⁴'),
            ('5', '⁵'),
            ('6', '⁶'),
            ('7', '⁷'),
            ('8', '⁸'),
            ('9', '⁹'),
            ('+', '⁺'),
            ('-', '⁻'),
            ('=', '⁼'),
            ('(', '⁽'),
            (')', '⁾'),
            ('n', 'ⁿ'),
            ('i', 'ⁱ'),
            ('x', 'ˣ'),
        ]
    } else {
        &[
            ('0', '₀'),
            ('1', '₁'),
            ('2', '₂'),
            ('3', '₃'),
            ('4', '₄'),
            ('5', '₅'),
            ('6', '₆'),
            ('7', '₇'),
            ('8', '₈'),
            ('9', '₉'),
            ('+', '₊'),
            ('-', '₋'),
            ('=', '₌'),
            ('(', '₍'),
            (')', '₎'),
            ('a', 'ₐ'),
            ('e', 'ₑ'),
            ('i', 'ᵢ'),
            ('n', 'ₙ'),
            ('x', 'ₓ'),
        ]
    };
    let converted: Option<Vec<char>> = text
        .chars()
        .map(|c| table.iter().find(|(k, _)| *k == c).map(|(_, v)| *v))
        .collect();
    match converted {
        Some(v) => v.into_iter().collect(),
        None => {
            // Not fully mappable: fall back to `<base>^text` / `<base>_text`.
            let marker = if sup { '^' } else { '_' };
            if text.chars().count() == 1 {
                format!("{marker}{text}")
            } else {
                format!("{marker}({text})")
            }
        }
    }
}

fn symbol(name: &str) -> Option<&'static str> {
    Some(match name {
        // Greek (lower)
        "alpha" => "α",
        "beta" => "β",
        "gamma" => "γ",
        "delta" => "δ",
        "epsilon" => "ε",
        "zeta" => "ζ",
        "eta" => "η",
        "theta" => "θ",
        "iota" => "ι",
        "kappa" => "κ",
        "lambda" => "λ",
        "mu" => "μ",
        "nu" => "ν",
        "xi" => "ξ",
        "pi" => "π",
        "rho" => "ρ",
        "sigma" => "σ",
        "tau" => "τ",
        "upsilon" => "υ",
        "phi" => "φ",
        "chi" => "χ",
        "psi" => "ψ",
        "omega" => "ω",
        // Greek (upper)
        "Gamma" => "Γ",
        "Delta" => "Δ",
        "Theta" => "Θ",
        "Lambda" => "Λ",
        "Xi" => "Ξ",
        "Pi" => "Π",
        "Sigma" => "Σ",
        "Phi" => "Φ",
        "Psi" => "Ψ",
        "Omega" => "Ω",
        // Operators / relations
        "times" => "×",
        "cdot" => "·",
        "div" => "÷",
        "pm" => "±",
        "mp" => "∓",
        "leq" | "le" => "≤",
        "geq" | "ge" => "≥",
        "neq" | "ne" => "≠",
        "approx" => "≈",
        "equiv" => "≡",
        "propto" => "∝",
        "to" | "rightarrow" => "→",
        "leftarrow" => "←",
        "leftrightarrow" => "↔",
        "Rightarrow" => "⇒",
        "Leftarrow" => "⇐",
        "Leftrightarrow" => "⇔",
        "in" => "∈",
        "notin" => "∉",
        "subset" => "⊂",
        "supset" => "⊃",
        "subseteq" => "⊆",
        "supseteq" => "⊇",
        "cup" => "∪",
        "cap" => "∩",
        "forall" => "∀",
        "exists" => "∃",
        "neg" | "lnot" => "¬",
        "wedge" | "land" => "∧",
        "vee" | "lor" => "∨",
        // Big operators / misc
        "sum" => "∑",
        "prod" => "∏",
        "int" => "∫",
        "iint" => "∬",
        "oint" => "∮",
        "partial" => "∂",
        "nabla" => "∇",
        "infty" => "∞",
        "sqrt" => "√",
        "angle" => "∠",
        "degree" | "deg" => "°",
        "prime" => "′",
        "lvert" | "rvert" | "vert" | "mid" => "|",
        "lVert" | "rVert" | "Vert" => "‖",
        "langle" => "⟨",
        "rangle" => "⟩",
        "lbrace" => "{",
        "rbrace" => "}",
        "colon" => ":",
        "sim" => "∼",
        "simeq" => "≃",
        "cong" => "≅",
        "ll" => "≪",
        "gg" => "≫",
        "oplus" => "⊕",
        "otimes" => "⊗",
        "emptyset" | "varnothing" => "∅",
        "mathbb" => "ℝ",
        "circ" => "∘",
        "bullet" => "•",
        "star" => "⋆",
        "dagger" => "†",
        "ddagger" => "‡",
        "Re" => "ℜ",
        "Im" => "ℑ",
        "wp" => "℘",
        "top" => "⊤",
        "bot" => "⊥",
        "vdash" => "⊢",
        "models" => "⊨",
        "implies" => "⟹",
        "iff" => "⟺",
        "mapsto" => "↦",
        "hookrightarrow" => "↪",
        "uparrow" => "↑",
        "downarrow" => "↓",
        "longrightarrow" => "⟶",
        "longleftarrow" => "⟵",
        "Longrightarrow" => "⟹",
        "ldots" | "dots" => "…",
        "cdots" => "⋯",
        "hbar" => "ℏ",
        "ell" => "ℓ",
        "aleph" => "ℵ",
        "therefore" => "∴",
        "because" => "∵",
        _ => return None,
    })
}

// ===========================================================================
// Native LaTeX layout & rendering
// ===========================================================================
//
// A real (if compact) math typesetter: parses fractions, roots, scripts, big
// operators with limits, matrices/aligned/cases environments, scalable
// delimiters, `\text`, accents and the usual symbol commands, lays them out
// against the egui font metrics, and paints the result. This replaces the old
// Unicode "prettifier" for actual rendering; `latex_to_unicode` is still used
// as a last-resort fallback elsewhere.

use eframe::egui;
use std::sync::Arc;

#[derive(Clone, Debug)]
enum Ast {
    Row(Vec<Ast>),
    Sym(String),
    Text(String),
    Space(f32),
    Frac(Box<Ast>, Box<Ast>),
    Sqrt(Option<Box<Ast>>, Box<Ast>),
    Script(Box<Ast>, Option<Box<Ast>>, Option<Box<Ast>>),
    BigOp(String, Option<Box<Ast>>, Option<Box<Ast>>),
    Delim(String, Box<Ast>, String),
    Matrix(Vec<Vec<Ast>>, MatrixKind, Vec<HAlign>),
    Accent(Box<Ast>, char),
    /// A coloured sub-expression (`\textcolor{red}{…}`, `\color{red} …`).
    Color(egui::Color32, Box<Ast>),
    /// A strike-through decoration (`\cancel`, `\bcancel`, `\xcancel`).
    Cancel(Box<Ast>, bool, bool),
    /// A horizontal line-only strike (`\sout`).
    Strike(Box<Ast>),
    /// A filled frame around the expression (`\boxed`).
    Boxed(Box<Ast>),
}

/// Horizontal alignment of an `array` column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatrixKind {
    Plain,
    Paren,
    Bracket,
    Brace,
    Bar,
    Aligned,
    Cases,
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ch(char),
    Cmd(String),
    L,
    R,
    Sup,
    Sub,
    Amp,
    NewRow,
    LBracket,
    RBracket,
}

fn lex(src: &str) -> Vec<Tok> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '{' => {
                out.push(Tok::L);
                i += 1;
            }
            '}' => {
                out.push(Tok::R);
                i += 1;
            }
            '^' => {
                out.push(Tok::Sup);
                i += 1;
            }
            '_' => {
                out.push(Tok::Sub);
                i += 1;
            }
            '&' => {
                out.push(Tok::Amp);
                i += 1;
            }
            '[' => {
                out.push(Tok::LBracket);
                i += 1;
            }
            ']' => {
                out.push(Tok::RBracket);
                i += 1;
            }
            '\\' => {
                if i + 1 < chars.len() && chars[i + 1] == '\\' {
                    out.push(Tok::NewRow);
                    i += 2;
                    continue;
                }
                i += 1;
                let mut name = String::new();
                while i < chars.len() && chars[i].is_ascii_alphabetic() {
                    name.push(chars[i]);
                    i += 1;
                }
                if name.is_empty() {
                    if i < chars.len() {
                        // Escaped single character (\% \{ ...).
                        out.push(Tok::Ch(chars[i]));
                        i += 1;
                    } else {
                        out.push(Tok::Ch('\\'));
                    }
                } else if name == "quad" || name == "qquad" {
                    out.push(Tok::Ch(' '));
                } else {
                    out.push(Tok::Cmd(name));
                }
            }
            _ => {
                out.push(Tok::Ch(c));
                i += 1;
            }
        }
    }
    out
}

struct Parser<'a> {
    toks: &'a [Tok],
    i: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stop {
    None,
    Brace,
    Right,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i)
    }

    fn eat(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.i).cloned();
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn parse_seq(&mut self, stop: Stop) -> Ast {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(Tok::R) if stop == Stop::Brace => break,
                Some(Tok::Cmd(c)) if stop == Stop::Right && c == "right" => break,
                Some(Tok::R) | Some(Tok::Amp) | Some(Tok::NewRow) => break,
                _ => {}
            }
            let atom = self.parse_atom();
            // `\color{red}` colours everything that follows in this group.
            if let Ast::Color(c, inner) = &atom {
                if matches!(**inner, Ast::Row(ref v) if v.is_empty()) {
                    let mut rest = Vec::new();
                    loop {
                        match self.peek() {
                            None => break,
                            Some(Tok::R) if stop == Stop::Brace => break,
                            Some(Tok::Cmd(c)) if stop == Stop::Right && c == "right" => break,
                            Some(Tok::R) | Some(Tok::Amp) | Some(Tok::NewRow) => break,
                            _ => {}
                        }
                        let a = self.parse_atom();
                        rest.push(self.attach_scripts(a));
                    }
                    items.push(Ast::Color(*c, Box::new(Ast::Row(rest))));
                    break;
                }
            }
            let atom = self.attach_scripts(atom);
            items.push(atom);
        }
        if items.len() == 1 {
            items.pop().unwrap()
        } else {
            Ast::Row(items)
        }
    }

    fn attach_scripts(&mut self, mut base: Ast) -> Ast {
        let mut sup: Option<Box<Ast>> = None;
        let mut sub: Option<Box<Ast>> = None;
        loop {
            match self.peek() {
                Some(Tok::Sup) => {
                    self.i += 1;
                    sup = Some(Box::new(self.parse_group()));
                }
                Some(Tok::Sub) => {
                    self.i += 1;
                    sub = Some(Box::new(self.parse_group()));
                }
                _ => break,
            }
        }
        if sup.is_none() && sub.is_none() {
            return base;
        }
        if let Ast::BigOp(ch, _, _) = &base {
            return Ast::BigOp(ch.clone(), sub, sup);
        }
        if let Ast::Row(items) = &base {
            if let Some(Ast::BigOp(ch, _, _)) = items.last() {
                let ch = ch.clone();
                let mut rest = items.clone();
                rest.pop();
                let op = Ast::BigOp(ch, sub, sup);
                rest.push(op);
                return Ast::Row(rest);
            }
        }
        base = Ast::Script(Box::new(base), sup, sub);
        base
    }

    fn parse_group(&mut self) -> Ast {
        match self.eat() {
            Some(Tok::L) => {
                let inner = self.parse_seq(Stop::Brace);
                if matches!(self.peek(), Some(Tok::R)) {
                    self.i += 1;
                }
                inner
            }
            Some(Tok::Ch(c)) => Ast::Sym(c.to_string()),
            Some(Tok::Cmd(c)) => self.command(&c),
            _ => Ast::Row(Vec::new()),
        }
    }

    fn read_env_name(&mut self) -> String {
        let mut name = String::new();
        if matches!(self.peek(), Some(Tok::L)) {
            self.i += 1;
            while let Some(t) = self.peek() {
                match t {
                    Tok::R => {
                        self.i += 1;
                        break;
                    }
                    Tok::Ch(c) => {
                        name.push(*c);
                        self.i += 1;
                    }
                    Tok::Cmd(s) => {
                        name.push_str(s);
                        self.i += 1;
                    }
                    _ => {
                        self.i += 1;
                    }
                }
            }
        }
        name
    }

    /// Read the raw tokens of an environment body (up to `\end{name}`).
    fn read_env_body(&mut self) -> Vec<Tok> {
        let mut body = Vec::new();
        let mut depth = 0usize;
        while let Some(t) = self.peek().cloned() {
            if let Tok::Cmd(c) = &t {
                if c == "begin" {
                    depth += 1;
                } else if c == "end" {
                    if depth == 0 {
                        self.i += 1;
                        let _ = self.read_env_name();
                        break;
                    }
                    depth -= 1;
                }
            }
            body.push(t);
            self.i += 1;
        }
        body
    }

    fn parse_matrix(&mut self, kind: MatrixKind, aligns: Vec<HAlign>) -> Ast {
        let body = self.read_env_body();
        let mut rows: Vec<Vec<Ast>> = Vec::new();
        let mut cells: Vec<Ast> = Vec::new();
        let mut chunk: Vec<Tok> = Vec::new();
        let flush_cell = |chunk: &mut Vec<Tok>, cells: &mut Vec<Ast>| {
            let mut p = Parser { toks: chunk, i: 0 };
            cells.push(p.parse_seq(Stop::None));
            chunk.clear();
        };
        for t in body {
            match t {
                Tok::Amp => flush_cell(&mut chunk, &mut cells),
                Tok::NewRow => {
                    flush_cell(&mut chunk, &mut cells);
                    rows.push(std::mem::take(&mut cells));
                }
                other => chunk.push(other),
            }
        }
        if !chunk.is_empty() || !cells.is_empty() {
            flush_cell(&mut chunk, &mut cells);
            rows.push(cells);
        }
        Ast::Matrix(rows, kind, aligns)
    }

    /// Read a `{name}` colour argument (also accepts a bare `red`).
    fn color_arg(&mut self) -> String {
        if matches!(self.peek(), Some(Tok::L)) {
            let inner = self.parse_group();
            if let Ast::Text(s) = inner {
                return s;
            }
            if let Ast::Row(items) = &inner {
                let mut out = String::new();
                for it in items {
                    match it {
                        Ast::Sym(s) | Ast::Text(s) => out.push_str(s),
                        _ => {}
                    }
                }
                return out;
            }
            String::new()
        } else {
            match self.eat() {
                Some(Tok::Ch(c)) => c.to_string(),
                Some(Tok::Cmd(c)) => c,
                _ => String::new(),
            }
        }
    }

    fn parse_delim(&mut self) -> String {
        match self.eat() {
            Some(Tok::Ch('.')) => "".into(),
            Some(Tok::Ch(c)) => c.to_string(),
            Some(Tok::Cmd(c)) => {
                if c == "langle" {
                    "⟨".into()
                } else if c == "rangle" {
                    "⟩".into()
                } else if c == "{" {
                    "{".into()
                } else if c == "}" {
                    "}".into()
                } else {
                    "".into()
                }
            }
            _ => "".into(),
        }
    }

    fn command(&mut self, name: &str) -> Ast {
        match name {
            "frac" | "dfrac" | "tfrac" => {
                let a = self.parse_group();
                let b = self.parse_group();
                Ast::Frac(Box::new(a), Box::new(b))
            }
            "sqrt" => {
                let mut index = None;
                if matches!(self.peek(), Some(Tok::LBracket)) {
                    self.i += 1;
                    let mut toks = Vec::new();
                    while let Some(t) = self.peek() {
                        if matches!(t, Tok::RBracket) {
                            self.i += 1;
                            break;
                        }
                        toks.push(t.clone());
                        self.i += 1;
                    }
                    let mut p = Parser { toks: &toks, i: 0 };
                    index = Some(Box::new(p.parse_seq(Stop::None)));
                }
                let body = self.parse_group();
                Ast::Sqrt(index, Box::new(body))
            }
            "text" | "textrm" | "mathrm" | "operatorname" => {
                let mut s = String::new();
                if matches!(self.eat(), Some(Tok::L)) {
                    while let Some(t) = self.eat() {
                        match t {
                            Tok::R => break,
                            Tok::Ch(c) => s.push(c),
                            Tok::Cmd(c) => s.push_str(&c),
                            _ => {}
                        }
                    }
                }
                Ast::Text(s)
            }
            "mathbb" | "mathcal" | "mathbf" | "mathit" | "boldsymbol" => self.parse_group(),
            "textcolor" | "colorbox" => {
                let name = self.color_arg();
                let body = self.parse_group();
                Ast::Color(latex_color(&name), Box::new(body))
            }
            "color" => {
                // Applies to the rest of the enclosing group.
                let name = self.color_arg();
                Ast::Color(latex_color(&name), Box::new(Ast::Row(Vec::new())))
            }
            "cancel" => Ast::Cancel(Box::new(self.parse_group()), false, false),
            "bcancel" => Ast::Cancel(Box::new(self.parse_group()), true, false),
            "xcancel" => Ast::Cancel(Box::new(self.parse_group()), true, true),
            "sout" => Ast::Strike(Box::new(self.parse_group())),
            "boxed" | "fbox" => Ast::Boxed(Box::new(self.parse_group())),
            "overline" | "bar" => Ast::Accent(Box::new(self.parse_group()), '-'),
            "hat" | "widehat" => Ast::Accent(Box::new(self.parse_group()), '^'),
            "vec" | "overrightarrow" => Ast::Accent(Box::new(self.parse_group()), '→'),
            "dot" => Ast::Accent(Box::new(self.parse_group()), '·'),
            "left" => {
                let left = self.parse_delim();
                let body = self.parse_seq(Stop::Right);
                let right = if matches!(self.peek(), Some(Tok::Cmd(c)) if c == "right") {
                    self.i += 1;
                    self.parse_delim()
                } else {
                    "".into()
                };
                Ast::Delim(left, Box::new(body), right)
            }
            "begin" => {
                let env = self.read_env_name();
                // `\begin{array}{lcr}` carries a column-alignment spec.
                let mut aligns: Vec<HAlign> = Vec::new();
                if env == "array" && matches!(self.peek(), Some(Tok::L)) {
                    self.i += 1;
                    while let Some(t) = self.peek().cloned() {
                        match t {
                            Tok::R => {
                                self.i += 1;
                                break;
                            }
                            Tok::Ch('l') => aligns.push(HAlign::Left),
                            Tok::Ch('c') => aligns.push(HAlign::Center),
                            Tok::Ch('r') => aligns.push(HAlign::Right),
                            _ => {}
                        }
                        self.i += 1;
                    }
                }
                let kind = match env.as_str() {
                    "pmatrix" => MatrixKind::Paren,
                    "bmatrix" => MatrixKind::Bracket,
                    "Bmatrix" => MatrixKind::Brace,
                    "vmatrix" => MatrixKind::Bar,
                    "aligned" | "align" | "align*" | "gather" | "gather*" | "split"
                    | "multline" | "multline*" => MatrixKind::Aligned,
                    "equation" | "equation*" => MatrixKind::Plain,
                    "array" => MatrixKind::Plain,
                    "cases" => MatrixKind::Cases,
                    _ => MatrixKind::Plain,
                };
                self.parse_matrix(kind, aligns)
            }
            "sum" | "prod" | "coprod" => Ast::BigOp(symbol(name).unwrap_or("∑").into(), None, None),
            "int" | "iint" | "iiint" | "oint" => {
                Ast::BigOp(symbol(name).unwrap_or("∫").into(), None, None)
            }
            "lim" | "max" | "min" | "sup" | "inf" => Ast::BigOp(name.to_string(), None, None),
            "," | ";" | ":" | "!" => Ast::Space(0.16),
            _ => match symbol(name) {
                Some(s) => Ast::Sym(s.to_string()),
                None => Ast::Sym(name.to_string()),
            },
        }
    }

    fn parse_atom(&mut self) -> Ast {
        match self.eat() {
            Some(Tok::Ch(c)) => Ast::Sym(c.to_string()),
            Some(Tok::L) => {
                let inner = self.parse_seq(Stop::Brace);
                if matches!(self.peek(), Some(Tok::R)) {
                    self.i += 1;
                }
                inner
            }
            Some(Tok::Cmd(c)) => self.command(&c),
            Some(Tok::LBracket) => Ast::Sym("[".into()),
            Some(Tok::RBracket) => Ast::Sym("]".into()),
            _ => Ast::Row(Vec::new()),
        }
    }
}

/// Map the common LaTeX/xcolor names to egui colours.
fn latex_color(name: &str) -> egui::Color32 {
    match name.trim().to_ascii_lowercase().as_str() {
        "red" => egui::Color32::from_rgb(0xe0, 0x5f, 0x5f),
        "orange" => egui::Color32::from_rgb(0xe0, 0x8c, 0x4f),
        "yellow" => egui::Color32::from_rgb(0xd8, 0xc0, 0x4f),
        "green" => egui::Color32::from_rgb(0x7d, 0xdc, 0x8a),
        "teal" | "cyan" => egui::Color32::from_rgb(0x5f, 0xc8, 0xc8),
        "blue" => egui::Color32::from_rgb(0x6a, 0xa8, 0xf0),
        "purple" | "violet" => egui::Color32::from_rgb(0xa8, 0x95, 0xff),
        "magenta" | "pink" => egui::Color32::from_rgb(0xd0, 0x8f, 0xc0),
        "brown" => egui::Color32::from_rgb(0xb0, 0x8a, 0x6a),
        "gray" | "grey" => egui::Color32::from_rgb(0x9a, 0x9a, 0x9a),
        "black" => egui::Color32::from_rgb(0x20, 0x20, 0x20),
        "white" => egui::Color32::from_rgb(0xf0, 0xf0, 0xf0),
        "darkblue" => egui::Color32::from_rgb(0x3a, 0x5a, 0xa8),
        "darkred" => egui::Color32::from_rgb(0xa8, 0x3a, 0x3a),
        "darkgreen" => egui::Color32::from_rgb(0x3a, 0x8a, 0x4a),
        _ => crate::theme::TEXT,
    }
}

fn parse_ast(src: &str) -> Ast {
    let toks = lex(src);
    let mut p = Parser { toks: &toks, i: 0 };
    p.parse_seq(Stop::None)
}

/// A laid-out math expression, in baseline-relative coordinates (`dy` grows
/// downward from the baseline).
pub struct MathLayout {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    items: Vec<MItem>,
}

enum MItem {
    Glyph {
        galley: Arc<egui::Galley>,
        x: f32,
        dy: f32,
        ascent: f32,
        /// Per-item colour override (`\textcolor{…}`); `None` = default.
        color: Option<egui::Color32>,
    },
    Rule {
        x: f32,
        dy: f32,
        w: f32,
        thick: f32,
        color: Option<egui::Color32>,
    },
    /// A decoration line drawn over a laid-out box (cancel / strike / frame).
    Line {
        p1: egui::Pos2,
        p2: egui::Pos2,
        thick: f32,
        color: egui::Color32,
    },
}

fn glyph(ui: &egui::Ui, text: &str, size: f32, italic: bool) -> (MItem, f32, f32) {
    glyph_colored(ui, text, size, italic, None)
}

fn glyph_colored(
    ui: &egui::Ui,
    text: &str,
    size: f32,
    italic: bool,
    color: Option<egui::Color32>,
) -> (MItem, f32, f32) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::text::TextFormat {
            font_id: egui::FontId::proportional(size),
            italics: italic,
            color: egui::Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    job.wrap.max_width = f32::INFINITY;
    let galley = ui.fonts(|f| f.layout_job(job));
    let w = galley.size().x;
    let h = galley.size().y;
    let asc = h * 0.78;
    let item = MItem::Glyph {
        galley,
        x: 0.0,
        dy: 0.0,
        ascent: asc,
        color,
    };
    (item, w, asc)
}

fn empty() -> MathLayout {
    MathLayout {
        width: 0.0,
        ascent: 0.0,
        descent: 0.0,
        items: Vec::new(),
    }
}

fn shift_item(item: &mut MItem, dx: f32, dy: f32) {
    match item {
        MItem::Glyph { x, dy: y, .. } => {
            *x += dx;
            *y += dy;
        }
        MItem::Rule { x, dy: y, .. } => {
            *x += dx;
            *y += dy;
        }
        MItem::Line { p1, p2, .. } => {
            p1.x += dx;
            p1.y += dy;
            p2.x += dx;
            p2.y += dy;
        }
    }
}

fn shift(l: &mut MathLayout, dx: f32, dy: f32) {
    for item in &mut l.items {
        match item {
            MItem::Glyph { x, dy: y, .. } => {
                *x += dx;
                *y += dy;
            }
            MItem::Rule { x, dy: y, .. } => {
                *x += dx;
                *y += dy;
            }
            MItem::Line { p1, p2, .. } => {
                p1.x += dx;
                p1.y += dy;
                p2.x += dx;
                p2.y += dy;
            }
        }
    }
    l.width += dx.max(0.0) * 0.0;
}

fn layout_glyph(ui: &egui::Ui, text: &str, size: f32, italic: bool) -> MathLayout {
    let (item, w, asc) = glyph(ui, text, size, italic);
    MathLayout {
        width: w,
        ascent: asc,
        descent: size * 0.24,
        items: vec![item],
    }
}

fn layout_ast(ui: &egui::Ui, ast: &Ast, size: f32, display: bool) -> MathLayout {
    match ast {
        Ast::Row(items) => {
            let mut out = empty();
            for item in items {
                let mut child = layout_ast(ui, item, size, display);
                let cw = child.width;
                shift(&mut child, out.width, 0.0);
                out.ascent = out.ascent.max(child.ascent);
                out.descent = out.descent.max(child.descent);
                out.width += cw;
                out.items.extend(child.items);
            }
            out
        }
        Ast::Sym(s) => {
            let italic = s.chars().count() == 1 && s.chars().next().unwrap().is_alphabetic();
            layout_glyph(ui, s, size, italic)
        }
        Ast::Text(s) => layout_glyph(ui, s, size, false),
        Ast::Space(f) => {
            let mut l = empty();
            l.width = size * f;
            l.descent = 0.0;
            l
        }
        Ast::Frac(num, den) => {
            let mut n = layout_ast(ui, num, size * 0.9, display);
            let mut d = layout_ast(ui, den, size * 0.9, display);
            let axis = size * 0.28;
            let gap = size * 0.12;
            let w = n.width.max(d.width) + size * 0.16;
            let nx = (w - n.width) / 2.0;
            let dx = (w - d.width) / 2.0;
            let nd = n.descent;
            shift(&mut n, nx, -(axis + gap + nd));
            let da = d.ascent;
            shift(&mut d, dx, -axis + gap + da);
            let mut out = empty();
            out.width = w;
            out.ascent = axis + gap + n.ascent + n.descent;
            out.descent = -axis + gap + d.ascent + d.descent;
            out.items.extend(n.items);
            out.items.extend(d.items);
            out.items.push(MItem::Rule {
                x: 0.0,
                dy: -axis,
                w,
                thick: (size * 0.05).max(1.0),
                color: None,
            });
            out
        }
        Ast::Sqrt(index, body) => {
            let mut b = layout_ast(ui, body, size, display);
            let (root_item, rw, rasc) = glyph(ui, "√", size * 1.25, false);
            let mut out = empty();
            let mut ix = 0.0;
            let mut iasc = 0.0;
            if let Some(idx) = index {
                let mut il = layout_ast(ui, idx, size * 0.6, false);
                shift(&mut il, 0.0, -size * 0.45);
                iasc = il.ascent + size * 0.45;
                ix = il.width;
                out.items.extend(il.items);
            }
            shift(&mut b, ix + rw, 0.0);
            out.items.push(root_item);
            // Overline above the radicand.
            out.items.push(MItem::Rule {
                x: ix + rw,
                dy: -b.ascent - size * 0.08,
                w: b.width,
                thick: (size * 0.04).max(1.0),
                color: None,
            });
            out.width = ix + rw + b.width;
            out.ascent = (b.ascent + size * 0.12).max(rasc).max(iasc);
            out.descent = b.descent;
            out.items.extend(b.items);
            out
        }
        Ast::Script(base, sup, sub) => {
            let b = layout_ast(ui, base, size, display);
            let ssize = size * 0.68;
            let mut sw: f32 = 0.0;
            let mut out = empty();
            let mut sup_top = 0.0f32;
            let mut sub_bottom = 0.0f32;
            out.ascent = b.ascent;
            out.descent = b.descent;
            if let Some(s) = sup {
                let mut sl = layout_ast(ui, s, ssize, false);
                shift(&mut sl, b.width, -size * 0.42);
                sup_top = size * 0.42 + sl.ascent;
                sw = sw.max(sl.width);
                out.items.extend(sl.items);
            }
            if let Some(s) = sub {
                let mut sl = layout_ast(ui, s, ssize, false);
                shift(&mut sl, b.width, size * 0.24);
                sub_bottom = size * 0.24 + sl.descent;
                sw = sw.max(sl.width);
                out.items.extend(sl.items);
            }
            out.items.extend(b.items);
            out.width = b.width + sw;
            out.ascent = out.ascent.max(sup_top);
            out.descent = out.descent.max(sub_bottom);
            out
        }
        Ast::BigOp(ch, sub, sup) => {
            let mut out = empty();
            let op_size = size * 1.35;
            let mut op = layout_glyph(ui, ch, op_size, false);
            if display {
                let mut total_w = op.width;
                let mut above: Option<MathLayout> = None;
                let mut below: Option<MathLayout> = None;
                if let Some(s) = sup {
                    let l = layout_ast(ui, s, size * 0.65, false);
                    total_w = total_w.max(l.width);
                    above = Some(l);
                }
                if let Some(s) = sub {
                    let l = layout_ast(ui, s, size * 0.65, false);
                    total_w = total_w.max(l.width);
                    below = Some(l);
                }
                let mut y = 0.0;
                if let Some(mut a) = above {
                    let aw = a.width;
                    let ad = a.descent;
                    shift(&mut a, (total_w - aw) / 2.0, -(op.ascent + size * 0.1 + ad));
                    out.ascent = out
                        .ascent
                        .max(op.ascent + size * 0.1 + a.ascent + a.descent);
                    y = 0.0;
                    out.items.extend(a.items);
                }
                let ow = op.width;
                shift(&mut op, (total_w - ow) / 2.0, y);
                out.ascent = out.ascent.max(op.ascent);
                out.descent = out.descent.max(op.descent);
                out.items.extend(op.items);
                if let Some(mut b) = below {
                    let bw = b.width;
                    let ba = b.ascent;
                    shift(&mut b, (total_w - bw) / 2.0, op.descent + size * 0.08 + ba);
                    out.descent = out
                        .descent
                        .max(op.descent + size * 0.08 + b.ascent + b.descent);
                    out.items.extend(b.items);
                }
                out.width = total_w;
            } else {
                // Inline: scripts go to the right, like a normal symbol.
                out = layout_ast(
                    ui,
                    &Ast::Script(Box::new(Ast::Sym(ch.clone())), sup.clone(), sub.clone()),
                    size,
                    display,
                );
                let _ = &mut op;
            }
            out
        }
        Ast::Delim(left, body, right) => {
            let mut b = layout_ast(ui, body, size, display);
            let height = (b.ascent + b.descent).max(size);
            let dsize = (height / 0.9).max(size);
            let mut out = empty();
            let mut x = 0.0;
            if !left.is_empty() {
                let mut l = layout_glyph(ui, left, dsize, false);
                let center = (b.descent - b.ascent) / 2.0;
                let la = l.ascent;
                shift(&mut l, 0.0, center + la * 0.22);
                x = l.width;
                out.ascent = out.ascent.max(l.ascent - (center + l.ascent * 0.22));
                out.descent = out.descent.max(l.descent + center + l.ascent * 0.22);
                out.items.extend(l.items);
            }
            shift(&mut b, x, 0.0);
            out.items.extend(b.items);
            let bw = b.width;
            out.ascent = out.ascent.max(b.ascent);
            out.descent = out.descent.max(b.descent);
            out.width = x + bw;
            if !right.is_empty() {
                let mut r = layout_glyph(ui, right, dsize, false);
                let ra = r.ascent;
                shift(&mut r, out.width, (b.descent - b.ascent) / 2.0 + ra * 0.22);
                out.width += r.width;
                out.items.extend(r.items);
            }
            out
        }
        Ast::Color(color, body) => {
            let mut l = layout_ast(ui, body, size, display);
            for item in &mut l.items {
                match item {
                    MItem::Glyph { color: c, .. } => *c = Some(*color),
                    MItem::Rule { color: c, .. } => *c = Some(*color),
                    MItem::Line { .. } => {}
                }
            }
            l
        }
        Ast::Cancel(body, back, both) => {
            let mut l = layout_ast(ui, body, size, display);
            let w = l.width;
            let a = l.ascent;
            let d = l.descent;
            let color = crate::theme::DANGER;
            let thick = (size * 0.06).max(1.0);
            if !back {
                l.items.push(MItem::Line {
                    p1: egui::pos2(-1.0, -a),
                    p2: egui::pos2(w + 1.0, d),
                    thick,
                    color,
                });
            }
            if *back || *both {
                l.items.push(MItem::Line {
                    p1: egui::pos2(-1.0, d),
                    p2: egui::pos2(w + 1.0, -a),
                    thick,
                    color,
                });
            }
            l
        }
        Ast::Strike(body) => {
            let mut l = layout_ast(ui, body, size, display);
            let w = l.width;
            let mid = (l.descent - l.ascent) / 2.0;
            l.items.push(MItem::Line {
                p1: egui::pos2(-1.0, mid),
                p2: egui::pos2(w + 1.0, mid),
                thick: (size * 0.05).max(1.0),
                color: crate::theme::DANGER,
            });
            l
        }
        Ast::Boxed(body) => {
            let mut l = layout_ast(ui, body, size, display);
            let pad = size * 0.18;
            let w = l.width;
            let a = l.ascent;
            let d = l.descent;
            let color = crate::theme::LINK;
            let thick = (size * 0.05).max(1.0);
            let rect = |x1: f32, y1: f32, x2: f32, y2: f32| MItem::Line {
                p1: egui::pos2(x1, y1),
                p2: egui::pos2(x2, y2),
                thick,
                color,
            };
            let (l0, r0) = (-pad, w + pad);
            let (t0, b0) = (-a - pad, d + pad);
            l.items.push(rect(l0, t0, r0, t0));
            l.items.push(rect(l0, b0, r0, b0));
            l.items.push(rect(l0, t0, l0, b0));
            l.items.push(rect(r0, t0, r0, b0));
            for item in &mut l.items {
                shift_item(item, pad, 0.0);
            }
            l.width = w + pad * 2.0;
            l.ascent = a + pad;
            l.descent = d + pad;
            l
        }
        Ast::Matrix(rows, kind, aligns) => layout_matrix(ui, rows, *kind, aligns, size, display),
        Ast::Accent(body, ch) => {
            let b = layout_ast(ui, body, size, display);
            let mut out = empty();
            let accent = if *ch == '-' {
                None
            } else {
                Some(layout_glyph(ui, &ch.to_string(), size * 0.8, false))
            };
            out.width = b.width;
            match accent {
                Some(mut a) => {
                    let ac = (b.width - a.width) / 2.0;
                    let aa = a.ascent;
                    let ad = a.descent;
                    shift(&mut a, ac, -(b.ascent + size * 0.16) + ad + aa * 0.2);
                    out.ascent = out.ascent.max(b.ascent + size * 0.32);
                    out.items.extend(a.items);
                }
                None => {
                    out.items.push(MItem::Rule {
                        x: 0.0,
                        dy: -b.ascent - size * 0.08,
                        w: b.width,
                        thick: (size * 0.05).max(1.0),
                        color: None,
                    });
                    out.ascent = out.ascent.max(b.ascent + size * 0.14);
                }
            }
            out.items.extend(b.items);
            out.descent = b.descent;
            out
        }
    }
}

fn layout_matrix(
    ui: &egui::Ui,
    rows: &[Vec<Ast>],
    kind: MatrixKind,
    aligns: &[HAlign],
    size: f32,
    display: bool,
) -> MathLayout {
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(0).max(1);
    let mut cells: Vec<Vec<MathLayout>> = Vec::new();
    for row in rows {
        let mut line = Vec::new();
        for c in 0..cols {
            match row.get(c) {
                Some(ast) => line.push(layout_ast(ui, ast, size, display)),
                None => line.push(empty()),
            }
        }
        cells.push(line);
    }
    let col_gap = size * 0.7;
    let row_gap = size * 0.35;
    let mut widths = vec![0.0f32; cols];
    for line in &cells {
        for (c, cell) in line.iter().enumerate() {
            widths[c] = widths[c].max(cell.width);
        }
    }
    let mut out = empty();
    let mut y = 0.0f32;
    for line in cells.iter_mut() {
        let asc = line.iter().fold(0.0f32, |m, c| m.max(c.ascent));
        let desc = line.iter().fold(0.0f32, |m, c| m.max(c.descent));
        let mut x = 0.0f32;
        for (c, cell) in line.iter_mut().enumerate() {
            let align = if matches!(kind, MatrixKind::Aligned | MatrixKind::Cases) {
                HAlign::Left
            } else {
                aligns.get(c).copied().unwrap_or(HAlign::Center)
            };
            let cx = match align {
                HAlign::Left => x,
                HAlign::Center => x + (widths[c] - cell.width) / 2.0,
                HAlign::Right => x + widths[c] - cell.width,
            };
            // Align at the baseline of this row.
            let cw = cell.width;
            let ca = cell.ascent;
            shift(cell, cx, y + asc - ca);
            let _ = cw;
            out.items.append(&mut cell.items);
            x += widths[c] + col_gap;
        }
        out.width = out.width.max(x - col_gap);
        out.ascent = out.ascent.max(asc + y);
        out.descent = out.descent.max(desc - y);
        y += asc + desc + row_gap;
    }
    out.descent = (y - row_gap - out.ascent).max(out.descent);
    let h = out.width;

    // Brackets / braces around the matrix.
    let (left, right) = match kind {
        MatrixKind::Paren => ("(", ")"),
        MatrixKind::Bracket => ("[", "]"),
        MatrixKind::Brace => ("{", "}"),
        MatrixKind::Bar => ("|", "|"),
        MatrixKind::Cases => ("{", ""),
        _ => ("", ""),
    };
    let total_h = (out.ascent + out.descent).max(size);
    if !left.is_empty() || !right.is_empty() {
        let dsize = (total_h / 0.85).max(size);
        let mut wrapped = empty();
        let mut x = 0.0;
        if !left.is_empty() {
            let mut l = layout_glyph(ui, left, dsize, false);
            let center = (out.descent - out.ascent) / 2.0;
            shift(&mut l, 0.0, center);
            x = l.width;
            wrapped.items.extend(l.items);
        }
        shift(&mut out, x, 0.0);
        wrapped.items.extend(out.items);
        wrapped.width = x + out.width;
        wrapped.ascent = total_h / 2.0 + size * 0.06;
        wrapped.descent = total_h / 2.0 + size * 0.06;
        if !right.is_empty() {
            let mut r = layout_glyph(ui, right, dsize, false);
            shift(&mut r, wrapped.width, (out.descent - out.ascent) / 2.0);
            wrapped.width += r.width;
            wrapped.items.extend(r.items);
        }
        let _ = h;
        wrapped
    } else {
        out
    }
}

/// Lay out a LaTeX expression for drawing at `size`.
pub fn layout_math(ui: &egui::Ui, tex: &str, size: f32, display: bool) -> MathLayout {
    let ast = parse_ast(tex);
    let mut l = layout_ast(ui, &ast, size, display);
    if l.ascent + l.descent < size * 0.5 {
        l.ascent = size * 0.75;
        l.descent = size * 0.25;
    }
    l
}

/// Paint a laid-out expression into `rect` (top-left anchored).
pub fn paint_math(
    painter: &egui::Painter,
    rect: egui::Rect,
    layout: &MathLayout,
    color: egui::Color32,
) {
    let baseline = rect.min.y + layout.ascent;
    for item in &layout.items {
        match item {
            MItem::Glyph {
                galley,
                x,
                dy,
                ascent,
                color: item_color,
            } => {
                let pos = egui::pos2(rect.min.x + x, baseline + dy - ascent);
                painter.galley(pos, galley.clone(), item_color.unwrap_or(color));
            }
            MItem::Rule {
                x,
                dy,
                w,
                thick,
                color: item_color,
            } => {
                let y = baseline + dy;
                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + x, y - thick / 2.0),
                        egui::vec2(*w, *thick),
                    ),
                    0.0,
                    item_color.unwrap_or(color),
                );
            }
            MItem::Line {
                p1,
                p2,
                thick,
                color: line_color,
            } => {
                painter.line_segment(
                    [
                        egui::pos2(rect.min.x + p1.x, baseline + p1.y),
                        egui::pos2(rect.min.x + p2.x, baseline + p2.y),
                    ],
                    egui::Stroke::new(*thick, *line_color),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fractions_scripts_and_matrices() {
        assert!(matches!(parse_ast(r"\frac{a}{b}"), Ast::Frac(..)));
        assert!(matches!(parse_ast("x^2"), Ast::Script(..)));
        assert!(matches!(parse_ast(r"\sum_{i=1}^{n} i"), Ast::Row(_)));
        assert!(matches!(
            parse_ast(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}"),
            Ast::Matrix(..)
        ));
        // Malformed input must not panic.
        let _ = parse_ast(r"\frac{");
        let _ = parse_ast(r"\left( x");
    }

    #[test]
    fn parses_colors_cancel_and_array() {
        assert!(matches!(parse_ast(r"\textcolor{red}{x}"), Ast::Color(..)));
        assert!(matches!(parse_ast(r"\cancel{x}"), Ast::Cancel(..)));
        assert!(matches!(parse_ast(r"\bcancel{x}"), Ast::Cancel(..)));
        assert!(matches!(parse_ast(r"\xcancel{x}"), Ast::Cancel(..)));
        assert!(matches!(parse_ast(r"\sout{x}"), Ast::Strike(..)));
        assert!(matches!(parse_ast(r"\boxed{x}"), Ast::Boxed(..)));
        match parse_ast(r"\begin{array}{lcr} a & b & c \\ d & e & f \end{array}") {
            Ast::Matrix(_, _, aligns) => {
                assert_eq!(aligns, vec![HAlign::Left, HAlign::Center, HAlign::Right]);
            }
            other => panic!("expected a matrix, got {other:?}"),
        }
        // `\color{blue}` wraps the rest of its group.
        match parse_ast(r"a + \color{blue} b + c") {
            Ast::Row(items) => assert!(items.iter().any(|i| matches!(i, Ast::Color(..)))),
            other => panic!("expected a row, got {other:?}"),
        }
    }

    #[test]
    fn latex_colors_are_known() {
        assert_ne!(latex_color("red"), crate::theme::TEXT);
        assert_eq!(latex_color("not-a-colour"), crate::theme::TEXT);
    }

    #[test]
    fn converts_greek_and_operators() {
        assert_eq!(latex_to_unicode(r"\alpha + \beta"), "α + β");
        assert_eq!(latex_to_unicode(r"a \leq b"), "a ≤ b");
        assert_eq!(latex_to_unicode(r"x \times y"), "x × y");
    }

    #[test]
    fn converts_superscripts_and_subscripts() {
        assert_eq!(latex_to_unicode("x^2"), "x²");
        assert_eq!(latex_to_unicode("x^{10}"), "x¹⁰");
        assert_eq!(latex_to_unicode("a_1"), "a₁");
    }

    #[test]
    fn unknown_superscript_is_kept() {
        // 'q' has no unicode superscript: fall back to ^q.
        assert_eq!(latex_to_unicode("y^q"), "y^q");
        assert_eq!(latex_to_unicode("y^{qq}"), "y^(qq)");
    }

    #[test]
    fn keeps_unknown_control_words() {
        assert_eq!(latex_to_unicode(r"\foo bar"), "foo bar");
    }

    #[test]
    fn sqrt_and_braces() {
        assert_eq!(latex_to_unicode(r"\sqrt{x}"), "√x");
        assert_eq!(latex_to_unicode(r"{a+b}"), "a+b");
    }

    #[test]
    fn escaped_chars() {
        assert_eq!(latex_to_unicode(r"100\%"), "100%");
    }
}
