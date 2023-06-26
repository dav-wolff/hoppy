use std::io;

pub struct Address {
	bytes: [u8; 4],
}

impl Address {
	pub fn from_ascii(ascii: &[u8]) -> Result<Self, io::Error> {
		if ascii.len() != 4 {
			return Err(io::ErrorKind::InvalidData.into());
		}
		
		for byte in ascii {
			if !(b'0'..=b'9').contains(byte) && !(b'A'..=b'F').contains(byte) {
				return Err(io::ErrorKind::InvalidData.into());
			}
		}
		
		Ok(Self {
			bytes: [
				ascii[0],
				ascii[1],
				ascii[2],
				ascii[3],
			],
		})
	}
	
	pub fn as_ascii_bytes(&self) -> &[u8] {
		&self.bytes
	}
}

impl Default for Address {
	fn default() -> Self {
		Self {
			bytes: *b"0000",
		}
	}
}