use std::fmt;

pub struct Packet {
    pub channel: u8,
    pub port: u8,
    pub data: Vec<u8>,
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "(ch: {}, port: {}, data: {})",
            self.channel, self.port, self.data[0]
        )
    }
}

impl Packet {
    pub fn new(port: u8, channel: u8, data: Vec<u8>) -> Packet {
        Packet {
            channel,
            port,
            data,
        }
    }

    pub fn from_vec(vec: Vec<u8>) -> Packet {
        let header = vec[0];

        let channel = header & 0x03;
        let port = (header & 0xF0) >> 4;
        let data = vec[1..].to_vec();

        Packet {
            channel,
            port,
            data,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        // bit 3 and 4 is reserved
        let mut header = 0x3 << 2; // header => 00001100

        // channel is at bit 1 to 2
        header |= self.channel & 0x03; // header => 000011cc

        // port is at bit 5 to 8
        header |= (self.port << 4) & 0xF0; // header => pppp11cc

        vec.push(header);
        vec.append(&mut self.data.to_vec());

        vec
    }
}
