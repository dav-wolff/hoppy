use std::{io, thread};
use std::io::{Read, Write};
use std::time::Duration;
use crate::command_parser::{Commands, CommandsError};
use CommandsError::*;
use address::Address;
use io::ErrorKind::TimedOut;

mod command_parser;
mod address;

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
	
	let mut state = State::default();
	
	for command_result in Commands::in_stream(reader) {
		let command = match command_result {
			Ok(command) => command,
			Err(IoError(TimedOut)) => continue,
			Err(LineTooLong | IncorrectLineEnding) => {
				port.write(b"AT,ERR:SYMBLE\r\n")
					.expect("couldn't write to port");
				continue;
			},
			Err(IoError(kind)) => panic!("io error occurred trying to read a command: {kind}"),
		};
		
		if let Err(err) = handle_command(&mut port, &mut state, command) {
			//panic!("io error occurred trying to handle a command: {err}");
			port.write(b"AT,ERR:PARA\r\n"); // TODO better error handling
		}
	}
}

fn handle_command(mut port: impl Read + Write, state: &mut State, command: Vec<u8>) -> Result<(), io::Error> {
	let reply = if command == b"AT" {
		b"AT,OK\r\n".to_vec()
	} else if command.starts_with(b"AT+SEND=") {
		handle_send(&mut port, &command[8..])?.to_owned()
	} else if command.starts_with(b"AT+ADDR=") {
		set_address(state, &command[8..])?.to_owned()
	} else if command.starts_with(b"AT+ADDR?") {
		get_address(state)?
	} else if command.starts_with(b"AT+DEST=") {
		set_destination(state, &command[8..])?.to_owned()
	} else if command.starts_with(b"AT+DEST?") {
		get_destination(state)?
	} else {
		b"AT,ERR:CMD\r\n".to_vec()
	};

	port.write(&reply)?;
	
	Ok(())
}

#[derive(Default)]
struct State {
	address: Address,
	destination: Address,
}

fn handle_send(mut port: impl Read + Write, args: &[u8]) -> Result<&'static [u8], io::Error> {
	let Ok(bytes_to_receive) = String::from_utf8_lossy(args).parse::<usize>() else {
		return Ok(b"AT,ERR:PARA\r\n");
	};
	
	if !(1..250).contains(&bytes_to_receive) {
		return Ok(b"AT,ERR:PARA\r\n");
	}
	
	port.write(b"AT,OK\r\n")?;
	
	let mut buffer: Vec<u8> = vec![0; bytes_to_receive];
	let mut available_buffer = buffer.as_mut_slice();
	
	loop {
		let length = match port.read(available_buffer) {
			Ok(length) => length,
			Err(err) => match err.kind() {
				TimedOut => continue,
				_ => return Err(err),
			},
		};
		
		available_buffer = &mut available_buffer[length..];
		
		if available_buffer.is_empty() {
			break;
		}
	}
	
	println!("Received data: {:?}", String::from_utf8_lossy(&buffer));
	
	port.write(b"AT,SENDING\r\n")?;
	thread::sleep(Duration::from_secs(1));
	Ok(b"AT,SENDED\r\n")
}

fn set_address(state: &mut State, args: &[u8]) -> Result<&'static [u8], io::Error> {
	state.address = Address::from_ascii(args)?;
	Ok(b"AT,OK\r\n")
}

fn get_address(state: &State) -> Result<Vec<u8>, io::Error> {
	let mut reply = Vec::with_capacity(12);
	reply.extend_from_slice(b"AT,");
	reply.extend_from_slice(state.address.as_ascii_bytes());
	reply.extend_from_slice(b",OK\r\n");
	
	Ok(reply)
}

fn set_destination(state: &mut State, args: &[u8]) -> Result<&'static [u8], io::Error> {
	state.destination = Address::from_ascii(args)?;
	Ok(b"AT,OK\r\n")
}

fn get_destination(state: &State) -> Result<Vec<u8>, io::Error> {
	let mut reply = Vec::with_capacity(12);
	reply.extend_from_slice(b"AT,");
	reply.extend_from_slice(state.destination.as_ascii_bytes());
	reply.extend_from_slice(b",OK\r\n");
	
	Ok(reply)
}