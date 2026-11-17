//! The app-wide visual theme.
//!
//! The palette and layout follow Obsidian running the **LYT Mode** theme
//! (Cecilia May's "Cybertron" evolution): deep blue-black sidebars, a lighter
//! blue-grey editor surface, periwinkle-blue accents, colored headings, a
//! yellow text highlight and soft, low-contrast borders. Panels are flat with
//! generous padding; the ribbon, tab bar and status bar are darker than the
//! editor so the writing surface reads as the foreground.

use eframe::egui::{self, Color32};

/// Editor surface (the lightest of the dark backgrounds).
pub const BASE: Color32 = Color32::from_rgb(0x2f, 0x30, 0x3a);
/// Sidebars / panels.
pub const PANEL: Color32 = Color32::from_rgb(0x1e, 0x1f, 0x24);
/// Ribbon, top bar and status bar.
pub const PANEL_DEEP: Color32 = Color32::from_rgb(0x1a, 0x1b, 0x21);
/// Dialogs and popovers.
pub const WINDOW_FILL: Color32 = Color32::from_rgb(0x26, 0x27, 0x31);
/// Inset blocks: code, inputs, cards.
pub const CODE_BG: Color32 = Color32::from_rgb(0x25, 0x26, 0x30);
pub const WIDGET: Color32 = Color32::from_rgb(0x33, 0x34, 0x3f);
pub const WIDGET_HOVER: Color32 = Color32::from_rgb(0x3e, 0x40, 0x50);
pub const WIDGET_ACTIVE: Color32 = Color32::from_rgb(0x4a, 0x4c, 0x60);
/// Primary accent: periwinkle blue.
pub const ACCENT: Color32 = Color32::from_rgb(0x7c, 0x9c, 0xe0);
/// Second accent used for tags and secondary highlights.
pub const ACCENT_2: Color32 = Color32::from_rgb(0x8d, 0x86, 0xd8);
/// Softer accent for selections / fills.
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x54, 0x58, 0x7d);
/// Neutral fill for primary buttons.
pub const PRIMARY: Color32 = Color32::from_rgb(0x3a, 0x3c, 0x48);
pub const PRIMARY_HOVER: Color32 = Color32::from_rgb(0x45, 0x47, 0x56);
pub const TEXT: Color32 = Color32::from_rgb(0xdc, 0xdd, 0xe3);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x9a, 0x9d, 0xae);
pub const LINK: Color32 = Color32::from_rgb(0x7f, 0xa6, 0xe8);
pub const BORDER: Color32 = Color32::from_rgb(0x3a, 0x3b, 0x47);
pub const DANGER: Color32 = Color32::from_rgb(0xd9, 0x6a, 0x6a);
pub const SUCCESS: Color32 = Color32::from_rgb(0x7b, 0xd9, 0xa5);
/// LYT's signature gold used for bullets, the active pane edge and list markers.
pub const GOLD: Color32 = Color32::from_rgb(0xd8, 0xc6, 0x5a);
/// Warm orange for unresolved links.
pub const ORANGE: Color32 = Color32::from_rgb(0xe0, 0x9a, 0x62);
/// Text highlight (`==…==`): LYT's signature warm yellow.
pub const HIGHLIGHT: Color32 = Color32::from_rgb(0xf2, 0xf0, 0x7e);
pub const HIGHLIGHT_TEXT: Color32 = Color32::from_rgb(0x3a, 0x36, 0x14);

/// Colored heading ramp (H1 → H6), like LYT Mode's rainbow headings.
pub const HEADINGS: [Color32; 6] = [
    Color32::from_rgb(0xc0, 0xa8, 0xf0), // lavender
    Color32::from_rgb(0xe6, 0xe0, 0x7a), // gold
    Color32::from_rgb(0x9d, 0xc9, 0xb0), // sage
    Color32::from_rgb(0xa9, 0xc4, 0xf5), // periwinkle
    Color32::from_rgb(0xd0, 0x9a, 0xc0), // rose
    Color32::from_rgb(0xd8, 0xbf, 0x94), // sand
];

/// The heading colour for a markdown level (1-based).
pub fn heading_color(level: usize) -> Color32 {
    HEADINGS[(level.saturating_sub(1)).min(HEADINGS.len() - 1)]
}

/// Install fonts, spacing, and colours.
pub fn apply(ctx: &egui::Context, font_size: f32) {
    let size = font_size;
    let mut style = (*ctx.style()).clone();

    // Typography — a slightly larger body and roomier line spacing.
    use egui::TextStyle::*;
    style
        .text_styles
        .insert(Body, egui::FontId::proportional(size + 1.0));
    style
        .text_styles
        .insert(Monospace, egui::FontId::monospace(size));
    style
        .text_styles
        .insert(Button, egui::FontId::proportional(size));
    style
        .text_styles
        .insert(Small, egui::FontId::proportional(size - 3.0));
    style
        .text_styles
        .insert(Heading, egui::FontId::proportional(size + 7.0));

    // Spacing: flat panels, airy lists.
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.interact_size.y = 25.0;
    style.spacing.window_margin = egui::Margin::same(14.0);
    style.spacing.indent = 16.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = false;

    // Colours.
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = WINDOW_FILL;
    v.extreme_bg_color = CODE_BG;
    v.faint_bg_color = CODE_BG;
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = LINK;
    v.warn_fg_color = Color32::from_rgb(0xe8, 0xd0, 0x9a);
    v.error_fg_color = DANGER;
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.window_rounding = egui::Rounding::same(7.0);
    v.menu_rounding = egui::Rounding::same(6.0);
    v.window_stroke = egui::Stroke::new(1.0_f32, BORDER);
    let shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 20.0,
        spread: 0.0,
        color: Color32::from_black_alpha(150),
    };
    v.window_shadow = shadow;
    v.popup_shadow = shadow;

    let r = egui::Rounding::same(4.0);
    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, BORDER);
    w.noninteractive.rounding = r;
    w.noninteractive.fg_stroke.color = TEXT;

    w.inactive.bg_fill = WIDGET;
    w.inactive.weak_bg_fill = WIDGET;
    w.inactive.bg_stroke = egui::Stroke::NONE;
    w.inactive.rounding = r;

    w.hovered.bg_fill = WIDGET_HOVER;
    w.hovered.weak_bg_fill = WIDGET_HOVER;
    w.hovered.bg_stroke = egui::Stroke::new(1.0_f32, BORDER);
    w.hovered.rounding = r;

    w.active.bg_fill = WIDGET_ACTIVE;
    w.active.weak_bg_fill = WIDGET_ACTIVE;
    w.active.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT.linear_multiply(0.6));
    w.active.rounding = r;

    w.open.bg_fill = WIDGET_ACTIVE;
    w.open.weak_bg_fill = WIDGET_ACTIVE;
    w.open.rounding = r;

    style.visuals = v;
    ctx.set_style(style);
}

/// A small uppercase section label used by the side panels.
pub fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .size(10.0)
            .strong()
            .color(TEXT_MUTED),
    );
}

/// A rounded tag pill.
pub fn tag_pill(ui: &mut egui::Ui, text: &str, active: bool) -> egui::Response {
    let fill = if active {
        ACCENT.linear_multiply(0.30)
    } else {
        ACCENT_2.linear_multiply(0.16)
    };
    let color = if active { TEXT } else { ACCENT_2 };
    egui::Frame::none()
        .fill(fill)
        .rounding(egui::Rounding::same(9.0))
        .inner_margin(egui::Margin::symmetric(7.0, 2.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(11.0).color(color));
        })
        .response
}
