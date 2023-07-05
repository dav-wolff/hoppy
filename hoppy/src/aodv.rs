mod packets;
mod routing_table;

use std::{io, thread, sync::{Mutex, Arc, RwLock, MutexGuard, RwLockWriteGuard, RwLockReadGuard}};

use crate::at_module::{ATModule, at_address::ATAddress, ATModuleBuilder};

use packets::*;
use routing_table::RoutingTable;

pub struct AODVController {
	at_module: Mutex<ATModule>,
	routing_table: RwLock<RoutingTable>,
}

impl AODVController {
	pub fn start<'scope>(scope: &'scope thread::Scope<'scope, '_>, at_module_builder: ATModuleBuilder) -> Arc<Self> {
		let (at_module, at_message_receiver) = at_module_builder.build();
		
		let controller = AODVController {
			at_module: Mutex::new(at_module),
			routing_table: Default::default(),
		};
		
		let controller = Arc::new(controller);
		let controller_ret = Arc::clone(&controller);
		
		scope.spawn(move || {
			for message in at_message_receiver {
				// TODO avoid clone
				let error_message = message.clone();
				let packet = match parse_packet(message) {
					Ok(packet) => packet,
					Err(err) => {
						eprintln!("[ERROR] Encountered invalid packet ({err}):\n\t{error_message}");
						continue;
					},
				};
				
				if let Err(err) = controller.handle_packet(packet) {
					// TODO display the packet
					eprintln!("[Error] Error occured trying to handle a packet ({err})")
				}
			}
		});
		
		controller_ret
	}
	
	fn at_module_write(&self) -> MutexGuard<ATModule> {
		self.at_module.lock()
			.expect("no threads should panic")
	}
	
	fn routing_table_read(&self) -> RwLockReadGuard<RoutingTable> {
		self.routing_table.read()
			.expect("no threads should panic")
	}
	
	fn routing_table_write(&self) -> RwLockWriteGuard<RoutingTable> {
		self.routing_table.write()
			.expect("no threads should panic")
	}
	
	pub fn send(&self, address: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		let mut at_module = self.at_module_write();
		
		// TODO find a route
		let next_hop = routing_table.get_route(address)
			.expect("could not find a route");
		
		let packet = DataPacket {
			destination: address,
			origin: at_module.address(),
			sequence: 0, // TODO figure out sequence number
			payload: data,
		};
		
		at_module.send(next_hop, &packet.to_bytes())
	}
	
	fn handle_packet(&self, packet: AODVPacket) -> Result<(), io::Error> {
		use AODVPacketBody::*;
		
		match packet.body {
			RouteRequest(packet) => todo!(),
			RouteReply(packet) => todo!(),
			RouteError(packet) => todo!(),
			Data(packet) => self.handle_data_packet(packet)?,
			DataAcknowledge(packet) => todo!(),
		}
		
		Ok(())
	}
	
	fn handle_data_packet(&self, packet: DataPacket) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		
		let Some(next_hop) = routing_table.get_route(packet.destination) else {
			eprintln!("[WARNING] Received DataPacket for unknown destination:\n{packet:#?}");
			
			// DataPackets without a valid route are dropped without a response
			return Ok(());
		};
		
		let mut at_module = self.at_module_write();
		at_module.send(next_hop, &packet.to_bytes())?;
		
		Ok(())
	}
}