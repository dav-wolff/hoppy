mod packets;
mod routing_table;

use std::{io, thread, sync::{Mutex, Arc, RwLock, MutexGuard, RwLockWriteGuard, RwLockReadGuard, atomic::{AtomicU16, Ordering}}, collections::{BTreeSet, BTreeMap}, time::{Duration, Instant}};

use crate::at_module::{ATModule, at_address::ATAddress, ATModuleBuilder};

use packets::*;
use routing_table::RoutingTable;

use self::routing_table::Route;

pub struct AODVController<C: Fn(ATAddress, &[u8]) + Send + Sync> {
	seen_requests: Mutex<BTreeSet<(ATAddress, u16)>>, // unfortunate mutex
	routing_table: RwLock<RoutingTable>,
	at_module: Mutex<ATModule>,
	outbound_messages: Mutex<BTreeMap<ATAddress, Vec<Box<[u8]>>>>,
	address: ATAddress,
	current_route_request_id: AtomicU16,
	hello_timeout: Duration,
	data_callback: C,
}

impl<'scope, C: Fn(ATAddress, &[u8]) + Send + Sync + 'scope> AODVController<C> {
	pub fn start(
		scope: &'scope thread::Scope<'scope, '_>,
		at_module_builder: ATModuleBuilder,
		hello_interval: Duration,
		hello_timeout: Duration,
		data_callback: C
	) -> Arc<Self> {
		let (at_module, at_message_receiver) = at_module_builder.build();
		let address = at_module.address();
		let routing_table = RoutingTable::new(address);
		
		let controller = AODVController {
			seen_requests: Default::default(),
			at_module: Mutex::new(at_module),
			routing_table: RwLock::new(routing_table),
			outbound_messages: Default::default(),
			address,
			current_route_request_id: 0.into(),
			hello_timeout,
			data_callback,
		};
		
		let controller = Arc::new(controller);
		let controller_receive = Arc::clone(&controller);
		let controller_hello = Arc::clone(&controller);
		
		scope.spawn(move || {
			for message in at_message_receiver {
				println!("[INFO] Received message:\n\t{message}");
				
				let packet = match parse_packet(&message) {
					Ok(packet) => packet,
					Err(err) => {
						eprintln!("[ERROR] Encountered invalid packet ({err}):\n\t{message}");
						continue;
					},
				};
				
				if let Err(err) = controller_receive.handle_packet(&packet) {
					eprintln!("[Error] Error occured trying to handle a packet ({err}):\n{packet:#?}");
				}
			}
		});
		
		scope.spawn(move || {
			loop {
				let result = controller_hello.send_hello();
				
				if let Err(err) = result {
					eprintln!("[ERROR] Could not send Hello packet ({err})");
				}
				
				let result = controller_hello.check_neighbor_hello();
				
				if let Err(err) = result {
					eprintln!("[ERROR] Could not send RouteErrorPacket ({err})")
				}
				
				thread::sleep(hello_interval);
			}
		});
		
		controller
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
	
	fn outbound_messages_write(&self) -> MutexGuard<BTreeMap<ATAddress, Vec<Box<[u8]>>>> {
		self.outbound_messages.lock()
			.expect("no threads should panic")
	}
	
	pub fn send(&self, address: ATAddress, data: Box<[u8]>) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		let mut at_module = self.at_module_write();
		
		if let Some(route) = routing_table.get_route(address) {
			let packet = DataPacket {
				destination: address,
				origin: self.address,
				sequence: 0, // TODO figure out sequence number
				payload: data,
			};
			
			at_module.send(route.next_hop, &packet.to_bytes())?;
			
			return Ok(());
		}
		
		let id = self.current_route_request_id.fetch_add(1, Ordering::Relaxed);
		
		let packet = RouteRequestPacket {
			id,
			hop_count: 0,
			destination: address,
			destination_sequence: None, // TODO figure out sequence number
			origin: self.address,
			origin_sequence: 0, // TODO figure out sequence
		};
		
		at_module.broadcast(&packet.to_bytes())?;
		
		let mut outbound_messages = self.outbound_messages.lock()
			.expect("no threads should panic");
		
		outbound_messages.entry(address)
			.or_insert_with(Vec::new)
			.push(data);
		
		Ok(())
	}
	
	fn send_hello(&self) -> Result<(), io::Error> {
		let mut at_module = self.at_module_write();
		
		let packet = RouteReplyPacket {
			hop_count: 0,
			request_destination: self.address,
			request_destination_sequence: 0, // TODO figure out sequence number
			request_origin: None,
		};
		
		at_module.broadcast(&packet.to_bytes())?;
		
		Ok(())
	}
	
	fn check_neighbor_hello(&self) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		
		let current_time = Instant::now();
		let mut timed_out_neighbors = Vec::new();
		
		for neighbor in routing_table.neighbors() {
			if current_time - neighbor.last_seen > self.hello_timeout {
				timed_out_neighbors.push(neighbor);
			}
		}
		
		// avoid taking write locks if no neighbor timed out
		if timed_out_neighbors.is_empty() {
			return Ok(());
		}
		
		// release read lock
		std::mem::drop(routing_table);
		
		let mut routing_table = self.routing_table_write();
		let mut at_module = self.at_module_write();
		
