use std::io::{self, ErrorKind};

pub fn parse_ascii_hex(ascii_data: &[u8]) -> Result<u32, io::Error> {
	if ascii_data.len() > 8 {
		// number larger than u32::MAX is invalid data
		return Err(ErrorKind::InvalidData.into());
	}
	
	let mut acc: u32 = 0;
	
	for &ascii_digit in ascii_data {
		acc <<= 4;
		
		let digit = parse_ascii_hex_digit(ascii_digit)?;
		
		acc += digit as u32;
	}
	
	Ok(acc)
}

fn parse_ascii_hex_digit(ascii_digit: u8) -> Result<u8, io::Error> {
	let digit = match ascii_digit {
		b'0'..=b'9' => ascii_digit - b'0',
		b'A'..=b'F' => ascii_digit - b'A' + 10,
		_ => return Err(ErrorKind::InvalidData.into()),
	};
	
	Ok(digit)
}