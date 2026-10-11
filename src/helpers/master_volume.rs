#[cfg(target_os = "windows")]
use winapi::{Interface, um::mmdeviceapi::IMMDeviceEnumerator};

#[cfg(target_os = "windows")]
pub fn get_master_volume() -> (f32, i32) {
    let mut current_volume = 0.0_f32;
    let mut mute = 0;

    let mut device_enumerator: *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator =
        std::ptr::null_mut();
    unsafe {
        winapi::um::combaseapi::CoCreateInstance(
            &winapi::um::mmdeviceapi::CLSID_MMDeviceEnumerator,
            std::ptr::null_mut(),
            winapi::um::combaseapi::CLSCTX_ALL,
            &IMMDeviceEnumerator::uuidof(),
            &mut device_enumerator as *mut *mut winapi::um::mmdeviceapi::IMMDeviceEnumerator
                as *mut _,
        );
        let mut default_device: *mut winapi::um::mmdeviceapi::IMMDevice = std::mem::zeroed();
        (*device_enumerator).GetDefaultAudioEndpoint(
            winapi::um::mmdeviceapi::eRender,
            winapi::um::mmdeviceapi::eConsole,
            &mut default_device,
        );
        if default_device == std::ptr::null_mut() {
            return (0.0, 0);
        }

        (*device_enumerator).Release();
        let mut endpoint_volume: *mut winapi::um::endpointvolume::IAudioEndpointVolume =
            std::mem::zeroed();

        (*default_device).Activate(
            &winapi::um::endpointvolume::IAudioEndpointVolume::uuidof(),
            winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER,
            std::ptr::null_mut(),
            &mut endpoint_volume as *mut *mut winapi::um::endpointvolume::IAudioEndpointVolume
                as *mut _,
        );

        (*default_device).Release();
        (*endpoint_volume).GetMasterVolumeLevelScalar(&mut current_volume as *mut f32);
        (*endpoint_volume).GetMute(&mut mute as *mut i32);
    }
    (current_volume, mute)
}

#[cfg(target_os = "linux")]
pub fn get_master_volume() -> (f32, i32) {
    // wpctl output example: "Volume: 0.45 [MUTED]"
    use std::process::Command;
    let output = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output();

    let Ok(output) = output else {
        return (0.0, 0);
    };

    if !output.status.success() {
        return (0.0, 0);
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let volume = text
        .split_whitespace()
        .find_map(|part| part.parse::<f32>().ok())
        .unwrap_or(0.0);

    let mute = i32::from(text.contains("[MUTED]"));

    (volume, mute)
}
