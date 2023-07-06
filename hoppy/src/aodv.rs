mod packets;
mod routing_table;

use std::{io, thread, sync::{Mutex, Arc, RwLock, MutexGuard, RwLockWriteGuard, RwLockReadGuard}, collections::BTreeSet};

use crate::at_module::{ATModule, at_address::ATAddress, ATModuleBuilder};

use packets::*;
use routing_table::RoutingTable;

pub struct AODVController {
	seen_requests: Mutex<BTreeSet<(ATAddress, u16)>>, // unfortunate mutex
	routing_table: RwLock<RoutingTable>,
	at_module: Mutex<ATModule>,
}

impl AODVController {
	pub fn start<'scope>(scope: &'scope thread::Scope<'scope, '_>, at_module_builder: ATModuleBuilder) -> Arc<Self> {
		let (at_module, at_message_receiver) = at_module_builder.build();
		let routing_table = RoutingTable::new(at_module.address());
		
		let controller = AODVController {
			seen_requests: Default::default(),
			at_module: Mutex::new(at_module),
			routing_table: RwLock::new(routing_table),
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
		let route = routing_table.get_route(address)
			.expect("could not find a route");
		
		let packet = DataPacket {
			destination: address,
			origin: at_module.address(),
			sequence: 0, // TODO figure out sequence number
			payload: data,
		};
		
		at_module.send(route.next_hop, &packet.to_bytes())
	}
	
	fn handle_packet(&self, packet: AODVPacket) -> Result<(), io::Error> {
		use AODVPacketBody::*;
		
		let sender = packet.sender;
		
		match packet.body {
			RouteRequest(packet) => self.handle_route_request(sender, packet)?,
			RouteReply(packet) => self.handle_route_reply(sender, packet)?,
			RouteError(packet) => todo!(),
			Data(packet) => self.handle_data(packet)?,
			DataAcknowledge(packet) => todo!(),
		}
		
		Ok(())
	}
	
	fn handle_route_request(&self, sender: ATAddress, packet: RouteRequestPacket) -> Result<(), io::Error> {
		let mut seen_requests = self.seen_requests.lock()
			.expect("should only be accessed from this thread");
		
		let is_new_request = seen_requests.insert((packet.origin, packet.id));
		
		if !is_new_request {
			return Ok(());
		}
		
		let mut routing_table = self.routing_table_write();
		let mut at_module = self.at_module_write();
		
		routing_table.add_route(packet.origin, packet.origin_sequence, sender, packet.hop_count);
		
		if let Some(route) = routing_table.get_route(packet.destination) {
			let reply = RouteReplyPacket {
				hop_count: packet.hop_count + route.hop_count,
				destination: packet.origin,
				destination_sequence: packet.origin_sequence,
				origin: packet.destination,
			};
			
			at_module.send(sender, &reply.to_bytes())?;
			
			return Ok(());
		}
		
		let packet = RouteRequestPacket {
			hop_count: packet.hop_count + 1,
			..packet
		};
		
		at_module.broadcast(&packet.to_bytes())?;
		
		Ok(())
	}
	
	fn handle_route_reply(&self, sender: ATAddress, packet: RouteReplyPacket) -> Result<(), io::Error> {
		let mut routing_table = self.routing_table_write();
		let mut at_module = self.at_module_write();
		
		routing_table.add_route(packet.origin, 0, sender, packet.hop_count); // TODO figure out sequence number
		
		if packet.destination == at_module.address() {
			// TODO send DataPacket for requested route
			return Ok(());
		}
		
		let Some(route) = routing_table.get_route(packet.destination) else {
			eprintln!("[WARNING] Received RouteReplyPacket for unknown destination:\n{packet:#?}");
			
			// RouteReplyPackets without a valid route are dropped without a response
			return Ok(());
		};
		
		let packet = RouteReplyPacket {
			hop_count: packet.hop_count + 1,
			..packet
		};
		
		at_module.send(route.next_hop, &packet.to_bytes())?;
		
		Ok(())
	}
	
	fn handle_data(&self, packet: DataPacket) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		
		let Some(route) = routing_table.get_route(packet.destination) else {
			eprintln!("[WARNING] Received DataPacket for unknown destination:\n{packet:#?}");
			
			// DataPackets without a valid route are dropped without a response
			return Ok(());
		};
		
		let mut at_module = self.at_module_write();
		at_module.send(route.next_hop, &packet.to_bytes())?;
		
		Ok(())
	}
}