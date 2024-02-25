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
            payload,
        }
    }

    pub fn as_bytes(&mut self) -> Vec<u8> {
        let mut returnable_vector: Vec<u8> = Vec::new();

        let mut len = self.start_bytes.len() as u32 + self.end_bytes.len() as u32;

        let mut escaped_bytes = self.escape_bytes();
        len += escaped_bytes.len() as u32;

        // convert to 3 byte value
        let mut len_as_bytes = len.to_le_bytes().to_vec();
        len_as_bytes.pop();

        returnable_vector.append(&mut len_as_bytes);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dada_packet() {
        let payload = vec![1, 2, 3, 4];
        let dada_packet = DadaPacket::new(payload);

        assert_eq!(dada_packet.start_bytes, vec![68, 65, 68, 65]);
        assert_eq!(dada_packet.end_bytes, vec![65, 68, 65, 68]);
        assert_eq!(dada_packet.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_as_bytes() {
        let payload = vec![65, 68, 65];
        let mut dada_packet = DadaPacket::new(payload);

        assert_eq!(dada_packet.start_bytes, vec![68, 65, 68, 65]);
        assert_eq!(dada_packet.end_bytes, vec![65, 68, 65, 68]);
        assert_eq!(dada_packet.payload, vec![65, 68, 65]);
        // length of a packet = 3 bytes (to save one byte on every transmission)
        assert_eq!(
            vec![14, 0, 0, 68, 65, 68, 65, 65, 65, 68, 68, 65, 65, 65, 68, 65, 68],
            dada_packet.as_bytes()
        );
    }

    #[test]
    fn test_escape_bytes() {
        let payload = vec![65, 68, 65, 66];
        let mut dada_packet = DadaPacket::new(payload);

        let escaped_bytes = dada_packet.escape_bytes();

        assert_eq!(escaped_bytes, vec![65, 65, 68, 68, 65, 65, 66]);
    }
}
