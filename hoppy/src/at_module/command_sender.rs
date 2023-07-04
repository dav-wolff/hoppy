use std::{sync::mpsc::{Receiver, Sender}, io::{ErrorKind, self}};

use serialport::SerialPort;

use super::{read_replies::ATReply, ATMessage};

pub fn send_messages(
	port: Box<dyn SerialPort>,
	message_receiver: Receiver<ATMessage>,
	reply_receiver: Receiver<ATReply>,
	result_sender: Sender<Result<(), io::Error>>,
) {
	let mut message_sender = MessageSender {
		port,
		reply_receiver,
	};
	
	for at_message in message_receiver {
		let result = message_sender.send_message(at_message);
		result_sender.send(result)
			.expect("mpsc receiver should not disconnect");
	}
}

struct MessageSender {
	port: Box<dyn SerialPort>,
	reply_receiver: Receiver<ATReply>,
}

impl MessageSender {
	fn send_message(&mut self, ATMessage { address, data }: ATMessage) -> Result<(), io::Error> {
		write!(self.port, "AT+DEST={address}\r\n")?;
		
		if !self.read_reply().is_ok() {
			return Err(ErrorKind::Other.into());
		}
		
		let length = data.len();
		write!(self.port, "AT+SEND={length}\r\n")?;
		
		if !self.read_reply().is_ok() {
			// TODO return a better error
			return Err(ErrorKind::Other.into());
		}
		
		self.port.write_all(&data)?;
		
		if !self.read_reply().is_sending() {
			return Err(ErrorKind::Other.into());
		}
		
		if !self.read_reply().is_sent() {
			return Err(ErrorKind::Other.into());
		}
		
		Ok(())
	}
	
	fn read_reply(&self) -> ATReply {
		self.reply_receiver.recv()
			.expect("mpsc sender should not disconnect")
	}
}