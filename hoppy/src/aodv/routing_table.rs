use std::{collections::BTreeMap, fmt::Display, time::Instant};

use crate::at_module::at_address::ATAddress;

#[derive(Debug, Clone, Copy)]
pub struct Route {
	pub destination_sequence: u16,
	pub next_hop: ATAddress,
	pub hop_count: u8,
	pub time_added: Instant,
}

pub struct RoutingTable {
	entries: BTreeMap<ATAddress, Route>,
	own_address: ATAddress,
}

impl RoutingTable {
	pub fn new(own_address: ATAddress) -> Self {
		let mut entries = BTreeMap::new();
		entries.insert(own_address, Route {
			destination_sequence: 0, // TODO figure out destination sequence
			next_hop: own_address,
			hop_count: 0,
			time_added: Instant::now(),
		});
		
		let routing_table = Self {
			entries,
			own_address,
		};
		
		println!("[INFO] Routing table updated:\n{routing_table}");
		
		routing_table
	}
	
	pub fn get_route(&self, destination: ATAddress) -> Option<Route> {
		self.entries.get(&destination)
			.copied()
	}
	
	pub fn add_route(&mut self, destination: ATAddress, destination_sequence: u16, next_hop: ATAddress, hop_count: u8) -> Option<Route> {
		if let Some(route) = self.entries.get(&destination) {
			if route.hop_count <= hop_count {
				return None;
			}
		}
		
		self.entries.insert(destination, Route {
			destination_sequence,
			next_hop,
			hop_count,
			time_added: Instant::now(),
		});
		
		println!("[INFO] Routing table updated:\n{self}");
		
		let route = self.entries.get(&destination)
			.expect("route was just inserted");
		
		Some(*route)
	}
	
	pub fn remove_route(&mut self, destination: ATAddress, next_hop: ATAddress) -> bool {
		if let Some(route) = self.entries.get(&destination) {
			if route.next_hop == next_hop {
				self.entries.remove(&destination);
				println!("[INFO] Routing table updated:\n{self}");
				return true;
			}
		}
		
		false
	}
	
	pub fn neighbors(&self) -> impl Iterator<Item = Route> + '_ {
		self.entries.iter()
			.filter(|(destination, route)| route.next_hop == **destination && **destination != self.own_address)
			.map(|(_, route)| route)
			.copied()
	}
}

impl Display for RoutingTable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "+----+----+----+----+")?;
		writeln!(f, "|DEST|DSEQ|NHOP|HCNT|")?;
		writeln!(f, "+----+----+----+----+")?;
		
		for (destination, Route { destination_sequence, next_hop, hop_count, .. }) in &self.entries {
			writeln!(f, "|{destination}|{destination_sequence:04X}|{next_hop}|  {hop_count:02X}|")?;
		}
		
		write!(f, "+----+----+----+----+")
	}
}