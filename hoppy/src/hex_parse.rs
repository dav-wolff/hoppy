use std::{io::{self, ErrorKind}, ops::{ShlAssign, AddAssign}};

pub fn parse_ascii_hex<I: Integer<I>>(ascii_data: &[u8]) -> Result<I, io::Error> {
	if ascii_data.len() > I::BYTES * 2 {
		// number larger than I::MAX is invalid data
		return Err(ErrorKind::InvalidData.into());
	}
	
	
	let mut acc = I::default(); // 0
	
	for &ascii_digit in ascii_data {
		acc <<= 4;
		
		let digit = parse_ascii_hex_digit(ascii_digit)?;
		
		acc += digit.into();
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

pub trait Integer<I: Integer<I>>: ShlAssign<i32> + AddAssign<I> + From<u8> + Default {
	const BYTES: usize;
}

impl Integer<u8> for u8 {
	const BYTES: usize = 1;
}

impl Integer<u16> for u16 {
	const BYTES: usize = 2;
}

impl Integer<u32> for u32 {
	const BYTES: usize = 4;
}