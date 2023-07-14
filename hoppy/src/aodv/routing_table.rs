use std::{collections::BTreeMap, fmt::Display, time::Instant};

use crate::{at_module::at_address::ATAddress, aodv::sequence_number_newer};

#[derive(Debug, Clone, Copy)]
enum Entry {
	Route(Route),
	UnreachableDestination {
		destination_sequence: u16,
	}
}

#[derive(Debug, Clone, Copy)]
pub struct Route {
	pub destination_sequence: u16,
	pub next_hop: ATAddress,
	pub hop_count: u8,
	pub last_seen: Instant,
}

pub struct RoutingTable {
	entries: BTreeMap<ATAddress, Entry>,
	own_address: ATAddress,
}

impl RoutingTable {
	pub fn new(own_address: ATAddress) -> Self {
		let mut entries = BTreeMap::new();
		entries.insert(own_address, Entry::Route(Route {
			destination_sequence: 0,
			next_hop: own_address,
			hop_count: 0,
			last_seen: Instant::now(),
		}));
		
		let routing_table = Self {
			entries,
			own_address,
		};
		
		println!("[INFO] Routing table updated:\n{routing_table}");
		
		routing_table
	}
	
	pub fn get_route(&self, destination: ATAddress) -> Option<Route> {
		self.entries.get(&destination)
			.map(|entry| match entry {
				Entry::Route(route) => Some(route),
				_ => None,
			})
			.flatten()
			.copied()
	}
	
	pub fn get_last_known_sequence(&self, destination: ATAddress) -> Option<u16> {
		self.entries.get(&destination)
			.map(|entry| match entry {
				Entry::Route(Route { destination_sequence, .. }) => destination_sequence,
				Entry::UnreachableDestination { destination_sequence } => destination_sequence,
			})
			.copied()
	}
	
	pub fn add_route(&mut self, destination: ATAddress, destination_sequence: u16, next_hop: ATAddress, hop_count: u8) -> Option<Route> {
		if destination == self.own_address {
			return None;
		}
		
		if let Some(Entry::Route(route)) = self.entries.get(&destination) {
			if !sequence_number_newer(destination_sequence, route.destination_sequence) {
				return None;
			}
		}
		
		let new_route = Route {
			destination_sequence,
			next_hop,
			hop_count,
			last_seen: Instant::now(),
		};
		
		let old_route = self.entries.insert(destination, Entry::Route(new_route));
		
		if let Some(Entry::Route(old_route)) = old_route {
			if old_route.destination_sequence == destination_sequence &&
				old_route.next_hop == next_hop &&
				old_route.hop_count == hop_count
			{
				// route was not updated, only last_seen
				return None;
			}
		}
		
		println!("[INFO] Routing table updated:\n{self}");
		
		Some(new_route)
	}
	
	pub fn remove_route(&mut self, destination: ATAddress, next_hop: ATAddress) -> bool {
		let Some(entry) = self.entries.get_mut(&destination) else {
			return false;
		};
		
		let Entry::Route(route) = entry else {
			return false;
		};
		
		if route.next_hop != next_hop {
			return false;
		}
		
		let unreachable_destination = Entry::UnreachableDestination {
			destination_sequence: route.destination_sequence,
		};
		
		*entry = unreachable_destination;
		
		println!("[INFO] Routing table updated:\n{self}");
		return true;
	}
	
	pub fn neighbors(&self) -> impl Iterator<Item = Route> + '_ {
		self.entries.iter()
			.filter_map(|(&destination, &entry)| match entry {
				Entry::Route(route) => Some((destination, route)),
				_ => None,
			})
			.filter(|(destination, route)| route.next_hop == *destination && *destination != self.own_address)
			.map(|(_, route)| route)
	}
	
	pub fn routes_with_next_hop(&self, next_hop: ATAddress) -> impl Iterator<Item = (ATAddress, Route)> + '_ {
		self.entries.iter()
			.filter_map(|(&destination, &entry)| match entry {
				Entry::Route(route) => Some((destination, route)),
				_ => None,
			})
			.filter(move |(_, route)| route.next_hop == next_hop)
	}
}

impl Display for RoutingTable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "+----+----+----+----+")?;
		writeln!(f, "|DEST|DSEQ|NHOP|HCNT|")?;
		writeln!(f, "+----+----+----+----+")?;
		
		for (destination, entry) in &self.entries {
			match entry {
				Entry::Route(Route { destination_sequence, next_hop, hop_count, .. }) => {
					writeln!(f, "|{destination}|{destination_sequence:04X}|{next_hop}|  {hop_count:02X}|")?;
				},
				Entry::UnreachableDestination { destination_sequence } => {
					writeln!(f, "|{destination}|{destination_sequence:04X}|None|None|")?;
				},
			}
		}
		
		write!(f, "+----+----+----+----+")
	}
}