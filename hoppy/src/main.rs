use std::{time::Duration, thread};
use at_config::{ATConfig, HeaderMode, ReceiveMode};
use at_module::{ATModule, at_address::ATAddress};

mod hex_parse;
mod no_timeout_reader;
mod at_config;
mod at_module;

const BAUD_RATE: u32 = 9600;

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let path = args.next()
		.expect("no path provided");
	
	let port = serialport::new(path, BAUD_RATE)
		.timeout(Duration::from_secs(10))
		.open()
		.expect("could not open serial port");
	
	let config = ATConfig {
		frequency: 433920000,
		power: 5,
		bandwidth: 9,
		spreading_factor: 7,
		error_coding: 4,
		crc: true,
		header_mode: HeaderMode::Explicit,
		receive_mode: ReceiveMode::Continue,
		frequency_hop: false,
		hop_period: 0,
		receive_timeout: 3000,
		payload_length: 8,
		preamble_length: 8,
	};
	
	let address = ATAddress::new(*b"4290")
		.expect("address literal should be valid");
	
	thread::scope(|s| {
		let mut module = ATModule::open(s, port, address, config, |message| {
			let address = message.address;
			let text = String::from_utf8_lossy(&message.data);
			println!("Received message from {address}: {text}");
		}).expect("could not open AT module");
		
		module.send(ATAddress::new(*b"1234").unwrap(), b"Holle world!")
			.expect("could not send message");
	});
}