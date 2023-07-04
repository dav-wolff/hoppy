mod packets;
mod routing_table;

use std::{io, sync::{Mutex, Arc}};

use crate::at_module::{ATModuleBuilder, ATModule, at_address::ATAddress};

use packets::*;
use routing_table::RoutingTable;

pub struct AODVController {
	at_module: Arc<Mutex<Option<ATModule>>>,
	routing_table: RoutingTable,
}

impl AODVController {
	pub fn start(at_module_builder: ATModuleBuilder) -> Result<Self, io::Error> {
		let arc = Arc::new(Mutex::new(None));
		let arc_clone = arc.clone();
		
		let at_module = at_module_builder.open(move |message| {
			// TODO remove test code
			let address = message.address;
			let text = String::from_utf8_lossy(&message.data);
			println!("Received message from {address}: {text}");
			
			let packet = parse_packet(message);
			println!("Packet: {packet:#?}");
			
			let mut at_module = arc_clone.lock().unwrap();
			let at_module: &mut ATModule = at_module.as_mut().unwrap();
			
			at_module.send(ATAddress::new(*b"FAFA").unwrap(), packet.unwrap().to_bytes())
				.expect("could not send packet");
		}).expect("could not open AT module");
		
		let mut at_option = arc.lock().unwrap();
		*at_option = Some(at_module);
		std::mem::drop(at_option);
		
		Ok(Self {
			at_module: arc,
			routing_table: RoutingTable::new(),
		})
	}
	
	pub fn send(&mut self, address: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		let next_hop = self.routing_table.get_route(address)
			.expect("could not find a route");
		
		let mut at_module = self.at_module.lock().unwrap();
		let at_module = at_module.as_mut().unwrap();
		
		let packet = DataPacket {
			destination: address,
			origin: at_module.address(),
			sequence: 0, // TODO figure out sequence number
			payload: data,
		};
		
		at_module.send(next_hop, packet.to_bytes())
	}
}