use std::{sync::atomic::Ordering, thread};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::{
    adjust_brightness_rgb, converters::image::ImageProcessor, dada_packet::DadaPacket,
    helpers::display_serial_com::*, CLOSE_REQUESTED, HIBERNATING, LAST_BME_INFO,
};

pub struct Device {
    identifier: String,
    baud: u32,
    use_dada_packet: bool,
    has_bme_sensor: bool,
    background_workers_started: std::sync::atomic::AtomicBool,
    image_processor: ImageProcessor,
    adjust_brightness_on_device: bool,
    pub brightness: std::sync::atomic::AtomicU8,
    pub sender: Sender<Vec<u8>>,
    pub receiver: Receiver<Vec<u8>>,
    pub awake: std::sync::Mutex<bool>,
    pub port: std::sync::Mutex<Option<Box<dyn serialport::SerialPort>>>,
    pub connected: std::sync::atomic::AtomicBool,
}

impl Device {
    const KEEP_ALIVE: u8 = 229;
    const SEND_NEW_IMAGE: u8 = 228;
    const ACCESS_BME_SENSOR: u8 = 205;
    const RESET_DISPLAY: u8 = 17;
    const STAND_BY: u8 = 18;
    const WAKE_UP: u8 = 19;
    const SET_BRIGHTNESS: u8 = 20;

    pub fn new(
        identifier: String,
        baud: u32,
        use_dada_packet: bool,
        image_processor: ImageProcessor,
        has_bme_sensor: bool,
        adjust_brightness_on_device: bool,
    ) -> Device {
        let (sender, receiver): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = bounded(1);
        Device {
            identifier,
            baud,
            use_dada_packet,
            sender,
            receiver,
            has_bme_sensor,
            image_processor,
            adjust_brightness_on_device,
            brightness: std::sync::atomic::AtomicU8::new(100),
            background_workers_started: std::sync::atomic::AtomicBool::new(false),
            awake: std::sync::Mutex::new(false),
            port: std::sync::Mutex::new(None),
            connected: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn screen_width(&self) -> u32 {
        return self.image_processor.screen_width();
    }

    pub fn screen_height(&self) -> u32 {
        return self.image_processor.screen_height();
    }

    pub fn is_connected(&self) -> bool {
        return self.connected.load(Ordering::Acquire);
    }

    pub fn set_connected(&self, status: bool) {
        self.connected.store(status, Ordering::Release);
    }

    pub fn set_port(&self, port: Option<std::boxed::Box<dyn serialport::SerialPort>>) -> bool {
        let port_valid = port.is_some();
        self.set_connected(port_valid);
        *self.port.lock().unwrap() = port;
        port_valid
    }

    pub fn connect(&self) -> bool {
        return self.set_port(init_serial(&self.identifier, self.baud));
    }

    pub fn disconnect(&self) {
        self.set_port(None);
    }

    pub fn write(&self, payload: &[u8]) -> bool {
        if self.send_command(Self::SEND_NEW_IMAGE) {
            if self.use_dada_packet {
                return write_screen_buffer(
                    &mut self.port.lock().unwrap(),
                    &DadaPacket::new(payload.to_vec()).as_bytes(),
                );
            }

            return write_screen_buffer(&mut self.port.lock().unwrap(), payload);
        }
        false
    }

    pub fn get_bme_info(&self) -> (String, String) {
        if self.send_command(Self::ACCESS_BME_SENSOR) {
            let mut result = read_bme_sensor(&mut self.port.lock().unwrap());
            result = result.trim_end_matches('\0').into();
            let mut parts = result.split(' ');
            return (
                parts.next().unwrap_or_default().into(),
                parts.next().unwrap_or_default().into(),
            );
        }
        (String::new(), String::new())
    }

    pub fn reset_display(&self) {
        // will be ignored on ESP32 since this is only necessary for the teensy display solution.
        self.send_command(Self::RESET_DISPLAY);
    }

    pub fn send_command(&self, command: u8) -> bool {
        return send_command(&mut self.port.lock().unwrap(), &command.to_le_bytes());
    }

    pub fn set_brightness(&self, brightness: u8) -> bool {
        if self.adjust_brightness_on_device {
            self.brightness.store(brightness, Ordering::Release);
            return self.send_command(Self::SET_BRIGHTNESS)
                && write_screen_buffer(
                    &mut self.port.lock().unwrap(),
                    &DadaPacket::new(brightness.to_le_bytes().to_vec()).as_bytes(),
                );
        } else {
            // the teensy does not determine its brightness right now in the same way
            // the esp32 does. therefore, we will use this brightness in the image processor only
            // and not send it to the device for now.
            self.brightness.store(brightness, Ordering::Release);
            true
        }
    }

    pub fn stand_by(&self) {
        if *self.awake.lock().unwrap() {
            if !self.send_command(Self::STAND_BY) {
                self.disconnect()
            } else {
                *self.awake.lock().unwrap() = false;
            }
        }
    }

    pub fn wake_up(&self) {
        if !*self.awake.lock().unwrap() {
            thread::sleep(std::time::Duration::from_millis(200));
            if !self.send_command(Self::WAKE_UP) {
                self.disconnect()
            } else {
                *self.awake.lock().unwrap() = true;
            }
        }
    }

    pub fn start_background_workers(self: &'static Device) {
        if !self.background_workers_started.load(Ordering::Acquire) {
            self.background_workers_started
                .store(true, Ordering::Release);
            self.start_writer();
            self.start_bme_sensor_background_thread();
        }
    }

    fn start_bme_sensor_background_thread(self: &'static Device) {
        if self.has_bme_sensor {
            thread::spawn(move || loop {
                if self.is_connected() {
                    let bme_info = self.get_bme_info();
                    if !bme_info.0.is_empty() && !bme_info.1.is_empty() {
                        *LAST_BME_INFO.lock().unwrap() = bme_info;
                    }
                }
                if CLOSE_REQUESTED.load(std::sync::atomic::Ordering::Acquire) {
                    return;
                }
                thread::sleep(std::time::Duration::from_millis(2000));
            });
        }
    }

    fn start_writer(self: &'static Device) {
        thread::spawn(move || {
            let mut last_sum = 0;
            let mut brightness_set = false;
            loop {
                let buf = self.receiver.recv();
                if self.is_connected() {
                    if !brightness_set {
                        self.set_brightness(self.brightness.load(Ordering::Acquire));
                        brightness_set = true;
                    }
                    match buf {
                        Ok(b) => {
                            if CLOSE_REQUESTED.load(std::sync::atomic::Ordering::Acquire) {
                                return;
                            }
                            if *HIBERNATING.lock().unwrap() {
                                last_sum = 0;
                                self.stand_by();
                            } else {
                                let mut payload = b;
                                // adjust brightness in app instead of on device
                                if !self.adjust_brightness_on_device {
                                    let brightness = self.brightness.load(Ordering::Acquire);
                                    payload = adjust_brightness_rgb(&payload, brightness as f32);
                                }
                                let crc_of_buf = crc32fast::hash(&payload);
                                self.image_processor.process_image(&mut payload);
                                if last_sum != crc_of_buf {
                                    if self.write(&payload) {
                                        last_sum = crc_of_buf;
                                    } else {
                                        self.disconnect();
                                    }
                                } else if !self.send_command(Self::KEEP_ALIVE) {
                                    self.disconnect();
                                }
                                self.wake_up();
                            }
                        }
                        Err(_) => {}
                    }
                } else if self.connect() {
                    brightness_set = false;
                    last_sum = 0;
                    self.reset_display()
                }
            }
        });
    }
}
