use hex_literal::hex;

use crate::display_serial_com::*;
use std::time::Duration;
pub struct Device {
    identifier: String,
    baud: u32,
    pub port: std::sync::Mutex<Option<Box<dyn serialport::SerialPort>>>,
    pub connected: std::sync::Mutex<bool>,
}

impl Device {
    pub fn new(identifier: String) -> Device {
        return Device {
            identifier: identifier.into(),
            baud: 4608000,
            port: std::sync::Mutex::new(None),
            connected: std::sync::Mutex::new(false),
        };
    }

    pub fn is_connected(&self) -> bool {
        return *self.connected.lock().unwrap();
    }

    pub fn set_connected(&self, status: bool) {
        *self.connected.lock().unwrap() = status;
    }

    pub fn set_port(&self, port: Option<std::boxed::Box<dyn serialport::SerialPort>>) -> bool {
        let port_valid = port.is_some();
        self.set_connected(port_valid);
        *self.port.lock().unwrap() = port;
        return port_valid;
    }

    pub fn connect(&self) -> bool {
        return self.set_port(init_serial(&self.identifier, self.baud));
    }

    pub fn write_screen_buffer(&self, buffer: &[u8]) -> bool {
        if send_command(&mut *self.port.lock().unwrap(), &hex!("e4")) {
            return write_screen_buffer(&mut *self.port.lock().unwrap(), buffer);
        }
        return true;
    }

    pub fn write_serialized_buffer(&self, buffer: &[u8]) -> bool {
        return write_screen_buffer(&mut *self.port.lock().unwrap(), buffer);
    }
    pub fn reset_display(&self, duration: u64) {
        reset_display(
            &mut *self.port.lock().unwrap(),
            Duration::from_millis(duration),
        );
    }

    pub fn send_command(&self, command: u8) -> bool {
        return send_command(&mut *self.port.lock().unwrap(), &command.to_le_bytes());
    }

    pub fn stand_by(&self) -> bool {
        if self.send_command(232) {
            return self.write_screen_buffer(&vec![0; 8192 as usize]);
        }
        return false;
    }

    #[allow(unused)]
    pub fn wake_up(&self) -> bool {
        return self.send_command(233);
    }
}
