use hex_literal::hex;

use crate::{dada_packet::DadaPacket, display_serial_com::*};
use std::time::Duration;
pub struct Device {
    identifier: String,
    baud: u32,
    pub awake: std::sync::Mutex<bool>,
    pub port: std::sync::Mutex<Option<Box<dyn serialport::SerialPort>>>,
    pub connected: std::sync::Mutex<bool>,
}

impl Device {
    pub fn new(identifier: String, baud: u32) -> Device {
        return Device {
            identifier: identifier.into(),
            baud: baud,
            port: std::sync::Mutex::new(None),
            connected: std::sync::Mutex::new(false),
            awake: std::sync::Mutex::new(false),
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

    pub fn disconnect(&self) {
        self.set_port(None);
    }

    pub fn write_screen_buffer(&self, buffer: &[u8]) -> bool {
        if send_command(&mut *self.port.lock().unwrap(), &hex!("e4")) {
            return write_screen_buffer(&mut *self.port.lock().unwrap(), buffer);
        }
        return false;
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

    pub fn stand_by(&self) {
        if *self.awake.lock().unwrap() {
            self.send_command(17);
            let mut dp: DadaPacket = DadaPacket::new(Vec::new());
            if !write_screen_buffer(&mut *self.port.lock().unwrap(), &dp.as_bytes()) {
                self.disconnect()
            } else {
                *self.awake.lock().unwrap() = false;
            }
        }
    }

    #[allow(unused)]
    pub fn wake_up(&self) {
        if !*self.awake.lock().unwrap() {
            self.send_command(18);
            let mut dp: DadaPacket = DadaPacket::new(Vec::new());

            if !write_screen_buffer(&mut *self.port.lock().unwrap(), &dp.as_bytes()) {
                self.disconnect()
            } else {
                *self.awake.lock().unwrap() = true;
            }
        }
    }
}
