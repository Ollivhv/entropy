//! Host stand-ins for the firmware-only dependencies of `qube_display.rs`.
//!
//! The point is to compile the *unmodified* driver file on the host: every
//! module below mimics just enough of the real API surface (types, fields,
//! methods) for the file to type-check. Nothing here is ever executed, except
//! `rmk::host_data` / `embassy_time`, which the renderer really calls.
//!
//! Origin: copy of `work/qube-preview/src/stubs.rs` (the file the host stand
//! `work/concepts-stand` compiles the firmware file against; the stand gets it
//! through `#[path = "../../qube-preview/src/stubs.rs"]`). Two adaptations for
//! this crate:
//!
//! 1. the intra-crate paths `crate::stubs::…` became `crate::qube_fw::stubs::…`
//!    because the module is mounted one level deeper here;
//! 2. [`embassy_time::Instant::now`] is pinned (see the comment there).

#![allow(dead_code, unused_variables, unused_imports, unreachable_code)]

use core::cell::UnsafeCell;
use core::marker::PhantomData;

/// Stand-in for `rmk_types::battery`.
pub mod rmk_types {
    pub mod battery {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum BatteryStatus {
            Available { level: Option<u8>, state: ChargeState },
            Unavailable,
        }
        impl BatteryStatus {
            pub const fn available(level: Option<u8>) -> Self {
                Self::Available {
                    level,
                    state: ChargeState::Discharging,
                }
            }
            pub const UNAVAILABLE: Self = Self::Unavailable;
        }
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ChargeState {
            Unknown,
            Charging,
            Discharging,
        }
    }

    /// Stand-in for `rmk_types::ble` — real shape, because the firmware's
    /// connection badge matches on the state.
    pub mod ble {
        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        pub enum BleState {
            Advertising,
            Connected,
            #[default]
            Inactive,
            Sleeping,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        pub struct BleStatus {
            pub profile: u8,
            pub state: BleState,
        }
    }

    /// Stand-in for `rmk_types::connection` (copied from the real crate).
    pub mod connection {
        use super::ble::{BleState, BleStatus};

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        pub enum ConnectionType {
            #[default]
            Usb,
            Ble,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        pub enum UsbState {
            #[default]
            Disabled,
            Enabled,
            Configured,
            Suspended,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        pub struct ConnectionStatus {
            pub usb: UsbState,
            pub ble: BleStatus,
            pub preferred: ConnectionType,
        }

        impl ConnectionStatus {
            fn usb_ready(&self) -> bool {
                matches!(self.usb, UsbState::Configured | UsbState::Suspended)
            }

            fn ble_ready(&self) -> bool {
                matches!(self.ble.state, BleState::Connected)
            }

            pub fn decide_active(&self) -> Option<ConnectionType> {
                match (self.usb_ready(), self.ble_ready()) {
                    (true, false) => Some(ConnectionType::Usb),
                    (false, true) => Some(ConnectionType::Ble),
                    (true, true) => Some(self.preferred),
                    (false, false) => None,
                }
            }
        }
    }
}

/// Stand-in for the `rmk` crate (only the modules `qube_display.rs` imports).
pub mod rmk {
    use core::marker::PhantomData;

    pub mod core_traits {
        pub trait Runnable {
            async fn run(&mut self) -> !;
        }
    }

    /// Stand-in for `rmk::config` — only the Vial device-settings types the
    /// firmware's `layer_names.rs` imports.
    pub mod config {
        pub const VIAL_DEVICE_SETTINGS_MAX_LEN: usize = 224;

        #[derive(Clone, Copy, Debug)]
        pub struct VialDeviceSettingsData {
            pub len: u8,
            pub data: [u8; VIAL_DEVICE_SETTINGS_MAX_LEN],
        }

        impl VialDeviceSettingsData {
            pub const fn empty() -> Self {
                Self {
                    len: 0,
                    data: [0; VIAL_DEVICE_SETTINGS_MAX_LEN],
                }
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub struct VialDeviceSettings<'a> {
            pub setting_keys: &'a [u16],
            pub get_setting: fn(u16, &mut [u8]) -> Option<usize>,
            pub set_setting: fn(u16, &[u8]) -> bool,
            pub serialize: fn() -> VialDeviceSettingsData,
            pub deserialize: fn(&[u8]),
        }
    }

    /// Host model of `rmk::host_data` — the renderer reads it for real.
    pub mod host_data {
        const HOST_TEXT_LEN: usize = 32;

        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        pub struct HostData {
            pub hour: Option<u8>,
            pub minute: Option<u8>,
            pub layout: Option<u8>,
            pub media_artist: heapless::String<HOST_TEXT_LEN>,
            pub media_title: heapless::String<HOST_TEXT_LEN>,
        }

