use std::{io::{Read, self, ErrorKind}, sync::mpsc::Sender, fmt::Display};

use read_buffer::DynReadBuffer;

use crate::hex::parse_ascii_hex;

use super::at_address::ATAddress;

#[derive(Debug)]
pub struct ATReply {
	data: Box<[u8]>,
}

impl ATReply {
	pub fn is_ok(&self) -> bool {
		&*self.data == b"OK"
	}
	
	pub fn is_sending(&self) -> bool {
		&*self.data == b"SENDING"
	}
	
	pub fn is_sent(&self) -> bool {
		&*self.data == b"SENDED"
	}
}

#[derive(Debug, Clone)]
pub struct ATMessage {
	pub address: ATAddress,
	pub data: Box<[u8]>,
}

impl Display for ATMessage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let address = self.address;
		let data = String::from_utf8_lossy(&self.data);
		
		write!(f, "<{address}> {data}")
	}
}

pub fn read_replies(reader: impl Read, reply_sender: Sender<ATReply>, message_sender: Sender<ATMessage>) {
	let mut buffer = DynReadBuffer::new(reader);
	
	loop {
		let reply_type = buffer.read_bytes(3);
		
		let result = match reply_type {
			Ok(b"AT,") => read_at(&mut buffer, &reply_sender),
			Ok(b"LR,") => read_lr(&mut buffer, &message_sender),
			Ok(data) => Err(io::Error::new(ErrorKind::InvalidData, String::from_utf8_lossy(data))),
			Err(err) => Err(err),
		};
		
		if let Err(err) = result {
			eprintln!("Encountered an error reading from AT module: {err}")
		}
	}
}

fn read_at(buffer: &mut DynReadBuffer<impl Read>, sender: &Sender<ATReply>) -> Result<(), io::Error> {
	let command = buffer.read_until(b'\n')?;
	
	if command[command.len() - 2] != b'\r' {
		return Err(ErrorKind::InvalidData.into());
	}
	
	// remove '\r\n'
	let command = &command[..command.len() - 2];
	
	sender.send(ATReply {
		data: command.into()
	}).expect("mpsc receiver should not disconnect");
	
	Ok(())
}

fn read_lr(buffer: &mut DynReadBuffer<impl Read>, message_sender: &Sender<ATMessage>) -> Result<(), io::Error> {
	let header = buffer.read_bytes(8)?;
	
	if header[4] != b',' || header[7] != b',' {
		return Err(ErrorKind::InvalidData.into());
	}
	
	let address = &header[..4];
	let address = ATAddress::new([address[0], address[1], address[2], address[3]])?;
	
	let length = &header[5..=6];
	let length: u8 = parse_ascii_hex(length)?;
	
	let data = buffer.read_bytes(length as usize + 2)?;
	
	if data[data.len() - 2..] != *b"\r\n" {
		return Err(io::Error::new(ErrorKind::InvalidData, "Did not receive \\r\\n from LR command"));
	}
	
	// remove '\r\n'
	let data = &data[..data.len() - 2];
	
	message_sender.send(ATMessage {
		address,
		data: data.into(),
	}).expect("mpsc receiver should not disconnect");
	
	Ok(())
}