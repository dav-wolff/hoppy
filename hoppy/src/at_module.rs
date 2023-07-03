pub mod at_address;

mod config;
mod read_replies;

pub use config::*;
pub use read_replies::ATMessage;

use std::{io::{self, ErrorKind}, thread, sync::{mpsc::{self, Receiver}, MutexGuard}, marker::PhantomData};
use serialport::SerialPort;
use crate::no_timeout_reader::NoTimeoutReader;

use self::{read_replies::ATReply, at_address::ATAddress};

use read_replies::read_replies;

type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;

pub struct ATModule {
	port: Box<dyn SerialPort>,
	receiver: Receiver<ATReply>,
	_unsend: PhantomUnsend, // SerialPort seems to deadlock when called from different threads
}

impl ATModule {
	pub fn open<'scope, F>(
		scope: &'scope thread::Scope<'scope, '_>,
		port: Box<dyn SerialPort>,
		address: ATAddress,
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
		
		let mut module = Self {
			port,
			receiver,
			_unsend: Default::default(),
		};
		
		write!(module.port, "AT+CFG={config}\r\n")?;
		
		if !module.read_reply().is_ok() {
			return Err(ErrorKind::Other.into());
		}
		
		write!(module.port, "AT+ADDR={address}\r\n")?;
		
		if !module.read_reply().is_ok() {
			return Err(ErrorKind::Other.into());
		}
		
		Ok(module)
	}
	
	fn read_reply(&mut self) -> ATReply {
		self.receiver.recv()
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
}