		for neighbor in timed_out_neighbors {
			routing_table.remove_route(neighbor.next_hop, neighbor.next_hop);
			
			let packet = RouteErrorPacket {
				destination: neighbor.next_hop,
				destination_sequence: neighbor.destination_sequence,
			};
			
			at_module.broadcast(&packet.to_bytes())?;
		}
		
		Ok(())
	}
	
	fn send_outbound_messages(&self, at_module: &mut ATModule, destination: ATAddress, route: Route) -> Result<(), io::Error> {
		let mut outbound_messages = self.outbound_messages_write();
		
		let Some(messages) = outbound_messages.get_mut(&destination) else {
			return Ok(());
		};
		
		for message in messages.drain(..) {
			let packet = DataPacket {
				destination,
				origin: self.address,
				sequence: 0, // TODO figure out sequence number
				payload: message,
			};
			
			at_module.send(route.next_hop, &packet.to_bytes())?;
		}
		
		Ok(())
	}
	
	fn handle_packet(&self, packet: &AODVPacket) -> Result<(), io::Error> {
		use AODVPacketBody::*;
		
		let sender = packet.sender;
		
		match &packet.body {
			RouteRequest(packet) => self.handle_route_request(sender, packet)?,
			RouteReply(packet) => self.handle_route_reply(sender, packet)?,
			RouteError(packet) => self.handle_route_error(sender, packet)?,
			Data(packet) => self.handle_data(packet)?,
			DataAcknowledge(packet) => self.handle_data_acknowledge(packet)?,
		}
		
		Ok(())
	}
	
	fn handle_route_request(&self, sender: ATAddress, packet: &RouteRequestPacket) -> Result<(), io::Error> {
		let mut seen_requests = self.seen_requests.lock()
			.expect("should only be accessed from this thread");
		
		let is_new_request = seen_requests.insert((packet.origin, packet.id));
		
		if !is_new_request {
			return Ok(());
		}
		
		let mut routing_table = self.routing_table_write();
		let mut at_module = self.at_module_write();
		
		if let Some(new_route) = routing_table.add_route(packet.origin, packet.origin_sequence, sender, packet.hop_count + 1) {
			self.send_outbound_messages(&mut at_module, packet.origin, new_route)?;
		}
		
		
		if let Some(route) = routing_table.get_route(packet.destination) {
			let reply = RouteReplyPacket {
				hop_count: packet.hop_count + route.hop_count,
				request_destination: packet.destination,
				request_destination_sequence: 0, // TODO figure out sequence number
				request_origin: Some(packet.origin),
			};
			
			at_module.send(sender, &reply.to_bytes())?;
			
			return Ok(());
		}
		
		let packet = RouteRequestPacket {
			hop_count: packet.hop_count + 1,
			..*packet
		};
		
		at_module.broadcast(&packet.to_bytes())?;
		
		Ok(())
	}
	
	fn handle_route_reply(&self, sender: ATAddress, packet: &RouteReplyPacket) -> Result<(), io::Error> {
		let mut routing_table = self.routing_table_write();
		let mut at_module = self.at_module_write();
		
		if let Some(new_route) = routing_table.add_route(packet.request_destination, 0, sender, packet.hop_count + 1) { // TODO figure out sequence number
			self.send_outbound_messages(&mut at_module, packet.request_destination, new_route)?;
		}
		
		// 'Hello' RouteReplyPackets should not be forwarded
		let Some(request_origin) = packet.request_origin else {
			return Ok(());
		};
		
		// RouteReplyPackets for self don't need to be forwarded
		if request_origin == self.address {
			return Ok(());
		}
		
		let Some(route) = routing_table.get_route(request_origin) else {
			eprintln!("[WARNING] Received RouteReplyPacket for unknown request origin:\n{packet:#?}");
			
			// RouteReplyPackets without a valid route are dropped without a response
			return Ok(());
		};
		
		let packet = RouteReplyPacket {
			hop_count: packet.hop_count + 1,
			..*packet
		};
		
		at_module.send(route.next_hop, &packet.to_bytes())?;
		
		Ok(())
	}
	
	fn handle_route_error(&self, sender: ATAddress, packet: &RouteErrorPacket) -> Result<(), io::Error> {
		let mut routing_table = self.routing_table_write();
		
		let is_route_removed = routing_table.remove_route(packet.destination, sender);
		
		if !is_route_removed {
			// no changes were made, so no need to notify others
			return Ok(());
		}
		
		let mut at_module = self.at_module_write();
		
		at_module.broadcast(&packet.to_bytes())?;
		
		Ok(())
	}
	
	fn handle_data(&self, packet: &DataPacket) -> Result<(), io::Error> {
		if packet.destination == self.address {
			let data_callback = &self.data_callback;
			data_callback(packet.origin, &packet.payload);
			
			return Ok(());
		}
		
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
	
	fn handle_data_acknowledge(&self, packet: &DataAcknowledgePacket) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		
		let Some(route) = routing_table.get_route(packet.destination) else {
			eprintln!("[WARNING] Received DataAcknowledgePacket for unknown destination:\n{packet:#?}");
			
			// DataAcknowledgePackets without a valid route are dropped without a response
			return Ok(());
		};
		
		let mut at_module = self.at_module_write();
		at_module.send(route.next_hop, &packet.to_bytes())?;
		
		Ok(())
	}
}