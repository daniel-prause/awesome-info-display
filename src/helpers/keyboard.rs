#[cfg(target_os = "windows")]
use rdev::{Event, EventType, Key, grab};
use std::thread;

use crate::{LAST_KEY, LAST_KEY_VALUE};

#[cfg(target_os = "windows")]
pub fn start_global_key_grabber(callback: rdev::GrabCallback) {
    thread::spawn({
        move || loop {
            match grab(callback) {
                Ok(_) => {}
                Err(error) => {
                    eprintln!("Global key grab error: {:?}", error)
                }
            }
        }
    });
}

#[cfg(target_os = "linux")]
pub fn start_global_key_grabber() {
    thread::spawn(|| {
        use std::fs;

        let Ok(entries) = fs::read_dir("/dev/input") else {
            eprintln!("Cannot read /dev/input");
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if !name.starts_with("event") {
                continue;
            }

            let path = path.clone();

            thread::spawn(move || {
                use evdev::Device;

                let mut device = match Device::open(&path) {
                    Ok(device) => device,
                    Err(error) => {
                        eprintln!("Cannot open {:?}: {}", path, error);
                        return;
                    }
                };

                loop {
                    match device.fetch_events() {
                        Ok(events) => {
                            for event in events {
                                if event.event_type() != evdev::EventType::KEY || event.value() != 1
                                {
                                    continue;
                                }

                                // Linux input keycodes.
                                let key = match event.code() {
                                    113 => 173, // KEY_MUTE
                                    114 => 174, // KEY_VOLUMEDOWN
                                    115 => 175, // KEY_VOLUMEUP
                                    163 => 176, // KEY_NEXTSONG
                                    165 => 177, // KEY_PREVIOUSSONG
                                    166 => 178, // KEY_STOPCD
                                    164 => 179, // KEY_PLAYPAUSE
                                    119 => 180, // KEY_PAUSE
                                    _ => continue,
                                };

                                set_last_key(key);
                            }
                        }
                        Err(error) => {
                            use std::time::Duration;

                            eprintln!("Error reading {:?}: {}", path, error);
                            thread::sleep(Duration::from_millis(250));
                        }
                    }
                }
            });
        }
    });
}

#[cfg(target_os = "windows")]
pub fn callback(event: Event) -> Option<Event> {
    match event.event_type {
        EventType::KeyPress(Key::Unknown(178)) => {
            set_last_key(178);
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(177)) => {
            set_last_key(177);
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(176)) => {
            set_last_key(176);
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(175)) => {
            set_last_key(175);
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(174)) => {
            set_last_key(174);
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(173)) => {
            set_last_key(173);
            Some(event)
        }
        EventType::KeyPress(Key::Unknown(179)) => {
            set_last_key(179);
            Some(event)
        }
        EventType::KeyPress(Key::Pause) => {
            set_last_key(180);
            Some(event)
        }
        _ => {
            println!("EVENT {:?}", event.event_type);
            Some(event)
        }
    }
}

pub fn set_last_key(key: u32) {
    *LAST_KEY_VALUE.lock().unwrap() = key;
    *LAST_KEY.lock().unwrap() = true;
}