        impl HostData {
            pub fn with_media(artist: &str, title: &str) -> Self {
                let mut data = Self::default();
                let _ = data.media_artist.push_str(artist);
                let _ = data.media_title.push_str(title);
                data
            }
        }

        pub fn snapshot() -> HostData {
            HostData::default()
        }
    }

    /// Small value types the display code touches indirectly.
    pub mod types {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub struct BleStatus {
            pub connected: bool,
        }

        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub struct LedIndicator {
            pub caps_lock: bool,
            pub num_lock: bool,
        }

        impl LedIndicator {
            pub const fn caps_lock(&self) -> bool {
                self.caps_lock
            }
            pub const fn num_lock(&self) -> bool {
                self.num_lock
            }
        }

        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub struct ModifierCombination {
            bits: u8,
        }

        impl ModifierCombination {
            pub const LEFT_CTRL: u8 = 1 << 0;
            pub const RIGHT_CTRL: u8 = 1 << 1;
            pub const LEFT_SHIFT: u8 = 1 << 2;
            pub const RIGHT_SHIFT: u8 = 1 << 3;
            pub const LEFT_ALT: u8 = 1 << 4;
            pub const RIGHT_ALT: u8 = 1 << 5;
            pub const LEFT_GUI: u8 = 1 << 6;
            pub const RIGHT_GUI: u8 = 1 << 7;

            pub const fn new() -> Self {
                Self { bits: 0 }
            }
            pub const fn from_bits(bits: u8) -> Self {
                Self { bits }
            }
            pub const fn bits(self) -> u8 {
                self.bits
            }
            pub const fn left_ctrl(self) -> bool {
                self.bits & Self::LEFT_CTRL != 0
            }
            pub const fn right_ctrl(self) -> bool {
                self.bits & Self::RIGHT_CTRL != 0
            }
            pub const fn left_shift(self) -> bool {
                self.bits & Self::LEFT_SHIFT != 0
            }
            pub const fn right_shift(self) -> bool {
                self.bits & Self::RIGHT_SHIFT != 0
            }
            pub const fn left_alt(self) -> bool {
                self.bits & Self::LEFT_ALT != 0
            }
            pub const fn right_alt(self) -> bool {
                self.bits & Self::RIGHT_ALT != 0
            }
            pub const fn left_gui(self) -> bool {
                self.bits & Self::LEFT_GUI != 0
            }
            pub const fn right_gui(self) -> bool {
                self.bits & Self::RIGHT_GUI != 0
            }
        }
    }

    /// Stand-in for `rmk::event` (event payloads + subscriber traits).
    pub mod event {
        use core::marker::PhantomData;

        use crate::qube_fw::stubs::rmk_types::battery::BatteryStatus;

        pub use super::types::{LedIndicator, ModifierCombination};
        /// Real `rmk_types::ble::BleStatus` — the payload of
        /// `ConnectionStatus` carries profile + state.
        pub use crate::qube_fw::stubs::rmk_types::ble::BleStatus;

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct BatteryStatusEvent(pub BatteryStatus);

        /// Real shape from `rmk_types::connection`, so the firmware's badge can
        /// call `decide_active()`.
        pub use crate::qube_fw::stubs::rmk_types::connection::ConnectionStatus;
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ConnectionStatusChangeEvent(pub ConnectionStatus);

