use std::{time::Duration, thread, sync::mpsc};
use at_config::{ATConfig, HeaderMode, ReceiveMode};
use at_module::{ATModule, at_address::ATAddress};

use crate::aodv::parse_packet;

mod hex;
mod no_timeout_reader;
mod at_config;
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
	
	let (packet_sender, packet_receiver) = mpsc::channel();
	
	thread::scope(|s| {
		let mut module = ATModule::open(s, port, address, config, move |message| {
			let address = message.address;
			let text = String::from_utf8_lossy(&message.data);
			println!("Received message from {address}: {text}");
			
			let packet = parse_packet(message);
			println!("Packet: {packet:#?}");
			
			let Ok(packet) = packet else {
				return;
			};
			
			packet_sender.send(packet.to_bytes())
				.expect("channel closed");
		}).expect("could not open AT module");
		
		module.send(ATAddress::new(*b"1234").unwrap(), b"Holle world!")
			.expect("could not send message");
		
		for packet in packet_receiver {
			module.send(ATAddress::new(*b"ABCD").unwrap(), &packet)
				.expect("could not send packet");
		}
	});
}