mod packets;
mod routing_table;

use std::io;

use crate::at_module::{ATModuleBuilder, ATModule, at_address::ATAddress};

use packets::*;
use routing_table::RoutingTable;

pub struct AODVController {
	at_module: ATModule,
	routing_table: RoutingTable,
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
			routing_table: RoutingTable::new(),
		})
	}
	
	pub fn send(&mut self, address: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		let next_hop = self.routing_table.get_route(address)
			.expect("could not find a route");
		
		let packet = DataPacket {
			destination: address,
			origin: self.at_module.address(),
			sequence: 0, // TODO figure out sequence number
			payload: data,
		};
		
		self.at_module.send(next_hop, &packet.to_bytes())
	}
}