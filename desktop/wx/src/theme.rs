use wxdragon::prelude::*;

// Minimal native-wx theming.
//
// Goal: keep the UI mostly light/system-native, and use dark gray only for trim
// (top/bottom bars and small accents). Use green/yellow for clear status signaling.

#[allow(dead_code)]
pub struct Palette;

#[allow(dead_code)]
impl Palette {
    // Trim.
    pub const TRIM_BG: Colour = Colour::rgb(0x2b, 0x2f, 0x36); // dark gray
    pub const TRIM_FG: Colour = Colour::rgb(0xf2, 0xf4, 0xf7); // near-white

    // Surfaces.
    pub const SURFACE_BG: Colour = Colour::rgb(0xf2, 0xf4, 0xf7); // light gray
    pub const CONTENT_BG: Colour = Colour::rgb(0xff, 0xff, 0xff); // white

    // Text.
    pub const TEXT_PRIMARY: Colour = Colour::rgb(0x11, 0x18, 0x27); // near-black
    pub const TEXT_SECONDARY: Colour = Colour::rgb(0x4b, 0x55, 0x63); // gray

    // Accents.
    pub const ACCENT_GREEN: Colour = Colour::rgb(0x16, 0xa3, 0x4a); // green
    pub const ACCENT_YELLOW: Colour = Colour::rgb(0xf5, 0x9e, 0x0b); // amber
    pub const ACCENT_RED: Colour = Colour::rgb(0xdc, 0x26, 0x26); // red
}

#[derive(Clone, Copy, Debug)]
pub enum StatusTone {
    Ready,
    Working,
    Error,
}

pub fn apply_surface(widget: &impl WxWidget) {
    widget.set_background_color(Palette::SURFACE_BG);
    widget.set_background_style(BackgroundStyle::Colour);
}

pub fn apply_trim(widget: &impl WxWidget) {
    widget.set_background_color(Palette::TRIM_BG);
    widget.set_background_style(BackgroundStyle::Colour);
    widget.set_foreground_color(Palette::TRIM_FG);
}

pub fn apply_content_text(ctrl: &TextCtrl, monospace: bool) {
    ctrl.set_background_color(Palette::CONTENT_BG);
    ctrl.set_background_style(BackgroundStyle::Colour);
    ctrl.set_foreground_color(Palette::TEXT_PRIMARY);

    if monospace {
        if let Some(font) = FontBuilder::default()
            .with_point_size(10)
            .with_family(FontFamily::Modern)
            .build()
        {
            ctrl.set_font(&font);
        }
    }
}

pub fn apply_content_dataview(ctrl: &DataViewCtrl) {
    ctrl.set_background_color(Palette::CONTENT_BG);
    ctrl.set_background_style(BackgroundStyle::Colour);
    ctrl.set_foreground_color(Palette::TEXT_PRIMARY);
}

pub fn set_status_tone(
    progress_text: &StaticText,
    status_pill: &Panel,
    progress_gauge: &Gauge,
    tone: StatusTone,
) {
    let accent = match tone {
        StatusTone::Ready => Palette::ACCENT_GREEN,
        StatusTone::Working => Palette::ACCENT_YELLOW,
        StatusTone::Error => Palette::ACCENT_RED,
    };

    progress_text.set_foreground_color(accent);
    status_pill.set_background_color(accent);
    status_pill.set_background_style(BackgroundStyle::Colour);
    status_pill.refresh(true, None);

    // Not all platforms respect these colours on native gauges, but when they do,
    // it makes the accent much more visible.
    progress_gauge.set_foreground_color(accent);
    progress_gauge.refresh(true, None);
}
