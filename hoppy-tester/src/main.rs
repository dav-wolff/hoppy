use std::io::{Read, Write};
use std::time::Duration;
use crate::command_parser::Commands;

mod command_parser;

const BAUD_RATE: u32 = 9600;

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let Some(path) = args.next() else {
		eprintln!("Usage: hoppy-tester <path-to-serial-port>");
		return;
	};
	
	let port = serialport::new(path.clone(), BAUD_RATE)
		.timeout(Duration::from_secs(10))
		.open();
	
	let mut port = match port {
		Ok(port) => port,
		Err(err) => {
			eprintln!("Couldn't open `{path}`: {err}");
			return;
		}
	};
	
	let reader = port.try_clone()
		.expect("couldn't clone serial port");
	
	for command_result in Commands::in_stream(reader) {
		let command = match command_result {
			Ok(command) => command,
			Err(err) => {
				println!("{:?}", err);
				return;
			},
		};
		
		if command == b"AT" {
			port.write(b"AT,OK\r\n").unwrap();
		} else {
			port.write(b"AT,ERR:CMD\r\n").unwrap();
		}
	}
}