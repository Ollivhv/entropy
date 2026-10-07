// ---------------------------------------------------------------------------
// Host glue for the ported firmware renderer.
//
// Included *inside* `crate::qube_fw::display`, right after the firmware file,
// so it reaches the private concept renderer, the palette and the look
// post-pass — the same arrangement `work/concepts-stand/src/host_glue.rs` uses
// against the same firmware file. Only the panel model and the assembly of the
// renderer's input live here; every pixel is drawn by the firmware code above.
//
// Adapted from the stand's glue:
//   * `MockLcd` became `HostLcd` (same clipping semantics, minus the stripe
//     band: the preview always paints a whole frame),
//   * the state builders now take a `PreviewInput` instead of the stand's
//     `calm` / `busy` demo states,
//   * the media ticker phase is pinned to zero, the phase of the golden frames
//     (the firmware derives it from the panel's uptime, which a preview of a
//     still frame does not have).
// ---------------------------------------------------------------------------

use crate::qube_fw::{Link, PreviewInput};

/// Panel size in pixels: the full landscape frame after `Deg90`.
pub const SCREEN_W_PX: u32 = SCREEN_W as u32;
pub const SCREEN_H_PX: u32 = SCREEN_H as u32;

/// Whole-panel host target. Clipping is the firmware's: a pixel outside the
/// panel is dropped (and counted), everything else lands in the buffer.
pub struct HostLcd {
    pub pixels: Vec<Rgb565>,
    /// Pixels the firmware tried to draw outside the panel.
    pub dropped: u64,
}

impl HostLcd {
    pub fn new() -> Self {
        Self {
            pixels: vec![Rgb565::new(0, 0, 0); SCREEN_W_PX as usize * SCREEN_H_PX as usize],
            dropped: 0,
        }
    }

    pub fn pixel(&self, x: i32, y: i32) -> Rgb565 {
        if x < 0 || y < 0 || x >= SCREEN_W_PX as i32 || y >= SCREEN_H_PX as i32 {
            return Rgb565::new(0, 0, 0);
        }
        self.pixels[(y as u32 * SCREEN_W_PX + x as u32) as usize]
    }

    fn put(&mut self, x: i32, y: i32, color: Rgb565) {
        if x < 0 || y < 0 || x >= SCREEN_W_PX as i32 || y >= SCREEN_H_PX as i32 {
            self.dropped += 1;
            return;
        }
        self.pixels[(y as u32 * SCREEN_W_PX + x as u32) as usize] = color;
    }
}

