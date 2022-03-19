use hex_literal::hex;
use serialport;
use std::cmp;
use std::io::Write;
use std::time::Duration;

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

#[allow(unused)]
pub fn write_screen_buffer(screen_buf: &[u8]) -> bool {
    match serialport::available_ports() {
        Ok(ports) => {
            for p in ports {
                match p.port_type {
                    serialport::SerialPortType::UsbPort(info) => {
                        let comp = format!("{:04x}{:04x}", info.vid, info.pid);
                        // look for teensy 4.0 vendor and product id
                        if ("16c00483".eq(&comp)) {
                            let mut port = serialport::new(p.port_name, 115_200)
                                .timeout(Duration::from_millis(10));
                            match port.open() {
                                Ok(mut port_ok) => match port_ok.write(&hex!("e4")) {
                                    Ok(_) => match std::io::stdout().flush() {
                                        Ok(_) => {
                                            let mut bytes_send = 0;
                                            while bytes_send < screen_buf.len() {
                                                let slice = &screen_buf[bytes_send
                                                    ..cmp::min(bytes_send + 64, screen_buf.len())];
                                                bytes_send += slice.len();

                                                match port_ok.write(&slice) {
                                                    Ok(_) => {
                                                        return true;
                                                    }
                                                    Err(_) => {
                                                        return false;
                                                    }
                                                }
                                            }
                                            return true;
                                        }
                                        Err(_) => {}
                                    },
                                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => (),
                                    Err(e) => eprintln!("{:?}", e),
                                },
                                Err(err) => {
                                    println!("ERR: {}", err);
                                }
                            }
                        }
                    }
                    _ => {
                        // ignore other types; we are only interested in
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            eprintln!("Error listing serial ports");
        }
    }

    return false;
}
