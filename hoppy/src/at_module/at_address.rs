use std::{io::{self, ErrorKind}, fmt::{Debug, Display, self}, str, error::Error};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ATAddress ([u8; 4]);

impl ATAddress {
	pub(super) const BROADCAST: ATAddress = ATAddress(*b"FFFF");
	
	pub fn new(mut data: [u8; 4]) -> Result<Self, ATAddressError> {
		if !validate_uppercase_hex_digits(&mut data) {
			return Err(ATAddressError::InvalidAddress);
		}
		
		// broadcast address not allowed as a regular address
		if data == *b"FFFF" {
			return Err(ATAddressError::BroadcastAddress);
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

fn validate_uppercase_hex_digits(chars: &mut [u8]) -> bool {
	for char in chars {
		match char {
			b'0'..=b'9' | b'A'..=b'F' => continue,
			b'a'..=b'f' => *char = *char - b'a' + b'A',
			_ => return false,
		}
	}
	
	true
}

#[derive(Debug, Eq, PartialEq)]
pub enum ATAddressError {
	InvalidAddress,
	BroadcastAddress,
}

impl Display for ATAddressError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{self:?}")
	}
}

impl Error for ATAddressError {}

impl From<ATAddressError> for io::Error {
	fn from(value: ATAddressError) -> Self {
		match value {
			ATAddressError::InvalidAddress => io::Error::new(ErrorKind::InvalidData, "Invalid AT address"),
			ATAddressError::BroadcastAddress => io::Error::new(ErrorKind::InvalidData, "Cannot use broadcast address"),
		}
	}
}