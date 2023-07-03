mod packets;

use std::{io, sync::mpsc};

use crate::{at_module::{ATModuleBuilder, ATModule, at_address::ATAddress}, aodv::packets::parse_packet};

pub struct AODVController {
	at_module: ATModule,
}

impl AODVController {
	pub fn start(at_module_builder: ATModuleBuilder) -> Result<Self, io::Error> {
		// TODO test code
		let (packet_sender, packet_receiver) = mpsc::channel();
		
		let mut at_module = at_module_builder.open(move |message| {
			let address = message.address;
			let text = String::from_utf8_lossy(&message.data);
			println!("Received message from {address}: {text}");
			
			let packet = parse_packet(message);
			println!("Packet: {packet:#?}");
			
			let Ok(packet) = packet else {
				return;
			};
			
			packet_sender.send(packet.to_bytes())
				.expect("channel closed");
		}).expect("could not open AT module");
		
		at_module.send(ATAddress::new(*b"1234").unwrap(), b"Holle world!")
			.expect("could not send message");
		
		for packet in packet_receiver {
			at_module.send(ATAddress::new(*b"ABCD").unwrap(), &packet)
				.expect("could not send packet");
		}
		
		Ok(Self {
			at_module,
		})
	}
}