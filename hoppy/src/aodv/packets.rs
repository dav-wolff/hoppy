use std::io::{self, ErrorKind};

use crate::{at_module::{ATMessage, at_address::ATAddress}, hex::{parse_ascii_hex, Integer, encode_ascii_hex}};

#[derive(Debug)]
pub enum AODVPacket {
	RouteRequest(RouteRequestPacket),
	RouteReply(RouteReplyPacket),
	RouteError(RouteErrorPacket),
	Data(DataPacket),
	DataAcknowledge(DataAcknowledgePacket),
}

impl AODVPacket {
	pub fn to_bytes(&self) -> Box<[u8]> {
		use AODVPacket::*;
		
		match self {
			RouteRequest(packet) => packet.to_bytes(),
			RouteReply(packet) => packet.to_bytes(),
			RouteError(packet) => packet.to_bytes(),
			Data(packet) => packet.to_bytes(),
			DataAcknowledge(packet) => packet.to_bytes(),
		}
	}
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
	destination_sequence: Option<u16>,
	origin: ATAddress,
	origin_sequence: u16,
}

impl RouteRequestPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		let unknown_destination_sequence = match take_bytes(&mut data, 1)? {
			b"Y" => true,
			b"N" => false,
			_ => return Err(ErrorKind::InvalidData.into()),
		};
		
		Ok(Self {
			hop_count: take_int(&mut data, 2)?,
			id: take_int(&mut data, 4)?,
			destination: take_address(&mut data)?,
			destination_sequence: {
				// make sure to always read out the sequence number, in order to move past those bytes of data
				let sequence = take_int(&mut data, 4)?;
				
				if unknown_destination_sequence {
					None
				} else {
					Some(sequence)
				}
			},
			origin: take_address(&mut data)?,
			origin_sequence: take_int(&mut data, 4)?,
		})
	}
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(23);
		data.push(b'0');
		data.push(if self.destination_sequence.is_none() {
			b'Y'
		} else {
			b'N'
		});
		data.extend(encode_ascii_hex(self.hop_count));
		data.extend(encode_ascii_hex(self.id));
		data.extend_from_slice(self.destination.as_bytes());
		data.extend(encode_ascii_hex(self.destination_sequence.unwrap_or_default()));
		data.extend_from_slice(self.origin.as_bytes());
		data.extend(encode_ascii_hex(self.origin_sequence));
		
		data.into()
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
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(15);
		data.push(b'1');
		data.extend(encode_ascii_hex(self.hop_count));
		data.extend_from_slice(self.destination.as_bytes());
		data.extend(encode_ascii_hex(self.destination_sequence));
		data.extend_from_slice(self.origin.as_bytes());
		
		data.into()
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
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(11);
		data.push(b'2');
		data.extend(encode_ascii_hex(self.destination_count));
		data.extend_from_slice(self.destination.as_bytes());
		data.extend(encode_ascii_hex(self.destination_sequence));
		
		data.into()
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
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(11 + self.payload.len());
		data.push(b'3');
		data.extend_from_slice(self.destination.as_bytes());
		data.extend_from_slice(self.origin.as_bytes());
		data.extend(encode_ascii_hex(self.sequence));
		data.extend_from_slice(&self.payload);
		
		data.into()
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
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(11);
		data.push(b'4');
		data.extend_from_slice(self.destination.as_bytes());
		data.extend_from_slice(self.origin.as_bytes());
		data.extend(encode_ascii_hex(self.sequence));
		
		data.into()
	}
}