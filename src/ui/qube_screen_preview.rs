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

    /// The value the firmware stores for one channel of this palette, and the
    /// value it reads back: colours live in RGB565, so red and blue keep five
    /// bits and green keeps six, expanded back to eight.
    pub(crate) fn stored_channel(&self, qsid: u16) -> Option<u8> {
        let index = crate::app::app_state::qube_color_channel(qsid)?;
        let triple = self
            .triples()
            .into_iter()
            .find(|(qsids, _)| qsids.contains(&qsid))?;
        Some(five_six_five_channel(index, triple.1[index]))
    }

    /// True when the device already shows this palette. Values are compared
    /// after the firmware's RGB565 quantisation - comparing raw 8-bit channels
    /// never matched anything, which left the theme row without an active entry.
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
                .enumerate()
                .any(|(index, value)| {
                    *value != u16::from(five_six_five_channel(index, channels[index]))
                })
            {
                return false;
            }
            compared += 1;
        }
        compared > 0
    }
}

/// RGB565 keeps five bits for red and blue and six for green, and the firmware
/// expands the stored value back to eight bits - so a channel written as 80
/// reads back as 82, and 21 reads back as 20.
pub(crate) fn five_six_five_channel(channel: usize, value: u8) -> u8 {
    if channel == 1 {
        let six = value >> 2;
        (six << 2) | (six >> 4)
    } else {
        let five = value >> 3;
        (five << 3) | (five >> 2)
    }
}

/// Dark is the firmware's factory palette, taken from the `DEFAULT_*`
/// constants of the Qube firmware (`layer_names.rs`), so picking it restores the
/// factory look exactly. The device reports these through RGB565, so the
/// comparison in `matches` quantises before comparing.
pub(crate) const QUBE_THEME_DARK: QubeThemePalette = QubeThemePalette {
    label_key: "qube_screen.theme_dark",
    accent: [24, 154, 255],
    accent_shadow: [8, 65, 148],
    background: [0, 8, 33],
    panel: [16, 24, 74],
    borders: [41, 52, 132],
    text: [239, 247, 247],
    labels: [90, 97, 165],
    bar: [24, 170, 247],
    warning: [255, 203, 0],
    critical: [255, 21, 42],
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
    /// QSID 318: software dimming of the finished frame, `10..=100`.
    pub(crate) brightness: u8,
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

// ---------------------------------------------------------------------------
// The firmware renderer
// ---------------------------------------------------------------------------
//
// The preview shows the frame the *device* paints. `crate::qube_fw` compiles
// the firmware's own display engine (`qube_display.rs` of `work/my-config`,
// branch `concepts`) against host stubs, renders it into a 280×240 RGB565
// buffer and hands the result over as an egui texture; nothing here draws the
// screen any more. The stylised painter below is kept as a fallback for targets
// that cannot host the engine (wasm) and for diagnosing the panel itself.

/// Paint the preview with the firmware engine. `false` selects the stylised
/// fallback.
pub(crate) const QUBE_PREVIEW_FIRMWARE: bool = cfg!(not(target_arch = "wasm32"));

/// Media line the preview shows. The app is not a media source: the line comes
/// from the host's now-playing over the RMK host protocol, so the preview uses
/// the same demo line the golden frames were rendered with.
const PREVIEW_MEDIA: &str = "Boards of Canada - Roygbiv";

/// The panel's own preview frame, as the firmware would paint it.
#[cfg(not(target_arch = "wasm32"))]
fn preview_frame(data: &QubePreviewData, clock: Option<(u8, u8)>) -> crate::qube_fw::HostLcd {
    crate::qube_fw::render_frame(&preview_input(data, clock))
}

/// One preview state in the firmware renderer's vocabulary: the settings the
/// device reports back, plus the host-protocol values the app has to supply.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn preview_input(
    data: &QubePreviewData,
    clock: Option<(u8, u8)>,
) -> crate::qube_fw::PreviewInput {
    let triple = |color: Color32| {
        let [r, g, b, _] = color.to_srgba_unmultiplied();
        [r, g, b]
    };
    let (hour, minute) = match clock {
        Some((hour, minute)) => (Some(hour), Some(minute)),
        None => (None, None),
    };
    crate::qube_fw::PreviewInput {
        concept: data.concept,
        // `COLOR_ROLES` order: accent, accent shadow, background, panel,
        // border, text, label, bar, warning, critical.
        colors: [
            triple(data.accent),
            triple(data.accent_shadow),
            triple(data.background),
            triple(data.panel),
            triple(data.borders),
            triple(data.text),
            triple(data.labels),
            triple(data.bar),
            triple(data.warning),
            triple(data.critical),
        ],
        brightness: data.brightness.clamp(10, 100),
        show_wpm: data.show_wpm,
        show_modifiers: data.show_modifiers,
        show_batteries: data.show_batteries,
        show_output: data.show_connection,
        header_mode: data.header_mode.min(2),
        output_place: data.badge_place.min(1),
        layer: data.layer_index.min(u8::MAX as usize) as u8,
        layer_name: data.layer_label(),
        // Both halves are treated as connected: the preview shows the panel a
        // working dongle would show, and the levels are the device's own.
        batteries: [
            (Some(data.battery_left.min(100)), true),
            (Some(data.battery_right.min(100)), true),
        ],
        // The app only knows whether the selected device talks over Bluetooth;
        // the preview shows the badge the stylised one showed for that case
        // (`BT2*`), which is the pairing state of profile 2.
        link: if data.bluetooth {
            crate::qube_fw::Link::Pairing { profile: 2 }
        } else {
            crate::qube_fw::Link::Usb
        },
        wpm: PREVIEW_WPM,
        // A preview has no keyboard attached, so no modifiers are held.
        modifiers: 0,
        caps_lock: false,
        num_lock: false,
        hour,
        minute,
        media: PREVIEW_MEDIA.to_owned(),
    }
}

/// Local wall clock, the value the device shows in its header.
#[cfg(not(target_arch = "wasm32"))]
fn local_clock() -> Option<(u8, u8)> {
    use chrono::Timelike;
    let now = chrono::Local::now();
    Some((now.hour() as u8, now.minute() as u8))
}

