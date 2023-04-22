#[derive(Debug)]
pub struct DadaPacket {
    start_bytes: Vec<u8>,
    end_bytes: Vec<u8>,
    pub payload: Vec<u8>,
}

impl DadaPacket {
    pub fn new(payload: Vec<u8>) -> DadaPacket {
        DadaPacket {
            start_bytes: "DADA".into(),
            end_bytes: "ADAD".into(),
            payload: payload,
        }
    }

    pub fn as_bytes(&mut self) -> Vec<u8> {
        let mut returnable_vector: Vec<u8> = Vec::new();

        let mut len = self.start_bytes.len() as u32 + self.end_bytes.len() as u32;

        let mut escaped_bytes = self.escape_bytes();
        len += escaped_bytes.len() as u32;

        returnable_vector.append(&mut len.to_le_bytes().to_vec());
        returnable_vector.append(&mut self.start_bytes);
        returnable_vector.append(&mut escaped_bytes);
        returnable_vector.append(&mut self.end_bytes);

        returnable_vector
    }

    fn escape_bytes(&mut self) -> Vec<u8> {
        let mut escaped_vec = Vec::new();

        for byte in &mut self.payload {
            if *byte == 65 {
                escaped_vec.push(65);
                escaped_vec.push(65);
            } else if *byte == 68 {
                escaped_vec.push(68);
                escaped_vec.push(68);
            } else {
                escaped_vec.push(*byte);
            }
        }
        escaped_vec
    }
}
