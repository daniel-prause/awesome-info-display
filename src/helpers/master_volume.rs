use winapi::{um::mmdeviceapi::IMMDeviceEnumerator, Interface};

pub fn get_master_volume() -> (f32, i32) {
    let mut current_volume = 0.0 as f32;
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
