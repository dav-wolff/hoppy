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
	current_sequence_number: AtomicU16,
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
			current_sequence_number: 0.into(),
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
		
		if let Some(route) = routing_table.get_route(address, None) {
			let packet = DataPacket {
				destination: address,
				origin: self.address,
				payload: data,
			};
			
			at_module.send(route.next_hop, &packet.to_bytes())?;
			
			return Ok(());
		}
		
		let packet = RouteRequestPacket {
			id: self.current_route_request_id.fetch_add(1, Ordering::Relaxed),
			hop_count: 0,
			destination: address,
			destination_sequence: routing_table.get_last_known_sequence(address),
			origin: self.address,
			origin_sequence: self.current_sequence_number.fetch_add(1, Ordering::Relaxed),
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
			request_destination_sequence: self.current_sequence_number.fetch_add(1, Ordering::Relaxed),
			request_origin: None,
		};
		
		at_module.broadcast(&packet.to_bytes())?;
		
		Ok(())
	}
	
	fn check_neighbor_hello(&self) -> Result<(), io::Error> {
		let routing_table = self.routing_table_read();
		
		let current_time = Instant::now();
		
		let timed_out_routes: Vec<_> = routing_table.neighbors()
			.filter(|neighbor| current_time - neighbor.last_seen > self.hello_timeout)
			.flat_map(|neighbor| routing_table.routes_with_next_hop(neighbor.next_hop))
			.collect();
		
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
		
		for (destination, route) in timed_out_routes {
			routing_table.remove_route(destination, route.next_hop);
			
			let packet = RouteErrorPacket {
				destination,
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
				payload: message,
			};
			
			at_module.send(route.next_hop, &packet.to_bytes())?;
		}
		
		Ok(())
	}
	
	fn update_sequence_number(&self, new_sequence_number: u16) {
		let current = &self.current_sequence_number;
		let new = new_sequence_number;
		let mut old = self.current_sequence_number.load(Ordering::Relaxed);
		
		loop {
			let max = if sequence_number_newer(new, old) {
				new
			} else {
				old
			};
			
			match current.compare_exchange_weak(old, max, Ordering::Relaxed, Ordering::Relaxed) {
				Ok(_) => break,
				Err(i) => old = i,
			}
		}
	}
	
	fn handle_packet(&self, packet: &AODVPacket) -> Result<(), io::Error> {
		use AODVPacketBody::*;
		
		let sender = packet.sender;
		
		match &packet.body {
			RouteRequest(packet) => self.handle_route_request(sender, packet)?,
			RouteReply(packet) => self.handle_route_reply(sender, packet)?,
			RouteError(packet) => self.handle_route_error(sender, packet)?,
			Data(packet) => self.handle_data(packet)?,
		}
		
		Ok(())
	}
	
	fn handle_route_request(&self, sender: ATAddress, packet: &RouteRequestPacket) -> Result<(), io::Error> {
		if packet.origin == self.address {
			return Ok(());
		}
		
		let mut seen_requests = self.seen_requests.lock()
			.expect("should only be accessed from this thread");
		
		let is_new_request = seen_requests.insert((packet.origin, packet.id));
		
		if !is_new_request {
			return Ok(());
		}
		
		self.update_sequence_number(packet.origin_sequence);
		
		let mut routing_table = self.routing_table_write();
		let mut at_module = self.at_module_write();
		
		if let Some(new_route) = routing_table.add_route(packet.origin, packet.origin_sequence, sender, packet.hop_count + 1) {
			self.send_outbound_messages(&mut at_module, packet.origin, new_route)?;
		}
		
		if let Some(route) = routing_table.get_route(packet.destination, packet.destination_sequence) {
			let sequence = if packet.destination == self.address {
				self.current_sequence_number.fetch_add(1, Ordering::Relaxed)
			} else {
				route.destination_sequence
			};
			
			let reply = RouteReplyPacket {
				hop_count: route.hop_count,
				request_destination: packet.destination,
				request_destination_sequence: sequence,
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
		self.update_sequence_number(packet.request_destination_sequence);
		
		let mut routing_table = self.routing_table_write();
		
		if let Some(new_route) = routing_table.add_route(packet.request_destination, packet.request_destination_sequence, sender, packet.hop_count + 1) {
			let mut at_module = self.at_module_write();
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
		
		let mut at_module = self.at_module_write();
		
		let Some(route) = routing_table.get_route(request_origin, None) else {
			eprintln!("[WARNING] Received RouteReplyPacket for unknown request origin:\n{packet:#?}");
			
			let packet = RouteErrorPacket {
				destination: request_origin,
			};
			
			at_module.broadcast(&packet.to_bytes())?;
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
		let mut at_module = self.at_module_write();
		
		let Some(route) = routing_table.get_route(packet.destination, None) else {
			eprintln!("[WARNING] Received DataPacket for unknown destination:\n{packet:#?}");
			
			let packet = RouteErrorPacket {
				destination: packet.destination,
			};
			
			at_module.broadcast(&packet.to_bytes())?;
			return Ok(());
		};
		
		at_module.send(route.next_hop, &packet.to_bytes())?;
		
		Ok(())
	}
}

fn sequence_number_newer(new_sequence_number: u16, old_sequence_number: u16) -> bool {
	let idifference: i16 = new_sequence_number as i16 - old_sequence_number as i16;
	idifference > 0
}