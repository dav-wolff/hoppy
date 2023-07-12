use std::{io, thread};
use std::io::{Read, Write};
use std::time::Duration;
use crate::command_parser::{Commands, CommandParseError};
use CommandParseError::*;
use serialport::SerialPort;
use super::address::Address;
use io::ErrorKind::TimedOut;

const BAUD_RATE: u32 = 9600;

pub fn mock(path: &str) {
	// low timeout is necessary on windows because read only returns when the timeout runs out
	let port = serialport::new(path, BAUD_RATE)
		.timeout(Duration::from_secs(1))
		.open();
	
	let port = match port {
		Ok(port) => port,
		Err(err) => {
			eprintln!("Couldn't open `{path}`: {err}");
			return;
		}
	};
	
	let writer = port.try_clone()
		.expect("couldn't clone serial port");
	
	thread::scope(|s| {
		s.spawn(|| mock_send(writer));
		mock_receive(port);
		
		// easier than getting the thread to quit
		std::process::exit(0);
	});
}

fn mock_receive(mut writer: impl Write) {
	let mut stdin_lines = io::stdin().lines();
	
	loop {
		let line = stdin_lines.next()
			.expect("couldn't read from stdin")
			.expect("couldn't read from stdin");
		
		if line == "\\exit" {
			break;
		}
		
		bytes_received(&mut writer, line.as_bytes());
	}
}

fn bytes_received(mut writer: impl Write, bytes: &[u8]) {
	let length = bytes.len();
	
	// source address hard-coded to '1234' for now
	write!(writer, "LR,1234,{length:02X},")
		.expect("couldn't write to port");
	writer.write_all(bytes)
		.expect("couldn't write to port");
	write!(writer, "\r\n")
		.expect("couldn't write to port");
}

fn mock_send(mut port: Box<dyn SerialPort>) {
	let reader = port.try_clone()
		.expect("couldn't clone serial port");
	
	let mut state = State::default();
	
	for command_result in Commands::in_stream(reader) {
		let command = match command_result {
			Ok(command) => command,
			Err(IoError(TimedOut)) => continue,
			Err(LineTooLong | IncorrectLineEnding) => {
				port.write_all(b"AT,ERR:SYMBLE\r\n")
					.expect("couldn't write to port");
				continue;
			},
			Err(IoError(kind)) => panic!("io error occurred trying to read a command: {kind}"),
		};
		
		match handle_command(&mut port, &mut state, command) {
			Err(CommandError::IoError(err)) => {
				panic!("io error occurred trying to handle a command: {err}");
			},
			Err(CommandError::IncorrectParameter) => {
				port.write_all(b"AT,ERR:PARA\r\n")
					.expect("couldn't write to port");
			},
			Ok(()) => (),
		};
	}
}

enum CommandError {
	IoError(io::Error),
	IncorrectParameter,
}

impl From<io::Error> for CommandError {
	fn from(err: io::Error) -> Self {
		CommandError::IoError(err)
	}
}

fn handle_command(mut port: impl Read + Write, state: &mut State, command: Vec<u8>) -> Result<(), CommandError> {
	let reply = if command == b"AT" {
		b"AT,OK\r\n".to_vec()
	} else if command.starts_with(b"AT+SEND=") {
		handle_send(&mut port, &state, &command[8..])?.to_owned()
	} else if command.starts_with(b"AT+ADDR=") {
		set_address(state, &command[8..])?.to_owned()
	} else if command.starts_with(b"AT+ADDR?") {
		get_address(state)?
	} else if command.starts_with(b"AT+DEST=") {
		set_destination(state, &command[8..])?.to_owned()
	} else if command.starts_with(b"AT+DEST?") {
		get_destination(state)?
	} else if command.starts_with(b"AT+CFG=") {
		set_config(state, &command[7..])?.to_owned()
	} else {
		b"AT,ERR:CMD\r\n".to_vec()
	};
	
	port.write_all(&reply)?;
	
	Ok(())
}

#[derive(Default)]
struct State {
	address: Address,
	destination: Address,
	config: Config,
}

