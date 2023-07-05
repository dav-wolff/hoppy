use std::{collections::BTreeMap, fmt::Display};

use crate::at_module::at_address::ATAddress;

#[derive(Debug, Clone, Copy)]
pub struct Route {
	pub destination_sequence: u16,
	pub next_hop: ATAddress,
	pub hop_count: u8,
}

pub struct RoutingTable {
	entries: BTreeMap<ATAddress, Route>,
}

impl RoutingTable {
	pub fn new(own_address: ATAddress) -> Self {
		let mut entries = BTreeMap::new();
		entries.insert(own_address, Route {
			destination_sequence: 0, // TODO figure out destination sequence
			next_hop: own_address,
			hop_count: 0,
		});
		
		// TODO remove test data
		entries.insert(ATAddress::new(*b"1234").unwrap(), Route {
			destination_sequence: 0,
			next_hop: ATAddress::new(*b"ABCD").unwrap(),
			hop_count: 2,
		});
		
		Self {
			entries,
		}
	}
	
	pub fn get_route(&self, destination: ATAddress) -> Option<Route> {
		self.entries.get(&destination)
			.copied()
	}
	
	pub fn add_route(&mut self, destination: ATAddress, destination_sequence: u16, next_hop: ATAddress, hop_count: u8) {
		let entry = self.entries.get(&destination);
		
		if let Some(route) = entry {
			if route.hop_count <= hop_count {
				return;
			}
		}
		
		self.entries.insert(destination, Route {
			destination_sequence,
			next_hop,
			hop_count,
		});
		
		println!("[INFO] Routing table updated:\n{self}");
	}
}

impl Display for RoutingTable {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		writeln!(f, "+----+----+----+----+")?;
		writeln!(f, "|DEST|DSEQ|NHOP|HCNT|")?;
		writeln!(f, "+----+----+----+----+")?;
		
		for (destination, Route { destination_sequence, next_hop, hop_count }) in &self.entries {
			writeln!(f, "|{destination}|{destination_sequence:04X}|{next_hop}|  {hop_count:02X}|")?;
		}
		
		write!(f, "+----+----+----+----+")
	}
}