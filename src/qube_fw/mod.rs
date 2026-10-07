//! The Qube screen renderer of the *firmware*, compiled into the app.
//!
//! The preview panel of the Qube screen settings page must show the frame the
//! dongle actually paints, so instead of re-drawing the concepts with egui this
//! module compiles the firmware's own display engine and runs it against a
//! host panel buffer. Anything the preview shows is therefore drawn by the same
//! code, with the same fonts, colours, geometry and post-pass the device runs.
//!
//! ## Provenance
//!
//! | file | origin |
//! |---|---|
//! | [`layer_names`] | `work/my-config/rmk-qube/layer_names.rs` (branch `concepts`), verbatim except the one `rmk::config` import below |
//! | [`display`] (`qube_display.rs`) | `work/my-config/rmk-qube/qube_display.rs`, verbatim except the `//!` → `//` prefix on its header comment |
//! | [`display`] (`host_glue.rs`) | adapted from `work/concepts-stand/src/host_glue.rs` (the oracle that renders the golden frames) |
//! | [`stubs`] | `work/qube-preview/src/stubs.rs`, with two documented adaptations |
//! | [`default_layer_names`] | stand-in for the firmware helper of the same name, copied from `work/concepts-stand/src/main.rs` |
//!
//! Two changes come with the copies, both of them mechanics of the host build:
//!
//! * the `//!` → `//` rewrite on the header of `qube_display.rs` — `include!`
//!   cannot carry inner doc comments (rustc E0753), which is exactly why the
//!   host stand keeps a staged copy with the same rewrite;
//! * `layer_names.rs` imports its Vial device-settings types from the stand-in
//!   module (`crate::qube_fw::stubs::rmk::config`) instead of `rmk::config`,
//!   because the host build has no `rmk` crate — the host stand patches the
//!   same line.
//!
//! The test
//! `crate::ui::qube_screen_preview::firmware_parity::firmware_sources_are_unmodified`
//! re-derives both files from the origin and fails if anything else drifts.
//!
//! ## Why the crate root carries re-exports
//!
//! `qube_display.rs` and `layer_names.rs` address their neighbours exactly as
//! they do inside the firmware crate — `crate::layer_names::…`,
//! `crate::DEFAULT_LAYER_NAMES`, `crate::default_layer_names::…`. `main.rs`
//! mounts this module and re-exports those three names so the firmware files
//! stay untouched.

#![allow(dead_code)]

pub mod stubs;

/// Verbatim copy of the firmware's settings module (`layer_names.rs`).
#[allow(dead_code)]
#[path = "layer_names.rs"]
pub mod layer_names;

/// Stand-in for the firmware's `default_layer_names` helper.
#[allow(dead_code)]
#[path = "default_layer_names.rs"]
pub mod default_layer_names;

/// Factory layer names of the K:03 dongle (`STANDARD_NO_MOUSE`), the constant
/// the firmware crate holds as `crate::DEFAULT_LAYER_NAMES`.
pub const DEFAULT_LAYER_NAMES: [&str; 16] = [
    "Base",
    "Navigation",
    "Symbols",
    "Adjust",
    "4",
    "5",
    "6",
    "7",
    "8",
    "9",
    "10",
    "11",
    "12",
    "13",
    "14",
    "15",
];

/// The firmware module itself: `qube_display.rs` plus the host glue, which has
/// to live *inside* it to reach the private concept renderer, the palette and
/// the look post-pass.
#[allow(dead_code, unused_imports, unused_variables)]
mod display {
    use crate::qube_fw::stubs::{
        defmt, embassy_futures, embassy_nrf, embassy_time, embedded_hal_bus, lcd_async, rmk,
        rmk_types, static_cell,
    };

    include!("qube_display.rs");
    include!("host_glue.rs");
}

// The layout helpers are the vocabulary of the acceptance tests in
// `ui::qube_screen_preview`, so they are dead code for a plain `cargo build`.
#[allow(unused_imports)]
pub use display::{
    render_frame, speedo_media_row, twocol_clock_box, twocol_divider_x, HostLcd, SCREEN_H_PX,
    SCREEN_W_PX,
};

impl HostLcd {
    /// Raw little-endian RGB565 byte image of the frame — the form the golden
    /// files hold and the parity test hashes.
    pub fn rgb565_bytes(&self) -> Vec<u8> {
        use embedded_graphics::pixelcolor::IntoStorage;
        let mut out = Vec::with_capacity(self.pixels.len() * 2);
        for pixel in &self.pixels {
            out.extend_from_slice(&pixel.into_storage().to_le_bytes());
        }
        out
    }

    /// Eight-bit RGB triples of the frame, expanded the way the panel and the
    /// golden PNGs expand an RGB565 word. This is what the texture takes.
    pub fn rgb8_bytes(&self) -> Vec<u8> {
        use embedded_graphics::pixelcolor::IntoStorage;
        let mut out = Vec::with_capacity(self.pixels.len() * 3);
        for pixel in &self.pixels {
            out.extend_from_slice(&layer_names::rgb565_components(pixel.into_storage()));
        }
        out
    }
}

/// Connection indicator the preview shows, in the firmware's own vocabulary
/// (`OutputState` in `qube_display.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    /// USB carries the active link.
    Usb,
    /// Bluetooth profile `n` is connected.
    Ble { profile: u8 },
    /// Advertising for a new host.
    Pairing { profile: u8 },
    /// Advertising to reconnect a bonded host.
    Reconnecting { profile: u8 },
    /// Nothing connected, nothing being searched for.
    Idle,
}

/// Everything one preview frame is built from: the device settings the
/// firmware reads back through its QSIDs, plus the values the RMK host protocol
/// delivers (clock, media, typing speed, modifiers).
///
/// The app has no device to talk to, so the panel fills the host-protocol half
/// with the same demo values the oracle used — see `qube_screen_preview.rs`.
pub struct PreviewInput {
    /// QSID 228; `0` is the dashboard, `1..=13` the concept screens.
    pub concept: u8,
    /// The ten configured colours in `layer_names::COLOR_ROLES` order — accent,
    /// accent shadow, background, panel, border, text, label, bar, warning,
    /// critical — as eight-bit RGB, exactly what the device reports.
    pub colors: [[u8; 3]; 10],
    /// QSID 318, `10..=100`.
    pub brightness: u8,
    /// QSID 216.
    pub show_wpm: bool,
    /// QSID 219.
    pub show_modifiers: bool,
    /// QSID 220.
    pub show_batteries: bool,
    /// QSID 221.
    pub show_output: bool,
    /// QSID 217: `0` media, `1` clock, `2` media + clock.
    pub header_mode: u8,
    /// QSID 227: `0` badge in the header, `1` badge as a chip.
    pub output_place: u8,
    /// Current layer, and the name the app shows for it.
    pub layer: u8,
    pub layer_name: String,
    /// Per half: `(level, connected)`.
    pub batteries: [(Option<u8>, bool); 2],
    /// Transport the badge reports.
    pub link: Link,
    /// Typing speed of the last `WPM` event.
    pub wpm: u16,
    /// Packed modifiers: bit0 L ctrl, bit1 R ctrl, bit2 L shift, bit3 R shift,
    /// bit4 L alt, bit5 R alt, bit6 L gui, bit7 R gui.
    pub modifiers: u8,
    pub caps_lock: bool,
    pub num_lock: bool,
    /// Host clock; `None` while the host has not sent one (`--:--`).
    pub hour: Option<u8>,
    pub minute: Option<u8>,
    /// Media line the host reports, `artist - title`; empty when unknown.
    pub media: String,
}
