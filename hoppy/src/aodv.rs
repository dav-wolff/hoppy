mod packets;

use std::io;

use crate::at_module::{ATModuleBuilder, ATModule, at_address::ATAddress};

use packets::*;

pub struct AODVController {
	at_module: ATModule,
}

impl AODVController {
	pub fn start(at_module_builder: ATModuleBuilder) -> Result<Self, io::Error> {
		let at_module = at_module_builder.open(|message| {
			// TODO remove test code
			let address = message.address;
			let text = String::from_utf8_lossy(&message.data);
			println!("Received message from {address}: {text}");
			
			let packet = parse_packet(message);
			println!("Packet: {packet:#?}");
		}).expect("could not open AT module");
		
		Ok(Self {
			at_module,
		})
	}
	
	pub fn send(&mut self, address: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		let packet = DataPacket {
			destination: address,
			origin: self.at_module.address(),
			sequence: 0, // TODO figure out sequence number
			payload: data,
		};
		
		self.at_module.broadcast(&packet.to_bytes())
	}
}