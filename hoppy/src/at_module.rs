mod at_address;
mod read_replies;

use std::{io, thread, sync::mpsc::{self, Receiver}};
use serialport::SerialPort;
use crate::no_timeout_reader::NoTimeoutReader;

use self::read_replies::{ATMessage, ATReply};

use super::at_config::ATConfig;
use read_replies::read_replies;

pub struct ATModule {
	port: Box<dyn SerialPort>,
	receiver: Receiver<ATReply>,
}

impl ATModule {
	pub fn open<'scope, F>(
		scope: &'scope thread::Scope<'scope, '_>,
		mut port: Box<dyn SerialPort>,
		config: ATConfig,
		message_callback: F
	) -> Result<Self, io::Error>
		where F: FnMut(ATMessage) + Send + 'scope
	{
		let reader = port.try_clone()?;
		let reader = NoTimeoutReader::new(reader);
		
		let (sender, receiver) = mpsc::channel();
		
		scope.spawn(|| {
			read_replies(reader, sender, message_callback);
		});
		
		write!(port, "AT+CFG={config}\r\n")?;
		
		let mut module = Self {
			port,
			receiver,
		};
		
		if !module.read_reply().is_ok() {
			return Err(io::ErrorKind::Other.into());
		}
		
		Ok(module)
	}
	
	fn read_reply(&mut self) -> ATReply {
		self.receiver.recv()
			.expect("mpsc sender should not disconnect")
	}
	
	pub fn send(&mut self, data: &[u8]) -> Result<(), io::Error> {
		let length = data.len();
		write!(self.port, "AT+SEND={length}\r\n")?;
		
		if !self.read_reply().is_ok() {
			// TODO return a better error
			return Err(io::ErrorKind::Other.into());
		}
		
		self.port.write_all(data)?;
		
		if !self.read_reply().is_sending() {
			return Err(io::ErrorKind::Other.into());
		}
		
		if !self.read_reply().is_sent() {
			return Err(io::ErrorKind::Other.into());
		}
		
		Ok(())
	}
}