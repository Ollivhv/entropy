//! Stand-in for the firmware's `default_layer_names` helper.
//!
//! `layer_names.rs` uses it to tell a stored layer name apart from a
//! placeholder an older firmware version wrote, so a device that still carries
//! `"1"`/`"Layer 1"`/`"Main"` is migrated to the factory profile. Copied from
//! `work/concepts-stand/src/main.rs`, which compiles the same firmware file.

const NUMERIC: [&str; 16] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
];
const GENERATED: [&str; 16] = [
    "Layer 0",
    "Layer 1",
    "Layer 2",
    "Layer 3",
    "Layer 4",
    "Layer 5",
    "Layer 6",
    "Layer 7",
    "Layer 8",
    "Layer 9",
    "Layer 10",
    "Layer 11",
    "Layer 12",
    "Layer 13",
    "Layer 14",
    "Layer 15",
];

#[allow(dead_code)]
pub fn is_legacy_placeholder(index: usize, bytes: &[u8]) -> bool {
    if index >= 16 {
        return false;
    }
    if bytes.is_empty() || core::str::from_utf8(bytes).is_err() {
        return true;
    }
    bytes == NUMERIC[index].as_bytes()
        || bytes.eq_ignore_ascii_case(GENERATED[index].as_bytes())
        || (index == 0 && bytes.eq_ignore_ascii_case(b"Main"))
}
