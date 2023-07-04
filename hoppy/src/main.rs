use std::{time::Duration, thread};
use aodv::AODVController;
use at_module::{ATModule, at_address::ATAddress, ATConfig, HeaderMode, ReceiveMode};

mod hex;
mod no_timeout_reader;
mod at_module;
mod aodv;

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
		let module_builder = ATModule::builder(s, port, address, config);
		
		let mut controller = AODVController::start(module_builder)
			.expect("failed to start aodv controller");
		
		controller.send(ATAddress::new(*b"1234").unwrap(), b"Test data".to_owned().into())
			.expect("could not send test message");
	});
}