impl Default for HostLcd {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginDimensions for HostLcd {
    fn size(&self) -> Size {
        Size::new(SCREEN_W_PX, SCREEN_H_PX)
    }
}

impl DrawTarget for HostLcd {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(point, color) in pixels {
            self.put(point.x, point.y, color);
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Self::Error> {
        let x0 = area.top_left.x;
        let y0 = area.top_left.y;
        let x1 = x0 + area.size.width as i32;
        let y1 = y0 + area.size.height as i32;
        for y in y0..y1 {
            for x in x0..x1 {
                self.put(x, y, color);
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Rgb565) -> Result<(), Self::Error> {
        self.pixels.fill(color);
        Ok(())
    }
}

/// WPM samples the preview feeds to the `sparkline` concept: the harness values
/// the oracle rendered its golden frames with
/// (`work/concepts-stand/src/host_glue.rs`), so the preview and the oracle draw
/// the same bars.
const PREVIEW_WPM_HISTORY: [u8; WPM_HISTORY_LEN] = [
    12, 20, 28, 24, 41, 55, 48, 63, 74, 66, 88, 96, 84, 102, 118, 106, 120, 112, 118, 126, 118,
    120, 112, 120,
];

/// Pushes as much of `value` as fits — the same rule `push_ascii_text` applies
/// to the labels the firmware builds internally.
fn push_chars<const N: usize>(buffer: &mut heapless::String<N>, value: &str) {
    for ch in value.chars() {
        if buffer.push(ch).is_err() {
            break;
        }
    }
}

/// The settings snapshot the firmware reads back from its QSIDs.
fn settings_from(input: &PreviewInput) -> ScreenSettings {
    let mut colors = [0u16; layer_names::COLOR_COUNT];
    for (index, triple) in input.colors.iter().enumerate() {
        colors[index] = layer_names::rgb8_to_565(*triple);
    }
    let mut settings = ScreenSettings::default();
    settings.wpm_visible = input.show_wpm;
    settings.header_mode = input.header_mode.min(layer_names::SCREEN_HEADER_BOTH);
    settings.timeout_s = 0;
    settings.show_modifiers = input.show_modifiers;
    settings.show_batteries = input.show_batteries;
    settings.output_visible = input.show_output;
    settings.output_place = input.output_place.min(SCREEN_OUTPUT_CHIP);
    settings.concept = input.concept.min(SCREEN_CONCEPT_MAX);
    settings.brightness = input
        .brightness
        .clamp(layer_names::SCREEN_BRIGHTNESS_MIN, SCREEN_BRIGHTNESS_MAX);
    settings.colors = colors;
    settings
}

/// `(level, connected)` → the event payload the dongle builds from
/// `PeripheralBatteryEvent` + `PeripheralConnectedEvent`.
fn battery_status((level, connected): (Option<u8>, bool)) -> BatteryStatus {
    match (level, connected) {
        (Some(percent), true) => BatteryStatus::available(Some(percent)),
        (None, true) => BatteryStatus::available(None),
        _ => BatteryStatus::UNAVAILABLE,
    }
}

/// `RenderContext` of one frame, assembled like the dongle assembles it.
fn context_from(input: &PreviewInput) -> RenderContext {
    let mut ctx = RenderContext::default();
    ctx.layer = input.layer;
    ctx.wpm = input.wpm;
    ctx.caps_lock = input.caps_lock;
    ctx.num_lock = input.num_lock;
    ctx.sleeping = false;
    ctx.modifiers = rmk::types::ModifierCombination::from_bits(input.modifiers);
    ctx.peripherals_connected = [input.batteries[0].1, input.batteries[1].1];
    ctx.peripheral_batteries = [
        BatteryStatusEvent(battery_status(input.batteries[0])),
        BatteryStatusEvent(battery_status(input.batteries[1])),
    ];
    ctx
}

/// `ConnectionStatus` plus the advertising mode the badge derives its text from.
fn connection_for(link: Link) -> (ConnectionStatus, Option<BleAdvertisingMode>) {
    let mut connection = ConnectionStatus::default();
    match link {
        Link::Usb => {
            connection.usb = rmk_types::connection::UsbState::Configured;
            connection.preferred = ConnectionType::Usb;
            (connection, None)
        }
        Link::Ble { profile } => {
            connection.ble = rmk_types::ble::BleStatus {
                profile: profile.min(9),
                state: BleState::Connected,
            };
            connection.preferred = ConnectionType::Ble;
            (connection, None)
        }
        Link::Pairing { profile } => {
            connection.ble = rmk_types::ble::BleStatus {
                profile: profile.min(9),
                state: BleState::Advertising,
            };
            (connection, Some(BleAdvertisingMode::Pairing))
        }
        Link::Reconnecting { profile } => {
            connection.ble = rmk_types::ble::BleStatus {
                profile: profile.min(9),
                state: BleState::Advertising,
            };
            (connection, Some(BleAdvertisingMode::Reconnecting))
        }
        Link::Idle => (connection, None),
    }
}

/// The connection indicator's own state, as the concepts read it.
fn output_state(link: Link) -> (OutputState, u8) {
    match link {
        Link::Usb => (OutputState::Usb, 0),
        Link::Ble { profile } => (OutputState::Ble { profile: profile.min(9) }, profile.min(9)),
        Link::Pairing { profile } => (
            OutputState::Pairing { profile: profile.min(9) },
            profile.min(9),
        ),
        Link::Reconnecting { profile } => (
            OutputState::Reconnecting { profile: profile.min(9) },
            profile.min(9),
        ),
        Link::Idle => (OutputState::Idle, 0),
    }
}

/// Host data the firmware reads: the clock and the media line.
fn host_data_from(input: &PreviewInput) -> rmk::host_data::HostData {
    let mut data = rmk::host_data::HostData::default();
    data.hour = input.hour;
    data.minute = input.minute;
    // The app gets one line, not an artist/title pair, so the whole line goes
    // in as the artist: `push_media_label` then builds exactly that text.
    push_chars(&mut data.media_artist, &input.media);
    data
}

/// `ConceptState` of the concept screens, built like the stand's `parity_state`:
/// the values the app knows come from the device, the two it cannot know (WPM
/// history and the ticker phase) are the ones the oracle frames used.
fn state_from(input: &PreviewInput, settings: &ScreenSettings) -> ConceptState {
    let mut name: heapless::String<CONCEPT_NAME_MAX> = heapless::String::new();
    push_chars(&mut name, &input.layer_name);
    let mut media: heapless::String<CONCEPT_MEDIA_MAX> = heapless::String::new();
    push_chars(&mut media, &input.media);
    let (link, link_profile) = output_state(input.link);
    ConceptState {
        layer: input.layer,
        name,
        wpm: input.wpm,
        mods: input.modifiers,
        caps_lock: input.caps_lock,
        num_lock: input.num_lock,
        link,
        link_profile,
        batteries: input.batteries,
        hour: input.hour,
        minute: input.minute,
        // The pinned RMK host protocol carries no date.
        date: None,
        media,
        media_offset: 0,
        sleeping: false,
        wpm_history: PREVIEW_WPM_HISTORY,
        show_wpm: settings.wpm_visible,
        show_modifiers: settings.show_modifiers,
        show_batteries: settings.show_batteries,
        show_clock: settings.shows_clock(),
        show_media: settings.shows_media(),
        show_output: settings.output_visible,
        compact_output: settings.output_place == SCREEN_OUTPUT_CHIP,
        left_label: settings.left_label,
        right_label: settings.right_label,
    }
}

/// The firmware's own look pass (palette substitution, software brightness,
/// idle blank) over a finished frame — the same `look_pixel` the panel driver
/// runs per stripe.
fn apply_look(lcd: &mut HostLcd, settings: &ScreenSettings, blank: bool) {
    let look = Look::from_settings(settings, blank);
    for pixel in lcd.pixels.iter_mut() {
        let raw = look_pixel(pixel.into_storage(), &look);
        *pixel = Rgb565::new(
            ((raw >> 11) & 0x1F) as u8,
            ((raw >> 5) & 0x3F) as u8,
            (raw & 0x1F) as u8,
        );
    }
}

/// One frame exactly as the panel shows it: the dashboard goes through the real
/// renderer (it is the one screen the renderer draws itself), every concept
/// through `render_concept`, and both through the look pass.
pub fn render_frame(input: &PreviewInput) -> HostLcd {
    let settings = settings_from(input);
    let mut lcd = HostLcd::new();
    if settings.concept == SCREEN_CONCEPT_DASHBOARD {
        let mut renderer = QubeStatusRenderer::new(host_data_from(input), false);
        let (connection, advertising) = connection_for(input.link);
        renderer.settings = settings;
        renderer.connection = connection;
        renderer.advertising = advertising;
        renderer.host_data = host_data_from(input);
        renderer.render(&context_from(input), &mut lcd);
    } else {
        let state = state_from(input, &settings);
        render_concept(&mut lcd, settings.concept, &state);
    }
    apply_look(&mut lcd, &settings, false);
    lcd
}

// --- Layout facts the acceptance tests check against -------------------------
//
// The three reported preview bugs were all layout: a clock in the wrong column
// and a duplicated, left-aligned battery/media row on the speedometer. The
// tests below check the pens those elements are written with, so the literals
// stay in one place — here, next to the firmware calls they mirror:
//
//   * `render_concept_twocol`: `ctext8(d, &st.time(), 158, 204, C_DIM)`,
//     `cfill(d, 139, 16, 2, 208, C_BORDER)`;
//   * `speedo_bottom`:      `ctext6c(d, &line, SPEEDO_CENTRE, SPEEDO_MEDIA_Y, …)`.

/// Glyph box `(x, y, width, height)` of the two-column screen's clock: the
/// right column, five `HH:MM` characters of FONT_8X13.
pub fn twocol_clock_box() -> (i32, i32, i32, i32) {
    const CLOCK_CHARS: i32 = 5;
    (
        158,
        204,
        FONT_8X13.character_size.width as i32 * CLOCK_CHARS,
        FONT_8X13.character_size.height as i32,
    )
}

/// Column the divider of the two-column screen is drawn at.
pub fn twocol_divider_x() -> i32 {
    139
}

/// Speedometer bottom band: `(rule row, ticker row, ticker centre x, ticker
/// glyph height)`. The ticker is the *only* thing the firmware draws down
/// there; below it the panel is empty.
pub fn speedo_media_row() -> (i32, i32, i32, i32) {
    (
        SPEEDO_MEDIA_RULE_Y,
        SPEEDO_MEDIA_Y,
        SPEEDO_CENTRE,
        FONT_6X10.character_size.height as i32,
    )
}
