use hex_literal::hex;
// use log::{debug, error, info, warn};
use serialport;
use std::cmp;
use std::io::Write;
use std::thread;
use std::time::Duration;

pub fn init_serial() -> std::boxed::Box<dyn serialport::SerialPort> {
    let ports = serialport::available_ports().expect("No ports found!");
    println!("Available ports {:?}", ports);
    loop {
        thread::sleep(Duration::from_millis(1000));
        // info!("Try to open port");
        
        for p in ports.clone() {
            println!("Try opening port {}", p.port_name);
            let mut port = match serialport::new(p.port_name, 4608000)
                .timeout(Duration::new(0, 500000))
                .open()
            {
                Ok(port) => port,
                Err(_) => continue,
            };
            port.flush().unwrap();
            return port;
        }
    }
}

pub fn convert_to_gray_scale(bytes: &Vec<u8>) -> Vec<u8> {
    let mut buffer = Vec::new();
    for chunk in bytes.chunks(6) {
        let gray = (0.299 * i32::pow(chunk[0] as i32, 2) as f32
            + 0.587 * i32::pow(chunk[1] as i32, 2) as f32
            + 0.114 * i32::pow(chunk[2] as i32, 2) as f32)
            .sqrt();
        let gray2 = (0.299 * i32::pow(chunk[3] as i32, 2) as f32
            + 0.587 * i32::pow(chunk[4] as i32, 2) as f32
            + 0.114 * i32::pow(chunk[5] as i32, 2) as f32)
            .sqrt();
        buffer.push(((gray / 16.0) as u8) << 4 | ((gray2 / 16.0) as u8));
    }
    buffer
}

pub fn write_screen_buffer(
    port: &mut std::boxed::Box<dyn serialport::SerialPort>,
    screen_buf: &[u8],
) {
    port.write(&hex!("e4")).unwrap();
    // send buffer
    let mut bytes_send = 0;
    while bytes_send < screen_buf.len() {
        let slice = &screen_buf[bytes_send..cmp::min(bytes_send + 64, screen_buf.len())];
        bytes_send += slice.len();
        let _wr = port.write(&slice).expect("Write failed!");
    }
}