/// Blits the firmware frame into the panel. The factor is a whole number while
/// the panel is at least as wide as the device screen, so every device pixel
/// covers an exact square of screen points and the frame stays sharp; a panel
/// narrower than 280 pt has to scale down fractionally and the filter stays
/// `NEAREST` so the edges do not smear.
#[cfg(not(target_arch = "wasm32"))]
fn paint_firmware_preview(ui: &egui::Ui, screen: egui::Rect, data: &QubePreviewData) {
    let frame = preview_frame(data, local_clock());
    let image = egui::ColorImage::from_rgb(
        [
            crate::qube_fw::SCREEN_W_PX as usize,
            crate::qube_fw::SCREEN_H_PX as usize,
        ],
        &frame.rgb8_bytes(),
    );
    let texture = ui.ctx().load_texture(
        "qube_screen_preview_frame",
        image,
        egui::TextureOptions::NEAREST,
    );
    let whole = (screen.width() / SCREEN_WIDTH).floor();
    let scale = if whole >= 1.0 {
        whole
    } else {
        screen.width() / SCREEN_WIDTH
    };
    let target = egui::Rect::from_center_size(
        screen.center(),
        egui::vec2(SCREEN_WIDTH * scale, SCREEN_HEIGHT * scale),
    );
    ui.painter().image(
        texture.id(),
        target,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub(crate) fn paint_qube_screen_preview(
    ui: &egui::Ui,
    screen: egui::Rect,
    data: &QubePreviewData,
) {
    #[cfg(not(target_arch = "wasm32"))]
    if QUBE_PREVIEW_FIRMWARE {
        paint_firmware_preview(ui, screen, data);
        return;
    }
    paint_stylized_preview(ui, screen, data);
}

/// Fallback painter: the same elements as the firmware, laid out by the app's
/// own font. Only reachable when [`QUBE_PREVIEW_FIRMWARE`] is off.
fn paint_stylized_preview(
    ui: &egui::Ui,
    screen: egui::Rect,
    data: &QubePreviewData,
) {
    let scale = screen.width() / SCREEN_WIDTH;
    let painter = ui.painter();

    // The dongle has no screen bezel: the panel shows the background colour all
    // the way to the edge. A stroke here would put the frame colour on glass
    // where the firmware never draws it.
    //
    // The terminal screen is the one concept that clears to pure black
    // (`C_BLACK`), which the palette post-pass leaves untouched - so it must not
    // be cleared with the configured background colour.
    let screen_fill = if data.concept == 2 {
        Color32::BLACK
    } else {
        data.background
    };
    painter.rect(
        screen,
        6.0 * scale,
        screen_fill,
        Stroke::NONE,
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
    let shades = data.shades();
    let bar = |r: egui::Rect, fraction: f32, color: Color32| {
        // Track of a progress bar. The dashboard battery card fills it with
        // `COL_BAR_BG` (`bar_bg`, one step off the panel shade) and every concept
        // bar is drawn by `cbar`, which fills with `COL_PANEL_HI` (`panel_hi`).
        // The media centre has no bar at all on the dongle (the footer prints
        // the percentages as text), so its preview bars sit on the background.
        let track = match data.concept {
            0 => shades.bar_bg,
            12 => data.background,
            _ => shades.panel_hi,
        };
        fill(r, track);
        let fraction = fraction.clamp(0.0, 1.0);
        if fraction > 0.0 {
            fill(
                egui::Rect::from_min_size(r.min, egui::vec2(r.width() * fraction, r.height())),
                color,
            );
        }
    };
    // Battery zones of the firmware (`battery_color`): red below 10 %, yellow
    // below 25 %, otherwise the "good" colour - `C_OK` (`battery_ok`) on the
    // dashboard and on concepts 1..10, `COL_BAR_FG` (`bar`) on the round-two
    // screens 11..13.
    let battery_color = |value: u8| -> Color32 {
        if value < 10 {
            shades.bad
        } else if value < 25 {
            shades.warn
        } else {
            shades.battery_ok
        }
    };
    let bar_battery_color = |value: u8| -> Color32 {
        if value < 10 {
            shades.bad
        } else if value < 25 {
            shades.warn
        } else {
            shades.bar
        }
    };
    // Segmented speed scale of the speedometer: green below 60 % of the scale,
    // yellow below 85 %, red above (the thresholds `render_concept_speedo` uses).
    let scale_color = |fraction: f32| -> Color32 {
        if fraction < 0.6 {
            shades.battery_ok
        } else if fraction < 0.85 {
            shades.warn
        } else {
            shades.bad
        }
    };
    // Connection badge: the firmware paints `COL_OUTPUT_OK` (green, derived from
    // the bar colour) while a transport is up and `COL_OUTPUT_SEARCH` (blue,
    // derived from the accent) while it is pairing or reconnecting - the preview
    // showed the plain bar colour here, which is why a theme looked different on
    // the dongle.
    let link_color = if data.bluetooth {
        shades.output_search
    } else {
        shades.output_ok
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
                fill(rect(screen, 20.0, 204.0, 6.0, 6.0), link_color);
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
            // `render_concept_terminal` clears to `C_BLACK` - which the palette
            // post-pass leaves alone - and draws with `C_TERM`, the phosphor
            // green derived from the bar colour (`terminal`). The preview used a
            // fixed RGB constant here, so the terminal screen kept its green
            // look while the rest of the theme changed.
            let phosphor = shades.terminal;
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
                fill(rect(screen, 14.0, 166.0, 6.0, 6.0), link_color);
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
                        battery_color(value),
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
                fill(rect(screen, 12.0, 20.0, 6.0, 6.0), link_color);
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
                    let color = scale_color(start);
                    let x = 40.0 + index as f32 * 7.0;
                    let filled = start <= wpm_fraction;
                    fill(
                        rect(screen, x, 108.0, 6.0, 10.0),
                        if filled { color } else { shades.panel_hi },
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
                fill(rect(screen, 20.0, 178.0, 6.0, 6.0), link_color);
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
                fill(rect(screen, 150.0, 34.0, 6.0, 6.0), link_color);
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
                        battery_color(value),
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
                // The firmware colours the newest bar with the accent, the
                // loud ones yellow and the rest with the raised panel shade.
                let color = if index + 1 == 24 {
                    data.accent
                } else if fraction > 0.8 {
                    shades.warn
                } else {
                    shades.panel_hi
                };
                fill(
                    rect(screen, x, 142.0 - height, 8.0, height),
                    color,
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
                    link_color,
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
                    if index < 3 {
                        shades.battery_ok
                    } else {
                        shades.panel_hi
                    },
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
                        battery_color(value),
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
                centered(176.0, data.connection_label(), 9.0, link_color);
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
                        bar_battery_color(value)
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
                fill(rect(screen, 12.0, 208.0, 6.0, 6.0), link_color);
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
            // `render_concept_mediacenter` fills no card and draws no progress
            // bar (the host protocol carries no track position); it only rules a
            // line under the block with `C_BORDER`. The preview filled both with
            // the panel colour, which the dongle never puts on this screen.
            fill(rect(screen, 8.0, 92.0, 264.0, 1.0), data.borders);
            text_at(16.0, 24.0, "NOW PLAYING", 7.0, data.labels, egui::Align2::LEFT_CENTER);
            text_at(16.0, 46.0, "Boards of Canada", 15.0, data.text, egui::Align2::LEFT_CENTER);
            text_at(16.0, 68.0, "Roygbiv", 11.0, data.accent, egui::Align2::LEFT_CENTER);
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
                // `pbadge` of the media centre is a bare dot plus the badge text
                // (no card), so the preview must not wrap it in a panel.
                fill(rect(screen, 18.0, 184.0, 6.0, 6.0), link_color);
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
                fill(rect(screen, 176.0, 92.0, 6.0, 6.0), link_color);
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
                fill(rect(screen, 214.0, 17.0, 6.0, 6.0), link_color);
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
                        bar_battery_color(value),
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

    // The firmware spends `accent_dim` only on the dashboard (the shadow under
    // the layer word and the fill of an active modifier chip); there is no
    // bottom band on any concept screen, so the preview must not invent one.
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
            brightness: 100,
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
            // The shape-collecting tests describe the fallback painter; the
            // firmware renderer is compared against the goldens in
            // `firmware_parity`.
            paint_stylized_preview(ui, screen, data);
        });
        let mut frame = Frame::default();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut frame);
        }
        frame
    }

    #[test]
    fn every_concept_paints_a_distinct_frame() {
        let mut signatures: Vec<(usize, usize, u64)> = Vec::new();
        for concept in 0..14u8 {
            let frame = paint(&sample(concept));
            assert!(
                frame.text.iter().any(|value| value.contains("Base")),
                "concept {concept} must show the layer name: {:?}",
                frame.text
            );
            assert!(
                // Every frame paints at least the screen background; concepts
                // that show nothing but text add no other shape (the dongle has
                // neither a bezel nor a bottom band, so the preview has none).
                frame.rects >= 1,
                "concept {concept} painted only {} shapes",
                frame.rects
            );
            // Text lines, painted rectangles and a hash of the paint colours in
            // order: three numbers that differ as soon as a concept lays its
            // elements out differently or colours them differently.
            let mut fingerprint: u64 = 0xcbf2_9ce4_8422_2325;
            for fill in &frame.fills {
                for byte in fill.to_srgba_unmultiplied() {
                    fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
                }
            }
            signatures.push((frame.text.len(), frame.rects, fingerprint));
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
        // The sparkline paints every zone: the newest bar in the accent, the
        // loud ones in the warning colour and the rest in the raised panel
        // shade the firmware's `C_PANEL_HI` becomes.
        let mut data = sample(8);
        data.accent = Color32::from_rgb(1, 2, 3);
        data.warning = Color32::from_rgb(255, 0, 0);
        data.panel = Color32::from_rgb(0, 0, 255);
        let shades = data.shades();
        let frame = paint(&data);
        assert!(
            frame.fills.contains(&Color32::from_rgb(1, 2, 3)),
            "the accent colour must be painted"
        );
        assert!(
            frame.fills.contains(&shades.warn),
            "the threshold colour must be painted"
        );
        assert!(
            frame.fills.contains(&shades.panel_hi),
            "the raised panel shade must be painted"
        );
    }

    #[test]
    fn every_theme_keeps_its_elements_readable() {
        // 3:1 is the graphics threshold: bars, frames and thresholds are shapes,
        // not body text. Text and labels are checked against both the background
        // and the panel, which is what a light theme gets wrong first.
        for theme in QUBE_THEMES {
            // The factory palette is the firmware's own choice of shades and its
            // labels sit at ~2.7:1 on the panel - keeping "Dark" bit-identical to
            // the factory look matters more than raising it to the graphics
            // threshold, so only the designed presets are held to 3:1.
            let minimum = if theme.label_key == "qube_screen.theme_dark" {
                2.2
            } else {
                3.0
            };
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
                        ratio >= minimum,
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
            let channel = crate::app::app_state::qube_color_channel(qsid).expect("colour channel");
            values.insert(qsid, u16::from(five_six_five_channel(channel, value)));
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

    #[test]
    fn writing_a_theme_selects_exactly_that_theme() {
        // The device stores every channel through RGB565, which is why a strict
        // 8-bit comparison used to match no theme at all and the selector ended
        // up pointing past the list. Writing theme N must select theme N.
        let supported: Vec<u16> = (216..=250)
            .chain([318])
            .chain(320..=322)
            .chain(330..=332)
            .collect();
        for (index, candidate) in QUBE_THEMES.iter().enumerate() {
            let mut values = std::collections::BTreeMap::new();
            for (qsid, value) in candidate.writes(&supported) {
                let channel = crate::app::app_state::qube_color_channel(qsid).expect("colour channel");
                values.insert(qsid, u16::from(five_six_five_channel(channel, value)));
            }
            let active = QUBE_THEMES
                .iter()
                .position(|theme| theme.matches(&values));
            assert_eq!(active, Some(index), "{} selected the wrong entry", candidate.label_key);
        }
        // A palette that matches nothing must not select a neighbour.
        let mut custom = std::collections::BTreeMap::new();
        custom.insert(330, 1);
        assert_eq!(QUBE_THEMES.iter().position(|theme| theme.matches(&custom)), None);
    }

    #[test]
    fn dark_theme_is_the_firmware_factory_palette() {
        // DEFAULT_* constants of the Qube firmware (keyboards/common/layer_names.rs).
        assert_eq!(QUBE_THEME_DARK.accent, [24, 154, 255]);
        assert_eq!(QUBE_THEME_DARK.accent_shadow, [8, 65, 148]);
        assert_eq!(QUBE_THEME_DARK.background, [0, 8, 33]);
        assert_eq!(QUBE_THEME_DARK.panel, [16, 24, 74]);
        assert_eq!(QUBE_THEME_DARK.borders, [41, 52, 132]);
        assert_eq!(QUBE_THEME_DARK.text, [239, 247, 247]);
        assert_eq!(QUBE_THEME_DARK.labels, [90, 97, 165]);
        assert_eq!(QUBE_THEME_DARK.bar, [24, 170, 247]);
        assert_eq!(QUBE_THEME_DARK.warning, [255, 203, 0]);
        assert_eq!(QUBE_THEME_DARK.critical, [255, 21, 42]);
    }

    #[test]
    fn quantisation_matches_the_measured_device_values() {
    #[test]
    fn factory_shades_match_the_firmware_constants_exactly() {
        // The firmware's derived constants (qube_display.rs) expanded back to
        // eight bits: PANEL_HI (3,9,13), BORDER_DIM (3,8,11), LABEL (16,36,28),
        // DIM (5,12,14), ACCENT_DIM (1,16,18), BAR_BG (2,7,9), C_OK (6,50,10),
        // OUTPUT_OK (4,46,12), OUTPUT_SEARCH (4,24,31), C_TERM (4,56,10).
        let shades = sample(0).shades();
        for (name, shade, expected) in [
            ("panel_hi", shades.panel_hi, [24, 36, 107]),
            ("border_dim", shades.border_dim, [24, 32, 91]),
            ("label_hi", shades.label_hi, [132, 146, 231]),
            ("dim", shades.dim, [40, 48, 115]),
            ("accent_dim", shades.accent_dim, [8, 65, 148]),
            ("bar_bg", shades.bar_bg, [16, 28, 74]),
            ("battery_ok", shades.battery_ok, [49, 203, 82]),
            ("output_ok", shades.output_ok, [33, 186, 99]),
            ("output_search", shades.output_search, [33, 97, 255]),
            ("terminal", shades.terminal, [33, 227, 82]),
        ] {
            assert_eq!(
                [shade.r(), shade.g(), shade.b()],
                expected,
                "{name} must equal the firmware shade"
            );
        }
    }

        // Values observed on the live dongle: requested -> read back.
        for (channel, requested, read_back) in [
            (0, 80u8, 82u8),
            (0, 120, 123),
            (0, 225, 231),
            (1, 225, 227),
            (1, 90, 89),
            (1, 21, 20),
            (2, 42, 41),
            (0, 255, 255),
            (1, 255, 255),
            (2, 0, 0),
        ] {
            assert_eq!(
                five_six_five_channel(channel, requested),
                read_back,
                "channel {channel}: {requested} -> {read_back}"
            );
        }
    }
}
/// The firmware draws most elements with shades derived from the configured
/// colours in RGB565 space: a base colour plus the per-channel delta the two
/// factory constants had (`shift_of` in `qube_display.rs`). The preview applies
/// exactly the same quantise-then-shift path, so a preview shade equals the
/// painted one - a wrong base would show as a delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QubeShades {
    pub(crate) background: Color32,
    pub(crate) panel: Color32,
    pub(crate) panel_hi: Color32,
    pub(crate) border: Color32,
    pub(crate) border_dim: Color32,
    pub(crate) ink: Color32,
    pub(crate) label: Color32,
    pub(crate) label_hi: Color32,
    pub(crate) dim: Color32,
    pub(crate) accent: Color32,
    pub(crate) accent_dim: Color32,
    pub(crate) bar: Color32,
    pub(crate) bar_bg: Color32,
    pub(crate) warn: Color32,
    pub(crate) bad: Color32,
    pub(crate) battery_ok: Color32,
    pub(crate) output_ok: Color32,
    pub(crate) output_search: Color32,
    pub(crate) terminal: Color32,
}

/// Raw RGB565 of an eight-bit colour triple (red/blue five bits, green six).
fn raw565(triple: [u8; 3]) -> (i32, i32, i32) {
    (
        i32::from(triple[0]) >> 3,
        i32::from(triple[1]) >> 2,
        i32::from(triple[2]) >> 3,
    )
}

/// A base colour shifted by a per-channel delta, clamped like the firmware.
fn shift565(base: [u8; 3], delta: (i32, i32, i32)) -> Color32 {
    let (r, g, b) = raw565(base);
    let r = (r + delta.0).clamp(0, 31) as u8;
    let g = (g + delta.1).clamp(0, 63) as u8;
    let b = (b + delta.2).clamp(0, 31) as u8;
    Color32::from_rgb(
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    )
}

impl QubePreviewData {
    fn triple(color: Color32) -> [u8; 3] {
        [color.r(), color.g(), color.b()]
    }
    fn background_triple(&self) -> [u8; 3] {
        Self::triple(self.background)
    }
    fn panel_triple(&self) -> [u8; 3] {
        Self::triple(self.panel)
    }
    fn border_triple(&self) -> [u8; 3] {
        Self::triple(self.borders)
    }
    fn label_triple(&self) -> [u8; 3] {
        Self::triple(self.labels)
    }
    fn text_triple(&self) -> [u8; 3] {
        Self::triple(self.text)
    }
    fn accent_triple(&self) -> [u8; 3] {
        Self::triple(self.accent)
    }
    fn bar_triple(&self) -> [u8; 3] {
        Self::triple(self.bar)
    }
    fn warning_triple(&self) -> [u8; 3] {
        Self::triple(self.warning)
    }
    fn critical_triple(&self) -> [u8; 3] {
        Self::triple(self.critical)
    }

    /// Every shade the firmware can paint, derived from the configured colours.
    pub(crate) fn shades(&self) -> QubeShades {
        let panel = self.panel_triple();
        let border = self.border_triple();
        let label = self.label_triple();
        let accent = self.accent_triple();
        let bar = self.bar_triple();
        QubeShades {
            background: shift565(self.background_triple(), (0, 0, 0)),
            panel: shift565(panel, (0, 0, 0)),
            // PANEL_HI: (2,6,9) -> (3,9,13)
            panel_hi: shift565(panel, (1, 3, 4)),
            border: shift565(border, (0, 0, 0)),
            // BORDER_DIM: (5,13,16) -> (3,8,11)
            border_dim: shift565(border, (-2, -5, -5)),
            ink: shift565(self.text_triple(), (0, 0, 0)),
            label: shift565(label, (0, 0, 0)),
            // LABEL_HI: (11,24,20) -> (16,36,28)
            label_hi: shift565(label, (5, 12, 8)),
            // DIM: (11,24,20) -> (5,12,14)
            dim: shift565(label, (-6, -12, -6)),
            accent: shift565(accent, (0, 0, 0)),
            // ACCENT_DIM: (3,38,31) -> (1,16,18)
            accent_dim: shift565(accent, (-2, -22, -13)),
            bar: shift565(bar, (0, 0, 0)),
            // BAR_BG: (2,6,9) -> (2,7,9)
            bar_bg: shift565(panel, (0, 1, 0)),
            warn: shift565(self.warning_triple(), (0, 0, 0)),
            bad: shift565(self.critical_triple(), (0, 0, 0)),
            // BATTERY_OK: (3,42,30) -> (6,50,10)
            battery_ok: shift565(bar, (3, 8, -20)),
            // OUTPUT_OK: (3,42,30) -> (4,46,12)
            output_ok: shift565(bar, (1, 4, -18)),
            // OUTPUT_SEARCH: (3,38,31) -> (4,24,31)
            output_search: shift565(accent, (1, -14, 0)),
            // TERMINAL: (3,42,30) -> (4,56,10)
            terminal: shift565(bar, (1, 14, -20)),
        }
    }
}

// ---------------------------------------------------------------------------
// Firmware parity
// ---------------------------------------------------------------------------
//
// The stand `work/concepts-stand` compiles the *shipped* firmware file
// (`work/my-config/rmk-qube/qube_display.rs`) against host stubs and dumps one
// raw RGB565 frame per (concept, theme) together with its SHA-256
// (`--goldens <dir>`). Those files are the oracle in `outputs\entropy-qube\goldens`.
//
// The preview now renders through the same engine (`crate::qube_fw`), so the
// test below is exact: every one of the 14 concepts × 4 themes must produce the
// golden frame byte for byte. A failure writes a diff PNG and prints the pair.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod firmware_parity {
    use super::*;

    /// `outputs\entropy-qube\goldens` — raw frames and the hash table written by
    /// `qube-concepts-stand --goldens <dir>`.
    const GOLDEN_DIR: &str =
        r"C:\Users\Ilya\Documents\Codex\2026-10-06\g\outputs\entropy-qube\goldens";
    /// Diff images of the diagnostic report.
    const DIFF_DIR: &str =
        r"C:\Users\Ilya\Documents\Codex\2026-10-06\g\outputs\screen-preview\diff";

    /// `(concept, theme slug, SHA-256 of the raw little-endian RGB565 frame)`
    /// for the parity state (layer 0 "Base", 68 WPM, 73 %/41 %, USB, 09:41,
    /// "Boards of Canada - Roygbiv"). Generated by the stand.
    const GOLDENS: [(u8, &str, &str); 56] = [
        (0, "dark", "d3c4df09ebd675de1951e39bd5669e6bc857ad9f10434600c14b6ac9769f497a"),
        (1, "dark", "1d936a56be283f1a5c84d7abba846f2c77426b8ed44f810f295ff6b566e50e54"),
        (2, "dark", "390443d1f9d05d6dd606f38a20e43729763aa3dc5cef78ed40286e1ce04bcadb"),
        (3, "dark", "3e6629e2caa5c0111c238752c25941b8d1064c5ded9042a3a795bfb3b4048c70"),
        (4, "dark", "b46ae27036574a0c5ebd4e443e9eeef88d73eee4c84f0011b6e96359becebb1f"),
        (5, "dark", "cf84198ea6760f1dc9f81152452d935f13f15e9d1476b91856888ebf63c8af2f"),
        (6, "dark", "3f21c86ed7c5b872e5a748018e02813b655a31c3ee8c1e67279c70af99b8a63c"),
        (7, "dark", "c2257531db624b2cd8c14d0c991bd452aaf71e6cc2843de40f706fe74f40ed71"),
        (8, "dark", "a3ab3c9f95b6205735133d382123fe2e53b1715d2201885f7d6938606a91c59f"),
        (9, "dark", "46197435bcc6976f2cf9553a80f684f1734c69a2f2ad39b3e9932ac6ceda6916"),
        (10, "dark", "5f71e6932b1c157aa3949ad935cfc27873c17bac1e75a2cbbd72a2be7718bf9d"),
        (11, "dark", "14abbdd395c80d5453353ad27d02871c69cfa8355e48be4f8ae2ec4e8fa96914"),
        (12, "dark", "adf81bce0a7fd804761558cd8453f451a81902d7167c11c8384070012ee6e798"),
        (13, "dark", "fb3b3766f1f162e40577836666d662c56a9aeab7797938a0375328e45053456a"),
        (0, "light", "ea841baeef964429bca27a6f766b943d3d606525e9427cde1b38e3aeafceac3c"),
        (1, "light", "4c43825f539722c2ffb96c9c79cd6ec7053552478b7a819b187a7af1bd045615"),
        (2, "light", "0156a5035846b0c878659abf7034aacdfa740e2ba0d9a98bc16016ad11c3d75f"),
        (3, "light", "a767ca078188b08aac12979279cf73bd799e49132846a60015fe3ebd2480ddae"),
        (4, "light", "5b67282b8d383a86bd8ecec63ba5085c9fead12fcd3b7e78608bb6de47d9c6ad"),
        (5, "light", "8713e2fa30d7ea4695bf967ceaa3c77e4ee32246d86ea57d1848214254250afd"),
        (6, "light", "20640f2c5ce54ee32372307f2fe8fbeffb05a0ca7600750abe9f6f1314ab696b"),
        (7, "light", "d2b213364c3dab819647d215ebc1505c1959f6eb04430822aa3eaefa6ca28133"),
        (8, "light", "8991d57419064fb09168beaa58801e83c03659dc903e5bce20130d5a6834fe3c"),
        (9, "light", "d2d40ac4f0d005e5652efe499c6569cd595520b6808b028a594c59ada4db4db0"),
        (10, "light", "fcf7e1456bc7517a8102969d30e928fac7f7336f874818d69a96c69ce58c03aa"),
        (11, "light", "ab69bafb06eb218c53088adf6396ae25e0b7a0664bc9f7def7c0a5967b3250ad"),
        (12, "light", "540d3b2a098f6a31ca3822c1a74b27d9b4b758540174384dd8fa7ebd6d80879d"),
        (13, "light", "e4f3e865e915b4306c3e2747a2e29f4dbc38f68b5fc3a2cdf5407ad87c728789"),
        (0, "phosphor", "6b207d8c2c937db297880469dbffc6cb9aeff27c94ab386817fd6c1844001e67"),
        (1, "phosphor", "a8bc143e2cfbac2de05a39f1de3dfa7ac7f7ce3d7eaff773e60b3cf0d5b38b94"),
        (2, "phosphor", "b5efea83154b7dc28b314a51a94e5b082762ed41b31a782458625cb1cc757715"),
        (3, "phosphor", "9629fa867644dcae99d82ed72a23dce4a9c9b50ba13e977445556ce00b362e3a"),
        (4, "phosphor", "b4eb0e2f715123ea9d54ae0558b48169ef033e195226f06c0af028cd165330d6"),
        (5, "phosphor", "5f448ea23c5594efb0073c9ba321b0ab25b52855eaab27a376ed9a7c087527bf"),
        (6, "phosphor", "170b6d9b342d2437f68974327d72b63270e8e69971898178b1857bd9847d49b0"),
        (7, "phosphor", "81149cdb5c01b436bdbb4790fb2623bc98941af7126823e73dad3903f9e2be46"),
        (8, "phosphor", "a4b9b67f6920dca2e9ed753e1722f3109ea82da7e6e5651b98145505fe5dc9b5"),
        (9, "phosphor", "2904195bde35d7cacf23ca6f5958cc48dcd06232e3231e59d1c127a417f82df9"),
        (10, "phosphor", "db38816774cfe73409f4a4ce07bcce414e738a4af0e37fe0bab830081b16dc77"),
        (11, "phosphor", "03482c1e01f6b4acf662f93a24fead252676dd83c76f61d61fd3ff970aff0c34"),
        (12, "phosphor", "0febbc5552b92d0671f651b7fb985fb3241509abba2b887f52494a5f40fc4195"),
        (13, "phosphor", "f8a4476ecab402577a24152adf7f60919784d6b071d43ff1a031fd015f967101"),
        (0, "amber", "3c059dabd393b41d8bbda4bdc07e07631f4349da989bc0bb6e9e97afc308ef6d"),
        (1, "amber", "fc9cb1f30610905f5e056bc9893197e28cc3cf9c2fb8f0b78f8aa781897402ad"),
        (2, "amber", "2be61abe99170b3c6367171419a1fd63e13e2d7bd4e34aa586f444cf8f007d36"),
        (3, "amber", "625dbbc2be51dc2b9a735d9b0c7652377320efa04d59066d04244fdb05e9a455"),
        (4, "amber", "6b7e480c1e4f07c871d2faa0937ca99d5f6fa10d280e0ef2f7c0a3a770de79ad"),
        (5, "amber", "66ab878c820cb10161e8d182a04a3ec375df268243c9e9ba5372237cf7ef5604"),
        (6, "amber", "43eba33da6466f811230f008956545d2c4bea7f21ea32c03fb80cd570b350f44"),
        (7, "amber", "cd5c5ef43a2baafbee8b4132c83d5649facfe40bb0cb0f771bb75d99709d51e7"),
        (8, "amber", "845fae64f86b75aa877f6d5dfee9fd10a018f0e3f70039d9aaacfa2b44717721"),
        (9, "amber", "3e3adbb253465573b970ad4b4c694f76eb0a20876f0319f86943a2890e95809e"),
        (10, "amber", "398394f53231bb53035fc16003a8a9d20b2f5e6d74ef6d19baa177d7af11f9ef"),
        (11, "amber", "dbf7cc30557f6219ce5509cec24b226443316080028f1745871e9fcfb50d016a"),
        (12, "amber", "db1cd94894300f68c45adcf58d61875d45786184866a26a8b30d096516aeed2f"),
        (13, "amber", "5e18a26f5d74770c0672e4e5cd605da365a5265aeafa092809a3f393cfc92320"),
    ];

    /// Theme order of `QUBE_THEMES`, with the slug the golden files use.
    const THEME_SLUGS: [&str; 4] = ["dark", "light", "phosphor", "amber"];

    /// The preview data the app builds after writing `theme`: every channel is
    /// quantised to RGB565, which is what the device reads back.
    fn data_for(concept: u8, theme: &QubeThemePalette) -> QubePreviewData {
        let colour = |triple: [u8; 3]| {
            Color32::from_rgb(
                five_six_five_channel(0, triple[0]),
                five_six_five_channel(1, triple[1]),
                five_six_five_channel(2, triple[2]),
            )
        };
        QubePreviewData {
            concept,
            show_wpm: true,
            show_modifiers: true,
            show_batteries: true,
            show_connection: true,
            header_mode: 2,
            badge_place: 0,
            brightness: 100,
            accent: colour(theme.accent),
            accent_shadow: colour(theme.accent_shadow),
            background: colour(theme.background),
            panel: colour(theme.panel),
            borders: colour(theme.borders),
            text: colour(theme.text),
            labels: colour(theme.labels),
            bar: colour(theme.bar),
            warning: colour(theme.warning),
            critical: colour(theme.critical),
            layer_name: "Base".to_owned(),
            layer_index: 0,
            battery_left: 73,
            battery_right: 41,
            bluetooth: false,
        }
    }

    /// Eight-bit expansion of a frame the firmware wrote in RGB565.
    fn expand565(raw: &[u8]) -> Vec<[u8; 3]> {
        raw.chunks_exact(2)
            .map(|pair| {
                let word = u16::from_le_bytes([pair[0], pair[1]]);
                let r = ((word >> 11) & 0x1F) as u8;
                let g = ((word >> 5) & 0x3F) as u8;
                let b = (word & 0x1F) as u8;
                [(r << 3) | (r >> 2), (g << 2) | (g >> 4), (b << 3) | (b >> 2)]
            })
            .collect()
    }

    fn golden_frame(concept: u8, slug: &str) -> Option<Vec<u8>> {
        std::fs::read(format!("{GOLDEN_DIR}\\{concept}-{slug}.bin")).ok()
    }

    fn theme(index: usize) -> QubeThemePalette {
        QUBE_THEMES[index]
    }

    /// 09:41 — the host clock the oracle frames were rendered with.
    const PARITY_CLOCK: Option<(u8, u8)> = Some((9, 41));

    /// Every screen the settings page can show must equal the frame the
    /// firmware paints for the same state — the same bytes, not "close". The
    /// table is printed for the report; a mismatch writes a diff PNG.
    #[test]
    fn frames_match_the_firmware_byte_for_byte() {
        let mut lines = vec!["концепция  тема       дельта, пикс.  sha256 превью".to_owned()];
        let mut identical = 0usize;
        let mut failures = Vec::new();
        for (index, slug) in THEME_SLUGS.iter().enumerate() {
            let theme = theme(index);
            for concept in 0..14u8 {
                let data = data_for(concept, &theme);
                let frame = preview_frame(&data, PARITY_CLOCK);
                let actual = frame.rgb565_bytes();
                let hash = sha256_hex(&actual);
                let expected = GOLDENS
                    .iter()
                    .find(|(id, theme_slug, _)| *id == concept && theme_slug == slug)
                    .map(|(_, _, hash)| *hash)
                    .expect("golden");
                let Some(reference) = golden_frame(concept, slug) else {
                    failures.push(format!(
                        "{concept}/{slug}: нет эталона {GOLDEN_DIR}\\{concept}-{slug}.bin"
                    ));
                    continue;
                };
                let delta = actual
                    .iter()
                    .zip(reference.iter())
                    .filter(|(a, b)| a != b)
                    .count()
                    / 2;
                lines.push(format!(
                    "{concept:<10} {slug:<9} {delta:>13}  {}",
                    &hash[..16]
                ));
                if delta != 0 || hash != expected {
                    let preview_rgb = expand565(&actual);
                    let _ = write_diff(concept, slug, &preview_rgb, &expand565(&reference));
                    failures.push(format!(
                        "{concept}/{slug}: {delta} пикс. отличаются (sha {hash} против {expected})"
                    ));
                } else {
                    identical += 1;
                }
                if frame.dropped != 0 {
                    failures.push(format!(
                        "{concept}/{slug}: {} пикселей за пределами панели",
                        frame.dropped
                    ));
                }
            }
        }
        for line in &lines {
            println!("{line}");
        }
        println!("кадров: 56, совпало байт в байт: {identical}");
        assert_eq!(identical, 56, "не все кадры совпали с прошивкой");
        assert!(
            failures.is_empty(),
            "превью расходится с кадрами прошивки:\n  {}",
            failures.join("\n  ")
        );
    }

    /// Ink of a region — `(pixels differing from the background, min x, min y,
    /// max x, max y)`. The background is the colour of the top-left pixel, which
    /// every concept clears the panel with.
    fn ink(
        frame: &crate::qube_fw::HostLcd,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
    ) -> (usize, i32, i32, i32, i32) {
        let background = frame.pixel(0, 0);
        let mut count = 0usize;
        let mut min = (i32::MAX, i32::MAX);
        let mut max = (i32::MIN, i32::MIN);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if frame.pixel(x, y) == background {
                    continue;
                }
                count += 1;
                min = (min.0.min(x), min.1.min(y));
                max = (max.0.max(x), max.1.max(y));
            }
        }
        (count, min.0, min.1, max.0, max.1)
    }

    /// Frame of one concept in one theme, first proved to be the firmware's own:
    /// the acceptance cases below only mean something on a frame that really
    /// equals the golden.
    fn checked_frame(concept: u8, index: usize, slug: &str) -> crate::qube_fw::HostLcd {
        let frame = preview_frame(&data_for(concept, &theme(index)), PARITY_CLOCK);
        let golden = golden_frame(concept, slug).expect("эталон");
        assert_eq!(
            frame.rgb565_bytes(),
            golden,
            "{concept}/{slug}: кадр предпросмотра не равен кадру прошивки"
        );
        frame
    }

    /// Reported: the app wrote the two-column clock into the *left* column
    /// (`text_at(12.0, 206.0, "09:41", …)` in the stylised painter), while the
    /// firmware writes it under the battery cards, in the right column
    /// (`ctext8(d, &st.time(), 158, 204, C_DIM)`).
    /// Visual evidence: `goldens\7-dark.png`.
    #[test]
    fn concept7_clock_lives_in_the_right_column() {
        let (x, y, width, height) = crate::qube_fw::twocol_clock_box();
        let divider = crate::qube_fw::twocol_divider_x();
        for (index, slug) in THEME_SLUGS.iter().enumerate() {
            let frame = checked_frame(7, index, slug);
            let (clock, min_x, min_y, max_x, max_y) =
                ink(&frame, x, y, x + width - 1, y + height - 1);
            assert!(clock > 0, "7/{slug}: часов нет на месте своей строки");
            assert!(
                min_x > divider,
                "7/{slug}: часы начинаются левее разделителя ({min_x} <= {divider})"
            );
            assert!(
                min_y >= y && max_y < y + height && max_x < x + width,
                "7/{slug}: часы вылезли из своего бокса ({min_x},{min_y})-({max_x},{max_y})"
            );
            // The stylised painter's slot — the bug — has to stay background.
            let (stray, ..) = ink(&frame, 12, y, divider - 1, y + height - 1);
            assert_eq!(
                stray, 0,
                "7/{slug}: в левой колонке на строке часов что-то нарисовано"
            );
        }
    }

    /// Reported: the app wrote the speedometer's media ticker at the bottom
    /// *left* (`text_at(12.0, 214.0, …)`), while the firmware centres it under
    /// the rule (`ctext6c(d, &line, SPEEDO_CENTRE, SPEEDO_MEDIA_Y, C_INK)`).
    /// Visual evidence: `goldens\5-dark.png`.
    #[test]
    fn concept5_media_ticker_is_centred() {
        let (rule_y, media_y, centre, glyph_h) = crate::qube_fw::speedo_media_row();
        let last_x = crate::qube_fw::SCREEN_W_PX as i32 - 1;
        for (index, slug) in THEME_SLUGS.iter().enumerate() {
            let frame = checked_frame(5, index, slug);
            // The rule above the ticker spans the panel's safe area.
            let (rule, rule_x0, _, rule_x1, _) = ink(&frame, 0, rule_y, last_x, rule_y);
            assert_eq!(
                (rule, rule_x0, rule_x1),
                (240, 20, 259),
                "5/{slug}: линейка над тикером не на месте"
            );
            // The ticker itself: inside its glyph box and centred on the panel.
            let (ticker, min_x, min_y, max_x, max_y) =
                ink(&frame, 0, media_y, last_x, media_y + glyph_h - 1);
            assert!(ticker > 0, "5/{slug}: строки медиа нет");
            assert!(
                min_y >= media_y && max_y < media_y + glyph_h,
                "5/{slug}: тикер вылез из своей строки ({min_y}..{max_y})"
            );
            let ink_centre = (min_x + max_x) / 2;
            assert!(
                (ink_centre - centre).abs() <= 2,
                "5/{slug}: тикер не по центру — чернила {min_x}..{max_x}, центр {ink_centre} против {centre}"
            );
        }
    }

    /// Reported: the app printed a second battery summary (`L 73% R 41%`) in
    /// the bottom right corner of the speedometer. The firmware draws the two
    /// battery cards once, in the upper right, and leaves everything below the
    /// media ticker empty (`speedo_bottom` draws nothing there).
    #[test]
    fn concept5_has_no_second_battery_row() {
        let (_, media_y, _, glyph_h) = crate::qube_fw::speedo_media_row();
        let first_empty = media_y + glyph_h;
        let last_y = crate::qube_fw::SCREEN_H_PX as i32 - 1;
        for (index, slug) in THEME_SLUGS.iter().enumerate() {
            let frame = checked_frame(5, index, slug);
            let (stray, min_x, min_y, max_x, max_y) =
                ink(&frame, 0, first_empty, crate::qube_fw::SCREEN_W_PX as i32 - 1, last_y);
            assert_eq!(
                stray, 0,
                "5/{slug}: под тикером дорисовано ({min_x},{min_y})-({max_x},{max_y})"
            );
        }
    }

    /// The panel must really blit the firmware frame: one headless egui pass has
    /// to upload the whole 280×240 frame as a texture and draw it with nearest
    /// filtering — an integer scale is what keeps the firmware pixels sharp.
    #[test]
    fn the_panel_blits_the_firmware_frame() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(SCREEN_WIDTH, SCREEN_HEIGHT),
            )),
            ..Default::default()
        };
        let output = ctx.run_ui(input, |ui| {
            let screen =
                egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(SCREEN_WIDTH, SCREEN_HEIGHT));
            paint_qube_screen_preview(ui, screen, &data_for(5, &QUBE_THEME_DARK));
        });

        let (id, delta) = output
            .textures_delta
            .set
            .first()
            .expect("кадр не загружен как текстура");
        assert_eq!(
            delta.image.size(),
            [SCREEN_WIDTH as usize, SCREEN_HEIGHT as usize],
            "текстура не во весь кадр"
        );
        assert_eq!(
            delta.options.magnification,
            egui::TextureFilter::Nearest,
            "кадр растягивается сглаживающим фильтром"
        );
        assert!(delta.pos.is_none(), "кадр должен загружаться целиком");
        let blitted = output.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Mesh(mesh) => mesh.texture_id == *id,
            _ => false,
        });
        assert!(blitted, "кадр загружен, но не нарисован");
    }

    /// The two firmware files are copies, not forks: apart from the documented
    /// adaptations (the `//!` → `//` header rewrite and the one `rmk::config`
    /// import) they have to stay identical to the origin.
    #[test]
    fn firmware_sources_are_unmodified() {
        let origin = std::path::Path::new(
            r"C:\Users\Ilya\Documents\Codex\2026-10-06\g\work\my-config\rmk-qube",
        );
        if !origin.is_dir() {
            eprintln!("источник {} недоступен — проверка пропущена", origin.display());
            return;
        }
        let normalize = |text: &str| {
            text.replace("\r\n", "\n")
                .replace("//!", "//")
                .replace("use rmk::config::", "use crate::qube_fw::stubs::rmk::config::")
        };
        for (copy, name) in [
            (include_str!("../qube_fw/qube_display.rs"), "qube_display.rs"),
            (include_str!("../qube_fw/layer_names.rs"), "layer_names.rs"),
        ] {
            let expected = std::fs::read_to_string(origin.join(name)).expect("источник");
            assert_eq!(
                normalize(copy),
                normalize(&expected),
                "{name} отклонился от исходника прошивки"
            );
        }
    }

    fn write_diff(concept: u8, slug: &str, preview: &[[u8; 3]], firmware: &[[u8; 3]]) {
        let width = SCREEN_WIDTH as u32;
        let height = SCREEN_HEIGHT as u32;
        let mut image = image::RgbImage::new(width, height);
        let mut preview_image = image::RgbImage::new(width, height);
        for (index, (a, b)) in preview.iter().zip(firmware.iter()).enumerate() {
            let (x, y) = (index as u32 % width, index as u32 / width);
            preview_image.put_pixel(x, y, image::Rgb(*a));
            if a != b {
                image.put_pixel(x, y, image::Rgb([255, 0, 0]));
            } else {
                let grey = ((u32::from(a[0]) + u32::from(a[1]) + u32::from(a[2])) / 8) as u8;
                image.put_pixel(x, y, image::Rgb([grey, grey, grey]));
            }
        }
        // The firmware frames themselves are written by the stand
        // (`qube-concepts-stand --goldens`) into `outputs\entropy-qube\goldens`.
        let _ = std::fs::create_dir_all(DIFF_DIR);
        let _ = image.save(format!("{DIFF_DIR}\\{concept}-{slug}.png"));
        let _ = preview_image.save(format!("{DIFF_DIR}\\{concept}-{slug}-preview.png"));
    }

    /// SHA-256 (FIPS 180-4). Verbatim twin of
    /// `work/concepts-stand/src/sha256.rs`: the oracle and this test must hash
    /// the frame the same way, so the code is duplicated instead of adding a
    /// crate the offline registry does not carry.
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn sha256_compress(state: &mut [u32; 8], block: &[u8]) {
        let mut w = [0u32; 64];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let base = index * 4;
            *word = u32::from_be_bytes([
                block[base],
                block[base + 1],
                block[base + 2],
                block[base + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 =
                w[index - 2].rotate_right(17) ^ w[index - 2].rotate_right(19) ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }
        let mut v = *state;
        for index in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
            let temp1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let temp2 = s0.wrapping_add(maj);
            v[7] = v[6];
            v[6] = v[5];
            v[5] = v[4];
            v[4] = v[3].wrapping_add(temp1);
            v[3] = v[2];
            v[2] = v[1];
            v[1] = v[0];
            v[0] = temp1.wrapping_add(temp2);
        }
        for index in 0..8 {
            state[index] = state[index].wrapping_add(v[index]);
        }
    }

    fn sha256_hex(data: &[u8]) -> String {
        let mut state: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut chunks = data.chunks_exact(64);
        for chunk in &mut chunks {
            sha256_compress(&mut state, chunk);
        }
        let tail = chunks.remainder();
        let mut last = [0u8; 128];
        last[..tail.len()].copy_from_slice(tail);
        last[tail.len()] = 0x80;
        let total_bits = (data.len() as u64).wrapping_mul(8);
        let block_len = if tail.len() + 9 <= 64 { 64 } else { 128 };
        last[block_len - 8..block_len].copy_from_slice(&total_bits.to_be_bytes());
        sha256_compress(&mut state, &last[..64]);
        if block_len == 128 {
            sha256_compress(&mut state, &last[64..128]);
        }
        state.iter().map(|word| format!("{word:08x}")).collect()
    }

    #[test]
    fn sha256_matches_the_known_vectors() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
