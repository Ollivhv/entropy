//! Offline preview of the Qube dongle screen (280×240 landscape).
//!
//! The frame is painted from the settings read from the device, so changing a
//! switch, a selector or a colour immediately shows on the preview - the same
//! relationship the macropad display page has with its own preview. Only shapes
//! and text are used: the firmware draws the same primitives (rectangles, bars
//! and three bitmap fonts), so the preview stays honest about what the screen
//! can express.

use super::*;

const SCREEN_WIDTH: f32 = 280.0;
const SCREEN_HEIGHT: f32 = 240.0;
/// WPM shown in the preview. The dongle computes the real value from typing;
/// the preview only has to place it.
pub(crate) const PREVIEW_WPM: u16 = 68;
/// Full-scale value of the speed bars, as used by the firmware concepts.
const WPM_SCALE_MAX: f32 = 150.0;

/// A whole-screen palette. Themes are only a shortcut: every channel is written
/// to the same QSIDs the individual colour rows use, so after picking a theme
/// the user can keep tuning single colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QubeThemePalette {
    /// i18n key of the theme name.
    pub(crate) label_key: &'static str,
    pub(crate) accent: [u8; 3],
    pub(crate) accent_shadow: [u8; 3],
    pub(crate) background: [u8; 3],
    pub(crate) panel: [u8; 3],
    pub(crate) borders: [u8; 3],
    pub(crate) text: [u8; 3],
    pub(crate) labels: [u8; 3],
    pub(crate) bar: [u8; 3],
    pub(crate) warning: [u8; 3],
    pub(crate) critical: [u8; 3],
}

impl QubeThemePalette {
    fn triples(&self) -> [([u16; 3], [u8; 3]); 10] {
        [
            ([320, 321, 322], self.accent),
            ([224, 225, 226], self.accent_shadow),
            ([330, 331, 332], self.background),
            ([230, 231, 232], self.panel),
            ([233, 234, 235], self.borders),
            ([236, 237, 238], self.text),
            ([239, 240, 241], self.labels),
            ([242, 243, 244], self.bar),
            ([245, 246, 247], self.warning),
            ([248, 249, 250], self.critical),
        ]
    }

    /// Writes this palette performs, skipping any colour the firmware does not
    /// announce - a theme must never touch a QSID the device did not declare.
    pub(crate) fn writes(&self, supported_qmk_settings: &[u16]) -> Vec<(u16, u8)> {
        let mut writes = Vec::new();
        for (qsids, channels) in self.triples() {
            if !qsids.iter().all(|qsid| supported_qmk_settings.contains(qsid)) {
                continue;
            }
            for (qsid, channel) in qsids.iter().zip(channels) {
                writes.push((*qsid, channel));
            }
        }
        writes
    }

    /// True when the device values already show this palette.
    pub(crate) fn matches(&self, values: &std::collections::BTreeMap<u16, u16>) -> bool {
        let mut compared = 0;
        for (qsids, channels) in self.triples() {
            let Some(current): Option<[u16; 3]> = qsids
                .iter()
                .map(|qsid| values.get(qsid).copied())
                .collect::<Option<Vec<_>>>()
                .and_then(|current| current.try_into().ok())
            else {
                continue;
            };
            if current
                .iter()
                .zip(channels)
                .any(|(value, channel)| *value != u16::from(channel))
            {
                return false;
            }
            compared += 1;
        }
        compared > 0
    }
}

/// Dark is the palette the firmware ships with.
pub(crate) const QUBE_THEME_DARK: QubeThemePalette = QubeThemePalette {
    label_key: "qube_screen.theme_dark",
    accent: [24, 154, 255],
    accent_shadow: [8, 65, 148],
    background: [0, 8, 33],
    panel: [0, 18, 52],
    borders: [30, 70, 120],
    text: [255, 255, 255],
    labels: [150, 170, 200],
    bar: [24, 154, 255],
    warning: [255, 200, 40],
    critical: [230, 60, 60],
};

/// Light: paper background, dark ink, muted frames. Every foreground colour is
/// dark enough for the light background *and* for the white panels (see the
/// contrast test), so no concept ends up with light text on a light surface.
pub(crate) const QUBE_THEME_LIGHT: QubeThemePalette = QubeThemePalette {
    label_key: "qube_screen.theme_light",
    accent: [12, 82, 168],
    accent_shadow: [186, 198, 216],
    background: [242, 245, 249],
    panel: [255, 255, 255],
    borders: [178, 186, 198],
    text: [22, 26, 34],
    labels: [82, 92, 108],
    bar: [22, 104, 190],
    warning: [176, 120, 0],
    critical: [186, 40, 40],
};

pub(crate) const QUBE_THEME_PHOSPHOR: QubeThemePalette = QubeThemePalette {
    label_key: "qube_screen.theme_phosphor",
    accent: [120, 255, 150],
    accent_shadow: [20, 80, 40],
    background: [0, 0, 0],
    panel: [0, 18, 6],
    borders: [40, 120, 60],
    text: [150, 255, 170],
    labels: [70, 190, 100],
    bar: [80, 255, 120],
    warning: [225, 225, 90],
    critical: [255, 90, 90],
};

pub(crate) const QUBE_THEME_AMBER: QubeThemePalette = QubeThemePalette {
    label_key: "qube_screen.theme_amber",
    accent: [255, 190, 80],
    accent_shadow: [90, 60, 10],
    background: [12, 8, 0],
    panel: [30, 20, 2],
    borders: [140, 100, 20],
    text: [255, 200, 110],
    labels: [200, 150, 60],
    bar: [255, 170, 40],
    warning: [255, 225, 100],
    critical: [255, 95, 60],
};

pub(crate) const QUBE_THEMES: [QubeThemePalette; 4] = [
    QUBE_THEME_DARK,
    QUBE_THEME_LIGHT,
    QUBE_THEME_PHOSPHOR,
    QUBE_THEME_AMBER,
];