        /// Host advertising mode used by the connection badge.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum BleAdvertisingMode {
            Pairing,
            Reconnecting,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct BleAdvertisingModeEvent(pub BleAdvertisingMode);

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct LayerChangeEvent(pub u8);
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct WpmUpdateEvent(pub u16);
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct LedIndicatorEvent(pub LedIndicator);
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct SleepStateEvent(pub bool);
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct KeyboardEvent {
            pub pressed: bool,
            pub row: u8,
            pub col: u8,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct ModifierEvent {
            pub modifier: ModifierCombination,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct PeripheralConnectedEvent {
            pub id: usize,
            pub connected: bool,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct CentralConnectedEvent {
            pub connected: bool,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct PeripheralBatteryEvent {
            pub id: usize,
            pub state: BatteryStatusEvent,
        }

        pub trait EventSubscriber {
            type Event;
            async fn next_event(&mut self) -> Self::Event;
        }

        /// Never yields — the host stand never runs the event loop.
        pub struct StubSubscriber<E> {
            _e: PhantomData<E>,
        }
        impl<E> EventSubscriber for StubSubscriber<E> {
            type Event = E;
            async fn next_event(&mut self) -> E {
                core::future::pending().await
            }
        }

        pub trait SubscribableEvent: Sized {
            type Subscriber: EventSubscriber<Event = Self>;
            fn subscriber() -> Self::Subscriber;
        }

        macro_rules! subscribable {
            ($($t:ty),* $(,)?) => {
                $(
                    impl SubscribableEvent for $t {
                        type Subscriber = StubSubscriber<$t>;
                        fn subscriber() -> Self::Subscriber {
                            StubSubscriber { _e: PhantomData }
                        }
                    }
                )*
            };
        }
        subscribable!(
            LayerChangeEvent,
            WpmUpdateEvent,
            LedIndicatorEvent,
            SleepStateEvent,
            KeyboardEvent,
            ModifierEvent,
            BatteryStatusEvent,
            ConnectionStatusChangeEvent,
            PeripheralConnectedEvent,
            PeripheralBatteryEvent,
            CentralConnectedEvent,
            BleAdvertisingModeEvent,
        );
    }

    /// Stand-in for `rmk::display`.
    pub mod display {
        use embedded_graphics::pixelcolor::PixelColor;
        use embedded_graphics::prelude::DrawTarget;

        use super::event::BatteryStatusEvent;
        pub use super::types::{BleStatus, ModifierCombination};

        /// Two halves, exactly like the classic_qube split configuration.
        pub const SPLIT_PERIPHERALS_NUM: usize = 2;

        pub struct RenderContext {
            pub layer: u8,
            pub wpm: u16,
            pub caps_lock: bool,
            pub num_lock: bool,
            pub battery: BatteryStatusEvent,
            pub sleeping: bool,
            /// Real `rmk_types::ble::BleStatus`, because the firmware copies the
            /// connection status out of the change event.
            pub ble_status: crate::qube_fw::stubs::rmk_types::ble::BleStatus,
            pub central_connected: bool,
            pub peripherals_connected: [bool; SPLIT_PERIPHERALS_NUM],
            pub peripheral_batteries: [BatteryStatusEvent; SPLIT_PERIPHERALS_NUM],
            pub modifiers: ModifierCombination,
            pub key_pressed: bool,
            pub key_press_latch: bool,
        }

        impl Default for RenderContext {
            fn default() -> Self {
                use crate::qube_fw::stubs::rmk_types::battery::BatteryStatus;
                Self {
                    layer: 0,
                    wpm: 0,
                    caps_lock: false,
                    num_lock: false,
                    battery: BatteryStatusEvent(BatteryStatus::UNAVAILABLE),
                    sleeping: false,
                    ble_status: crate::qube_fw::stubs::rmk_types::ble::BleStatus::default(),
                    central_connected: false,
                    peripherals_connected: [false; SPLIT_PERIPHERALS_NUM],
                    peripheral_batteries: [BatteryStatusEvent(BatteryStatus::UNAVAILABLE);
                        SPLIT_PERIPHERALS_NUM],
                    modifiers: ModifierCombination::new(),
                    key_pressed: false,
                    key_press_latch: false,
                }
            }
        }

        pub trait DisplayRenderer<C: PixelColor> {
            fn render<D: DrawTarget<Color = C>>(&mut self, ctx: &RenderContext, display: &mut D);
        }
    }

    /// Stand-in for `rmk::processor`.
    pub mod processor {
        use super::core_traits::Runnable;
        use super::event::EventSubscriber;

        pub trait Processor: Runnable {
            type Event;
            fn subscriber() -> impl EventSubscriber<Event = Self::Event>;
            async fn process(&mut self, event: Self::Event);
            async fn process_loop(&mut self) -> !;
        }
    }

    /// Stand-in for `rmk::split::ble::central`.
    pub mod split {
        pub mod ble {
            pub mod central {
                use crate::qube_fw::stubs::embassy_time::Duration;

                pub fn pointing_quiet_period_remaining(_quiet_period: Duration) -> Duration {
                    Duration::MIN
                }
            }
        }
    }
}

/// Stand-in for the `static_cell` crate.
pub mod static_cell {
    use core::cell::UnsafeCell;

    pub struct StaticCell<T> {
        value: UnsafeCell<Option<T>>,
    }

    unsafe impl<T> Sync for StaticCell<T> {}

    impl<T> StaticCell<T> {
        pub const fn new() -> Self {
            Self {
                value: UnsafeCell::new(None),
            }
        }

        pub fn init(&'static self, value: T) -> &'static mut T {
            unsafe {
                let slot = &mut *self.value.get();
                *slot = Some(value);
                slot.as_mut().unwrap()
            }
        }
    }
}

/// Stand-in for `embassy_time` — `Instant` is backed by std for realism.
pub mod embassy_time {
    use core::future::Future;
    use core::pin::Pin;
    use core::task::{Context, Poll};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Duration(u64);

    impl Duration {
        /// Embassy semantics: the smallest duration (zero).
        pub const MIN: Duration = Duration(0);
        pub const fn from_millis(ms: u64) -> Self {
            Duration(ms)
        }
        /// Embassy takes seconds here; this stub keeps milliseconds.
        pub const fn from_secs(secs: u64) -> Self {
            Duration(secs * 1_000)
        }
        pub const fn from_ticks(ticks: u64) -> Self {
            Duration(ticks)
        }
        pub const fn as_millis(&self) -> u64 {
            self.0
        }
        pub const fn is_zero(&self) -> bool {
            self.0 == 0
        }
        pub const fn checked_sub(self, other: Duration) -> Option<Duration> {
            match self.0.checked_sub(other.0) {
                Some(v) => Some(Duration(v)),
                None => None,
            }
        }
        pub const fn saturating_sub(self, other: Duration) -> Duration {
            Duration(self.0.saturating_sub(other.0))
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
    pub struct Instant(u64);

    impl Instant {
        /// Always zero: the preview paints a single frame, not a run of them.
        ///
        /// The firmware's media ticker phase comes from
        /// `Instant::now().duration_since(Instant::from_ticks(0))` and the
        /// goldens were taken on the stand's *first* frame, i.e. with a phase of
        /// zero. A monotonic host clock would make the same preview state paint
        /// a different frame seconds after the process started (the marquee
        /// steps every 300 ms), so the phase is pinned to the value the oracle
        /// used.
        pub fn now() -> Self {
            Instant(0)
        }
        pub const fn from_ticks(ticks: u64) -> Self {
            Instant(ticks)
        }
        pub fn elapsed(&self) -> Duration {
            Duration(Self::now().0.saturating_sub(self.0))
        }
        pub fn duration_since(&self, earlier: Instant) -> Duration {
            Duration(self.0.saturating_sub(earlier.0))
        }
    }

    /// The host stand never awaits a real timer.
    pub struct Delay;

    pub struct TimerFuture;
    impl Future for TimerFuture {
        type Output = ();
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
            Poll::Pending
        }
    }

    pub struct Timer;
    impl Timer {
        pub fn after(_duration: Duration) -> TimerFuture {
            TimerFuture
        }
    }
}

/// Stand-in for `embassy_futures::select`.
pub mod embassy_futures {
    pub mod select {
        use core::future::Future;

        pub enum Either<A, B> {
            First(A),
            Second(B),
        }
        pub enum Either3<A, B, C> {
            First(A),
            Second(B),
            Third(C),
        }

        pub async fn select<A, B>(
            a: impl Future<Output = A>,
            b: impl Future<Output = B>,
        ) -> Either<A, B> {
            let _ = &b;
            Either::First(a.await)
        }

        pub async fn select3<A, B, C>(
            a: impl Future<Output = A>,
            b: impl Future<Output = B>,
            c: impl Future<Output = C>,
        ) -> Either3<A, B, C> {
            let _ = (&b, &c);
            Either3::First(a.await)
        }
    }
}

/// Stand-in for `defmt` (logging macros are compiled to no-ops).
pub mod defmt {
    macro_rules! defmt_info {
        ($($arg:tt)*) => {{}};
    }
    macro_rules! defmt_warn {
        ($($arg:tt)*) => {{}};
    }
    // Re-exporting under the real names needs the alias form: a plain
    // `use warn;` is ambiguous with the built-in attribute.
    #[allow(unused_imports)]
    pub(crate) use {defmt_info as info, defmt_warn as warn};
}

/// Stand-in for `embassy_nrf` (pins, SPIM, interrupt bindings).
pub mod embassy_nrf {
    use core::marker::PhantomData;

    /// Embassy's zero-sized peripheral/peri handle.
    pub struct Peri<'d, T> {
        _p: PhantomData<(&'d (), T)>,
    }

    pub mod interrupt {
        pub mod typelevel {
            pub trait Binding<A, B>: Copy {}
        }
    }

    pub mod gpio {
        use core::marker::PhantomData;

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Level {
            Low,
            High,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum OutputDrive {
            Standard,
            High,
        }

        pub struct Output<'d> {
            _p: PhantomData<&'d ()>,
        }

        impl<'d> Output<'d> {
            pub fn new<P>(_pin: super::Peri<'d, P>, _level: Level, _drive: OutputDrive) -> Self {
                Self { _p: PhantomData }
            }

            /// The firmware switches the panel backlight off while the screen
            /// is blanked by the idle timeout.
            pub fn set_level(&mut self, _level: Level) {}
        }
    }

    pub mod peripherals {
        macro_rules! peripherals {
            ($($name:ident),* $(,)?) => {
                $(pub struct $name;)*
            };
        }
        peripherals!(P0_02, P0_03, P0_28, P1_10, P1_11, P1_13, SPI3);
    }

    pub mod spim {
        use core::marker::PhantomData;

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Frequency {
            M8,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub struct Config {
            pub frequency: Frequency,
        }
        impl Default for Config {
            fn default() -> Self {
                Self {
                    frequency: Frequency::M8,
                }
            }
        }

        pub trait Instance {
            type Interrupt;
        }
        impl Instance for super::peripherals::SPI3 {
            type Interrupt = ();
        }

        pub struct InterruptHandler<T> {
            _p: PhantomData<T>,
        }

        pub struct Spim<'d> {
            _p: PhantomData<&'d ()>,
        }

        impl<'d> Spim<'d> {
            pub fn new_txonly<A, B, C, I>(
                _spi: super::Peri<'d, A>,
                _irq: I,
                _sck: super::Peri<'d, B>,
                _mosi: super::Peri<'d, C>,
                _config: Config,
            ) -> Spim<'d> {
                Spim { _p: PhantomData }
            }
        }
    }
}

/// Stand-in for `embedded_hal_bus::spi`.
pub mod embedded_hal_bus {
    pub mod spi {
        use core::marker::PhantomData;

        pub struct NoDelay;

        pub struct ExclusiveDevice<BUS, CS, DELAY> {
            _p: PhantomData<(BUS, CS, DELAY)>,
        }

        impl<BUS, CS, DELAY> ExclusiveDevice<BUS, CS, DELAY> {
            pub fn new_no_delay(bus: BUS, cs: CS) -> Result<Self, core::convert::Infallible> {
                let _ = (bus, cs);
                Ok(Self { _p: PhantomData })
            }
        }
    }
}

/// Stand-in for `lcd_async` (ST7789 builder chain).
pub mod lcd_async {
    use core::marker::PhantomData;

    #[derive(Debug)]
    pub struct Error;

    pub mod interface {
        use core::marker::PhantomData;

        pub struct SpiInterface<BUS, DC> {
            _p: PhantomData<(BUS, DC)>,
        }

        impl<BUS, DC> SpiInterface<BUS, DC> {
            pub fn new(_bus: BUS, _dc: DC) -> Self {
                Self { _p: PhantomData }
            }
        }
    }

    pub mod models {
        pub struct ST7789;
    }

    pub mod options {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ColorInversion {
            Normal,
            Inverted,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum Rotation {
            Deg0,
            Deg90,
            Deg180,
            Deg270,
        }

        #[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
        pub struct Orientation;

        impl Orientation {
            pub const fn new() -> Self {
                Orientation
            }
            pub const fn rotate(self, _rotation: Rotation) -> Self {
                self
            }
        }
    }

    pub struct Display<IFACE, MODEL, RST> {
        _p: PhantomData<(IFACE, MODEL, RST)>,
    }

    impl<IFACE, MODEL, RST> Display<IFACE, MODEL, RST> {
        pub async fn show_raw_data(
            &mut self,
            _x: u16,
            _y: u16,
            _w: u16,
            _h: u16,
            _data: &[u8],
        ) -> Result<(), Error> {
            Ok(())
        }
    }

    pub struct Builder<IFACE, MODEL, RST> {
        _p: PhantomData<(IFACE, MODEL, RST)>,
    }

    impl<IFACE, MODEL> Builder<IFACE, MODEL, ()> {
        pub fn new(_model: MODEL, _iface: IFACE) -> Self {
            Self { _p: PhantomData }
        }
    }

    impl<IFACE, MODEL, RST> Builder<IFACE, MODEL, RST> {
        pub fn reset_pin<RST2>(self, _rst: RST2) -> Builder<IFACE, MODEL, RST2> {
            Builder { _p: PhantomData }
        }
        pub fn display_size(self, _width: u16, _height: u16) -> Self {
            self
        }
        pub fn display_offset(self, _x: u16, _y: u16) -> Self {
            self
        }
        pub fn invert_colors(self, _inversion: options::ColorInversion) -> Self {
            self
        }
        pub fn orientation(self, _orientation: options::Orientation) -> Self {
            self
        }
        pub async fn init<D>(self, _delay: &mut D) -> Result<Display<IFACE, MODEL, RST>, Error> {
            Ok(Display { _p: PhantomData })
        }
    }
}
