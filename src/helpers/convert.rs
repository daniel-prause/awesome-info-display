use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub fn to_wstring(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn convert_brightness(value: u16) -> f32 {
    let old_range = 100f32 - 20f32;
    let new_range = 100f32;
    let new_value = ((value as f32 - 20f32) * new_range) / old_range;
    return new_value;
}
