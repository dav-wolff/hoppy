use std::io::{self, ErrorKind};

use crate::at_module::ATMessage;

#[derive(Debug)]
pub enum AODVPacket {
	RouteRequest,
	RouteReply,
	RouteError,
	Data,
	DataAcknowledge,
}

pub fn parse_packet(message: ATMessage) -> Result<AODVPacket, io::Error> {
	Ok(match message.data.first().ok_or(ErrorKind::UnexpectedEof)? {
		b'0' => AODVPacket::RouteRequest,
		b'1' => AODVPacket::RouteReply,
		b'2' => AODVPacket::RouteError,
		b'3' => AODVPacket::Data,
		b'4' => AODVPacket::DataAcknowledge,
		_ => return Err(ErrorKind::InvalidData.into()),
	})
}