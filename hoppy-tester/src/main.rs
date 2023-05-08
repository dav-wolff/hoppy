use std::{io, thread};
use std::io::{Read, Write};
use std::time::Duration;
use crate::command_parser::{Commands, CommandsError};
use CommandsError::*;
use io::ErrorKind::TimedOut;

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
		let command = match command_result {
			Ok(command) => command,
			Err(IoError(TimedOut)) => continue,
			Err(LineTooLong | IncorrectLineEnding) => {
				port.write(b"AT,ERR:SYMBLE")
					.expect("couldn't write to port");
				continue;
			},
			Err(IoError(kind)) => panic!("io error occurred trying to read a command: {kind}"),
		};
		
		if let Err(err) = handle_command(&mut port, command) {
			panic!("io error occurred trying to handle a command: {err}");
		}
	}
}

fn handle_command(mut port: impl Read + Write, command: Vec<u8>) -> Result<(), io::Error> {
	if command == b"AT" {
		port.write(b"AT,OK\r\n")?;
	} else if command.starts_with(b"AT+SEND=") {
		handle_send(port, &command[8..])?;
	} else {
		port.write(b"AT,ERR:CMD\r\n")?;
	}
	
	Ok(())
}

fn handle_send(mut port: impl Read + Write, args: &[u8]) -> Result<(), io::Error> {
	let Ok(bytes_to_receive) = String::from_utf8_lossy(args).parse::<usize>() else {
		port.write(b"AT,ERR:PARA\r\n")?;
		return Ok(());
	};
	
	if !(1..250).contains(&bytes_to_receive) {
		port.write(b"AT,ERR:PARA\r\n")?;
		return Ok(());
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
	port.write(b"AT,SENDED\r\n")?;
	
	Ok(())
}