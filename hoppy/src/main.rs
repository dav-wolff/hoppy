use std::{time::Duration, thread};
use aodv::AODVController;
use at_module::{ATModule, at_address::ATAddress, ATConfig, HeaderMode, ReceiveMode};

mod hex;
mod no_timeout_reader;
mod at_module;
mod aodv;

const BAUD_RATE: u32 = 9600;
const HELLO_INTERVAL: Duration = Duration::from_secs(10);
const HELLO_TIMEOUT: Duration = Duration::from_secs(25);

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let address = args.next()
		.expect("no address provided");
	let address = ATAddress::new(
		address.as_bytes()
			.try_into()
			.expect("address in invalid format")
	).expect("address in invalid format");
	
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
	
	thread::scope(|scope| {
		let at_module_builder = ATModule::open(scope, port, address, config)
			.expect("failed to open at module");
		
		let controller = AODVController::start(scope, at_module_builder, HELLO_INTERVAL, HELLO_TIMEOUT, |address, data| {
			let text = String::from_utf8_lossy(data);
			println!("[DATA] {address}: {text}");
		});
		
		controller.send(ATAddress::new(*b"1234").unwrap(), b"Test data".to_owned().into())
			.expect("could not send test message");
	});
}