use rdev::{grab, Event, EventType, Key};
use std::thread;

use crate::{LAST_KEY, LAST_KEY_VALUE};

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
        _ => Some(event),
    }
}

pub fn set_last_key(key: u32) {
    *LAST_KEY_VALUE.lock().unwrap() = key;
    *LAST_KEY.lock().unwrap() = true;
}
