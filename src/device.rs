use std::thread;

use crossbeam_channel::{bounded, Receiver, Sender};
use image::ImageFormat;

use crate::{
    dada_packet::DadaPacket,
    helpers::{convert_image::convert_to_webp, display_serial_com::*},
    CLOSE_REQUESTED, HIBERNATING,
};
pub struct Device {
    identifier: String,
    baud: u32,
    use_dada_packet: bool,
    pub image_format: ImageFormat,
    pub sender: Sender<Vec<u8>>,
    pub receiver: Receiver<Vec<u8>>,
    pub awake: std::sync::Mutex<bool>,
    pub port: std::sync::Mutex<Option<Box<dyn serialport::SerialPort>>>,
    pub connected: std::sync::Mutex<bool>,
}

impl Device {
    pub fn new(
        identifier: String,
        baud: u32,
        use_dada_packet: bool,
        image_format: ImageFormat,
    ) -> Device {
        let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(1);
        return Device {
            identifier: identifier.into(),
            baud,
            use_dada_packet,
            sender,
            receiver,
            image_format,
            awake: std::sync::Mutex::new(false),
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

    pub fn disconnect(&self) {
        self.set_port(None);
    }

    pub fn write(&self, payload: &[u8]) -> bool {
        if self.send_command(228) {
            if self.use_dada_packet {
                return write_screen_buffer(
                    &mut *self.port.lock().unwrap(),
                    &DadaPacket::new(payload.to_vec()).as_bytes(),
                );
            }

            return write_screen_buffer(&mut *self.port.lock().unwrap(), payload);
        }
        return false;
    }

    pub fn reset_display(&self) {
        self.send_command(17);
    }

    pub fn send_command(&self, command: u8) -> bool {
        return send_command(&mut *self.port.lock().unwrap(), &command.to_le_bytes());
    }

    pub fn stand_by(&self) {
        if *self.awake.lock().unwrap() {
            if !self.send_command(18) {
                self.disconnect()
            } else {
                *self.awake.lock().unwrap() = false;
            }
        }
    }

    pub fn wake_up(&self) {
        if !*self.awake.lock().unwrap() {
            if !self.send_command(19) {
                self.disconnect()
            } else {
                *self.awake.lock().unwrap() = true;
            }
        }
    }

    pub fn start_background_worker(self: &'static Device) {
        thread::spawn(move || {
            let mut last_sum = 0;
            loop {
                let buf = self.receiver.recv();
                if self.is_connected() {
                    match buf {
                        Ok(b) => {
                            if CLOSE_REQUESTED.load(std::sync::atomic::Ordering::Acquire) {
                                return;
                            }
                            if *HIBERNATING.lock().unwrap() {
                                self.stand_by();
                            } else {
                                self.wake_up();

                                let crc_of_buf = crc32fast::hash(&b);
                                let mut payload = b;
                                if last_sum != crc_of_buf {
                                    if self.image_format == ImageFormat::WebP {
                                        payload = convert_to_webp(&payload, 320, 170);
                                    }
                                    if self.write(&payload) {
                                        last_sum = crc_of_buf;
                                    } else {
                                        self.disconnect();
                                    }
                                } else {
                                    if !self.send_command(229) {
                                        self.disconnect();
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                } else {
                    if self.connect() {
                        last_sum = 0;
                        self.reset_display()
                    }
                }
            }
        });
    }
}