/// Everything the preview needs, resolved from the device values.
#[derive(Clone, Debug)]
pub(crate) struct QubePreviewData {
    pub(crate) concept: u8,
    pub(crate) show_wpm: bool,
    pub(crate) show_modifiers: bool,
    pub(crate) show_batteries: bool,
    pub(crate) show_connection: bool,
    /// 0 media, 1 clock, 2 media + clock.
    pub(crate) header_mode: u8,
    /// 0 badge in the header, 1 badge as a chip.
    pub(crate) badge_place: u8,
    pub(crate) accent: Color32,
    pub(crate) accent_shadow: Color32,
    pub(crate) background: Color32,
    pub(crate) panel: Color32,
    pub(crate) borders: Color32,
    pub(crate) text: Color32,
    pub(crate) labels: Color32,
    pub(crate) bar: Color32,
    pub(crate) warning: Color32,
    pub(crate) critical: Color32,
    pub(crate) layer_name: String,
    pub(crate) layer_index: usize,
    pub(crate) battery_left: u8,
    pub(crate) battery_right: u8,
    pub(crate) bluetooth: bool,
}

impl QubePreviewData {
    fn connection_label(&self) -> &'static str {
        if self.bluetooth {
            "BT2*"
        } else {
            "USB"
        }
    }

    fn layer_label(&self) -> String {
        let name = self.layer_name.trim();
        if name.is_empty() {
            self.layer_index.to_string()
        } else {
            name.to_owned()
        }
    }

    fn media_ticker(&self) -> &'static str {
        "Boards of Canada - Roygbiv"
    }
}

/// Preview on the left, settings on the right when the window is wide enough;
/// stacked (preview above) otherwise. Mirrors the macropad display page.
pub(crate) fn qube_screen_panel_rects(
    body: egui::Rect,
    settings_width: f32,
) -> (egui::Rect, egui::Rect, bool) {
    let gap = 18.0;
    let side_by_side = body.width() >= 780.0_f32.max(settings_width + gap + 240.0);
    if side_by_side {
        let preview_width = (body.width() - settings_width - gap).clamp(240.0, 320.0);
        let group_width = preview_width + gap + settings_width;
        let group_left = body.center().x - group_width / 2.0;
        return (
            egui::Rect::from_min_size(
                egui::pos2(group_left, body.top()),
                egui::vec2(preview_width, body.height()),
            ),
            egui::Rect::from_min_size(
                egui::pos2(group_left + preview_width + gap, body.top()),
                egui::vec2(settings_width, body.height()),
            ),
            true,
        );
    }

    let preview_height = (body.height() * 0.46).clamp(210.0, 300.0);
    (
        egui::Rect::from_min_size(body.min, egui::vec2(body.width(), preview_height)),
        egui::Rect::from_min_max(
            egui::pos2(body.left(), body.top() + preview_height + 12.0),
            body.max,
        ),
        false,
    )
}

/// Largest 280×240 frame that fits the panel, keeping the panel's centre.
pub(crate) fn qube_screen_fit(panel: egui::Rect) -> egui::Rect {
    let width = panel
        .width()
        .min(panel.height() * SCREEN_WIDTH / SCREEN_HEIGHT)
        .max(1.0);
    let size = egui::vec2(width, width * SCREEN_HEIGHT / SCREEN_WIDTH);
    egui::Rect::from_center_size(panel.center(), size)
}

fn rect(screen: egui::Rect, x: f32, y: f32, width: f32, height: f32) -> egui::Rect {
    let scale = screen.width() / SCREEN_WIDTH;
    egui::Rect::from_min_size(
        screen.min + egui::vec2(x * scale, y * scale),
        egui::vec2(width * scale, height * scale),
    )
}

fn font(size: f32, scale: f32) -> egui::FontId {
    egui::FontId::new(size * scale, egui::FontFamily::Name("display_preview".into()))
}

