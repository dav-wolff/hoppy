use std::{io, thread};
use std::io::{Read, Write};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use read_buffer::ReadBuffer;
use mock::mock;

mod command_parser;
mod address;
mod mock;

const BAUD_RATE: u32 = 9600;

enum Mode {
	Mock,
	Send,
	Receive,
	List,
	Conversation,
}

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let mode = match args.next().expect("no mode provided").as_str() {
		"mock" => Mode::Mock,
		"send" => Mode::Send,
		"recv" => Mode::Receive,
		"conv" => Mode::Conversation,
		"list" => Mode::List,
		_ => panic!("unknown mode"),
	};
	
	let path = args.next().unwrap_or_default();
	
	match mode {
		Mode::Mock => mock(&path),
		Mode::List => list(),
		Mode::Send => send(&path),
		Mode::Receive => receive(&path),
		Mode::Conversation => conversation(&path),
	}
}

fn list() {
	let available_ports = serialport::available_ports()
		.expect("couldn't list available ports");
	
	for port in available_ports {
		let name = port.port_name;
		let port_type = port.port_type;
		
		println!("{name}: {:?}", port_type);
	}
}

fn send(path: &str) {
	let mut port = serialport::new(path, BAUD_RATE)
		.open()
		.expect("couldn't open serial port");
	
	port.write("Hello world".as_bytes())
		.expect("couldn't write to port");
}

fn receive(path: &str) {
	let mut port = serialport::new(path, BAUD_RATE)
		.timeout(Duration::from_secs(10))
		.open()
		.expect("couldn't open serial port");
	
	let mut buffer: ReadBuffer<256> = ReadBuffer::new();
	let data = buffer.read_from(&mut port)
		.expect("couldn't read from serial port");
	
	let text = String::from_utf8_lossy(data);
	
	println!("{text}");
}

fn conversation(path: &str) {
	let port = serialport::new(path, BAUD_RATE)
		.timeout(Duration::from_secs(1))
		.open()
		.expect("couldn't open serial port");
	
	let (tx, rx) = mpsc::channel();
	
	let reader = port.try_clone()
		.expect("couldn't clone serial port");
	
	thread::scope(|s| {
		s.spawn(|| listen_for_replies(reader, tx));
		send_requests(port, rx);
		
		// easier than getting the thread to quit
		std::process::exit(1);
	});
}

fn send_requests(mut writer: impl Write, rx: Receiver<String>) {
	let mut stdin_lines = io::stdin().lines();
	
	loop {
		print!("> ");
		let _ = io::stdout().flush();
		
		let line = stdin_lines.next()
			.expect("couldn't read from stdin")
			.expect("couldn't read from stdin");
		
		if line == "\\exit" {
			break;
		}
		
		writer.write(line.as_bytes())
			.expect("couldn't write to port");
		writer.write(b"\r\n")
			.expect("couldn't write to port");
		
		loop {
			let Ok(reply_text) = rx.recv_timeout(Duration::from_secs(2)) else {
				// timeout
				break;
			};
			
			println!("< {reply_text}");
		}
	}
}

fn listen_for_replies(mut reader: impl Read, tx: Sender<String>) {
	let mut buffer: ReadBuffer<256> = ReadBuffer::new();
	
	loop {
		let reply = buffer.read_while(&mut reader, |chunk| {
			!chunk.contains(&b'\n')
		});
		
		let reply = match reply {
			Ok(reply) => reply,
			Err(err) => match err.kind() {
				io::ErrorKind::TimedOut => continue,
				_ => panic!("error reading from port: {err}"),
			}
		};
		
		let reply_text = String::from_utf8_lossy(reply);
		
		for line in reply_text.lines() {
			tx.send(line.to_owned())
				.expect("could not send reply between threads");
		}
	}
}