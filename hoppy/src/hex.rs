use std::{io::{self, ErrorKind}, ops::{ShlAssign, AddAssign, DivAssign, ShrAssign, BitAnd}, fmt::Debug};

pub fn parse_ascii_hex<I: Integer<I>>(ascii_data: &[u8]) -> Result<I, io::Error> {
	if ascii_data.len() > I::HEX_DIGITS {
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
		b'A'..=b'F' => ascii_digit - b'A' + 0xA,
		_ => return Err(ErrorKind::InvalidData.into()),
	};
	
	Ok(digit)
}

pub fn encode_ascii_hex<I: Integer<I>>(mut number: I) -> impl Iterator<Item = u8> // ideally would return `[u8; I::HEX_DIGITS]`
where
	I: BitAnd<Output = I>,
	I::Error: Debug
{
	let mut ascii_data: Vec<u8> = Vec::with_capacity(I::HEX_DIGITS);
	
	for _ in 0..I::HEX_DIGITS {
		let digit = number & 0xFu8.into();
		let ascii_digit = encode_ascii_hex_digit(digit.try_into().unwrap());
		ascii_data.push(ascii_digit);
		
		number >>= 4;
	}
	
	ascii_data.into_iter().rev()
}

fn encode_ascii_hex_digit(digit: u8) -> u8 {
	match digit {
		0..=9 => digit + b'0',
		0xA..=0xF => digit - 0xA + b'A',
		_ => panic!("this function should only be called with integers between 0 and 15")
	}
}

pub trait Integer<I: Integer<I>>:
	ShlAssign<i32> +
	ShrAssign<i32> +
	AddAssign<I> +
	DivAssign<I> +
	BitAnd<I> +
	From<u8> +
	TryInto<u8> +
	Default +
	Copy
{
	const HEX_DIGITS: usize;
}

impl Integer<u8> for u8 {
	const HEX_DIGITS: usize = 2;
}

impl Integer<u16> for u16 {
	const HEX_DIGITS: usize = 4;
}

impl Integer<u32> for u32 {
	const HEX_DIGITS: usize = 8;
}