pub(crate) fn paint_qube_screen_preview(
    ui: &egui::Ui,
    screen: egui::Rect,
    data: &QubePreviewData,
) {
    let scale = screen.width() / SCREEN_WIDTH;
    let painter = ui.painter();

    painter.rect(
        screen,
        6.0 * scale,
        data.background,
        Stroke::new(1.0_f32, data.borders),
        egui::StrokeKind::Inside,
    );

    let text_at = |x: f32, y: f32, value: &str, size: f32, color: Color32, align: egui::Align2| {
        painter.text(
            rect(screen, x, y, 0.0, 0.0).min,
            align,
            value,
            font(size, scale),
            color,
        );
    };
    let centered = |y: f32, value: &str, size: f32, color: Color32| {
        text_at(
            SCREEN_WIDTH / 2.0,
            y,
            value,
            size,
            color,
            egui::Align2::CENTER_CENTER,
        );
    };
    let panel = |r: egui::Rect| {
        painter.rect(
            r,
            3.0 * scale,
            data.panel,
            Stroke::new(1.0_f32, data.borders),
            egui::StrokeKind::Inside,
        );
    };
    let fill = |r: egui::Rect, color: Color32| {
        painter.rect_filled(r, 2.0 * scale, color);
    };
    let bar = |r: egui::Rect, fraction: f32, color: Color32| {
        fill(r, data.panel);
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction > 0.0 {
            fill(
                egui::Rect::from_min_size(r.min, egui::vec2(r.width() * fraction, r.height())),
                color,
            );
        }
    };
    // Green below the yellow threshold, amber between, red above - the zones the
    // firmware paints on its speed bars.
    let zone_color = |fraction: f32| -> Color32 {
        if fraction >= 0.9 {
            data.critical
        } else if fraction >= 0.6 {
            data.warning
        } else {
            data.bar
        }
    };
    let wpm_fraction = f32::from(PREVIEW_WPM) / WPM_SCALE_MAX;
    let layer = data.layer_label();

    match data.concept {
        // HUD: progress bars for speed and both batteries, modifier chips.
        1 => {
            panel(rect(screen, 8.0, 8.0, 264.0, 30.0));
            text_at(14.0, 23.0, "layer", 9.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(
                252.0,
                23.0,
                &layer,
                12.0,
                data.text,
                egui::Align2::RIGHT_CENTER,
            );
            if data.show_wpm {
                text_at(14.0, 46.0, "WPM", 9.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(
                    266.0,
                    46.0,
                    &format!("{PREVIEW_WPM} / 150"),
                    9.0,
                    data.text,
                    egui::Align2::RIGHT_CENTER,
                );
            }
            if data.show_batteries {
                text_at(14.0, 96.0, "LEFT", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(266.0, 96.0, &format!("{}%", data.battery_left), 8.0, data.text, egui::Align2::RIGHT_CENTER);
                text_at(14.0, 124.0, "RIGHT", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(266.0, 124.0, &format!("{}%", data.battery_right), 8.0, data.text, egui::Align2::RIGHT_CENTER);
            }
            if data.show_modifiers {
                text_at(14.0, 152.0, "MODS", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                for (index, chip) in ["CTRL", "SHIFT", "ALT", "GUI", "CAPS"].iter().enumerate() {
                    let x = 16.0 + index as f32 * 34.0;
                    panel(rect(screen, x, 160.0, 32.0, 16.0));
                    text_at(x + 16.0, 168.0, chip, 7.0, data.text, egui::Align2::CENTER_CENTER);
                }
            }
            if data.show_connection {
                panel(rect(screen, 14.0, 196.0, 92.0, 22.0));
                fill(rect(screen, 20.0, 204.0, 6.0, 6.0), data.bar);
                text_at(
                    32.0,
                    207.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
            }
        }
        // Terminal: phosphor status lines in a monospaced frame.
        2 => {
            let phosphor = Color32::from_rgb(0x3D, 0xF0, 0x6A);
            painter.rect_filled(screen, 6.0 * scale, Color32::BLACK);
            painter.rect_stroke(
                rect(screen, 6.0, 6.0, 268.0, 228.0),
                0.0,
                Stroke::new(1.0_f32, phosphor),
                egui::StrokeKind::Inside,
            );
            fill(rect(screen, 7.0, 7.0, 266.0, 14.0), phosphor);
            text_at(12.0, 14.0, "QUBE // status", 8.0, Color32::BLACK, egui::Align2::LEFT_CENTER);
            text_at(266.0, 14.0, "busy", 8.0, Color32::BLACK, egui::Align2::RIGHT_CENTER);
            let rows: [String; 5] = [
                format!("layer  : {} ({})", data.layer_index, layer),
                format!("wpm    : {}", if data.show_wpm { PREVIEW_WPM } else { 0 }),
                format!(
                    "link   : {} [{}]",
                    data.connection_label(),
                    if data.bluetooth { "bt pairing" } else { "usb wired" }
                ),
                format!(
                    "bat    : L {}% / R {}%",
                    if data.show_batteries { data.battery_left } else { 0 },
                    if data.show_batteries { data.battery_right } else { 0 }
                ),
                format!(
                    "mods   : {}",
                    if data.show_modifiers { "ctrl shift caps" } else { "none" }
                ),
            ];
            for (index, row) in rows.iter().enumerate() {
                text_at(
                    12.0,
                    34.0 + index as f32 * 16.0,
                    row,
                    8.0,
                    phosphor,
                    egui::Align2::LEFT_CENTER,
                );
            }
            text_at(12.0, 122.0, "media  :", 8.0, phosphor, egui::Align2::LEFT_CENTER);
            text_at(12.0, 138.0, data.media_ticker(), 8.0, phosphor, egui::Align2::LEFT_CENTER);
            text_at(12.0, 154.0, "clock  : 09:41", 8.0, phosphor, egui::Align2::LEFT_CENTER);
            fill(rect(screen, 260.0, 150.0, 8.0, 8.0), phosphor);
            text_at(12.0, 210.0, "> ready", 8.0, phosphor, egui::Align2::LEFT_CENTER);
        }
        // Minimal: one word and a time, nothing else.
        3 => {
            centered(112.0, &layer, 26.0, data.text);
            let line = 0.5 * (34.0 + layer.chars().count() as f32 * 15.0);
            fill(
                rect(screen, SCREEN_WIDTH / 2.0 - line, 132.0, line * 2.0, 2.0),
                data.accent,
            );
            centered(206.0, "09:41", 9.0, data.labels);
        }
        // Tiles: four labelled panes.
        4 => {
            let tiles = [
                (8.0, 8.0, "LAYER"),
                (146.0, 8.0, "WPM"),
                (8.0, 124.0, "LINK"),
                (146.0, 124.0, "BATTERY"),
            ];
            for (x, y, title) in tiles {
                panel(rect(screen, x, y, 126.0, 108.0));
                text_at(x + 8.0, y + 14.0, title, 8.0, data.labels, egui::Align2::LEFT_CENTER);
            }
            text_at(14.0, 60.0, &layer, 15.0, data.text, egui::Align2::LEFT_CENTER);
            if data.show_wpm {
                text_at(
                    209.0,
                    60.0,
                    &PREVIEW_WPM.to_string(),
                    20.0,
                    data.accent,
                    egui::Align2::CENTER_CENTER,
                );
            }
            if data.show_connection {
                fill(rect(screen, 14.0, 166.0, 6.0, 6.0), data.bar);
                text_at(
                    26.0,
                    169.0,
                    data.connection_label(),
                    10.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(
                    14.0,
                    190.0,
                    if data.bluetooth { "bt pairing" } else { "usb wired" },
                    8.0,
                    data.labels,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_batteries {
                for (index, (name, value)) in [
                    ("L", data.battery_left),
                    ("R", data.battery_right),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = 156.0 + index as f32 * 22.0;
                    text_at(154.0, y, name, 8.0, data.labels, egui::Align2::LEFT_CENTER);
                    bar(
                        rect(screen, 168.0, y - 5.0, 70.0, 9.0),
                        f32::from(value) / 100.0,
                        data.bar,
                    );
                    text_at(
                        266.0,
                        y,
                        &format!("{value}%"),
                        8.0,
                        data.text,
                        egui::Align2::RIGHT_CENTER,
                    );
                }
            }
        }
        // Speedo: one large number and a segmented scale.
        5 => {
            text_at(140.0, 20.0, &layer, 11.0, data.text, egui::Align2::CENTER_CENTER);
            if data.show_connection {
                fill(rect(screen, 12.0, 20.0, 6.0, 6.0), data.bar);
                text_at(
                    26.0,
                    23.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
            }
            centered(58.0, &format!("{PREVIEW_WPM}"), 30.0, data.text);
            centered(88.0, "words per minute", 8.0, data.labels);
            if data.show_wpm {
                for index in 0..20 {
                    let start = index as f32 / 20.0;
                    let color = zone_color(start);
                    let x = 40.0 + index as f32 * 7.0;
                    let filled = start <= wpm_fraction;
                    fill(
                        rect(screen, x, 108.0, 6.0, 10.0),
                        if filled { color } else { data.panel },
                    );
                }
                text_at(40.0, 124.0, "0", 7.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(180.0, 124.0, "150", 7.0, data.labels, egui::Align2::RIGHT_CENTER);
            }
            if data.show_batteries {
                text_at(
                    266.0,
                    58.0,
                    &format!("{}%", data.battery_left),
                    20.0,
                    data.text,
                    egui::Align2::RIGHT_CENTER,
                );
                text_at(
                    266.0,
                    88.0,
                    &format!("{}%", data.battery_right),
                    12.0,
                    data.labels,
                    egui::Align2::RIGHT_CENTER,
                );
            }
            text_at(12.0, 214.0, data.media_ticker(), 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(266.0, 214.0, "L 73% R 41%", 8.0, data.labels, egui::Align2::RIGHT_CENTER);
        }
        // Infocenter: clock first, then what is playing.
        6 => {
            centered(40.0, "09:41", 30.0, data.text);
            centered(66.0, "SAT 06 OCT", 8.0, data.labels);
            centered(96.0, "NOW PLAYING", 7.0, data.labels);
            centered(112.0, data.media_ticker(), 8.0, data.text);
            let status: [String; 4] = [
                "layer".to_owned(),
                if data.show_wpm { "wpm" } else { "bat" }.to_owned(),
                layer.clone(),
                format!("{PREVIEW_WPM}"),
            ];
            for (index, value) in status.iter().enumerate() {
                let x = 20.0 + index as f32 * 62.0;
                text_at(x, 152.0, value, 7.0, data.labels, egui::Align2::LEFT_CENTER);
            }
            if data.show_connection {
                fill(rect(screen, 20.0, 178.0, 6.0, 6.0), data.bar);
                text_at(
                    32.0,
                    181.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_batteries {
                text_at(
                    266.0,
                    181.0,
                    &format!("L{}% R{}%", data.battery_left, data.battery_right),
                    8.0,
                    data.labels,
                    egui::Align2::RIGHT_CENTER,
                );
            }
            text_at(266.0, 214.0, "09:41", 8.0, data.labels, egui::Align2::RIGHT_CENTER);
        }
        // Two-column: keyboard on the left, connection on the right.
        7 => {
            fill(rect(screen, 139.0, 8.0, 1.0, 224.0), data.borders);
            text_at(12.0, 16.0, "KEYBOARD", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(150.0, 16.0, "CONNECTION", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(12.0, 44.0, &layer, 16.0, data.text, egui::Align2::LEFT_CENTER);
            text_at(
                12.0,
                66.0,
                &format!("layer {}", data.layer_index),
                8.0,
                data.labels,
                egui::Align2::LEFT_CENTER,
            );
            if data.show_wpm {
                text_at(
                    12.0,
                    104.0,
                    &format!("{PREVIEW_WPM}"),
                    18.0,
                    data.accent,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(12.0, 124.0, "wpm", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            }
            text_at(12.0, 206.0, "09:41", 10.0, data.labels, egui::Align2::LEFT_CENTER);
            if data.show_connection {
                fill(rect(screen, 150.0, 34.0, 6.0, 6.0), data.bar);
                text_at(
                    162.0,
                    37.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(
                    150.0,
                    56.0,
                    if data.bluetooth { "bt pairing" } else { "usb wired" },
                    8.0,
                    data.labels,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(150.0, 76.0, "profile 0", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            }
            if data.show_batteries {
                for (index, (name, value)) in [
                    ("LEFT", data.battery_left),
                    ("RIGHT", data.battery_right),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = 108.0 + index as f32 * 30.0;
                    text_at(150.0, y, name, 8.0, data.labels, egui::Align2::LEFT_CENTER);
                    bar(
                        rect(screen, 150.0, y + 8.0, 118.0, 8.0),
                        f32::from(value) / 100.0,
                        data.bar,
                    );
                    text_at(
                        268.0,
                        y,
                        &format!("{value}%"),
                        8.0,
                        data.text,
                        egui::Align2::RIGHT_CENTER,
                    );
                }
            }
        }
        // Sparkline: speed history with the current value called out.
        8 => {
            text_at(12.0, 18.0, &layer, 13.0, data.text, egui::Align2::LEFT_CENTER);
            if data.show_wpm {
                text_at(
                    268.0,
                    18.0,
                    &format!("{PREVIEW_WPM}"),
                    16.0,
                    data.accent,
                    egui::Align2::RIGHT_CENTER,
                );
            }
            text_at(12.0, 40.0, "wpm history", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            fill(rect(screen, 12.0, 52.0, 256.0, 1.0), data.borders);
            for index in 0..24 {
                let fraction = 0.25 + 0.75 * (index as f32 / 23.0);
                let height = 90.0 * fraction;
                let x = 12.0 + index as f32 * 10.8;
                fill(
                    rect(screen, x, 142.0 - height, 8.0, height),
                    zone_color(fraction),
                );
            }
            text_at(12.0, 178.0, "0", 7.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(
                268.0,
                178.0,
                &format!("peak {PREVIEW_WPM}"),
                7.0,
                data.labels,
                egui::Align2::RIGHT_CENTER,
            );
            if data.show_batteries {
                text_at(
                    12.0,
                    206.0,
                    &format!("L{}% R{}%", data.battery_left, data.battery_right),
                    8.0,
                    data.labels,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_connection {
                text_at(
                    268.0,
                    206.0,
                    data.connection_label(),
                    8.0,
                    data.text,
                    egui::Align2::RIGHT_CENTER,
                );
            }
        }
        // Signal: link state as a radio panel.
        9 => {
            text_at(12.0, 18.0, "LINK", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(268.0, 18.0, &layer, 12.0, data.text, egui::Align2::RIGHT_CENTER);
            if data.show_connection {
                text_at(
                    12.0,
                    46.0,
                    data.connection_label(),
                    26.0,
                    data.bar,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(
                    12.0,
                    70.0,
                    if data.bluetooth { "bt pairing" } else { "usb wired" },
                    8.0,
                    data.labels,
                    egui::Align2::LEFT_CENTER,
                );
            }
            for index in 0..4 {
                let height = 14.0 + index as f32 * 12.0;
                fill(
                    rect(screen, 196.0 + index as f32 * 18.0, 60.0 - height + 14.0, 14.0, height),
                    if index < 3 { data.bar } else { data.panel },
                );
            }
            text_at(12.0, 116.0, "PROFILE", 7.0, data.labels, egui::Align2::LEFT_CENTER);
            for index in 0..5 {
                let x = 12.0 + index as f32 * 20.0;
                panel(rect(screen, x, 126.0, 16.0, 16.0));
                text_at(
                    x + 8.0,
                    134.0,
                    &index.to_string(),
                    8.0,
                    if index == 1 { data.text } else { data.labels },
                    egui::Align2::CENTER_CENTER,
                );
            }
            text_at(12.0, 176.0, "BATTERY", 7.0, data.labels, egui::Align2::LEFT_CENTER);
            if data.show_batteries {
                for (index, (name, value)) in [
                    ("L", data.battery_left),
                    ("R", data.battery_right),
                ]
                .into_iter()
                .enumerate()
                {
                    let x = 84.0 + index as f32 * 96.0;
                    text_at(x, 176.0, name, 8.0, data.labels, egui::Align2::LEFT_CENTER);
                    bar(
                        rect(screen, x + 12.0, 171.0, 72.0, 9.0),
                        f32::from(value) / 100.0,
                        data.bar,
                    );
                }
            }
            text_at(12.0, 212.0, "plug", 7.0, data.labels, egui::Align2::LEFT_CENTER);
            for index in 0..3 {
                panel(rect(screen, 40.0 + index as f32 * 12.0, 206.0, 10.0, 12.0));
            }
            text_at(268.0, 212.0, "09:41", 8.0, data.labels, egui::Align2::RIGHT_CENTER);
        }
        // Mood: a small character that follows the state.
        10 => {
            centered(60.0, "( _ _ )", 22.0, data.accent);
            centered(94.0, "idle", 9.0, data.labels);
            centered(124.0, &layer, 16.0, data.text);
            if data.show_wpm {
                centered(150.0, &format!("{PREVIEW_WPM} wpm"), 9.0, data.labels);
            }
            if data.show_connection {
                centered(176.0, data.connection_label(), 9.0, data.bar);
            }
            text_at(268.0, 212.0, "09:41", 8.0, data.labels, egui::Align2::RIGHT_CENTER);
        }
        // Battiles: the two halves as big battery tiles.
        11 => {
            text_at(12.0, 18.0, "BATTERY", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(268.0, 18.0, &layer, 11.0, data.text, egui::Align2::RIGHT_CENTER);
            for (index, (name, value)) in [
                ("LEFT", data.battery_left),
                ("RIGHT", data.battery_right),
            ]
            .into_iter()
            .enumerate()
            {
                let x = 12.0 + index as f32 * 134.0;
                if !data.show_batteries {
                    continue;
                }
                panel(rect(screen, x, 34.0, 122.0, 130.0));
                text_at(x + 10.0, 52.0, name, 9.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(
                    x + 112.0,
                    56.0,
                    &format!("{value}%"),
                    24.0,
                    data.text,
                    egui::Align2::RIGHT_CENTER,
                );
                // A segmented cell battery, not just a bar.
                let segments = 10;
                for segment in 0..segments {
                    let filled = f32::from(value) / 100.0 > segment as f32 / segments as f32;
                    let color = if filled {
                        zone_color(f32::from(value) / 100.0)
                    } else {
                        data.background
                    };
                    fill(
                        rect(screen, x + 12.0 + segment as f32 * 10.0, 96.0, 8.0, 22.0),
                        color,
                    );
                }
                text_at(
                    x + 10.0,
                    140.0,
                    if data.bluetooth { "wireless" } else { "wired" },
                    8.0,
                    data.labels,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_wpm {
                text_at(12.0, 186.0, "WPM", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(
                    88.0,
                    186.0,
                    &PREVIEW_WPM.to_string(),
                    12.0,
                    data.accent,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_connection {
                fill(rect(screen, 12.0, 208.0, 6.0, 6.0), data.bar);
                text_at(
                    24.0,
                    211.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
            }
            text_at(268.0, 211.0, "09:41", 9.0, data.labels, egui::Align2::RIGHT_CENTER);
        }
        // Mediacenter: what is playing is the screen.
        12 => {
            panel(rect(screen, 8.0, 8.0, 264.0, 92.0));
            text_at(16.0, 24.0, "NOW PLAYING", 7.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(16.0, 46.0, "Boards of Canada", 15.0, data.text, egui::Align2::LEFT_CENTER);
            text_at(16.0, 68.0, "Roygbiv", 11.0, data.accent, egui::Align2::LEFT_CENTER);
            bar(rect(screen, 16.0, 84.0, 248.0, 6.0), 0.42, data.bar);
            text_at(16.0, 112.0, "01:12", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(264.0, 112.0, "-01:41", 8.0, data.labels, egui::Align2::RIGHT_CENTER);
            text_at(140.0, 128.0, "VOL  42", 9.0, data.labels, egui::Align2::CENTER_CENTER);
            if data.show_wpm {
                text_at(12.0, 154.0, "WPM", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                text_at(
                    58.0,
                    154.0,
                    &PREVIEW_WPM.to_string(),
                    14.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_connection {
                panel(rect(screen, 12.0, 176.0, 74.0, 22.0));
                fill(rect(screen, 18.0, 184.0, 6.0, 6.0), data.bar);
                text_at(
                    30.0,
                    187.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_batteries {
                for (index, (name, value)) in [
                    ("L", data.battery_left),
                    ("R", data.battery_right),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = 178.0 + index as f32 * 20.0;
                    text_at(148.0, y, name, 8.0, data.labels, egui::Align2::LEFT_CENTER);
                    bar(
                        rect(screen, 162.0, y - 5.0, 70.0, 9.0),
                        f32::from(value) / 100.0,
                        data.bar,
                    );
                    text_at(
                        268.0,
                        y,
                        &format!("{value}%"),
                        8.0,
                        data.text,
                        egui::Align2::RIGHT_CENTER,
                    );
                }
            }
            text_at(12.0, 214.0, &layer, 9.0, data.labels, egui::Align2::LEFT_CENTER);
        }
        // Board: keyboard state on the left, phone/connection on the right.
        13 => {
            panel(rect(screen, 8.0, 8.0, 150.0, 224.0));
            text_at(16.0, 22.0, "KEYBOARD", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(16.0, 46.0, &layer, 15.0, data.text, egui::Align2::LEFT_CENTER);
            text_at(
                16.0,
                66.0,
                &format!("layer {}", data.layer_index),
                8.0,
                data.labels,
                egui::Align2::LEFT_CENTER,
            );
            if data.show_wpm {
                text_at(
                    16.0,
                    102.0,
                    &PREVIEW_WPM.to_string(),
                    20.0,
                    data.accent,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(16.0, 122.0, "wpm", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            }
            if data.show_modifiers {
                text_at(16.0, 148.0, "MODS", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                for (index, chip) in ["SHIFT", "CTRL", "ALT"].iter().enumerate() {
                    panel(rect(screen, 16.0, 156.0 + index as f32 * 20.0, 58.0, 17.0));
                    text_at(
                        45.0,
                        164.0 + index as f32 * 20.0,
                        chip,
                        7.0,
                        data.text,
                        egui::Align2::CENTER_CENTER,
                    );
                }
            }
            panel(rect(screen, 166.0, 8.0, 106.0, 140.0));
            text_at(176.0, 22.0, "PHONE", 8.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(176.0, 44.0, "Media", 10.0, data.text, egui::Align2::LEFT_CENTER);
            text_at(176.0, 60.0, "Roygbiv", 8.0, data.accent, egui::Align2::LEFT_CENTER);
            bar(rect(screen, 176.0, 76.0, 86.0, 5.0), 0.42, data.bar);
            if data.show_connection {
                fill(rect(screen, 176.0, 92.0, 6.0, 6.0), data.bar);
                text_at(
                    188.0,
                    95.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
                text_at(
                    176.0,
                    114.0,
                    if data.bluetooth { "bt pairing" } else { "usb wired" },
                    8.0,
                    data.labels,
                    egui::Align2::LEFT_CENTER,
                );
            }
            if data.show_batteries {
                panel(rect(screen, 166.0, 156.0, 106.0, 76.0));
                text_at(176.0, 172.0, "BATTERY", 8.0, data.labels, egui::Align2::LEFT_CENTER);
                for (index, (name, value)) in [
                    ("L", data.battery_left),
                    ("R", data.battery_right),
                ]
                .into_iter()
                .enumerate()
                {
                    let y = 192.0 + index as f32 * 20.0;
                    text_at(176.0, y, name, 8.0, data.labels, egui::Align2::LEFT_CENTER);
                    bar(
                        rect(screen, 190.0, y - 5.0, 62.0, 9.0),
                        f32::from(value) / 100.0,
                        data.bar,
                    );
                    text_at(
                        264.0,
                        y,
                        &format!("{value}%"),
                        8.0,
                        data.text,
                        egui::Align2::RIGHT_CENTER,
                    );
                }
            }
        }
        // Dashboard v2 (0) and anything the firmware adds later.
        _ => {
            let header_bottom = 40.0;
            fill(rect(screen, 0.0, 0.0, SCREEN_WIDTH, header_bottom), data.panel);
            fill(rect(screen, 0.0, header_bottom - 1.0, SCREEN_WIDTH, 1.0), data.borders);
            let badge_in_header = data.badge_place == 0;
            let mut header_right = 268.0;
            if data.show_connection && badge_in_header {
                panel(rect(screen, 208.0, 10.0, 60.0, 20.0));
                fill(rect(screen, 214.0, 17.0, 6.0, 6.0), data.bar);
                text_at(
                    226.0,
                    20.0,
                    data.connection_label(),
                    9.0,
                    data.text,
                    egui::Align2::LEFT_CENTER,
                );
                header_right = 200.0;
            }
            if data.header_mode != 0 {
                text_at(
                    header_right,
                    20.0,
                    "09:41",
                    13.0,
                    data.text,
                    egui::Align2::RIGHT_CENTER,
                );
            }
            if data.header_mode != 1 {
                text_at(12.0, 20.0, data.media_ticker(), 8.0, data.labels, egui::Align2::LEFT_CENTER);
            }

            text_at(14.0, 70.0, &layer, 22.0, data.text, egui::Align2::LEFT_CENTER);
            if data.show_wpm {
                text_at(
                    266.0,
                    70.0,
                    &PREVIEW_WPM.to_string(),
                    22.0,
                    data.accent,
                    egui::Align2::RIGHT_CENTER,
                );
                text_at(266.0, 92.0, "wpm", 8.0, data.labels, egui::Align2::RIGHT_CENTER);
            }
            if data.show_modifiers {
                for (index, chip) in ["SHIFT", "CTRL", "ALT", "GUI"].iter().enumerate() {
                    let x = 14.0 + index as f32 * 48.0;
                    panel(rect(screen, x, 108.0, 44.0, 18.0));
                    text_at(
                        x + 22.0,
                        117.0,
                        chip,
                        7.0,
                        data.text,
                        egui::Align2::CENTER_CENTER,
                    );
                }
            }
            if data.show_batteries {
                for (index, (name, value)) in [
                    ("LEFT", data.battery_left),
                    ("RIGHT", data.battery_right),
                ]
                .into_iter()
                .enumerate()
                {
                    let x = 14.0 + index as f32 * 140.0;
                    panel(rect(screen, x, 140.0, 126.0, 74.0));
                    text_at(x + 10.0, 158.0, name, 8.0, data.labels, egui::Align2::LEFT_CENTER);
                    text_at(
                        x + 116.0,
                        158.0,
                        &format!("{value}%"),
                        10.0,
                        data.text,
                        egui::Align2::RIGHT_CENTER,
                    );
                    bar(
                        rect(screen, x + 10.0, 176.0, 106.0, 12.0),
                        f32::from(value) / 100.0,
                        zone_color(f32::from(value) / 100.0),
                    );
                }
            }
            if data.show_connection && !badge_in_header {
                panel(rect(screen, 14.0, 108.0, 44.0, 18.0));
                text_at(
                    36.0,
                    117.0,
                    data.connection_label(),
                    8.0,
                    data.text,
                    egui::Align2::CENTER_CENTER,
                );
            }
        }
    }

    // Accent shadow: a soft band under the layer word, as on the real screen.
    if data.concept != 2 {
        fill(
            rect(screen, 0.0, SCREEN_HEIGHT - 3.0, SCREEN_WIDTH, 3.0),
            data.accent_shadow,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(concept: u8) -> QubePreviewData {
        QubePreviewData {
            concept,
            show_wpm: true,
            show_modifiers: true,
            show_batteries: true,
            show_connection: true,
            header_mode: 2,
            badge_place: 0,
            accent: Color32::from_rgb(24, 154, 255),
            accent_shadow: Color32::from_rgb(8, 65, 148),
            background: Color32::from_rgb(0, 8, 33),
            panel: Color32::from_rgb(0, 18, 52),
            borders: Color32::from_rgb(30, 70, 120),
            text: Color32::WHITE,
            labels: Color32::from_rgb(150, 170, 200),
            bar: Color32::from_rgb(24, 154, 255),
            warning: Color32::from_rgb(255, 200, 40),
            critical: Color32::from_rgb(230, 60, 60),
            layer_name: "Base".to_owned(),
            layer_index: 0,
            battery_left: 73,
            battery_right: 41,
            bluetooth: false,
        }
    }

    fn theme(key: &str) -> QubeThemePalette {
        QUBE_THEMES
            .iter()
            .copied()
            .find(|theme| theme.label_key == key)
            .expect("theme")
    }

    fn relative_luminance(color: [u8; 3]) -> f32 {
        let channel = |value: u8| {
            let value = f32::from(value) / 255.0;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(color[0]) + 0.7152 * channel(color[1]) + 0.0722 * channel(color[2])
    }

    fn contrast(a: [u8; 3], b: [u8; 3]) -> f32 {
        let (la, lb) = (relative_luminance(a), relative_luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[derive(Default)]
    struct Frame {
        text: Vec<String>,
        rects: usize,
        fills: Vec<Color32>,
    }

    fn collect(shape: &egui::Shape, frame: &mut Frame) {
        match shape {
            egui::Shape::Text(text_shape) => frame.text.push(text_shape.galley.job.text.clone()),
            egui::Shape::Rect(rect) => {
                frame.rects += 1;
                frame.fills.push(rect.fill);
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, frame);
                }
            }
            _ => {}
        }
    }

    /// The app binds this family from the embedded display font at startup; a
    /// bare `egui::Context` has no such family and epaint panics on the lookup.
    fn test_context() -> egui::Context {
        let ctx = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "display_preview".to_owned(),
            egui::FontData::from_static(include_bytes!("../../assets/Montserrat-Medium.ttf")).into(),
        );
        fonts.families.insert(
            egui::FontFamily::Name("display_preview".into()),
            vec!["display_preview".to_owned()],
        );
        ctx.set_fonts(fonts);
        ctx
    }

    fn paint(data: &QubePreviewData) -> Frame {
        let ctx = test_context();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(320.0, 300.0),
            )),
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| {
            let screen = egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
            );
            paint_qube_screen_preview(ui, screen, data);
        });
        let mut frame = Frame::default();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut frame);
        }
        frame
    }

    #[test]
    fn every_concept_paints_a_distinct_frame() {
        let mut signatures: Vec<(usize, usize, usize)> = Vec::new();
        for concept in 0..14u8 {
            let frame = paint(&sample(concept));
            assert!(
                frame.text.iter().any(|value| value.contains("Base")),
                "concept {concept} must show the layer name: {:?}",
                frame.text
            );
            assert!(
                // Every frame paints at least the screen background and the
                // accent band; the text-only concepts add nothing else.
                frame.rects >= 2,
                "concept {concept} painted only {} shapes",
                frame.rects
            );
            // Text lines, painted rectangles and filled shapes: three numbers
            // that differ as soon as a concept lays its elements out differently.
            signatures.push((frame.text.len(), frame.rects, frame.fills.len()));
        }
        let unique: std::collections::HashSet<_> = signatures.iter().collect();
        assert_eq!(
            unique.len(),
            signatures.len(),
            "concepts should differ, got {signatures:?}"
        );
    }

    #[test]
    fn settings_change_the_painted_frame() {
        let on = paint(&sample(0));
        assert!(on.text.iter().any(|value| value == "LEFT"));
        assert!(on.text.iter().any(|value| value == "wpm"));

        let mut off = sample(0);
        off.show_batteries = false;
        off.show_wpm = false;
        let off = paint(&off);
        assert!(
            !off.text.iter().any(|value| value == "LEFT"),
            "batteries off must remove the battery card: {:?}",
            off.text
        );
        assert!(
            !off.text.iter().any(|value| value == "wpm"),
            "WPM off must remove the speed label"
        );
        assert!(
            off.rects < on.rects,
            "fewer elements must paint fewer shapes"
        );
    }

    #[test]
    fn layer_name_falls_back_to_the_index() {
        let mut data = sample(4);
        data.layer_name.clear();
        data.layer_index = 4;
        let frame = paint(&data);
        assert!(frame.text.iter().any(|value| value == "4"), "{:?}", frame.text);
    }

    #[test]
    fn colors_reach_the_frame() {
        // The sparkline paints the whole scale, so all three zones show up; the
        // speedo only colours the part below the current speed.
        let mut data = sample(8);
        data.bar = Color32::from_rgb(1, 2, 3);
        data.warning = Color32::from_rgb(4, 5, 6);
        data.critical = Color32::from_rgb(7, 8, 9);
        let frame = paint(&data);
        assert!(
            frame.fills.contains(&Color32::from_rgb(1, 2, 3)),
            "the bar colour must be painted"
        );
        assert!(
            frame.fills.contains(&Color32::from_rgb(4, 5, 6)),
            "the threshold colour must be painted"
        );
        assert!(
            frame.fills.contains(&Color32::from_rgb(7, 8, 9)),
            "the critical colour must be painted"
        );
    }

    #[test]
    fn every_theme_keeps_its_elements_readable() {
        // 3:1 is the graphics threshold: bars, frames and thresholds are shapes,
        // not body text. Text and labels are checked against both the background
        // and the panel, which is what a light theme gets wrong first.
        for theme in QUBE_THEMES {
            for surface in [theme.background, theme.panel] {
                for (name, color) in [
                    ("text", theme.text),
                    ("labels", theme.labels),
                    ("accent", theme.accent),
                    ("bar", theme.bar),
                    ("warning", theme.warning),
                    ("critical", theme.critical),
                ] {
                    let ratio = contrast(color, surface);
                    assert!(
                        ratio >= 3.0,
                        "{} {name} on {surface:?}: {ratio:.2}:1",
                        theme.label_key
                    );
                }
                // The frame must be visible against the surface it outlines.
                assert!(
                    contrast(theme.borders, surface) >= 1.4,
                    "{} borders on {surface:?}",
                    theme.label_key
                );
            }
        }
    }

    #[test]
    fn light_theme_uses_dark_ink_on_a_light_background() {
        let light = theme("qube_screen.theme_light");
        assert!(relative_luminance(light.background) > 0.7);
        assert!(relative_luminance(light.text) < 0.1);
        assert!(
            contrast(light.text, light.background) >= 7.0,
            "light theme text must stay high contrast"
        );
    }

    #[test]
    fn theme_writes_touch_only_announced_channels() {
        let light = theme("qube_screen.theme_light");
        // A Qube firmware that announces the classic palette only: the theme must
        // not write the newer panel/frame/text colour blocks.
        let classic: Vec<u16> = (216..=228)
            .chain([318])
            .chain(320..=322)
            .chain(330..=332)
            .chain(224..=226)
            .collect();
        let writes = light.writes(&classic);
        assert!(writes.iter().all(|(qsid, _)| classic.contains(qsid)));
        assert!(writes.iter().any(|(qsid, _)| *qsid == 330));
        assert!(!writes.iter().any(|(qsid, _)| (230..=250).contains(qsid)));

        // With the full palette announced every channel is written.
        let full: Vec<u16> = (216..=250)
            .chain([318])
            .chain(320..=322)
            .chain(330..=332)
            .collect();
        assert_eq!(light.writes(&full).len(), 30);
    }

    #[test]
    fn theme_matches_detects_the_active_palette() {
        let light = theme("qube_screen.theme_light");
        let mut values = std::collections::BTreeMap::new();
        for (qsid, value) in light.writes(&(216..=250).collect::<Vec<u16>>()) {
            values.insert(qsid, u16::from(value));
        }
        assert!(light.matches(&values));
        assert!(!theme("qube_screen.theme_dark").matches(&values));
        values.remove(&330);
        assert!(
            light.matches(&values),
            "a palette still matches when the firmware cannot store every channel"
        );
        assert!(!light.matches(&std::collections::BTreeMap::new()));
    }
}
