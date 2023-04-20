use crate::display_serial_com::init_serial;
pub struct Device {
    identifier: String,
    baud: u32,
    pub port: std::sync::Mutex<Option<Box<dyn serialport::SerialPort>>>,
    pub connected: std::sync::Mutex<bool>,
}

impl Device {
    pub fn new() -> Device {
        return Device {
            identifier: "16c00483".into(),
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

    pub fn set_port(&self, port: Option<std::boxed::Box<dyn serialport::SerialPort>>) {
        self.set_connected(port.is_some());
        *self.port.lock().unwrap() = port;
    }

    pub fn connect(&self) {
        self.set_port(init_serial(&self.identifier, self.baud))
    }
}
