use std::io::{self, ErrorKind};

use crate::{at_module::{ATMessage, at_address::ATAddress}, hex_parse::{parse_ascii_hex, Integer}};

#[derive(Debug)]
pub enum AODVPacket {
	RouteRequest(RouteRequestPacket),
	RouteReply(RouteReplyPacket),
	RouteError(RouteErrorPacket),
	Data(DataPacket),
	DataAcknowledge(DataAcknowledgePacket),
}

fn take_bytes<'a>(data: &mut &'a[u8], amount: usize) -> Result<&'a[u8], io::Error> {
	if amount > data.len() {
		return Err(ErrorKind::UnexpectedEof.into());
	}
	
	let bytes;
	(bytes, *data) = data.split_at(amount);
	
	Ok(bytes)
}

fn take_int<'a, I: Integer<I>>(data: &mut &'a[u8], amount: usize) -> Result<I, io::Error> {
	let bytes = take_bytes(data, amount)?;
	parse_ascii_hex(bytes)
}

fn take_address<'a>(data: &mut &'a[u8]) -> Result<ATAddress, io::Error> {
	let bytes = take_bytes(data, 4)?;
	ATAddress::new(bytes.try_into().expect("take_bytes(_, 4) should always return 4 bytes"))
}

pub fn parse_packet(message: ATMessage) -> Result<AODVPacket, io::Error> {
	let mut data: &[u8] = &message.data;
	
	Ok(match take_bytes(&mut data, 1)?[0] {
		b'0' => AODVPacket::RouteRequest(RouteRequestPacket::parse_from(data)?),
		b'1' => AODVPacket::RouteReply(RouteReplyPacket::parse_from(data)?),
		b'2' => AODVPacket::RouteError(RouteErrorPacket::parse_from(data)?),
		b'3' => AODVPacket::Data(DataPacket::parse_from(data)?),
		b'4' => AODVPacket::DataAcknowledge(DataAcknowledgePacket::parse_from(data)?),
		_ => return Err(ErrorKind::InvalidData.into()),
	})
}

#[derive(Debug)]
pub struct RouteRequestPacket {
	id: u16,
	hop_count: u8,
	destination: ATAddress,
	destination_sequence: u16,
	origin: ATAddress,
	origin_sequence: u16,
}

impl RouteRequestPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			hop_count: take_int(&mut data, 2)?,
			id: take_int(&mut data, 4)?,
			destination: take_address(&mut data)?,
			destination_sequence: take_int(&mut data, 4)?,
			origin: take_address(&mut data)?,
			origin_sequence: take_int(&mut data, 4)?,
		})
	}
}

#[derive(Debug)]
pub struct RouteReplyPacket {
	hop_count: u8,
	destination: ATAddress,
	destination_sequence: u16,
	origin: ATAddress,
}

impl RouteReplyPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			hop_count: take_int(&mut data, 2)?,
			destination: take_address(&mut data)?,
			destination_sequence: take_int(&mut data, 4)?,
			origin: take_address(&mut data)?,
		})
	}
}

#[derive(Debug)]
pub struct RouteErrorPacket {
	destination: ATAddress,
	destination_sequence: u16,
	destination_count: u8,
}

impl RouteErrorPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			destination_count: take_int(&mut data, 2)?,
			destination: take_address(&mut data)?,
			destination_sequence: take_int(&mut data, 4)?,
		})
	}
}

#[derive(Debug)]
pub struct DataPacket {
	destination: ATAddress,
	origin: ATAddress,
	sequence: u8,
	payload: Box<[u8]>,
}

impl DataPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			destination: take_address(&mut data)?,
			origin: take_address(&mut data)?,
			sequence: take_int(&mut data, 2)?,
			payload: data.into(),
		})
	}
}

#[derive(Debug)]
pub struct DataAcknowledgePacket {
	destination: ATAddress,
	origin: ATAddress,
	sequence: u8,
}

impl DataAcknowledgePacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			destination: take_address(&mut data)?,
			origin: take_address(&mut data)?,
			sequence: take_int(&mut data, 2)?,
		})
	}
}