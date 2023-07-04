pub mod at_address;

mod config;
mod read_replies;
mod command_sender;

pub use config::*;

use std::{io::{self, ErrorKind}, thread, sync::{mpsc::{self, Receiver, Sender}, MutexGuard}, marker::PhantomData};
use serialport::SerialPort;
use crate::no_timeout_reader::NoTimeoutReader;

use self::{at_address::ATAddress, command_sender::send_messages};

use read_replies::read_replies;

#[derive(Debug)]
pub struct ATMessage {
	pub address: ATAddress,
	pub data: Box<[u8]>,
}

pub struct ATModuleBuilder<'scope, 'env> {
	scope: &'scope thread::Scope<'scope, 'env>,
	port: Box<dyn SerialPort>,
	address: ATAddress,
	config: ATConfig,
}

impl<'scope, 'env> ATModuleBuilder<'scope, 'env> {
	pub fn open<F>(
		self,
		message_callback: F
	) -> Result<ATModule, io::Error>
		where F: FnMut(ATMessage) + Send + 'scope
	{
		let ATModuleBuilder {
			scope,
			port,
			address,
			config
		} = self;
		
		let reader = port.try_clone()?;
		let reader = NoTimeoutReader::new(reader);
		
		let (reply_sender, reply_receiver) = mpsc::channel();
		
		scope.spawn(|| {
			read_replies(reader, reply_sender, message_callback);
		});
		
		let (message_sender, message_receiver) = mpsc::channel::<ATMessage>();
		let (result_sender, result_receiver) = mpsc::channel();
		
		scope.spawn(move || {
			let mut port = port;
			
			let read_reply = || {
				reply_receiver.recv()
					.expect("mpsc sender should not disconnect")
			};
			
			let mut setup = || -> Result<(), io::Error> {
				write!(port, "AT+CFG={config}\r\n")?;
				
				if !read_reply().is_ok() {
					return Err(ErrorKind::Other.into());
				}
				
				write!(port, "AT+ADDR={address}\r\n")?;
				
				if !read_reply().is_ok() {
					return Err(ErrorKind::Other.into());
				}
				
				Ok(())
			};
			
			result_sender.send(setup())
					.expect("mpsc receiver should not disconnect");
			
			send_messages(port, message_receiver, reply_receiver, result_sender);
		});
		
		// return error if setup failed
		result_receiver.recv()
			.expect("mpsc sender should not disconnect")?;
		
		Ok(ATModule {
			address,
			message_sender,
			result_receiver
			// _unsend: Default::default(),
		})
	}
}

type PhantomUnsend = PhantomData<MutexGuard<'static, ()>>;

pub struct ATModule {
	address: ATAddress,
	message_sender: Sender<ATMessage>,
	result_receiver: Receiver<Result<(), io::Error>>,
	// _unsend: PhantomUnsend, // SerialPort seems to deadlock when called from different threads
}

impl ATModule {
	pub fn builder<'scope, 'env>(
		scope: &'scope thread::Scope<'scope, 'env>,
		port: Box<dyn SerialPort>,
		address: ATAddress,
		config: ATConfig,
	) -> ATModuleBuilder<'scope, 'env> {
		ATModuleBuilder {
			scope,
			port,
			address,
			config,
		}
	}
	
	pub fn address(&self) -> ATAddress {
		self.address
	}
	
	pub fn send(&mut self, destination: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		self.message_sender.send(ATMessage {
			address: destination,
			data,
		}).expect("mpsc receiver should not disconnect");
		
		self.result_receiver.recv()
			.expect("mpsc sender should not disconnect")
	}
	
	pub fn broadcast(&mut self, data: Box<[u8]>) -> Result<(), io::Error> {
		self.send(ATAddress::BROADCAST, data)
	}
}