mod packets;
mod routing_table;

use std::{io, thread, sync::{Mutex, Arc}};

use crate::at_module::{ATModule, at_address::ATAddress, ATModuleBuilder};

use packets::*;
use routing_table::RoutingTable;

pub struct AODVController {
	at_module: Arc<Mutex<ATModule>>,
	routing_table: RoutingTable,
}

impl AODVController {
	pub fn start<'scope>(scope: &'scope thread::Scope<'scope, '_>, at_module_builder: ATModuleBuilder) -> Result<Self, io::Error> {
		let (at_module, at_message_receiver) = at_module_builder.build();
		
		let mutex = Arc::new(Mutex::new(at_module));
		let mutex_clone = mutex.clone();
		
		scope.spawn(move || {
			for message in at_message_receiver {
				// TODO remove test code
				let address = message.address;
				let text = String::from_utf8_lossy(&message.data);
				println!("Received message from {address}: {text}");
				
				let packet = parse_packet(message);
				println!("Packet: {packet:#?}");
				
				let mut at_module = mutex_clone.lock().unwrap();
				at_module.send(ATAddress::new(*b"FAFA").unwrap(), &packet.unwrap().to_bytes())
					.expect("could not send packet");
			}
		});
		
		Ok(Self {
			at_module: mutex,
			routing_table: RoutingTable::new(),
		})
	}
	
	pub fn send(&mut self, address: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		let next_hop = self.routing_table.get_route(address)
			.expect("could not find a route");
		
		let mut at_module = self.at_module.lock().unwrap();
		
		let packet = DataPacket {
			destination: address,
			origin: at_module.address(),
			sequence: 0, // TODO figure out sequence number
			payload: data,
		};
		
		at_module.send(next_hop, &packet.to_bytes())
	}
}