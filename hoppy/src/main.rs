use std::time::Duration;
use at_module::ATModule;

mod at_module;

const BAUD_RATE: u32 = 9600;

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let path = args.next()
		.expect("no path provided");

	let port = serialport::new(path, BAUD_RATE)
		.timeout(Duration::from_secs(1))
		.open()
		.expect("could not open serial port");
	
	let mut module = ATModule::new(port);

	module.send(b"Holle world!")
		.expect("could not send message");
}