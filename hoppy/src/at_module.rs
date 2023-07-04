pub mod at_address;

mod config;
mod read_replies;

pub use config::*;
pub use read_replies::ATMessage;

use std::{io::{self, ErrorKind}, thread, sync::mpsc::{self, Receiver}};
use serialport::SerialPort;
use crate::no_timeout_reader::NoTimeoutReader;

use self::{read_replies::ATReply, at_address::ATAddress};

use read_replies::read_replies;

pub struct ATModuleBuilder {
	port: Box<dyn SerialPort>,
	address: ATAddress,
	reply_receiver: Receiver<ATReply>,
	message_receiver: Receiver<ATMessage>,
}

impl ATModuleBuilder {
	pub fn build(self) -> (ATModule, Receiver<ATMessage>) {
		let module = ATModule {
			port: self.port,
			address: self.address,
			reply_receiver: self.reply_receiver,
		};
		
		let message_receiver = self.message_receiver;
		
		(module, message_receiver)
	}
}

pub struct ATModule {
	port: Box<dyn SerialPort>,
	address: ATAddress,
	reply_receiver: Receiver<ATReply>,
}

impl ATModule {
	pub fn open<'scope>(
		scope: &'scope thread::Scope<'scope, '_>,
		mut port: Box<dyn SerialPort>,
		address: ATAddress,
		config: ATConfig,
	) -> Result<ATModuleBuilder, io::Error> {
		let reader = port.try_clone()?;
		let reader = NoTimeoutReader::new(reader);
		
		let (reply_sender, reply_receiver) = mpsc::channel();
		let (message_sender, message_receiver) = mpsc::channel();
		
		scope.spawn(|| {
			read_replies(reader, reply_sender, message_sender);
		});
		
		let read_reply = || {
			reply_receiver.recv()
				.expect("mpsc sender should not disconnect")
		};
		
		write!(port, "AT+CFG={config}\r\n")?;
		
		if !read_reply().is_ok() {
			return Err(ErrorKind::Other.into());
		}
		
		write!(port, "AT+ADDR={address}\r\n")?;
		
		if !read_reply().is_ok() {
			return Err(ErrorKind::Other.into());
		}
		
		Ok(ATModuleBuilder {
			port,
			address,
			reply_receiver,
			message_receiver
		})
	}
	
	pub fn address(&self) -> ATAddress {
		self.address
	}
	
	fn read_reply(&mut self) -> ATReply {
		self.reply_receiver.recv()
			.expect("mpsc sender should not disconnect")
	}
	
	pub fn send(&mut self, destination: ATAddress, data: &[u8]) -> Result<(), io::Error> {
		write!(self.port, "AT+DEST={destination}\r\n")?;
		
		if !self.read_reply().is_ok() {
			return Err(ErrorKind::Other.into());
		}
		
		let length = data.len();
		write!(self.port, "AT+SEND={length}\r\n")?;
		
		if !self.read_reply().is_ok() {
			// TODO return a better error
			return Err(ErrorKind::Other.into());
		}
		
		self.port.write_all(data)?;
		
		if !self.read_reply().is_sending() {
			return Err(ErrorKind::Other.into());
		}
		
		if !self.read_reply().is_sent() {
			return Err(ErrorKind::Other.into());
		}
		
		Ok(())
	}
	
	pub fn broadcast(&mut self, data: &[u8]) -> Result<(), io::Error> {
		self.send(ATAddress::BROADCAST, data)
	}
}