#[derive(Default, Debug)]
#[allow(dead_code)] // values are only logged using Debug
struct Config {
	frequency: u32,
	power: u32,
	bandwidth: u32,
	spreading_factor: u32,
	error_coding: u32,
	crc: u32,
	header_mode: u32,
	receive_mode: u32,
	frequency_hop: u32,
	hop_period: u32,
	receive_timeout: u32,
	payload_length: u32,
	preamble_length: u32,
}

fn handle_send(mut port: impl Read + Write, state: &State, args: &[u8]) -> Result<&'static [u8], CommandError> {
	let Ok(bytes_to_receive) = String::from_utf8_lossy(args).parse::<usize>() else {
		return Err(CommandError::IncorrectParameter);
	};
	
	if !(1..250).contains(&bytes_to_receive) {
		return Err(CommandError::IncorrectParameter);
	}
	
	port.write_all(b"AT,OK\r\n")?;
	
	let mut buffer: Vec<u8> = vec![0; bytes_to_receive];
	let mut available_buffer = buffer.as_mut_slice();
	
	loop {
		let length = match port.read(available_buffer) {
			Ok(length) => length,
			Err(err) => match err.kind() {
				TimedOut => continue,
				_ => Err(err)?,
			},
		};
		
		available_buffer = &mut available_buffer[length..];
		
		if available_buffer.is_empty() {
			break;
		}
	}
	
	println!(
		"Sending {:?} from {} to {}",
		String::from_utf8_lossy(&buffer),
		String::from_utf8_lossy(state.address.as_ascii_bytes()),
		String::from_utf8_lossy(state.destination.as_ascii_bytes())
	);
	
	port.write_all(b"AT,SENDING\r\n")?;
	thread::sleep(Duration::from_secs(1));
	Ok(b"AT,SENDED\r\n")
}

fn set_address(state: &mut State, args: &[u8]) -> Result<&'static [u8], CommandError> {
	state.address = Address::from_ascii(args)?;
	Ok(b"AT,OK\r\n")
}

fn get_address(state: &State) -> Result<Vec<u8>, CommandError> {
	let mut reply = Vec::with_capacity(12);
	reply.extend_from_slice(b"AT,");
	reply.extend_from_slice(state.address.as_ascii_bytes());
	reply.extend_from_slice(b",OK\r\n");
	
	Ok(reply)
}

fn set_destination(state: &mut State, args: &[u8]) -> Result<&'static [u8], CommandError> {
	state.destination = Address::from_ascii(args)?;
	Ok(b"AT,OK\r\n")
}

fn get_destination(state: &State) -> Result<Vec<u8>, CommandError> {
	let mut reply = Vec::with_capacity(12);
	reply.extend_from_slice(b"AT,");
	reply.extend_from_slice(state.destination.as_ascii_bytes());
	reply.extend_from_slice(b",OK\r\n");
	
	Ok(reply)
}

fn set_config(state: &mut State, args: &[u8]) -> Result<&'static [u8], CommandError> {
	let mut iter = args.split(|char| *char == b',');

	state.config = Config {
    	frequency: parse_int(iter.next())?,
    	power: parse_int(iter.next())?,
    	bandwidth: parse_int(iter.next())?,
    	spreading_factor: parse_int(iter.next())?,
    	error_coding: parse_int(iter.next())?,
    	crc: parse_int(iter.next())?,
    	header_mode: parse_int(iter.next())?,
    	receive_mode: parse_int(iter.next())?,
    	frequency_hop: parse_int(iter.next())?,
    	hop_period: parse_int(iter.next())?,
    	receive_timeout: parse_int(iter.next())?,
    	payload_length: parse_int(iter.next())?,
    	preamble_length: parse_int(iter.next())?,
	};
	
	if iter.next().is_some() {
		return Err(CommandError::IncorrectParameter);
	}
	
	println!("Set config to: {:#?}", state.config);
	
	Ok(b"AT,OK\r\n")
}

fn parse_int(bytes: Option<&[u8]>) -> Result<u32, CommandError> {
	let bytes = bytes.ok_or(CommandError::IncorrectParameter)?;
	let string = std::str::from_utf8(bytes)
		.map_err(|_| CommandError::IncorrectParameter)?;
	string.parse()
		.map_err(|_| CommandError::IncorrectParameter)
}