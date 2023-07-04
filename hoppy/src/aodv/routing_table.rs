use std::collections::BTreeMap;

use crate::at_module::at_address::ATAddress;

struct Entry {
	destination_sequence: u16,
	next_hop: ATAddress,
	hop_count: u8,
	precursors: Vec<ATAddress>,
}

pub struct RoutingTable {
	entries: BTreeMap<ATAddress, Entry>,
}

impl RoutingTable {
	pub fn new() -> Self {
		// TODO remove test data
		let mut entries = BTreeMap::new();
		entries.insert(ATAddress::new(*b"1234").unwrap(), Entry {
			destination_sequence: 0,
			next_hop: ATAddress::new(*b"ABCD").unwrap(),
			hop_count: 2,
			precursors: Vec::new(),
		});
		
		Self {
			entries,
		}
	}
	
	pub fn get_route(&self, destination: ATAddress) -> Option<ATAddress> {
		self.entries.get(&destination)
			.map(|entry| entry.next_hop)
	}
}