use std::io::{self, ErrorKind};
use std::fmt::Debug;

use crate::{at_module::{ATMessage, at_address::ATAddress}, hex::{parse_ascii_hex, Integer, encode_ascii_hex}};

#[derive(Debug)]
pub struct AODVPacket {
	pub sender: ATAddress,
	pub body: AODVPacketBody,
}

pub enum AODVPacketBody {
	RouteRequest(RouteRequestPacket),
	RouteReply(RouteReplyPacket),
	RouteError(RouteErrorPacket),
	Data(DataPacket),
}

impl Debug for AODVPacketBody {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use AODVPacketBody::*;
		
		match self {
			RouteRequest(packet) => packet.fmt(f),
			RouteReply(packet) => packet.fmt(f),
			RouteError(packet) => packet.fmt(f),
			Data(packet) => packet.fmt(f),
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

fn take_int<I: Integer<I>>(data: &mut &[u8], amount: usize) -> Result<I, io::Error> {
	let bytes = take_bytes(data, amount)?;
	parse_ascii_hex(bytes)
}

fn take_address(data: &mut &[u8]) -> Result<ATAddress, io::Error> {
	let bytes = take_bytes(data, 4)?;
	ATAddress::new(bytes.try_into().expect("take_bytes(_, 4) should always return 4 bytes"))
}

pub fn parse_packet(message: &ATMessage) -> Result<AODVPacket, io::Error> {
	use AODVPacketBody::*;
	
	let mut data: &[u8] = &message.data;
	
	let body = match take_bytes(&mut data, 1)?[0] {
		b'0' => RouteRequest(RouteRequestPacket::parse_from(data)?),
		b'1' => RouteReply(RouteReplyPacket::parse_from(data)?),
		b'2' => RouteError(RouteErrorPacket::parse_from(data)?),
		b'3' => Data(DataPacket::parse_from(data)?),
		_ => return Err(ErrorKind::InvalidData.into()),
	};
	
	Ok(AODVPacket {
		sender: message.address,
		body,
	})
}

#[derive(Debug)]
pub struct RouteRequestPacket {
	pub hop_count: u8,
	pub id: u16,
	pub destination: ATAddress,
	pub destination_sequence: Option<u16>,
	pub origin: ATAddress,
	pub origin_sequence: u16,
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
		let mut data = Vec::with_capacity(24);
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
	pub hop_count: u8,
	pub request_destination: ATAddress,
	pub request_destination_sequence: u16,
	pub request_origin: Option<ATAddress>,
}

impl RouteReplyPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			hop_count: take_int(&mut data, 2)?,
			request_destination: take_address(&mut data)?,
			request_destination_sequence: take_int(&mut data, 4)?,
			request_origin: {
				let bytes = take_bytes(&mut data, 4)?;
				
				if bytes == b"FFFF" { // broadcast is used for hello packages which have no request_origin
					None
				} else {
					Some(ATAddress::new(bytes.try_into().expect("take_bytes(_, 4) should always return 4 bytes"))?)
				}
			},
		})
	}
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(15);
		data.push(b'1');
		data.extend(encode_ascii_hex(self.hop_count));
		data.extend_from_slice(self.request_destination.as_bytes());
		data.extend(encode_ascii_hex(self.request_destination_sequence));
		data.extend_from_slice(
			self.request_origin
				.as_ref()
				.map(|address| address.as_bytes())
				.unwrap_or(b"FFFF") // broadcast is used for hello packages which have no request_origin
		);
		
		data.into()
	}
}

#[derive(Debug)]
pub struct RouteErrorPacket {
	pub destination: ATAddress,
	pub destination_sequence: u16,
}

impl RouteErrorPacket {
	fn parse_from(mut data: &[u8]) -> Result<Self, io::Error> {
		Ok(Self {
			destination: take_address(&mut data)?,
			destination_sequence: take_int(&mut data, 4)?,
		})
	}
	
	pub fn to_bytes(&self) -> Box<[u8]> {
		let mut data = Vec::with_capacity(11);
		data.push(b'2');
		data.extend_from_slice(self.destination.as_bytes());
		data.extend(encode_ascii_hex(self.destination_sequence));
		
		data.into()
	}
}

#[derive(Debug)]
pub struct DataPacket {
	pub destination: ATAddress,
	pub origin: ATAddress,
	pub sequence: u8,
	pub payload: Box<[u8]>,
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