#[cfg(target_os = "windows")]
extern crate winres;

fn main() {
    #[cfg(target_os = "windows")]
    let mut res = winres::WindowsResource::new();
    #[cfg(target_os = "windows")]
    res.set_icon("icon.ico");
    #[cfg(target_os = "windows")]
    res.compile().unwrap_or_default();
    #[cfg(target_os = "windows")]
    static_vcruntime::metabuild();
}
