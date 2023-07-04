use std::{io::{self, ErrorKind}, ops::RangeInclusive, fmt::{Debug, Display, self}, str};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ATAddress ([u8; 4]);

impl ATAddress {
	pub(super) const BROADCAST: ATAddress = ATAddress(*b"FFFF");
	
	pub fn new(data: [u8; 4]) -> Result<Self, io::Error> {
		// broadcast address not allowed as a regular address
		if data == *b"FFFF" {
			return Err(ErrorKind::InvalidData.into());
		}
		
		if !is_hex_digits(&data) {
			return Err(ErrorKind::InvalidData.into());
		}
		
		Ok(Self(data))
	}
	
	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}
}

impl Display for ATAddress {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let text = str::from_utf8(&self.0)
			.expect("address should always be valid ASCII");
		
		write!(f, "{text}")
	}
}

impl Debug for ATAddress {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "ATAddress({self})")
	}
}

fn is_hex_digits(chars: &[u8]) -> bool {
	const DEC_DIGITS: RangeInclusive<u8> = b'0'..=b'9';
	const HEX_DIGITS: RangeInclusive<u8> = b'A'..=b'F';
	
	chars.iter()
		.all(|char| DEC_DIGITS.contains(char) || HEX_DIGITS.contains(char))
}