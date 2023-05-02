use std::str::from_utf8;
use std::{io, thread};
use std::io::Write;
use std::time::Duration;
use read_buffer::ReadBuffer;

const BAUD_RATE: u32 = 9600;

enum Mode {
	Send,
	Receive,
	List,
	Conversation,
}

fn main() {
	let mut args = std::env::args();
	args.next(); // ignore first arg, which should be the executable's name
	
	let mode = match args.next().expect("no mode provided").as_str() {
		"send" => Mode::Send,
		"recv" => Mode::Receive,
		"conv" => Mode::Conversation,
		"list" => Mode::List,
		_ => panic!("unknown mode"),
	};
	
	let path = args.next().unwrap_or_default();
	
	match mode {
		Mode::List => list(),
		Mode::Send => send(path.as_str()),
		Mode::Receive => receive(path.as_str()),
		Mode::Conversation => conversation(path.as_str()),
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
	
	let text = from_utf8(data)
		.expect("received invalid utf-8 over serial port");
	
	println!("{text}");
}

fn conversation(path: &str) {
	let mut port = serialport::new(path, BAUD_RATE)
		.timeout(Duration::from_secs(10))
		.open()
		.expect("couldn't open serial port");
	
	let mut buffer: ReadBuffer<256> = ReadBuffer::new();
	
	let mut stdin_lines = io::stdin().lines();
	
	loop {
		let line = stdin_lines.next()
			.expect("couldn't read from stdin")
			.expect("couldn't read from stdin");
		
		port.write(line.as_bytes())
			.expect("couldn't write to port");
		port.write(b"\r\n")
			.expect("couldn't write to port");
		println!("> {line}");
		
		let reply = buffer.read_while(&mut port, |chunk| {
			!chunk.contains(&b'\n')
		}).expect("couldn't read from port");
		
		let reply_text = from_utf8(reply)
			.expect("received invalid utf-8 reply");
		
		let reply_text = &reply_text[..reply_text.len() - 2]; // cut off `\r\n`
		println!("< {reply_text}");
		
		thread::sleep(Duration::from_secs(2));
	}
}