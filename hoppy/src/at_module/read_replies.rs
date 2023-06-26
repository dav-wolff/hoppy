use std::{io::{Read, self, ErrorKind}, sync::mpsc::Sender};

use read_buffer::DynReadBuffer;

use crate::hex_parse::parse_ascii_hex;

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

#[derive(Debug)]
pub struct ATMessage {
	pub address: ATAddress,
	pub data: Box<[u8]>,
}

pub fn read_replies<F>(reader: impl Read, sender: Sender<ATReply>, mut callback: F)
	where F: FnMut(ATMessage)
{
	let mut buffer = DynReadBuffer::new(reader);
	
	loop {
		let reply_type = buffer.read_bytes(3);
		
		let result = match reply_type {
			Ok(b"AT,") => read_at(&mut buffer, &sender),
			Ok(b"LR,") => read_lr(&mut buffer, &mut callback),
			Ok(_) => Err(ErrorKind::InvalidData.into()),
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

fn read_lr<F>(buffer: &mut DynReadBuffer<impl Read>, callback: &mut F) -> Result<(), io::Error>
	where F: FnMut(ATMessage)
{
	let header = buffer.read_bytes(8)?;
	
	if header[4] != b',' || header[7] != b',' {
		return Err(ErrorKind::InvalidData.into());
	}
	
	let address = &header[..4];
	let address = ATAddress::new([address[0], address[1], address[2], address[3]])?;
	
	let length = &header[5..=6];
	let length = parse_ascii_hex(length)?;
	
	let data = buffer.read_bytes(length as usize)?;
	
	callback(ATMessage {
		address,
		data: data.into(),
	});
	
	Ok(())
}