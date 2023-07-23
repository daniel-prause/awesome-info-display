use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub fn to_wstring(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
