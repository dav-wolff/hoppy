use std::{io::{self, ErrorKind}, ops::RangeInclusive, fmt::{Display, self}, str};

#[derive(Debug, Clone, Copy)]
pub struct ATAddress ([u8; 4]);

impl ATAddress {
	pub fn new(data: [u8; 4]) -> Result<Self, io::Error> {
		if !is_hex_digits(&data) {
			return Err(ErrorKind::InvalidData.into());
		}
		
		Ok(Self(data))
	}
}

impl Display for ATAddress {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let text = str::from_utf8(&self.0)
			.expect("address should always be valid ASCII");
		
		write!(f, "{text}")
	}
}

fn is_hex_digits(chars: &[u8]) -> bool {
	const DEC_DIGITS: RangeInclusive<u8> = b'0'..=b'9';
	const HEX_DIGITS: RangeInclusive<u8> = b'A'..=b'F';
	
	chars.iter()
		.all(|char| DEC_DIGITS.contains(char) || HEX_DIGITS.contains(char))
}