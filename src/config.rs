//! Persisted application configuration.
//!
//! Stored as JSON under `$XDG_CONFIG_HOME/mahzen/config.json`
//! (falling back to `~/.config/mahzen/config.json`). We deliberately
//! avoid extra crates such as `directories` to keep the dependency tree small.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Base directory for app config/state.
///
/// The app was originally called *Encrypted Note* and stored its config in
/// `…/encrypted-note/`. On first launch after the rename we move that directory
/// to `…/mahzen/` so existing settings survive.
pub fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    let new = base.join("mahzen");
    let old = base.join("encrypted-note");
    if !new.exists() && old.is_dir() {
        let _ = fs::rename(&old, &new);
    }
    new
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    /// The vault that was open last time the app ran.
    pub last_vault: Option<PathBuf>,
    /// Most recently opened vaults, newest first.
    #[serde(default)]
    pub recent_vaults: Vec<PathBuf>,
    /// Whether the live markdown preview is shown next to the editor.
    #[serde(default = "default_true")]
    pub show_preview: bool,
    /// Editor font size in points.
    #[serde(default = "default_font_size")]
    pub font_size: f32,

    /// Folder (relative to a vault root) for daily notes.
    #[serde(default = "default_daily")]
    pub daily_dir: String,
    /// Moment-style date format for daily note file names.
    #[serde(default = "default_daily_format")]
    pub daily_format: String,
    /// Optional template note (title or vault-relative path) for new daily notes.
    #[serde(default)]
    pub daily_template: String,
    /// Folder (relative to a vault root) holding note templates.
    #[serde(default = "default_templates")]
    pub templates_dir: String,
    /// Whether the Vim editing mode is enabled.
    #[serde(default)]
    pub vim: bool,
    /// Render markdown syntax concealed while editing (WYSIWYG-style).
    #[serde(default = "default_true")]
    pub wysiwyg: bool,
    /// Underline misspelled words in the editor.
    #[serde(default)]
    pub spellcheck: bool,
    /// UI language: `"en"` or `"tr"`.
    #[serde(default = "default_lang")]
    pub language: String,
    /// Startup window mode: `"fullscreen"`, `"maximized"`, or `"windowed"`.
    #[serde(default = "default_window_mode")]
    pub window_mode: String,
    /// Starred ("bookmarked") notes.
    #[serde(default)]
    pub bookmarks: Vec<std::path::PathBuf>,
    /// Whether the second pane follows links clicked in the first pane.
    #[serde(default)]
    pub linked_pane: bool,
    /// Show tabs as vertical, letter-stacked strips (sliding-panes look).
    #[serde(default = "default_true")]
    pub vertical_tabs: bool,
    /// Sort the tag pane by note count instead of name.
    #[serde(default)]
    pub tag_sort_count: bool,
    /// Poll the vault for changes made by other programs.
    #[serde(default = "default_true")]
    pub watch_external: bool,
    /// Stack tabs as full-width rows (Obsidian's stacked tabs).
    #[serde(default)]
    pub stacked_tabs: bool,
    /// Share of the editor area taken by the right-hand split pane (0.05–0.95).
    #[serde(default = "default_split_frac")]
    pub split_frac: f32,
    /// Graph colour groups: a tag prefix → palette index (0..7).
    #[serde(default)]
    pub graph_color_groups: Vec<(String, usize)>,
    /// Graph force layout: repulsion multiplier.
    #[serde(default = "default_force")]
    pub graph_repulsion: f32,
    /// Graph force layout: spring/attraction multiplier.
    #[serde(default = "default_force")]
    pub graph_attraction: f32,

    // --- Workspace state (restored on next launch) ---
    /// Paths of the notes that were open, in order.
    #[serde(default)]
    pub open_tabs: Vec<PathBuf>,
    /// Index of the active tab.
    #[serde(default)]
    pub active_tab: usize,
    /// Whether the right-hand panel was visible.
    #[serde(default = "default_true")]
    pub show_right: bool,
}

fn default_true() -> bool {
    true
}

fn default_font_size() -> f32 {
    15.0
}

fn default_daily() -> String {
    "Daily".to_string()
}

fn default_daily_format() -> String {
    "YYYY-MM-DD".to_string()
}

fn default_templates() -> String {
    "Templates".to_string()
}

fn default_window_mode() -> String {
    "fullscreen".to_string()
}

fn default_force() -> f32 {
    1.0
}

fn default_split_frac() -> f32 {
    0.45
}

fn default_lang() -> String {
    "en".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            last_vault: None,
            recent_vaults: Vec::new(),
            show_preview: true,
            font_size: default_font_size(),
            daily_dir: default_daily(),
            daily_format: default_daily_format(),
            daily_template: String::new(),
            templates_dir: default_templates(),
            vim: false,
            wysiwyg: true,
            spellcheck: false,
            language: default_lang(),
            window_mode: default_window_mode(),
            bookmarks: Vec::new(),
            linked_pane: false,
            vertical_tabs: true,
            tag_sort_count: false,
            watch_external: true,
            stacked_tabs: false,
            split_frac: default_split_frac(),
            graph_color_groups: Vec::new(),
            graph_repulsion: 1.0,
            graph_attraction: 1.0,
            open_tabs: Vec::new(),
            active_tab: 0,
            show_right: true,
        }
    }
}

impl Config {
    /// Absolute path to the config file.
    pub fn file_path() -> PathBuf {
        dir().join("config.json")
    }

    /// Load the config from disk, returning defaults if it is missing or malformed.
    pub fn load() -> Self {
        let path = Self::file_path();
        match fs::read_to_string(&path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    /// Persist the config to disk, creating the directory if needed.
    pub fn save(&self) {
        let path = Self::file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    /// Record a vault as the most recent one.
    pub fn remember_vault(&mut self, path: &std::path::Path) {
        self.recent_vaults.retain(|p| p != path);
        self.recent_vaults.insert(0, path.to_path_buf());
        self.recent_vaults.truncate(10);
        self.last_vault = Some(path.to_path_buf());
    }
}
