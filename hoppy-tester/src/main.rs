use std::io;
use std::io::{Read, Write};
use std::time::Duration;
use crate::command_parser::{Commands, CommandsError};

mod command_parser;

const BAUD_RATE: u32 = 9600;

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let Some(path) = args.next() else {
		eprintln!("Usage: hoppy-tester <path-to-serial-port>");
		return;
	};
	
	// low timeout is necessary on windows because read only returns when the timeout runs out
	let port = serialport::new(path.clone(), BAUD_RATE)
		.timeout(Duration::from_secs(1))
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
		use CommandsError::IoError;
		use io::ErrorKind::TimedOut;
		
		let command = match command_result {
			Ok(command) => command,
			Err(IoError(TimedOut)) => continue,
			Err(err) => {
				todo!();
			},
		};
		
		if let Err(err) = handle_command(&mut port, command) {
			todo!();
		}
	}
}

fn handle_command(mut port: impl Read + Write, command: Vec<u8>) -> Result<(), io::Error> {
	if command == b"AT" {
		port.write(b"AT,OK\r\n")?;
	} else {
		port.write(b"AT,ERR:CMD\r\n")?;
	}
	
	Ok(())
}